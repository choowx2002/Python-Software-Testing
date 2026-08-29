use crate::state::{process_alive, terminate_pid, AppState};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
// ============================================
// Command 1: Scan Test Files
// ============================================
#[tauri::command]
pub fn scan_test_files(project_path: String) -> Result<Vec<TestFile>, String> {
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    if !project.is_dir() {
        return Err(format!("Project path is not a directory: {}", project_path));
    }

    // 自愈：生成测试目录补 __init__.py，避免 pytest 同名模块冲突
    super::ensure_generated_test_packages(&project);

    let tests_dir = project.join("tests");

    if !tests_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();

    scan_test_directory(&tests_dir, &project, &mut files)?;

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(files)
}

fn scan_test_directory(
    directory: &Path,
    project_root: &Path,
    files: &mut Vec<TestFile>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|e| format!("Failed to read directory {:?}: {}", directory, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            scan_test_directory(&path, project_root, files)?;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };

        if !is_test_file(file_name) {
            continue;
        }

        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");

        files.push(TestFile {
            name: file_name.to_string(),
            path: path.to_string_lossy().to_string(),
            relative_path,
        });
    }

    Ok(())
}

#[tauri::command]
pub fn collect_test_cases(project_path: String) -> Result<Vec<TestCase>, String> {
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err(format!(
            "Project path does not exist: {}",
            project_path
        ));
    }

    if !project.is_dir() {
        return Err(format!(
            "Project path is not a directory: {}",
            project_path
        ));
    }

    // 自愈：生成测试目录补 __init__.py，避免 pytest 同名模块冲突
    super::ensure_generated_test_packages(&project);

    let python = find_project_python(&project)?;

    let output = Command::new(&python)
        .current_dir(&project)
        .env(
            "PYTHONPATH",
            build_python_path(&project)
        )
        .args([
            "-m",
            "pytest",
            "--collect-only",
            "-q",
        ])
        .output()
        .map_err(|error| {
            format!(
                "Failed to start pytest using {:?}: {}",
                python, error
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!(
            "pytest collection failed.\n\n{}{}",
            stdout,
            stderr
        ));
    }

    parse_pytest_collection(&stdout, &project)
}

fn find_project_python(project: &Path) -> Result<PathBuf, String> {
    let candidates = if cfg!(target_os = "windows") {
        vec![
            project.join(".venv").join("Scripts").join("python.exe"),
            project.join("venv").join("Scripts").join("python.exe"),
        ]
    } else {
        vec![
            project.join(".venv").join("bin").join("python"),
            project.join("venv").join("bin").join("python"),
        ]
    };

    for python in candidates {
        if python.is_file() {
            return Ok(python);
        }
    }

    Err(format!(
        "No project Python interpreter found in {}. \
Expected .venv or venv.",
        project.display()
    ))
}

fn parse_pytest_collection(
    stdout: &str,
    project: &Path,
) -> Result<Vec<TestCase>, String> {
    let mut cases = Vec::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();

        if line.is_empty() {
            continue;
        }

        // pytest -q collection output usually looks like:
        //
        // tests/test_user.py::test_create_user
        // tests/test_user.py::TestUser::test_update_user
        //
        // Ignore summary / warnings / collection messages.
        if !line.contains("::") {
            continue;
        }

        let id = line.to_string();

        let parts: Vec<&str> = line.split("::").collect();

        if parts.len() < 2 {
            continue;
        }

        let relative_file = parts[0];

        let file_path = project.join(relative_file);

        let file = normalize_relative_path(
            &file_path,
            project,
        )?;

        let test_name = parts.last().unwrap().to_string();

        let class_name = if parts.len() >= 3 {
            Some(parts[parts.len() - 2].to_string())
        } else {
            None
        };

        let line = find_test_line(&file_path, class_name.as_deref(), &test_name);

        cases.push(TestCase {
            id,
            name: test_name,
            file,
            class_name,
            line,
        });
    }

    Ok(cases)
}

fn normalize_relative_path(
    path: &Path,
    project: &Path,
) -> Result<String, String> {
    let relative = path
        .strip_prefix(project)
        .map_err(|error| error.to_string())?;

    Ok(relative
        .to_string_lossy()
        .replace('\\', "/"))
}

/// 在测试文件中定位 `def <test_name>` 的行号（1-based）。
/// 有类名时先定位 `class <class_name>` 再从其后查找，降低同名方法误判。
fn find_test_line(file_path: &Path, class_name: Option<&str>, test_name: &str) -> Option<u32> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    // 起始搜索下标：有类名时从类定义之后开始
    let mut start: usize = 0;
    if let Some(class) = class_name {
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("class ") {
                let name = rest
                    .split(|c: char| c == '(' || c == ':' || c.is_whitespace())
                    .next()
                    .unwrap_or("");
                if name == class {
                    start = i + 1;
                    break;
                }
            }
        }
    }

    for (i, line) in lines.iter().enumerate().skip(start) {
        let trimmed = line.trim_start();
        let def_rest = trimmed
            .strip_prefix("async def ")
            .or_else(|| trimmed.strip_prefix("def "));
        if let Some(def_rest) = def_rest {
            let name = def_rest
                .split(|c: char| c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("");
            if name == test_name {
                return Some((i + 1) as u32);
            }
        }
    }

    None
}


// TEST DELCARE
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestOutputEvent {
    pub run_id: String,
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStartedEvent {
    pub run_id: String,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub id: String,
    pub name: String,
    pub file: String,
    pub status: String,
    pub duration: f64,
    pub error_message: Option<String>,
    /// JUnit `<testcase line="...">` 中的行号（pytest junitxml 提供；缺失时为 None）
    pub line: Option<u32>,
    /// `<skipped message="...">` 中的跳过原因（pytest 可能不提供，此时为 None）
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFinishedEvent {
    pub run_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: f64,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub results: Vec<TestResult>,
    pub command: String,
    pub execution_type: String,
    pub regression_suite_id: Option<i64>,
}

#[tauri::command]
pub async fn run_tests(
    app_handle: AppHandle,
    project_id: i64,
    project_path: String,
    interpreter_path: String,
    test_cases: Vec<String>,
    pytest_args: Vec<String>,
    regression_suite_id: Option<i64>,
) -> Result<String, String> {
    run_tests_core(
        app_handle,
        project_id,
        project_path,
        interpreter_path,
        test_cases,
        pytest_args,
        regression_suite_id,
    )
    .await
}

#[allow(unused_variables)]
pub async fn run_tests_core(
    app_handle: AppHandle,
    project_id: i64,
    project_path: String,
    interpreter_path: String,
    test_cases: Vec<String>,
    pytest_args: Vec<String>,
    regression_suite_id: Option<i64>,
) -> Result<String, String> {
    let run_id = Uuid::new_v4().to_string();
    let execution_type = if regression_suite_id.is_some() {
        "REGRESSION"
    } else {
        "MANUAL"
    };
    let project = PathBuf::from(&project_path);
    if !project.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }
    if !project.is_dir() {
        return Err(format!("Project path is not a directory: {}", project_path));
    }

    let interpreter = PathBuf::from(&interpreter_path);
    if !interpreter.exists() {
        return Err(format!(
            "Python interpreter does not exist: {}",
            interpreter_path
        ));
    }

    let total = test_cases.len();
    let _ = app_handle.emit(
        "test-started",
        TestStartedEvent {
            run_id: run_id.clone(),
            total,
        },
    );

    let junit_path = std::env::temp_dir().join(format!("pytest-{}.xml", run_id));

    let mut args = vec!["-m".to_string(), "pytest".to_string(), "-v".to_string()];
    args.extend(pytest_args);
    if !test_cases.is_empty() {
        args.extend(test_cases);
    }
    // 默认 junit_family=xunit2 会过滤掉 <testcase> 的 file/line 属性，
    // 导致结果列表无法定位文件与行号（打开/预览/复制路径失效）。
    // xunit1 保留 file/line，解析器对两种 family 均兼容。
    args.push("-o".to_string());
    args.push("junit_family=xunit1".to_string());
    args.push(format!("--junitxml={}", junit_path.to_string_lossy()));

    let mut command_str = interpreter.display().to_string();
    for arg in &args {
        command_str.push(' ');
        command_str.push_str(arg);
    }

    let start = std::time::Instant::now();

    let mut child = match AsyncCommand::new(&interpreter)
        .current_dir(&project)
        .env(
            "PYTHONPATH",
            build_python_path(&project)
        )
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = app_handle.emit("test-finished", TestFinishedEvent {
                run_id: run_id.clone(),
                success: false,
                exit_code: None,
                duration: 0.0,
                passed: 0,
                failed: 0,
                skipped: 0,
                results: vec![],
                command: command_str.clone(),
                execution_type: execution_type.to_string(),
                regression_suite_id,
            });
            return Err(format!("Failed to start pytest: {}", e));
        }
    };

    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture pytest stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture pytest stderr".to_string())?;

    register_run(&app_handle, &run_id, child.id().unwrap_or(0));

    let stdout_handle = app_handle.clone();
    let stdout_run_id = run_id.clone();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stdout_handle.emit("test-output", TestOutputEvent {
                run_id: stdout_run_id.clone(),
                stream: "stdout".to_string(),
                line,
            });
        }
    });

    let stderr_handle = app_handle.clone();
    let stderr_run_id = run_id.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_handle.emit("test-output", TestOutputEvent {
                run_id: stderr_run_id.clone(),
                stream: "stderr".to_string(),
                line,
            });
        }
    });

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            unregister_run(&app_handle, &run_id);
            return Err(format!("Failed waiting for pytest: {}", e));
        }
    };
    unregister_run(&app_handle, &run_id);
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let duration = start.elapsed().as_secs_f64();

    let results = match parse_junit_results(&junit_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to parse JUnit XML: {}", e);
            vec![]
        }
    };

    let passed = results.iter().filter(|r| r.status == "passed").count();
    let failed = results.iter().filter(|r| r.status == "failed" || r.status == "error").count();
    // xfail 在 junit 中表现为 skipped 子类，汇总时并入 skipped，保持与 pytest 报告一致
    let skipped = results
        .iter()
        .filter(|r| r.status == "skipped" || r.status == "xfailed")
        .count();

    let _ = app_handle.emit(
        "test-finished",
        TestFinishedEvent {
            run_id: run_id.clone(),
            success: status.success(),
            exit_code: status.code(),
            duration,
            passed,
            failed,
            skipped,
            results,
            command: command_str,
            execution_type: execution_type.to_string(),
            regression_suite_id,
        },
    );

    let _ = std::fs::remove_file(junit_path);
    Ok(run_id)
}

// ============================================
// Command: Cancel Run
// ============================================

/// 取消一个正在运行的子进程（测试执行 / 覆盖率采集 / 测试生成）
///
/// 策略：先发送 SIGTERM 优雅终止，2 秒后仍存活则 SIGKILL 强制终止。
/// 被终止的进程由其所属命令的 wait() 感知到，随后照常发出对应的 *-finished 事件（success=false）。
#[tauri::command]
pub async fn cancel_run(state: tauri::State<'_, AppState>, run_id: String) -> Result<(), String> {
    if let Ok(mut cancelled) = state.cancelled_runs.lock() {
        cancelled.insert(run_id.clone(), true);
    }

    let pid = state
        .processes
        .lock()
        .map_err(|e| format!("Failed to lock process state: {}", e))?
        .get(&run_id)
        .copied();

    if let Some(pid) = pid {
        if process_alive(pid) {
            let _ = terminate_pid(pid, false);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if process_alive(pid) {
                let _ = terminate_pid(pid, true);
            }
        }
    }

    Ok(())
}

fn parse_junit_results(path: &Path) -> Result<Vec<TestResult>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read JUnit XML: {}", e))?;

    parse_junit_results_from_str(&content)
}

pub fn parse_junit_results_from_str(content: &str) -> Result<Vec<TestResult>, String> {
    let mut reader = quick_xml::Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut results = Vec::new();
    let mut current: Option<TestResult> = None;
    let mut current_failure: Option<String> = None;

    loop {
        match reader.read_event() {
            // ---- 自闭合: <testcase ... /> → 一定是 passed ----
            Ok(quick_xml::events::Event::Empty(ref event))
                if event.name().as_ref() == b"testcase" =>
            {
                let test = parse_testcase_attributes(event);
                results.push(test); // ✅ 直接入结果，无需等 End
            }

            // ---- 非自闭合: <testcase ...> → 可能有 failure/error/skipped ----
            Ok(quick_xml::events::Event::Start(ref event))
                if event.name().as_ref() == b"testcase" =>
            {
                current = Some(parse_testcase_attributes(event));
            }

            Ok(quick_xml::events::Event::Start(ref event))
                if event.name().as_ref() == b"failure" =>
            {
                if let Some(test) = current.as_mut() {
                    test.status = "failed".to_string();
                }
                current_failure = Some(String::new());
            }

            Ok(quick_xml::events::Event::Start(ref event))
                if event.name().as_ref() == b"error" =>
            {
                if let Some(test) = current.as_mut() {
                    test.status = "error".to_string();
                }
                current_failure = Some(String::new());
            }

            Ok(quick_xml::events::Event::Start(ref event))
                if event.name().as_ref() == b"skipped" =>
            {
                if let Some(test) = current.as_mut() {
                    apply_skipped_status(test, event);
                }
            }

            // ---- 自闭合空元素：pytest 的 JUnit 输出中 <skipped/> 通常不带文本 ----
            // 此前只处理 Event::Start，导致自闭合 <skipped/> 被漏判为 "passed"（由回归测试捕获）
            Ok(quick_xml::events::Event::Empty(ref event))
                if event.name().as_ref() == b"skipped" =>
            {
                if let Some(test) = current.as_mut() {
                    apply_skipped_status(test, event);
                }
            }

            Ok(quick_xml::events::Event::Empty(ref event))
                if event.name().as_ref() == b"failure" =>
            {
                if let Some(test) = current.as_mut() {
                    test.status = "failed".to_string();
                }
            }

            Ok(quick_xml::events::Event::Empty(ref event))
                if event.name().as_ref() == b"error" =>
            {
                if let Some(test) = current.as_mut() {
                    test.status = "error".to_string();
                }
            }

            Ok(quick_xml::events::Event::Text(text)) => {
                if let Some(message) = current_failure.as_mut() {
                    let value = text
                        .unescape()
                        .map_err(|e| e.to_string())?;
                    message.push_str(&value);
                }
            }

            Ok(quick_xml::events::Event::End(ref event))
                if event.name().as_ref() == b"failure"
                    || event.name().as_ref() == b"error" =>
            {
                if let Some(test) = current.as_mut() {
                    test.error_message = current_failure.take();
                }
            }

            Ok(quick_xml::events::Event::End(ref event))
                if event.name().as_ref() == b"testcase" =>
            {
                if let Some(test) = current.take() {
                    results.push(test);
                }
            }

            Ok(quick_xml::events::Event::Eof) => break,

            Err(e) => {
                return Err(format!("Failed to parse JUnit XML: {}", e));
            }

            _ => {}
        }
    }

    Ok(results)
}

fn parse_testcase_attributes(
    event: &quick_xml::events::BytesStart,
) -> TestResult {
    let mut name = String::new();
    let mut file = String::new();
    let mut duration = 0.0;
    let mut line: Option<u32> = None;

    for attribute in event.attributes().flatten() {
        match attribute.key.as_ref() {
            b"name" => {
                name =
                    String::from_utf8_lossy(&attribute.value).to_string();
            }
            b"file" => {
                file =
                    String::from_utf8_lossy(&attribute.value).to_string();
            }
            b"time" => {
                duration = String::from_utf8_lossy(&attribute.value)
                    .parse()
                    .unwrap_or(0.0);
            }
            b"line" => {
                line = String::from_utf8_lossy(&attribute.value)
                    .parse()
                    .ok();
            }
            _ => {}
        }
    }

    let id = if file.is_empty() {
        name.clone()
    } else {
        format!("{}::{}", file, name)
    };

    TestResult {
        id,
        name,
        file,
        status: "passed".to_string(),
        duration,
        error_message: None,
        line,
        skip_reason: None,
    }
}

/// 读取 XML 元素的某个属性值（找不到返回 None）。
fn attr_value(event: &quick_xml::events::BytesStart, key: &[u8]) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).to_string())
}

/// 处理 `<skipped>` 元素：pytest 的 xfail 会以 `<skipped type="pytest.xfail">` 形式出现，
/// 需要单独标记为 "xfailed" 状态（区别于普通 skipped），原因取自 message 属性。
fn apply_skipped_status(test: &mut TestResult, event: &quick_xml::events::BytesStart) {
    let is_xfail = attr_value(event, b"type")
        .map(|t| t.contains("xfail"))
        .unwrap_or(false);
    test.status = if is_xfail {
        "xfailed".to_string()
    } else {
        "skipped".to_string()
    };
    test.skip_reason = attr_value(event, b"message")
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty());
}

#[cfg(test)]
mod tests {
    use super::parse_junit_results_from_str;

    #[test]
    fn junit_parses_passed_self_closing_with_line() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_user" name="test_add_0" file="tests/generated/test_user.py" line="3" time="0.001"/>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "passed");
        assert_eq!(results[0].line, Some(3));
        assert_eq!(results[0].skip_reason, None);
    }

    #[test]
    fn junit_parses_failed_with_error_message() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_user" name="test_add_0" file="tests/generated/test_user.py" line="7" time="0.002">
    <failure message="AssertionError">assert 1 == 2</failure>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "failed");
        assert!(results[0].error_message.as_deref().unwrap().contains("assert 1 == 2"));
        assert_eq!(results[0].line, Some(7));
    }

    #[test]
    fn junit_parses_skipped_with_reason_and_line() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_user" name="test_requires_db" file="tests/generated/test_user.py" line="12" time="0.0">
    <skipped message="requires a running MySQL instance"/>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "skipped");
        assert_eq!(results[0].skip_reason.as_deref(), Some("requires a running MySQL instance"));
        assert_eq!(results[0].line, Some(12));
    }

    #[test]
    fn junit_parses_skipped_self_closing_without_reason() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_user" name="test_skipped_no_reason" file="tests/generated/test_user.py" time="0.0">
    <skipped/>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "skipped");
        assert_eq!(results[0].skip_reason, None);
        assert_eq!(results[0].line, None);
    }

    #[test]
    fn junit_marks_xfail_as_distinct_status() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_calculator" name="test_known_bug" file="tests/test_calculator.py" line="21" time="0.01">
    <skipped type="pytest.xfail" message="known issue #123"/>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "xfailed");
        assert_eq!(results[0].skip_reason.as_deref(), Some("known issue #123"));
        assert_eq!(results[0].line, Some(21));
    }
}
