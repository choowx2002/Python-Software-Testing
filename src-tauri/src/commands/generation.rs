use crate::state::AppState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
use crate::proc::NoConsole;
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
#[allow(unused_variables)]
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
        if app_handle
            .state::<AppState>()
            .cancelled_runs
            .lock()
            .map(|m| m.contains_key(&run_id))
            .unwrap_or(false)
        {
            let _ = app_handle.emit(
                "generation-output",
                GenerationOutputEvent {
                    run_id: run_id.clone(),
                    stream: "stdout".to_string(),
                    line: "\n[Stopped] Generation cancelled by user.".to_string(),
                },
            );
            overall_success = false;
            break;
        }

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

        // 生成目录以 Python 包形式存在（写入 __init__.py）：
        // 否则 pytest 会把 tests/generated/cart/test_cart.py 按裸模块名 "test_cart" 导入，
        // 与手写的 tests/test_cart.py 同名冲突（import file mismatch）。
        // 幂等：每次生成都确保存在。
        let _ = std::fs::write(output_base.join("__init__.py"), "");
        let _ = std::fs::write(module_output_dir.join("__init__.py"), "");

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
            .no_console()
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

        register_run(&app_handle, &run_id, child.id().unwrap_or(0));

        // 捕获该模块完整输出，用于区分"无对象可测"与真正的失败
        let module_output = Arc::new(Mutex::new(String::new()));

        let stdout_handle = app_handle.clone();
        let stdout_run_id = run_id.clone();
        let stdout_buf = module_output.clone();
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let emit_start = tokio::time::Instant::now();
                    let _ = stdout_handle.emit(
                        "generation-output",
                        GenerationOutputEvent {
                            run_id: stdout_run_id.clone(),
                            stream: "stdout".to_string(),
                            line: line.clone(),
                        },
                    );
                    if let Ok(mut buf) = stdout_buf.lock() {
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                    crate::perf::record_emit(emit_start.elapsed());
                }
            }
        });

        let stderr_handle = app_handle.clone();
        let stderr_run_id = run_id.clone();
        let stderr_buf = module_output.clone();
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let emit_start = tokio::time::Instant::now();
                    let _ = stderr_handle.emit(
                        "generation-output",
                        GenerationOutputEvent {
                            run_id: stderr_run_id.clone(),
                            stream: "stderr".to_string(),
                            line: line.clone(),
                        },
                    );
                    if let Ok(mut buf) = stderr_buf.lock() {
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                    crate::perf::record_emit(emit_start.elapsed());
                }
            }
        });

        let status = child.wait().await;
        let _ = stdout_task.await;
        let _ = stderr_task.await;

        let combined_output = module_output
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default();

        let success = status.as_ref().map(|s| s.success()).unwrap_or(false);
        last_exit_code = status.as_ref().ok().and_then(|s| s.code());

        // ── 生成后重命名：把 test_case_n 模板名改成语义化名字 ──
        // 见 scripts/rename_generated_tests.py；失败只告警，不影响生成结果。
        if success {
            if let Some(rename_script) = locate_generation_rename_script() {
                let rename_result = AsyncCommand::new(&interpreter)
                    .no_console()
                    .current_dir(&project)
                    .args([
                        rename_script.to_string_lossy().to_string(),
                        module_output_dir.to_string_lossy().to_string(),
                        module_name.clone(),
                    ])
                    .output()
                    .await;
                match rename_result {
                    Ok(out) if out.status.success() => {
                        let summary = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !summary.is_empty() {
                            let _ = app_handle.emit(
                                "generation-output",
                                GenerationOutputEvent {
                                    run_id: run_id.clone(),
                                    stream: "stdout".to_string(),
                                    line: format!("[Rename] {}", summary),
                                },
                            );
                        }
                    }
                    Ok(out) => {
                        let _ = app_handle.emit(
                            "generation-output",
                            GenerationOutputEvent {
                                run_id: run_id.clone(),
                                stream: "stderr".to_string(),
                                line: format!(
                                    "[Rename] 重命名失败（不影响生成结果）: {}",
                                    String::from_utf8_lossy(&out.stderr).trim()
                                ),
                            },
                        );
                    }
                    Err(e) => {
                        let _ = app_handle.emit(
                            "generation-output",
                            GenerationOutputEvent {
                                run_id: run_id.clone(),
                                stream: "stderr".to_string(),
                                line: format!("[Rename] 无法运行重命名脚本: {}", e),
                            },
                        );
                    }
                }
            }
        }

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
            if module_generated.is_empty() {
                let stem = Path::new(rel_path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();

                // "SUT contains nothing we can test." 不是错误：模块没有可测对象
                // （如纯 GUI/常量模块），视为 empty 结果，不把整次生成标为失败。
                if combined_output.contains("SUT contains nothing we can test") {
                    let _ = app_handle.emit(
                        "generation-output",
                        GenerationOutputEvent {
                            run_id: run_id.clone(),
                            stream: "stdout".to_string(),
                            line: format!("\n[Empty] {} has nothing we can test — skipped", module_name),
                        },
                    );

                    module_generated.push(GeneratedFile {
                        name: format!("test_{}.py", stem),
                        path: String::new(),
                        relative_path: String::new(),
                        test_case_count: 0,
                        status: "empty".to_string(),
                    });
                } else {
                    overall_success = false;
                    module_generated.push(GeneratedFile {
                        name: format!("test_{}.py", stem),
                        path: String::new(),
                        relative_path: String::new(),
                        test_case_count: 0,
                        status: "failed".to_string(),
                    });
                }
            } else {
                overall_success = false;
            }
        }

        let real_files = module_generated
            .iter()
            .filter(|g| !g.path.is_empty())
            .count();
        let _ = app_handle.emit(
            "generation-output",
            GenerationOutputEvent {
                run_id: run_id.clone(),
                stream: "stdout".to_string(),
                line: format!(
                    "\n[Done] {} — {} test file(s) generated\n",
                    module_name, real_files
                ),
            },
        );

        generated_files.extend(module_generated);
    }

    let duration = start.elapsed().as_secs_f64();

    unregister_run(&app_handle, &run_id);

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

/// 定位生成后重命名脚本 scripts/rename_generated_tests.py。
/// 优先按 CARGO_MANIFEST_DIR 解析（开发构建），其次回退到当前工作目录。
fn locate_generation_rename_script() -> Option<PathBuf> {
    let candidates = [
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/rename_generated_tests.py"),
        PathBuf::from("scripts/rename_generated_tests.py"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

// ============================================
// UTF-8 BOM 处理（Pynguin 兼容性）
// Pynguin 通过 ast.parse(字符串) 解析源码，带 BOM 的文件会触发
// "invalid non-printable character U+FEFF" SyntaxError。
// Python 解释器本身可容忍磁盘文件的 BOM，故仅在生成前检查并修复。
// ============================================

/// 检查指定源文件是否带 UTF-8 BOM，返回带 BOM 的文件相对路径列表
#[tauri::command]
pub async fn check_python_bom(
    project_path: String,
    files: Vec<String>,
) -> Result<Vec<String>, String> {
    let root = PathBuf::from(&project_path);
    let mut bom_files = Vec::new();

    for rel in &files {
        let path = root.join(rel);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.len() >= 3 && bytes[..3] == [0xEF, 0xBB, 0xBF] {
            bom_files.push(rel.clone());
        }
    }

    Ok(bom_files)
}

/// 无损移除指定文件的 UTF-8 BOM（仅去掉 EF BB BF 前缀，其余字节原样保留），
/// 返回被修复的文件相对路径列表
#[tauri::command]
pub async fn strip_python_bom(
    project_path: String,
    files: Vec<String>,
) -> Result<Vec<String>, String> {
    use std::io::Write;

    let root = PathBuf::from(&project_path);
    let mut fixed = Vec::new();

    for rel in &files {
        let path = root.join(rel);
        let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read {}: {}", rel, e))?;
        if bytes.len() < 3 || bytes[..3] != [0xEF, 0xBB, 0xBF] {
            continue;
        }

        let mut file =
            std::fs::File::create(&path).map_err(|e| format!("Failed to rewrite {}: {}", rel, e))?;
        file.write_all(&bytes[3..])
            .map_err(|e| format!("Failed to write {}: {}", rel, e))?;
        fixed.push(rel.clone());
    }

    Ok(fixed)
}
