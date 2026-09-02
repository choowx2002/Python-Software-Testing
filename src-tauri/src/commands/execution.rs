use crate::state::{process_alive, terminate_pid, AppState};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
use crate::proc::NoConsole;
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
pub async fn collect_test_cases(
    app: tauri::AppHandle,
    project_path: String,
    run_id: String,
    file: Option<String>,
) -> Result<Vec<TestCase>, String> {
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

    let mut args = vec![
        "-m".to_string(),
        "pytest".to_string(),
        "--collect-only".to_string(),
        "-q".to_string(),
    ];
    if let Some(f) = &file {
        args.push(f.clone());
    }

    let mut child = match AsyncCommand::new(&python)
        .no_console()
        .current_dir(&project)
        .env("PYTHONPATH", build_python_path(&project))
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to start pytest: {}", e)),
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture pytest stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture pytest stderr".to_string())?;

    register_run(&app, &run_id, child.id().unwrap_or(0));

    // 读取收集输出：保留全部行用于解析，同时实时上报已发现的用例数
    let progress_run_id = run_id.clone();
    let progress_app = app.clone();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut collected: Vec<String> = Vec::new();
        let mut count: u64 = 0;
        while let Ok(Some(line)) = lines.next_line().await {
            if line.contains("::") {
                count += 1;
                let _ = progress_app.emit(
                    "collect-progress",
                    CollectProgressEvent {
                        run_id: progress_run_id.clone(),
                        count,
                    },
                );
            }
            collected.push(line);
        }
        collected
    });

    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut text = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            text.push_str(&line);
            text.push('\n');
        }
        text
    });

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            unregister_run(&app, &run_id);
            return Err(format!("Failed waiting for pytest: {}", e));
        }
    };

    let lines = stdout_task.await.unwrap_or_default();
    let stderr_text = stderr_task.await.unwrap_or_default();
    let was_cancelled = run_cancelled(&app, &run_id);
    unregister_run(&app, &run_id);

    if was_cancelled {
        return Err("Test case collection cancelled.".to_string());
    }

    if !status.success() {
        let trimmed = stderr_text.trim().to_string();
        let tail: String = trimmed
            .chars()
            .rev()
            .take(2000)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if tail.is_empty() {
            return Err("pytest collection failed.".to_string());
        }
        return Err(format!("pytest collection failed.\n\n{}", tail));
    }

    let stdout_content = lines.join("\n");
    parse_pytest_collection(&stdout_content, &project)
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

/// 收集用例时的实时进度（已发现多少个用例）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectProgressEvent {
    pub run_id: String,
    pub count: u64,
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

    // Windows CreateProcess 对命令行长度有限制：一次传入过多用例会报
    // “os error 206: The filename or extension is too long”。预估超限时，
    // 自动把选中用例分批执行（每批独立 junit，结果合并后统一发出 test-finished）。
    if !test_cases.is_empty() {
        let mut probe_args = vec![
            "-m".to_string(),
            "pytest".to_string(),
            "-v".to_string(),
        ];
        if crate::perf::enabled() {
            probe_args.push("-s".to_string());
        }
        probe_args.extend(pytest_args.clone());
        probe_args.push("-o".to_string());
        probe_args.push("junit_family=xunit1".to_string());
        probe_args.push("--junitxml=<placeholder>".to_string());

        let estimated: usize = interpreter.to_string_lossy().len()
            + 2
            + probe_args.iter().map(|a| a.len() + 3).sum::<usize>()
            + test_cases.iter().map(|t| t.len() + 3).sum::<usize>();

        if estimated > CHUNK_CMD_LIMIT {
            return run_tests_chunked(
                app_handle.clone(),
                run_id,
                execution_type.to_string(),
                project,
                interpreter,
                test_cases,
                pytest_args,
                regression_suite_id,
            )
            .await;
        }
    }

    let junit_path = std::env::temp_dir().join(format!("pytest-{}.xml", run_id));

    let mut args = vec!["-m".to_string(), "pytest".to_string(), "-v".to_string()];
    // NFR003 取证：TESTMATE_PERF=1 时加 -s（--capture=no）。pytest 默认 fd 捕获
    // 会把用例 stdout/stderr 缓存吞掉，日志洪流到不了 emit 管线（实测 n 只有几十）；
    // -s 让输出逐行流经 test-output 事件，heavy log streaming 测量才有意义。
    // 正常模式（未设 TESTMATE_PERF）保持原行为，零影响。
    if crate::perf::enabled() {
        args.push("-s".to_string());
    }
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
        .no_console()
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
            let emit_start = tokio::time::Instant::now();
            let _ = stdout_handle.emit("test-output", TestOutputEvent {
                run_id: stdout_run_id.clone(),
                stream: "stdout".to_string(),
                line,
            });
            crate::perf::record_emit(emit_start.elapsed());
        }
    });

    let stderr_handle = app_handle.clone();
    let stderr_run_id = run_id.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let emit_start = tokio::time::Instant::now();
            let _ = stderr_handle.emit("test-output", TestOutputEvent {
                run_id: stderr_run_id.clone(),
                stream: "stderr".to_string(),
                line,
            });
            crate::perf::record_emit(emit_start.elapsed());
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
    // strict-xfail 意外通过（xpassed）不算失败；作为通过统计
    let xpassed = results.iter().filter(|r| r.status == "xpassed").count();
    let failed = results.iter().filter(|r| r.status == "failed" || r.status == "error").count();
    // xfail 在 junit 中表现为 skipped 子类，汇总时并入 skipped，保持与 pytest 报告一致
    let skipped = results
        .iter()
        .filter(|r| r.status == "skipped" || r.status == "xfailed")
        .count();

    // 只有 [XPASS(strict)]（无真实失败/错误）时，pytest 退出码非 0 但应视为通过
    let success = status.success()
        || (!results.is_empty() && failed == 0 && xpassed > 0);

    let _ = app_handle.emit(
        "test-finished",
        TestFinishedEvent {
            run_id: run_id.clone(),
            success,
            exit_code: status.code(),
            duration,
            passed: passed + xpassed,
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

/// 分批执行时，单条 pytest 命令行的长度上限（Windows 下留足余量）
const CHUNK_CMD_LIMIT: usize = 28_000;

/// 检查某个 run 是否已被用户取消（取消标记在最后一轮 unregister 前持续生效）
fn run_cancelled(app: &AppHandle, run_id: &str) -> bool {
    app.state::<AppState>()
        .cancelled_runs
        .lock()
        .map(|m| m.contains_key(run_id))
        .unwrap_or(false)
}

/// 分批执行选中用例：每批一个 pytest 进程（命令行长度受限），
/// 流式输出共用同一 run_id，结果/计数/时长聚合后统一发出 test-finished。
async fn run_tests_chunked(
    app_handle: AppHandle,
    run_id: String,
    execution_type: String,
    project: PathBuf,
    interpreter: PathBuf,
    test_cases: Vec<String>,
    pytest_args: Vec<String>,
    regression_suite_id: Option<i64>,
) -> Result<String, String> {
    let mut base_args = vec![
        "-m".to_string(),
        "pytest".to_string(),
        "-v".to_string(),
    ];
    if crate::perf::enabled() {
        base_args.push("-s".to_string());
    }
    base_args.extend(pytest_args);
    base_args.push("-o".to_string());
    base_args.push("junit_family=xunit1".to_string());

    let estimate = |args: &[String], extra: &[String]| -> usize {
        interpreter.to_string_lossy().len()
            + 2
            + args.iter().map(|a| a.len() + 3).sum::<usize>()
            + extra.iter().map(|a| a.len() + 3).sum::<usize>()
    };

    let base_len = estimate(&base_args, &[]);

    // 按命令行长度把用例拆批
    let mut batches: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_len = base_len;
    for case in &test_cases {
        let add = case.len() + 3;
        if !current.is_empty() && current_len + add > CHUNK_CMD_LIMIT {
            batches.push(std::mem::take(&mut current));
            current_len = base_len;
        }
        current.push(case.clone());
        current_len += add;
    }
    if !current.is_empty() || batches.is_empty() {
        batches.push(current);
    }

    let mut all_results: Vec<TestResult> = Vec::new();
    let mut total_duration = 0.0_f64;
    let mut all_exit_ok = true;
    let mut last_exit_code: Option<i32> = None;
    let mut fatal: Option<String> = None;
    let mut command_display = String::new();

    for (idx, batch) in batches.iter().enumerate() {
        if run_cancelled(&app_handle, &run_id) {
            all_exit_ok = false;
            break;
        }

        let mut args = base_args.clone();
        args.extend(batch.iter().cloned());
        let junit_path = std::env::temp_dir()
            .join(format!("pytest-{}-{}.xml", run_id, idx));
        args.push(format!("--junitxml={}", junit_path.to_string_lossy()));

        if command_display.is_empty() {
            let mut display = interpreter.to_string_lossy().to_string();
            for a in &args {
                display.push(' ');
                display.push_str(a);
            }
            command_display = display;
        }

        match run_pytest_batch(
            &app_handle,
            &run_id,
            &project,
            &interpreter,
            &args,
            &junit_path,
        )
        .await
        {
            Ok((exit_ok, exit_code, mut results, duration)) => {
                all_results.append(&mut results);
                total_duration += duration;
                all_exit_ok &= exit_ok;
                last_exit_code = exit_code;
            }
            Err(e) => {
                fatal = Some(e);
                break;
            }
        }

        let _ = std::fs::remove_file(&junit_path);
    }

    unregister_run(&app_handle, &run_id);

    if batches.len() > 1 {
        command_display.push_str(&format!(" [{} batches]", batches.len()));
    }

    let passed = all_results.iter().filter(|r| r.status == "passed").count();
    let xpassed = all_results.iter().filter(|r| r.status == "xpassed").count();
    let failed = all_results
        .iter()
        .filter(|r| r.status == "failed" || r.status == "error")
        .count();
    let skipped = all_results
        .iter()
        .filter(|r| r.status == "skipped" || r.status == "xfailed")
        .count();

    // 仅 strict-xpass（无真实失败）也算通过，与单次运行语义一致
    let success = fatal.is_none()
        && (all_exit_ok || (!all_results.is_empty() && failed == 0 && xpassed > 0));

    let _ = app_handle.emit(
        "test-finished",
        TestFinishedEvent {
            run_id: run_id.clone(),
            success,
            exit_code: if fatal.is_some() {
                None
            } else {
                last_exit_code
            },
            duration: total_duration,
            passed: passed + xpassed,
            failed,
            skipped,
            results: all_results,
            command: command_display,
            execution_type,
            regression_suite_id,
        },
    );

    if let Some(msg) = fatal {
        return Err(msg);
    }
    Ok(run_id)
}

/// 执行单个 pytest 批次：流式输出 → wait → 解析该批 junit。
/// register/unregister 由外层统一管理（同一 run_id 跨批保持可取消）。
async fn run_pytest_batch(
    app: &AppHandle,
    run_id: &str,
    project: &Path,
    interpreter: &Path,
    args: &[String],
    junit_path: &Path,
) -> Result<(bool, Option<i32>, Vec<TestResult>, f64), String> {
    let start = std::time::Instant::now();

    let mut child = match AsyncCommand::new(interpreter)
        .no_console()
        .current_dir(project)
        .env("PYTHONPATH", build_python_path(project))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to start pytest: {}", e)),
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture pytest stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture pytest stderr".to_string())?;

    register_run(app, run_id, child.id().unwrap_or(0));

    let stdout_handle = app.clone();
    let stdout_run_id = run_id.to_string();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let emit_start = tokio::time::Instant::now();
            let _ = stdout_handle.emit("test-output", TestOutputEvent {
                run_id: stdout_run_id.clone(),
                stream: "stdout".to_string(),
                line,
            });
            crate::perf::record_emit(emit_start.elapsed());
        }
    });

    let stderr_handle = app.clone();
    let stderr_run_id = run_id.to_string();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let emit_start = tokio::time::Instant::now();
            let _ = stderr_handle.emit("test-output", TestOutputEvent {
                run_id: stderr_run_id.clone(),
                stream: "stderr".to_string(),
                line,
            });
            crate::perf::record_emit(emit_start.elapsed());
        }
    });

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            return Err(format!("Failed waiting for pytest: {}", e));
        }
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let duration = start.elapsed().as_secs_f64();

    let results = match parse_junit_results(junit_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to parse JUnit XML: {}", e);
            vec![]
        }
    };

    Ok((status.success(), status.code(), results, duration))
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
            #[cfg(target_os = "windows")]
            {
                // Windows 无真正的 SIGTERM，且必须连进程树一起杀
                // （否则 pynguin/pytest 的子 worker 会继续攥着输出管道）。
                let _ = terminate_pid(pid, true);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = terminate_pid(pid, false);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                if process_alive(pid) {
                    let _ = terminate_pid(pid, true);
                }
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
                    test.status = if is_xpass_strict(event) {
                        "xpassed".to_string()
                    } else {
                        "failed".to_string()
                    };
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
                    test.status = if is_xpass_strict(event) {
                        "xpassed".to_string()
                    } else {
                        "failed".to_string()
                    };
                    let msg = attr_value(event, b"message").unwrap_or_default();
                    test.error_message = Some(msg);
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
                    // strict xfail 意外通过：pytest 记为 failure，正文以 [XPASS(strict)] 开头
                    if test.status == "failed"
                        && test
                            .error_message
                            .as_deref()
                            .map(|m| m.contains("[XPASS(strict)]"))
                            .unwrap_or(false)
                    {
                        test.status = "xpassed".to_string();
                    }
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

/// strict xfail 意外通过时，pytest 在 JUnit 里以 failure 形式输出，
/// message/正文以 `[XPASS(strict)]` 开头 —— 应在解析阶段归为独立状态 "xpassed"。
fn is_xpass_strict(event: &quick_xml::events::BytesStart) -> bool {
    attr_value(event, b"message")
        .map(|m| m.contains("[XPASS(strict)]"))
        .unwrap_or(false)
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

    #[test]
    fn junit_maps_strict_xpass_to_xpassed() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_core_db" name="test_TaskDatabase_0" file="tests/generated/core_db/test_core_db.py" time="0.01">
    <failure message="[XPASS(strict)]">[XPASS(strict)] marker outdated
  assert 1 == 1
</failure>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "xpassed");
        let msg = results[0].error_message.as_deref().unwrap_or_default();
        assert!(msg.contains("[XPASS(strict)]"));
    }

    #[test]
    fn junit_keeps_real_failure_as_failed() {
        let xml = r#"<?xml version="1.0"?>
<testsuite>
  <testcase classname="test_user" name="test_a" file="tests/test_user.py" time="0.01">
    <failure>assert 1 == 2</failure>
  </testcase>
</testsuite>"#;
        let results = parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "failed");
        assert_eq!(
            results[0].error_message.as_deref(),
            Some("assert 1 == 2")
        );
    }
}
