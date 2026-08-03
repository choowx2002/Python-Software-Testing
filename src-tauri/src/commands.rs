use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

// ============================================
// Data Structures
// ============================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvDetectionResult {
    pub python_path: Option<String>,
    pub python_version: Option<String>,
    pub venv_path: Option<String>,
    // 重命名: venv_activated -> venv_exists
    // 语义更准确：表示 Tauri 找到了可用的 venv 目录，可直接调用其 python，无需 shell activate
    pub venv_exists: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PipPackage {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub installed: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstallStep {
    pub package: String,
    pub status: String, // "starting", "success", "failed"
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFile {
    pub name: String,
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub file: String,
    pub class_name: Option<String>,
    pub line: Option<u32>,
}

// ============================================
// Command 1: Detect Python Environment
// ============================================
#[tauri::command]
pub async fn detect_python_env(project_path: String) -> Result<EnvDetectionResult, String> {
    let path = PathBuf::from(&project_path);

    if !path.exists() || !path.is_dir() {
        return Err("Invalid project directory".into());
    }

    let mut result = EnvDetectionResult {
        python_path: None,
        python_version: None,
        venv_path: None,
        venv_exists: false,
        dependencies: Vec::new(),
    };

    // ----------------------------------------
    // 1. Detect Virtual Environment
    // ----------------------------------------
    let venv_candidates = [".venv", "venv"];

    for candidate in venv_candidates {
        let venv_dir = path.join(candidate);
        if !venv_dir.exists() {
            continue;
        }

        let python_path = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        if python_path.exists() {
            result.venv_exists = true;
            result.venv_path = Some(venv_dir.to_string_lossy().to_string());
            result.python_path = Some(python_path.to_string_lossy().to_string());
            break;
        }
    }

    // ----------------------------------------
    // 2. Fallback: Detect Global Python (If no venv found)
    // ----------------------------------------
    if !result.venv_exists {
        let global_python = if cfg!(target_os = "windows") { "python" } else { "python3" };

        if let Ok(output) = Command::new(global_python).arg("--version").output() {
            if output.status.success() {
                let version = if output.stdout.is_empty() {
                    String::from_utf8_lossy(&output.stderr).trim().to_string()
                } else {
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                };
                result.python_path = Some(global_python.to_string());
                result.python_version = Some(version);
            }
        }
        // 注意：如果没有 venv 也没有全局 python，这里会返回 python_path 为 None 的结果
        // 前端可以根据 python_path.is_none() 提示用户安装 Python
        return Ok(result);
    }

    // ----------------------------------------
    // 3. Python Version (for existing venv)
    // ----------------------------------------
    let python_path = result.python_path.as_ref().unwrap();
    if let Ok(output) = Command::new(python_path).arg("--version").output() {
        if output.status.success() {
            let version = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            };
            result.python_version = Some(version);
        }
    }

    // ----------------------------------------
    // 4. Dependency Detection (requirements.txt vs venv)
    // ----------------------------------------

    // 4.1 尝试从 requirements.txt 读取项目声明的依赖
    let mut required_deps: Vec<String> = Vec::new();
    let req_file_path = path.join("requirements.txt");

    if req_file_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&req_file_path) {
            for line in content.lines() {
                let line = line.trim();
                // 跳过空行和注释
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                // 简单解析包名：处理 "package==1.0", "package>=1.0", "package[extra]" 等情况
                // 按常见分隔符分割，取第一部分作为包名
                let pkg_name = line
                    .split(|c| c == '=' || c == '>' || c == '<' || c == '~' || c == '[' || c == ' ')
                    .next()
                    .unwrap_or(line)
                    .trim()
                    .to_lowercase();

                if !pkg_name.is_empty() {
                    required_deps.push(pkg_name);
                }
            }
        }
    }

    // 4.2 如果没找到 requirements.txt，回退到你的默认硬编码列表
    if required_deps.is_empty() {
        required_deps = vec!["pytest".to_string(), "coverage".to_string(), "pynguin".to_string()];
    }

    // 4.3 检查这些依赖在当前 Python 环境 (venv) 中的安装状态
    match Command::new(python_path)
        .args(["-m", "pip", "list", "--format=json"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let installed_packages: Vec<PipPackage> =
                serde_json::from_str(&stdout).unwrap_or_default();

            for dep in required_deps {
                if let Some(pkg) = installed_packages
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(&dep))
                {
                    result.dependencies.push(DependencyStatus {
                        name: dep.clone(),
                        installed: true,
                        version: Some(pkg.version.clone()),
                    });
                } else {
                    result.dependencies.push(DependencyStatus {
                        name: dep.clone(),
                        installed: false,
                        version: None,
                    });
                }
            }
        }
        _ => {
            // pip list 失败时，将所有声明的依赖标记为未安装
            for dep in required_deps {
                result.dependencies.push(DependencyStatus {
                    name: dep,
                    installed: false,
                    version: None,
                });
            }
        }
    }

    Ok(result)
}

// ============================================
// Command 2: Install Missing Dependencies
// ============================================
#[tauri::command]
pub async fn install_dependencies(
    app: tauri::AppHandle,
    python_path: String,
    packages: Vec<String>,
) -> Result<InstallResult, String> {
    if packages.is_empty() {
        return Ok(InstallResult {
            success: true,
            installed: vec![],
            failed: vec![],
        });
    }

    let mut installed = vec![];
    let mut failed = vec![];
    // 🧹 移除了 let mut all_logs = String::new();

    for package in &packages {
        // 1️⃣ Emit: 开始安装
        let _ = app.emit("install_step", InstallStep {
            package: package.clone(),
            status: "starting".to_string(),
        });

        let output = Command::new(&python_path)
            .args(["-m", "pip", "install", package])
            .output()
            .map_err(|e| format!("Failed to execute pip for {}: {}", package, e))?;

        if output.status.success() {
            installed.push(package.clone());

            // 2️⃣ Emit: 安装成功
            let _ = app.emit("install_step", InstallStep {
                package: package.clone(),
                status: "success".to_string(),
            });
        } else {
            failed.push(package.clone());

            // 3️⃣ Emit: 安装失败
            let _ = app.emit("install_step", InstallStep {
                package: package.clone(),
                status: "failed".to_string(),
            });
        }
    }

    Ok(InstallResult {
        success: failed.is_empty(),
        installed,
        failed,
    })
}

// ============================================
// Command 3: Validate Project Directory
// ============================================
#[tauri::command]
pub async fn validate_project_directory(project_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&project_path);

    if !path.exists() {
        return Err("Directory does not exist".to_string());
    }
    if !path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    // 放宽校验：检查 .py 文件或常见的 Python 项目配置文件
    let has_python_indicators = std::fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            name_str.ends_with(".py") ||
            name_str.eq_ignore_ascii_case("requirements.txt") ||
            name_str.eq_ignore_ascii_case("pyproject.toml") ||
            name_str.eq_ignore_ascii_case("setup.py") ||
            name_str.eq_ignore_ascii_case("Pipfile")
        });

    if !has_python_indicators {
        return Err("No Python files (.py) or common project configuration files (requirements.txt, pyproject.toml, etc.) found in the selected directory".to_string());
    }

    Ok(true)
}

// ============================================
// Command 4: Create Virtual Environment
// ============================================
#[tauri::command]
pub async fn create_virtual_env(
    project_path: String,
    python_executable: String,
) -> Result<String, String> {
    let path = PathBuf::from(&project_path);
    let venv_dir = path.join(".venv");

    // 幂等性检查：如果 .venv 已存在，先验证它是否有效
    if venv_dir.exists() {
        let test_python = if cfg!(target_os = "windows") {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        };

        if test_python.exists() {
            if let Ok(output) = Command::new(&test_python).arg("--version").output() {
                if output.status.success() {
                    // 已经是一个有效的虚拟环境，直接返回成功，避免重复创建
                    return Ok(venv_dir.to_string_lossy().to_string());
                }
            }
        }
        // 如果存在但无效（例如损坏的目录），返回明确错误，避免意外覆盖用户数据
        return Err("A corrupted or invalid '.venv' directory already exists. Please remove it manually and try again.".to_string());
    }

    // 执行创建命令
    let output = Command::new(&python_executable)
        .current_dir(&path)
        .args(["-m", "venv", ".venv"])
        .output()
        .map_err(|e| format!("Failed to execute python executable: {}", e))?;

    if output.status.success() {
        Ok(venv_dir.to_string_lossy().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub fn scan_test_files(project_path: String) -> Result<Vec<TestFile>, String> {
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    if !project.is_dir() {
        return Err(format!("Project path is not a directory: {}", project_path));
    }

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

fn is_test_file(file_name: &str) -> bool {
    file_name.starts_with("test_") && file_name.ends_with(".py")
        || file_name.ends_with("_test.py")
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

    let python = find_project_python(&project)?;

    let python_root = project
        .parent()
        .ok_or("Failed to determine project parent directory")?;

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

        cases.push(TestCase {
            id,
            name: test_name,
            file,
            class_name,
            line: None,
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
pub struct TestProgressEvent {
    pub run_id: String,
    pub completed: usize,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub current_test: Option<String>,
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
}

#[tauri::command]
pub async fn run_tests(
    app_handle: AppHandle,
    project_id: i64,
    project_path: String,
    interpreter_path: String,
    test_cases: Vec<String>,
    pytest_args: Vec<String>,
) -> Result<String, String> {
    let run_id = Uuid::new_v4().to_string();
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
    args.push(format!("--junitxml={}", junit_path.to_string_lossy()));

    let mut command_str = interpreter.display().to_string();
    for arg in &args {
        command_str.push(' ');
        command_str.push_str(arg);
    }

    let start = std::time::Instant::now();
    let python_root = project
        .parent()
        .ok_or("Failed to determine project parent directory")?;

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
            });
            return Err(format!("Failed to start pytest: {}", e));
        }
    };

    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture pytest stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture pytest stderr".to_string())?;

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

    let status = child.wait().await.map_err(|e| format!("Failed waiting for pytest: {}", e))?;
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
    let skipped = results.iter().filter(|r| r.status == "skipped").count();

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
        },
    );

    let _ = std::fs::remove_file(junit_path);
    Ok(run_id)
}

fn parse_junit_results(path: &Path) -> Result<Vec<TestResult>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read JUnit XML: {}", e))?;

    let mut reader = quick_xml::Reader::from_str(&content);
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
                    test.status = "skipped".to_string();
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
    }
}

#[tauri::command]
pub async fn open_in_file_manager(path: String) -> Result<(), String> {
    let target = std::path::Path::new(&path);

    if !target.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Linux: 根据桌面环境选择文件管理器
    #[cfg(target_os = "linux")]
    {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();

        let result = if desktop.contains("kde") {
            Command::new("dolphin").arg(&path).spawn()
        } else if desktop.contains("xfce") {
            Command::new("thunar").arg(&path).spawn()
        } else {
            // GNOME 或其他，用 nautilus；fallback 到 xdg-open
            Command::new("nautilus").arg(&path).spawn()
                .or_else(|_| Command::new("xdg-open").arg(&path).spawn())
        };

        result.map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    // macOS
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }

    // Windows
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
    }

    Ok(())
}

fn build_python_path(project: &Path) -> String {
    let mut paths = vec![];

    if let Some(parent) = project.parent() {
        paths.push(parent.to_string_lossy().to_string());
    }

    if let Ok(existing) = std::env::var("PYTHONPATH") {
        if !existing.is_empty() {
            paths.push(existing);
        }
    }

    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    paths.join(separator)
}

// ============================================
// Generate Tests Data Structures
// ============================================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SourceFile {
    pub name: String,
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationStartedEvent {
    pub run_id: String,
    pub total_files: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationProgressEvent {
    pub run_id: String,
    pub current_file: String,
    pub completed_files: usize,
    pub total_files: usize,
    pub elapsed_time: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationOutputEvent {
    pub run_id: String,
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFile {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub test_case_count: usize,
    pub status: String, // "success", "empty", "failed"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationFinishedEvent {
    pub run_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: f64,
    pub generated_files: Vec<GeneratedFile>,
    pub command: String,
}

// ============================================
// Command: Scan Source Files
// ============================================

#[tauri::command]
pub fn scan_source_files(project_path: String) -> Result<Vec<SourceFile>, String> {
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    if !project.is_dir() {
        return Err(format!("Project path is not a directory: {}", project_path));
    }

    let mut files = Vec::new();
    scan_source_directory(&project, &project, &mut files)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

fn scan_source_directory(
    directory: &Path,
    project_root: &Path,
    files: &mut Vec<SourceFile>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|e| format!("Failed to read directory {:?}: {}", directory, e))?;

    // Directories to always skip
    let skip_dirs = [
        "__pycache__",
        ".venv",
        "venv",
        "env",
        ".env",
        "node_modules",
        ".git",
        ".hg",
        ".svn",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        "pynguin-results",
        ".idea",
        ".vscode",
        "build",
        "dist",
        ".eggs",
    ];

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if path.is_dir() {
            // Skip hidden directories and known non-source directories
            if name_str.starts_with('.') || skip_dirs.contains(&name_str.as_ref()) {
                continue;
            }
            scan_source_directory(&path, project_root, files)?;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        // Only include .py files
        if !name_str.ends_with(".py") {
            continue;
        }

        // Skip test files (we don't want to generate tests for tests)
        if is_test_file(&name_str) {
            continue;
        }

        // Skip conftest.py (pytest config, not application code)
        if name_str == "conftest.py" {
            continue;
        }

        if name_str == "__init__.py" {
            continue;
        }

        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");

        files.push(SourceFile {
            name: name_str.to_string(),
            path: path.to_string_lossy().to_string(),
            relative_path,
        });
    }

    Ok(())
}

// ============================================
// Command: Generate Tests (Pynguin)
// ============================================

#[tauri::command]
pub async fn generate_tests(
    app_handle: AppHandle,
    project_id: i64,
    project_path: String,
    interpreter_path: String,
    source_files: Vec<String>, // relative paths like "src/services/user.py"
    max_search_time: u64,
    algorithm: String,
    assertion_generation: bool,
    max_test_cases: u64,
    output_folder: String,
    seed: Option<i64>,
    chromosome_length: u64,
    population_size: u64,
) -> Result<String, String> {
    let run_id = Uuid::new_v4().to_string();
    let project = PathBuf::from(&project_path);

    if !project.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let interpreter = PathBuf::from(&interpreter_path);
    if !interpreter.exists() {
        return Err(format!(
            "Python interpreter does not exist: {}",
            interpreter_path
        ));
    }

    let total_files = source_files.len();
    let _ = app_handle.emit(
        "generation-started",
        GenerationStartedEvent {
            run_id: run_id.clone(),
            total_files,
        },
    );

    let output_base = project.join(&output_folder);
    std::fs::create_dir_all(&output_base)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let start = std::time::Instant::now();
    let mut generated_files: Vec<GeneratedFile> = Vec::new();
    let mut overall_success = true;
    let mut last_exit_code: Option<i32> = None;
    let mut command_str = String::new();

    for (i, rel_path) in source_files.iter().enumerate() {
        let file_path = project.join(rel_path);

        if !file_path.exists() {
            let _ = app_handle.emit(
                "generation-output",
                GenerationOutputEvent {
                    run_id: run_id.clone(),
                    stream: "stderr".to_string(),
                    line: format!("[Error] Source file not found: {}", rel_path),
                },
            );
            continue;
        }

        // Convert relative path to module name
        // e.g., "src/services/user.py" -> "src.services.user"
        let module_name = rel_path
            .trim_end_matches(".py")
            .replace('/', ".")
            .replace('\\', ".");

        // Create a per-module output directory
        let dir_name = rel_path
            .trim_end_matches(".py")
            .replace('/', "_")
            .replace('\\', "_");
        let module_output_dir = output_base.join(&dir_name);
        std::fs::create_dir_all(&module_output_dir)
            .map_err(|e| format!("Failed to create module output directory: {}", e))?;

        // Build Pynguin arguments
        let mut args = vec![
            "-m".to_string(),
            "pynguin".to_string(),
            "--project-path".to_string(),
            project.to_string_lossy().to_string(),
            "--module-name".to_string(),
            module_name.clone(),
            "--output-path".to_string(),
            module_output_dir.to_string_lossy().to_string(),
            "--algorithm".to_string(),
            algorithm.clone(),
            "--maximum-search-time".to_string(),
            max_search_time.to_string(),
            "--chromosome-length".to_string(),
            chromosome_length.to_string(),
            "--population".to_string(),
            population_size.to_string(),
            "--maximum-iterations".to_string(),
            max_test_cases.to_string(),
        ];

        if assertion_generation {
            args.push("--assertion-generation".to_string());
            args.push("SIMPLE".to_string());
        }

        if let Some(s) = seed {
            args.push("--seed".to_string());
            args.push(s.to_string());
        }

        // Build command string for display
        let mut cmd_display = interpreter.display().to_string();
        for arg in &args {
            cmd_display.push(' ');
            cmd_display.push_str(arg);
        }
        command_str = cmd_display.clone();

        // Emit progress
        let _ = app_handle.emit(
            "generation-progress",
            GenerationProgressEvent {
                run_id: run_id.clone(),
                current_file: rel_path.clone(),
                completed_files: i,
                total_files,
                elapsed_time: start.elapsed().as_secs_f64(),
            },
        );

        let separator = "=".repeat(60);

        let _ = app_handle.emit(
            "generation-output",
            GenerationOutputEvent {
                run_id: run_id.clone(),
                stream: "stdout".to_string(),
                line: format!("\n{}", separator),
            },
        );

        let _ = app_handle.emit(
            "generation-output",
            GenerationOutputEvent {
                run_id: run_id.clone(),
                stream: "stdout".to_string(),
                line: format!("Generating tests for module: {}", module_name),
            },
        );

        let _ = app_handle.emit(
            "generation-output",
            GenerationOutputEvent {
                run_id: run_id.clone(),
                stream: "stdout".to_string(),
                line: format!("{}\n", separator),
            },
        );

        // Run Pynguin
        let mut child = match AsyncCommand::new(&interpreter)
            .current_dir(&project)
            .env("PYTHONPATH", build_python_path(&project))
            .env("PYNGUIN_DANGER_AWARE", "1")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit(
                    "generation-output",
                    GenerationOutputEvent {
                        run_id: run_id.clone(),
                        stream: "stderr".to_string(),
                        line: format!("[Fatal] Failed to start Pynguin: {}", e),
                    },
                );
                overall_success = false;
                continue;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_handle = app_handle.clone();
        let stdout_run_id = run_id.clone();
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = stdout_handle.emit(
                        "generation-output",
                        GenerationOutputEvent {
                            run_id: stdout_run_id.clone(),
                            stream: "stdout".to_string(),
                            line,
                        },
                    );
                }
            }
        });

        let stderr_handle = app_handle.clone();
        let stderr_run_id = run_id.clone();
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = stderr_handle.emit(
                        "generation-output",
                        GenerationOutputEvent {
                            run_id: stderr_run_id.clone(),
                            stream: "stderr".to_string(),
                            line,
                        },
                    );
                }
            }
        });

        let status = child.wait().await;
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        let success = status.as_ref().map(|s| s.success()).unwrap_or(false);
        last_exit_code = status.as_ref().ok().and_then(|s| s.code());

        // Scan output directory for generated test files
        let mut module_generated: Vec<GeneratedFile> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&module_output_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let entry_name = entry.file_name().to_string_lossy().to_string();

                if entry_name.ends_with(".py") && entry_name.starts_with("test_") {
                    let test_count = count_test_cases(&entry_path);

                    let file_status = if test_count > 0 {
                        "success".to_string()
                    } else {
                        "empty".to_string()
                    };

                    let rel = entry_path
                        .strip_prefix(&project)
                        .unwrap_or(&entry_path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    module_generated.push(GeneratedFile {
                        name: entry_name.clone(),
                        path: entry_path.to_string_lossy().to_string(),
                        relative_path: rel,
                        test_case_count: test_count,
                        status: file_status,
                    });
                }
            }
        }

        if !success {
            overall_success = false;

            if module_generated.is_empty() {
                let stem = Path::new(rel_path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();

                module_generated.push(GeneratedFile {
                    name: format!("test_{}.py", stem),
                    path: String::new(),
                    relative_path: String::new(),
                    test_case_count: 0,
                    status: "failed".to_string(),
                });
            }
        }

        let _ = app_handle.emit(
            "generation-output",
            GenerationOutputEvent {
                run_id: run_id.clone(),
                stream: "stdout".to_string(),
                line: format!(
                    "\n[Done] {} — {} test file(s) generated\n",
                    module_name,
                    module_generated.len()
                ),
            },
        );

        generated_files.extend(module_generated);
    }

    let duration = start.elapsed().as_secs_f64();

    let _ = app_handle.emit(
        "generation-finished",
        GenerationFinishedEvent {
            run_id: run_id.clone(),
            success: overall_success,
            exit_code: last_exit_code,
            duration,
            generated_files,
            command: command_str,
        },
    );

    Ok(run_id)
}

/// Count test cases in a generated file by looking for "def test_" lines
fn count_test_cases(path: &Path) -> usize {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("def test_") || trimmed.starts_with("async def test_")
        })
        .count()
}

// ============================================
// Command: Open File
// ============================================

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    let target = Path::new(&path);

    if !target.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}

// ============================================
// Command: Reveal in Folder
// ============================================

#[tauri::command]
pub async fn reveal_in_folder(path: String) -> Result<(), String> {
    let target = Path::new(&path);

    if !target.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    #[cfg(target_os = "linux")]
    {
        let parent = target.parent().unwrap_or(target);
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();

        let result = if desktop.contains("kde") {
            Command::new("dolphin").arg("--select").arg(&path).spawn()
        } else if desktop.contains("xfce") {
            Command::new("thunar").arg(parent).spawn()
        } else {
            Command::new("nautilus")
                .arg(parent)
                .spawn()
                .or_else(|_| Command::new("xdg-open").arg(parent).spawn())
        };

        result.map_err(|e| format!("Failed to reveal in folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to reveal in Explorer: {}", e))?;
    }

    Ok(())
}
