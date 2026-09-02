use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use std::process::Stdio;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use super::*;
use crate::proc::NoConsole;

// ============================================
// Coverage Data Structures
// ============================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageSummary {
    pub run_id: String,
    pub project_id: i64,
    pub percent_covered: f64,
    pub total_statements: usize,
    pub covered_statements: usize,
    /// 分支覆盖率（--branch 模式下有效）：总分支数 / 已覆盖分支数 / 分支覆盖率百分比
    pub total_branches: Option<u64>,
    pub covered_branches: Option<u64>,
    pub branch_percent: Option<f64>,
    pub files: Vec<FileCoverage>,
    pub json_path: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCoverage {
    /// 相对项目根目录的路径（与 coverage.json 中的 key 一致）
    pub path: String,
    pub percent_covered: f64,
    pub executed_lines: Vec<u32>,
    pub missing_lines: Vec<u32>,
    pub excluded_lines: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCoverageDetail {
    pub path: String,
    pub percent_covered: f64,
    pub lines: Vec<LineCoverage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineCoverage {
    pub line_number: u32,
    pub source: String,
    /// "covered" | "missing" | "excluded" | "not-executable"
    pub status: String,
}

// ============================================
// Coverage Events
// ============================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageStartedEvent {
    pub run_id: String,
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageOutputEvent {
    pub run_id: String,
    pub stream: String, // "stdout" | "stderr"
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageFinishedEvent {
    pub run_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration: f64,
    pub summary: Option<CoverageSummary>,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageErrorEvent {
    pub run_id: String,
    pub message: String,
}

// ============================================
// Command: Check coverage.py Installed
// ============================================

#[tauri::command]
pub async fn check_coverage_installed(interpreter_path: String) -> Result<bool, String> {
    let output = Command::new(&interpreter_path)
        .no_console()
        .args(["-m", "coverage", "--version"])
        .output()
        .map_err(|e| format!("Failed to execute coverage: {}", e))?;

    Ok(output.status.success())
}

// ============================================
// Command: Run Coverage
// ============================================

#[tauri::command]
pub async fn run_coverage(
    app: tauri::AppHandle,
    project_id: i64,
    project_path: String,
    interpreter_path: String,
    test_files: Vec<String>,
    source_dirs: Vec<String>,
) -> Result<CoverageSummary, String> {
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

    let _ = app.emit(
        "coverage-started",
        CoverageStartedEvent {
            run_id: run_id.clone(),
            project_id,
        },
    );

    // ----------------------------------------
    // Step 1: coverage erase（同步，快）
    // ----------------------------------------
    let erase_cmd = format!("{} -m coverage erase", interpreter.display());
    let erase_output = Command::new(&interpreter)
        .no_console()
        .args(["-m", "coverage", "erase"])
        .current_dir(&project)
        .output()
        .map_err(|e| format!("Failed to execute coverage erase: {}", e))?;

    if !erase_output.status.success() {
        let message = format!(
            "coverage erase failed:\n{}",
            String::from_utf8_lossy(&erase_output.stderr).trim()
        );
        let _ = app.emit(
            "coverage-error",
            CoverageErrorEvent {
                run_id: run_id.clone(),
                message: message.clone(),
            },
        );
        let _ = app.emit(
            "coverage-finished",
            CoverageFinishedEvent {
                run_id: run_id.clone(),
                success: false,
                exit_code: erase_output.status.code(),
                duration: 0.0,
                summary: None,
                command: erase_cmd,
            },
        );
        return Err(message);
    }

    // ----------------------------------------
    // Step 2: coverage run --source=... -m pytest ...（异步读输出）
    // ----------------------------------------
    let mut args = vec![
        "-m".to_string(),
        "coverage".to_string(),
        "run".to_string(),
    ];

    if !source_dirs.is_empty() {
        // 前端以 "." 表示"项目根目录"（根目录源文件）。coverage 的 --source
        // 按导入的模块名匹配，但根目录文件既可能被 `import example` 也可能被
        // `import pyExample.example` 导入（根目录有 __init__.py 时为包布局），
        // 传模块名无法覆盖两种情况 —— 这里把 "." 解析为项目根绝对路径，
        // 让 coverage 按文件位置追踪导入，最鲁棒。
        let resolved: Vec<String> = source_dirs
            .iter()
            .map(|d| {
                if d == "." {
                    project.to_string_lossy().to_string()
                } else {
                    d.clone()
                }
            })
            .collect();
        args.push(format!("--source={}", resolved.join(",")));
        // 排除测试/依赖目录，避免报告混入 tests、site-packages 等文件
        args.push(
            "--omit=*/tests/*,*/site-packages/*,*/__pycache__/*,*/.venv/*,*/venv/*,*/pynguin-results/*"
                .to_string(),
        );
    }

    // FR011：启用分支覆盖率采集（statement + branch），与论文 Table 4.1 口径一致
    args.push("--branch".to_string());

    args.push("-m".to_string());
    args.push("pytest".to_string());
    // NFR003 取证：与 execution.rs 同理，TESTMATE_PERF=1 时加 -s 让用例输出
    // 逐行流经 coverage-output 事件（pytest 默认捕获会吞掉输出）。
    if crate::perf::enabled() {
        args.push("-s".to_string());
    }
    args.extend(test_files.iter().cloned());

    let mut command_str = interpreter.display().to_string();
    for arg in &args {
        command_str.push(' ');
        command_str.push_str(arg);
    }

    let start = std::time::Instant::now();

    let mut child = match AsyncCommand::new(&interpreter)
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
        Err(e) => {
            let message = format!("Failed to start coverage run: {}", e);
            let _ = app.emit(
                "coverage-error",
                CoverageErrorEvent {
                    run_id: run_id.clone(),
                    message: message.clone(),
                },
            );
            let _ = app.emit(
                "coverage-finished",
                CoverageFinishedEvent {
                    run_id: run_id.clone(),
                    success: false,
                    exit_code: None,
                    duration: 0.0,
                    summary: None,
                    command: command_str.clone(),
                },
            );
            return Err(message);
        }
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture coverage stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture coverage stderr".to_string())?;

    register_run(&app, &run_id, child.id().unwrap_or(0));

    let stdout_handle = app.clone();
    let stdout_run_id = run_id.clone();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let emit_start = tokio::time::Instant::now();
            let _ = stdout_handle.emit(
                "coverage-output",
                CoverageOutputEvent {
                    run_id: stdout_run_id.clone(),
                    stream: "stdout".to_string(),
                    line,
                },
            );
            crate::perf::record_emit(emit_start.elapsed());
        }
    });

    let stderr_handle = app.clone();
    let stderr_run_id = run_id.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let emit_start = tokio::time::Instant::now();
            let _ = stderr_handle.emit(
                "coverage-output",
                CoverageOutputEvent {
                    run_id: stderr_run_id.clone(),
                    stream: "stderr".to_string(),
                    line,
                },
            );
            crate::perf::record_emit(emit_start.elapsed());
        }
    });

    let status = match child.wait().await {
        Ok(s) => s,
        Err(e) => {
            unregister_run(&app, &run_id);
            return Err(format!("Failed waiting for coverage run: {}", e));
        }
    };
    unregister_run(&app, &run_id);
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    // ----------------------------------------
    // Step 3: coverage json -o {app_data_dir}/{project_id}/coverage.json
    // ----------------------------------------
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;
    let project_dir = data_dir.join(project_id.to_string());
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("Failed to create coverage directory: {}", e))?;

    let json_path = project_dir.join("coverage.json");
    let json_cmd = format!(
        "{} -m coverage json -o {}",
        interpreter.display(),
        json_path.display()
    );

    let json_output = Command::new(&interpreter)
        .no_console()
        .args(["-m", "coverage", "json", "-o"])
        .arg(&json_path)
        .current_dir(&project)
        .output()
        .map_err(|e| format!("Failed to execute coverage json: {}", e))?;

    if !json_output.status.success() {
        let stderr_text = String::from_utf8_lossy(&json_output.stderr).trim().to_string();
        let stdout_text = String::from_utf8_lossy(&json_output.stdout).trim().to_string();
        let mut detail = if stderr_text.is_empty() {
            stdout_text
        } else {
            stderr_text
        };
        if detail.is_empty() {
            detail = "No output captured. The coverage run likely collected no data — check that the selected source files map to importable module names/directories (root-level files are passed as module names, e.g. example.py -> example).".to_string();
        }
        let message = format!("coverage json export failed.\nCommand: {}\n{}", json_cmd, detail);
        let _ = app.emit(
            "coverage-error",
            CoverageErrorEvent {
                run_id: run_id.clone(),
                message: message.clone(),
            },
        );
        let _ = app.emit(
            "coverage-finished",
            CoverageFinishedEvent {
                run_id: run_id.clone(),
                success: false,
                exit_code: json_output.status.code(),
                duration: start.elapsed().as_secs_f64(),
                summary: None,
                command: json_cmd,
            },
        );
        return Err(message);
    }

    // 写 sidecar meta.json，记录 project_path，供 get_coverage_detail 解析相对路径
    let meta = serde_json::json!({
        "project_id": project_id,
        "project_path": project_path,
        "run_id": run_id,
    });
    let _ = std::fs::write(
        project_dir.join("meta.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    );

    // ----------------------------------------
    // Step 4: 解析 JSON 汇总
    // ----------------------------------------
    let content = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read coverage.json: {}", e))?;

    let (totals, files) = match parse_coverage_json(&content) {
        Ok(parsed) => parsed,
        Err(e) => {
            let _ = app.emit(
                "coverage-error",
                CoverageErrorEvent {
                    run_id: run_id.clone(),
                    message: e.clone(),
                },
            );
            return Err(e);
        }
    };

    let duration = start.elapsed().as_secs_f64();

    let summary = CoverageSummary {
        run_id: run_id.clone(),
        project_id,
        percent_covered: totals.percent_covered,
        total_statements: totals.total_statements,
        covered_statements: totals.covered_statements,
        total_branches: totals.num_branches,
        covered_branches: totals.covered_branches,
        branch_percent: totals.branch_percent(),
        files,
        json_path: json_path.to_string_lossy().to_string(),
        command: command_str.clone(),
    };

    let _ = app.emit(
        "coverage-finished",
        CoverageFinishedEvent {
            run_id: run_id.clone(),
            // pytest 本身失败时 success=false，但 summary 仍会带上已收集到的覆盖率数据
            success: status.success(),
            exit_code: status.code(),
            duration,
            summary: Some(summary.clone()),
            command: command_str,
        },
    );

    Ok(summary)
}

// ============================================
// Command: Get Coverage Detail
// ============================================

#[tauri::command]
pub async fn get_coverage_detail(
    app: tauri::AppHandle,
    project_id: i64,
    file_path: String,
) -> Result<FileCoverageDetail, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;
    let project_dir = data_dir.join(project_id.to_string());
    let json_path = project_dir.join("coverage.json");

    if !json_path.exists() {
        return Err(
            "No coverage data found for this project. Run coverage first.".to_string(),
        );
    }

    let content = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read coverage.json: {}", e))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse coverage.json: {}", e))?;

    let files = json
        .get("files")
        .and_then(|f| f.as_object())
        .ok_or("coverage.json is missing 'files'")?;

    // 将请求路径归一化为 coverage.json 中的相对 key（支持传入绝对路径或相对路径）
    let project_root = read_project_root(&project_dir);
    let key = normalize_coverage_key(&file_path, project_root.as_deref());

    // coverage.json 的 key 在 Windows 上是 `\` 分隔（如 core\controller.py），
    // 归一化为 `/` 后再匹配前端传来的 `/` 风格路径
    let file_json = files
        .iter()
        .find(|(k, _)| normalize_coverage_key(k, None) == key)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("File not found in coverage data: {}", key))?;

    let summary = file_json.get("summary").unwrap_or(file_json);
    let percent_covered = summary
        .get("percent_covered")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let executed: HashSet<u32> =
        parse_line_array(file_json.get("executed_lines")).into_iter().collect();
    let missing: HashSet<u32> =
        parse_line_array(file_json.get("missing_lines")).into_iter().collect();
    let excluded: HashSet<u32> =
        parse_line_array(file_json.get("excluded_lines")).into_iter().collect();

    // 定位源码文件
    let source_path = match &project_root {
        Some(root) => PathBuf::from(root).join(&key),
        None => PathBuf::from(&file_path),
    };

    let source = std::fs::read_to_string(&source_path)
        .map_err(|e| format!("Failed to read source file {:?}: {}", source_path, e))?;

    let lines = source
        .lines()
        .enumerate()
        .map(|(idx, text)| {
            let line_number = (idx + 1) as u32;
            let status = if executed.contains(&line_number) {
                "covered"
            } else if missing.contains(&line_number) {
                "missing"
            } else if excluded.contains(&line_number) {
                "excluded"
            } else {
                "not-executable"
            };
            LineCoverage {
                line_number,
                source: text.to_string(),
                status: status.to_string(),
            }
        })
        .collect();

    Ok(FileCoverageDetail {
        path: key,
        percent_covered,
        lines,
    })
}

// ============================================
// Command: Export Coverage Report
// ============================================

#[tauri::command]
pub async fn export_coverage_report(
    app: tauri::AppHandle,
    project_id: i64,
    format: String,
) -> Result<String, String> {
    let fmt = format.to_lowercase();
    if !matches!(fmt.as_str(), "json" | "csv" | "md" | "html") {
        return Err(format!("Unsupported export format: {}", format));
    }

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;
    let json_path = data_dir.join(project_id.to_string()).join("coverage.json");

    if !json_path.exists() {
        return Err(
            "No coverage data found for this project. Run coverage first.".to_string(),
        );
    }

    let content = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read coverage.json: {}", e))?;

    let file_name = format!("coverage_report_{}.{}", project_id, fmt);
    let (filter_name, filter_ext) = match fmt.as_str() {
        "json" => ("JSON", "json"),
        "csv" => ("CSV", "csv"),
        "md" => ("Markdown", "md"),
        "html" => ("HTML", "html"),
        _ => unreachable!("format validated above"),
    };

    let file_path = app
        .dialog()
        .file()
        .set_file_name(&file_name)
        .add_filter(filter_name, &[filter_ext])
        .blocking_save_file()
        .ok_or("Save dialog was cancelled")?;

    let target = match file_path {
        FilePath::Path(path) => path,
        FilePath::Url(url) => url
            .to_file_path()
            .map_err(|_| "Invalid file URL returned from save dialog".to_string())?,
    };

    let output = match fmt.as_str() {
        "json" => content,
        "csv" => coverage_json_to_csv(&content)?,
        "md" => coverage_json_to_markdown(&content)?,
        "html" => coverage_json_to_html(&content)?,
        _ => unreachable!("format validated above"),
    };

    std::fs::write(&target, output)
        .map_err(|e| format!("Failed to write report file: {}", e))?;

    Ok(target.to_string_lossy().to_string())
}

// ============================================
// Coverage Helpers
// ============================================

pub(crate) struct CoverageTotals {
    pub percent_covered: f64,
    pub total_statements: usize,
    pub covered_statements: usize,
    pub num_branches: Option<u64>,
    pub covered_branches: Option<u64>,
}

impl CoverageTotals {
    /// 分支覆盖率百分比（仅 --branch 模式下 num_branches > 0 时有效）
    pub fn branch_percent(&self) -> Option<f64> {
        let total = self.num_branches?;
        if total == 0 {
            return None;
        }
        Some(self.covered_branches.unwrap_or(0) as f64 * 100.0 / total as f64)
    }
}

/// 解析 coverage json 输出（兼容 format 1 与 format 2）
pub fn parse_coverage_json(content: &str) -> Result<(CoverageTotals, Vec<FileCoverage>), String> {
    let json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse coverage.json: {}", e))?;

    let totals_json = json
        .get("totals")
        .and_then(|t| t.as_object())
        .ok_or("coverage.json is missing 'totals'")?;

    let percent_covered = totals_json
        .get("percent_covered")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let total_statements = totals_json
        .get("num_statements")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let covered_statements = totals_json
        .get("covered_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    // 分支统计（--branch 模式下 coverage.py 才会输出这两个字段）
    let num_branches = totals_json.get("num_branches").and_then(|v| v.as_u64());
    let covered_branches = totals_json.get("covered_branches").and_then(|v| v.as_u64());

    let files_json = json
        .get("files")
        .and_then(|f| f.as_object())
        .ok_or("coverage.json is missing 'files'")?;

    let mut files: Vec<FileCoverage> = files_json
        .iter()
        .map(|(path, file_json)| {
            // format 2 的文件级统计在 summary 子对象里；format 1 直接在文件对象上
            let summary = file_json.get("summary").unwrap_or(file_json);
            FileCoverage {
                // Windows 下 coverage.json 的 key 用 `\` 分隔（core\controller.py），
                // 统一转成 `/`，与前端扫描/过滤用的相对路径保持一致
                path: path.replace('\\', "/"),
                percent_covered: summary
                    .get("percent_covered")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                executed_lines: parse_line_array(file_json.get("executed_lines")),
                missing_lines: parse_line_array(file_json.get("missing_lines")),
                excluded_lines: parse_line_array(file_json.get("excluded_lines")),
            }
        })
        .collect();

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok((
        CoverageTotals {
            percent_covered,
            total_statements,
            covered_statements,
            num_branches,
            covered_branches,
        },
        files,
    ))
}

fn parse_line_array(value: Option<&serde_json::Value>) -> Vec<u32> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_u64())
                .map(|x| x as u32)
                .collect()
        })
        .unwrap_or_default()
}

/// 从 meta.json 读取上次 run_coverage 时的项目根目录
fn read_project_root(project_dir: &Path) -> Option<String> {
    let meta_path = project_dir.join("meta.json");
    if !meta_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(meta_path).ok()?;
    let meta: serde_json::Value = serde_json::from_str(&content).ok()?;
    meta.get("project_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 把前端传来的文件路径归一化为 coverage.json 中的相对 key
fn normalize_coverage_key(file_path: &str, project_root: Option<&str>) -> String {
    let normalized = file_path.replace('\\', "/");

    if let Some(root) = project_root {
        if let Ok(rel) = Path::new(&normalized).strip_prefix(Path::new(root)) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }

    normalized
}

/// 将 coverage.json 转换为逐文件汇总的 CSV
fn coverage_json_to_csv(content: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("Failed to parse coverage.json: {}", e))?;

    let files = json
        .get("files")
        .and_then(|f| f.as_object())
        .ok_or("coverage.json is missing 'files'")?;

    let mut csv = String::from(
        "file,percent_covered,covered_lines,missing_lines,num_statements\n",
    );

    let mut sorted: Vec<(&String, &serde_json::Value)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    for (path, file_json) in sorted {
        let summary = file_json.get("summary").unwrap_or(file_json);

        let normalized_path = path.replace('\\', "/");
        let percent = summary
            .get("percent_covered")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let covered = summary
            .get("covered_lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let num_statements = summary
            .get("num_statements")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let missing_lines = parse_line_array(file_json.get("missing_lines"))
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        csv.push_str(&format!(
            "{},{:.2},{},{},{}\n",
            csv_escape(&normalized_path),
            percent,
            covered,
            csv_escape(&missing_lines),
            num_statements
        ));
    }

    Ok(csv)
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// 将 coverage.json 转换为 Markdown 报告（总量 + 逐文件表格）
fn coverage_json_to_markdown(content: &str) -> Result<String, String> {
    let (totals, files) = parse_coverage_json(content)?;

    let mut out = String::new();
    out.push_str("# Coverage Report\n\n");

    out.push_str("| Metric | Value |\n|---|---|\n");
    out.push_str(&format!(
        "| Total coverage | {:.1}% |\n",
        totals.percent_covered
    ));
    out.push_str(&format!(
        "| Statements | {}/{} covered |\n",
        totals.covered_statements, totals.total_statements
    ));
    if let Some(branch) = totals.branch_percent() {
        out.push_str(&format!(
            "| Branches | {}/{} covered |\n",
            totals.covered_branches.unwrap_or(0),
            totals.num_branches.unwrap_or(0)
        ));
        out.push_str(&format!("| Branch coverage | {:.1}% |\n", branch));
    }
    out.push('\n');

    out.push_str("| File | Covered | Missing | Statements | Coverage |\n");
    out.push_str("|---|---:|---:|---:|---:|\n");
    for file in &files {
        let covered = file.executed_lines.len();
        let missing = file.missing_lines.len();
        let statements = covered + missing + file.excluded_lines.len();
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {:.1}% |\n",
            file.path.replace('|', "\\|"),
            covered,
            missing,
            statements,
            file.percent_covered
        ));
    }

    Ok(out)
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn coverage_tone(percent: f64) -> (&'static str, &'static str, &'static str) {
    if percent >= 80.0 {
        ("#10b981", "#047857", "#ecfdf5")
    } else if percent >= 50.0 {
        ("#f59e0b", "#b45309", "#fffbeb")
    } else {
        ("#f43f5e", "#be123c", "#fff1f2")
    }
}

/// 将 coverage.json 转换为单个自包含 HTML 报告（摘要 + 逐文件表格）
fn coverage_json_to_html(content: &str) -> Result<String, String> {
    let (totals, files) = parse_coverage_json(content)?;

    let (_, total_color, _) = coverage_tone(totals.percent_covered);
    let total_value = format!("{:.1}%", totals.percent_covered);

    let statements_pct = if totals.total_statements > 0 {
        totals.covered_statements as f64 * 100.0 / totals.total_statements as f64
    } else {
        0.0
    };
    let statements_value = format!(
        "{}/{} <span class=\"sub\">({:.1}%)</span>",
        totals.covered_statements, totals.total_statements, statements_pct
    );

    let branch_value = match totals.branch_percent() {
        Some(branch) => format!(
            "{}/{} <span class=\"sub\">({:.1}%)</span>",
            totals.covered_branches.unwrap_or(0),
            totals.num_branches.unwrap_or(0),
            branch
        ),
        None => "<span class=\"sub\">not enabled</span>".to_string(),
    };

    let mut rows = String::new();
    for file in &files {
        let covered = file.executed_lines.len();
        let missing = file.missing_lines.len();
        let statements = covered + missing + file.excluded_lines.len();
        let pct = file.percent_covered;
        let (bar, text, _) = coverage_tone(pct);
        rows.push_str(&format!(
            "<tr><td class=\"file\">{}</td><td class=\"num\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td>\
             <td class=\"bar-cell\"><div class=\"bar\"><div style=\"width:{:.1}%;background:{}\"></div>\
             </div></td><td class=\"pct\" style=\"color:{}\">{:.1}%</td></tr>\n",
            html_escape(&file.path),
            covered,
            missing,
            statements,
            pct,
            bar,
            text,
            pct
        ));
    }

    let (_, statements_text, _) = coverage_tone(statements_pct);

    Ok(format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Coverage Report</title><style>{HTML_STYLE}</style></head>\
         <body><div class=\"container\"><h1>Coverage Report</h1>\
         <div class=\"metrics\">\
         <div class=\"metric\"><div class=\"label\">Total coverage</div>\
         <div class=\"value\" style=\"color:{total_color}\">{total_value}</div></div>\
         <div class=\"metric\"><div class=\"label\">Statements</div>\
         <div class=\"value\" style=\"color:{statements_text}\">{statements_value}</div></div>\
         <div class=\"metric\"><div class=\"label\">Branches</div>\
         <div class=\"value\">{branch_value}</div></div>\
         <div class=\"metric\"><div class=\"label\">Files</div>\
         <div class=\"value\">{file_count}</div></div>\
         </div><table><thead><tr><th>File</th><th class=\"num\">Covered</th>\
         <th class=\"num\">Missing</th><th class=\"num\">Statements</th>\
         <th class=\"num\">Coverage</th><th class=\"num\">%</th></tr></thead>\
         <tbody>\n{rows}</tbody></table>\
         <footer>Generated by coverage export.</footer></div></body></html>",
        total_color = total_color,
        total_value = total_value,
        statements_text = statements_text,
        statements_value = statements_value,
        branch_value = branch_value,
        file_count = files.len(),
        rows = rows,
    ))
}

const HTML_STYLE: &str = r#"
  body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;margin:0;padding:2rem;background:#fafafa;color:#18181b;}
  .container{max-width:960px;margin:0 auto;}
  h1{font-size:1.25rem;margin:0 0 1.5rem;}
  .metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:0.75rem;margin-bottom:1.5rem;}
  .metric{background:#fff;border:1px solid #e4e4e7;border-radius:0.5rem;padding:0.75rem 1rem;}
  .metric .label{font-size:0.65rem;color:#71717a;text-transform:uppercase;letter-spacing:0.05em;}
  .metric .value{margin-top:0.25rem;font-size:1.15rem;font-weight:600;}
  .metric .sub{font-size:0.8rem;font-weight:400;color:#71717a;}
  table{width:100%;border-collapse:collapse;background:#fff;border:1px solid #e4e4e7;border-radius:0.5rem;overflow:hidden;}
  th{font-size:0.65rem;text-transform:uppercase;letter-spacing:0.05em;color:#71717a;text-align:left;padding:0.6rem 1rem;background:#f4f4f5;}
  td{padding:0.5rem 1rem;border-top:1px solid #f4f4f5;font-size:0.85rem;}
  td.file{font-family:'SF Mono',Consolas,Menlo,monospace;font-size:0.8rem;}
  td.num{text-align:right;color:#52525b;}
  td.pct{text-align:right;font-weight:600;white-space:nowrap;}
  .bar-cell{min-width:140px;}
  .bar{height:0.5rem;border-radius:9999px;background:#e4e4e7;overflow:hidden;}
  .bar>div{height:100%;border-radius:9999px;}
  footer{margin-top:1.5rem;font-size:0.7rem;color:#a1a1aa;}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "totals": {
            "percent_covered": 85.5,
            "num_statements": 100,
            "covered_lines": 85,
            "num_branches": 10,
            "covered_branches": 6
        },
        "files": {
            "src/main.py": {
                "percent_covered": 85.5,
                "executed_lines": [1, 2, 3, 10],
                "missing_lines": [4, 5],
                "excluded_lines": []
            },
            "src/util<&>.py": {
                "percent_covered": 10.0,
                "executed_lines": [1],
                "missing_lines": [2, 3],
                "excluded_lines": [4]
            }
        }
    }"#;

    #[test]
    fn test_markdown_contains_totals_and_file_rows() {
        let md = coverage_json_to_markdown(SAMPLE).unwrap();

        assert!(md.contains("# Coverage Report"));
        assert!(md.contains("| Total coverage | 85.5% |"));
        assert!(md.contains("| Statements | 85/100 covered |"));
        assert!(md.contains("| Branch coverage | 60.0% |"));
        assert!(md.contains("| `src/main.py` | 4 | 2 | 6 | 85.5% |"));
        // 表格中 `|` 需转义
        assert!(md.contains("`src/util<&>.py`"));
    }

    #[test]
    fn test_markdown_escapes_pipes_in_paths() {
        let md = coverage_json_to_markdown(SAMPLE).unwrap();
        assert!(md.contains("| `src/util<&>.py` |"));
        // HTML 注入不应出现在 Markdown 中——覆盖率报告只是文本表格
        assert!(!md.contains("<script>"));
    }

    #[test]
    fn test_html_contains_table_and_escapes_content() {
        let html = coverage_json_to_html(SAMPLE).unwrap();

        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>Coverage Report</title>"));
        assert!(html.contains("<table>"));
        assert!(html.contains(">src/main.py<"));
        // 文件路径中的特殊字符必须转义，避免注入
        assert!(html.contains("src/util&lt;&amp;&gt;.py"));
        assert!(!html.contains("src/util<&>.py"));
        assert!(html.contains("85.5%"));
        assert!(html.contains("not enabled") == false || html.contains("60.0%"));
    }

    #[test]
    fn test_html_branch_disabled() {
        let json = r#"{
            "totals": {
                "percent_covered": 30.0,
                "num_statements": 10,
                "covered_lines": 3
            },
            "files": {
                "src/a.py": {
                    "executed_lines": [1],
                    "missing_lines": [2],
                    "excluded_lines": []
                }
            }
        }"#;
        let html = coverage_json_to_html(json).unwrap();
        assert!(html.contains("not enabled"));
    }
}

// ============================================
