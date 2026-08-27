use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use tauri::Emitter;

use super::*;
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
        // Windows 下 GUI 进程可能拿不到完整 PATH（如 MS Store 别名、仅装 py 启动器），
        // 依次尝试 python / python3 / py（Windows launcher）
        let candidates: &[(&str, &[&str])] = if cfg!(target_os = "windows") {
            &[("python", &[]), ("python3", &[]), ("py", &["-3"])]
        } else {
            &[("python3", &[]), ("python", &[])]
        };

        for (cmd, args) in candidates {
            let mut command = Command::new(cmd);
            command.args(*args).arg("--version");

            let Ok(output) = command.output() else {
                continue;
            };
            if !output.status.success() {
                continue;
            }

            let version = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            } else {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            };
            result.python_path = Some(cmd.to_string());
            result.python_version = Some(version);
            break;
        }
        // 注意：如果所有候选都失败（未安装/不在 PATH），python_path 保持 None，
        // 前端根据 python_path.is_none() 提示用户安装 Python。
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
            // 兼容 UTF-8 BOM（Windows 记事本 / VS Code 常见）：
            // 否则首行包名会带上 \u{feff}，导致 "Invalid requirement: '\ufeffpytest'"
            let content = content.trim_start_matches('\u{feff}');

            for line in content.lines() {
                let line = line.trim().trim_start_matches('\u{feff}');
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

/// 从 pip 的 stderr 中提取最可能说明问题的末尾几行（去掉空行，最长 400 字符）
fn summarize_pip_error(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let tail: Vec<&str> = lines.iter().rev().take(4).copied().collect();
    let mut summary = tail.iter().rev().cloned().collect::<Vec<_>>().join("\n");
    if summary.len() > 400 {
        summary = summary.chars().take(400).collect::<String>() + "…";
    }
    summary
}

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
            failed_reasons: vec![],
        });
    }

    let mut installed = vec![];
    let mut failed = vec![];
    let mut failed_reasons = vec![];

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
            failed_reasons.push(summarize_pip_error(&output.stderr));

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
        failed_reasons,
    })
}

// ============================================
// 生成环境健康检查（Python / pynguin / bytecode 版本）
// 用于 Generate 页提前提示已知兼容性问题
// ============================================
#[tauri::command]
pub async fn check_generation_env(
    interpreter_path: String,
) -> Result<GenerationEnvInfo, String> {
    let script = concat!(
        "import sys, importlib.metadata as m\n",
        "print(sys.version.split()[0])\n",
        "for pkg in ('pynguin', 'bytecode'):\n",
        "    try:\n",
        "        print(pkg + '=' + m.version(pkg))\n",
        "    except Exception:\n",
        "        print(pkg + '=missing')\n",
    );

    let output = Command::new(&interpreter_path)
        .args(["-c", script])
        .output()
        .map_err(|e| format!("Failed to run python: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut python_version = String::new();
    let mut pynguin_version: Option<String> = None;
    let mut bytecode_version: Option<String> = None;

    for (i, line) in stdout.lines().enumerate() {
        if i == 0 {
            python_version = line.trim().to_string();
            continue;
        }
        if let Some((pkg, ver)) = line.trim().split_once('=') {
            if ver == "missing" {
                continue;
            }
            match pkg {
                "pynguin" => pynguin_version = Some(ver.to_string()),
                "bytecode" => bytecode_version = Some(ver.to_string()),
                _ => {}
            }
        }
    }

    Ok(GenerationEnvInfo {
        python_version,
        pynguin_version,
        bytecode_version,
    })
}

// ============================================
// 一键修复生成环境：安装 Python 3.11 → 重建 venv → 装依赖 → 更新项目
// ============================================

/// 定位可用的 Python 3.11 解释器（py launcher → 已知安装路径）
fn resolve_python_311() -> Option<String> {
    // 1) py launcher
    if let Ok(output) = Command::new("py")
        .args(["-3.11", "-c", "import sys; print(sys.executable)"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    // 2) 已知的 per-user 安装路径
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let p = PathBuf::from(local)
            .join("Programs")
            .join("Python")
            .join("Python311")
            .join("python.exe");
        if p.exists() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

/// 一键修复：winget 安装 Python 3.11（如需）→ 重建 .venv → 安装依赖 → 更新项目解释器
/// project_id 为空时（Import 流程）跳过数据库更新，由前端重新检测。
/// 进度通过 "env-fix-step" 事件逐阶段推送。
#[tauri::command]
pub async fn fix_python_env(
    app: tauri::AppHandle,
    project_path: String,
    project_id: Option<i64>,
) -> Result<String, String> {
    let emit = |stage: &str, status: &str, message: &str| {
        let _ = app.emit(
            "env-fix-step",
            crate::commands::EnvFixStep {
                stage: stage.to_string(),
                status: status.to_string(),
                message: message.to_string(),
            },
        );
    };

    // 1) 定位 Python 3.11；缺失则用 winget 静默安装
    if resolve_python_311().is_none() {
        emit("winget", "running", "Installing Python 3.11 via winget...");
        let output = AsyncCommand::new("winget")
            .args([
                "install",
                "-e",
                "--id",
                "Python.Python.3.11",
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run winget: {}. Please install Python 3.11 manually from python.org.", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to install Python 3.11 via winget: {}. Please install it manually from python.org.",
                stderr.trim()
            ));
        }
        emit("winget", "success", "Python 3.11 installed.");
    } else {
        emit("winget", "success", "Python 3.11 already installed.");
    }

    let py311 = resolve_python_311()
        .ok_or_else(|| "Python 3.11 not found after installation. Please install it manually from python.org.".to_string())?;

    let root = PathBuf::from(&project_path);
    if !root.is_dir() {
        return Err(format!("Project directory not found: {}", project_path));
    }
    let venv_dir = root.join(".venv");

    // 2) 删除旧 venv
    if venv_dir.exists() {
        emit("venv", "running", "Removing old .venv...");
        std::fs::remove_dir_all(&venv_dir)
            .map_err(|e| format!("Failed to remove old .venv: {}", e))?;
    }

    // 3) 用 3.11 创建新 venv
    emit("venv", "running", "Creating .venv with Python 3.11...");
    let output = AsyncCommand::new(&py311)
        .args(["-m", "venv", ".venv"])
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| format!("Failed to run python -m venv: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to create .venv: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    emit("venv", "success", ".venv created.");

    let venv_python = if cfg!(target_os = "windows") {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    };
    if !venv_python.exists() {
        return Err("New .venv has no python executable.".to_string());
    }

    // 4) 安装依赖（有 requirements.txt 则按它装，否则装默认三件套）
    emit("deps", "running", "Installing project dependencies...");
    let req = root.join("requirements.txt");
    let mut pip = AsyncCommand::new(&venv_python);
    pip.arg("-m").arg("pip").arg("install");
    if req.exists() {
        pip.args(["-r", "requirements.txt"]);
    } else {
        pip.args(["pytest", "coverage", "pynguin"]);
    }
    let output = pip
        .current_dir(&root)
        .output()
        .await
        .map_err(|e| format!("Failed to run pip install: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to install dependencies: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .take(5)
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    emit("deps", "success", "Dependencies installed.");

    // 5) 更新项目解释器（Import 流程无 project_id 则跳过）
    if let Some(pid) = project_id {
        emit("db", "running", "Updating project interpreter...");
        crate::db::update_project_interpreter(&app, pid, &venv_python.to_string_lossy()).await?;
        emit("db", "success", "Project interpreter updated.");
    }

    emit("done", "success", "Environment ready.");
    Ok(venv_python.to_string_lossy().to_string())
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
// Command: Clone Repository
// ============================================

/// 克隆 Git 仓库到指定目录（git clone），返回克隆出的项目目录绝对路径
#[tauri::command]
pub async fn clone_repository(
    repo_url: String,
    target_dir: String,
) -> Result<String, String> {
    let url = repo_url.trim();
    if url.is_empty() {
        return Err("Repository URL must not be empty.".to_string());
    }
    let is_supported = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("ssh://")
        || url.starts_with("git@");
    if !is_supported {
        return Err(
            "Unsupported repository URL format. Use https://, http://, ssh:// or git@."
                .to_string(),
        );
    }

    let target = PathBuf::from(&target_dir);
    if !target.is_dir() {
        return Err(format!("Target directory does not exist: {}", target_dir));
    }

    let output = AsyncCommand::new("git")
        .args(["clone", url])
        .current_dir(&target)
        .output()
        .await
        .map_err(|e| format!("Failed to run git clone (is git installed?): {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "git clone failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let trimmed = url.trim_end_matches('/');
    let repo_name = trimmed
        .rsplit('/')
        .next()
        .unwrap_or("repository")
        .trim_end_matches(".git")
        .to_string();

    Ok(target.join(repo_name).to_string_lossy().to_string())
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

// ============================================
// 启动校验：项目解释器路径有效性 + 自动恢复
// ============================================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InterpreterValidation {
    /// "ok"（已存路径有效）| "recovered"（已重探测并更新）| "missing"（找不到）
    pub status: String,
    pub interpreter_path: Option<String>,
}

/// 依次尝试项目 venv，再尝试全局 Python，返回首个可用的解释器
fn find_interpreter_for_project(project: &Path) -> Option<String> {
    for candidate in [".venv", "venv"] {
        let python = if cfg!(target_os = "windows") {
            project.join(candidate).join("Scripts").join("python.exe")
        } else {
            project.join(candidate).join("bin").join("python")
        };
        if python.is_file() {
            return Some(python.to_string_lossy().to_string());
        }
    }

    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "windows") {
        &[("python", &[]), ("python3", &[]), ("py", &["-3"])]
    } else {
        &[("python3", &[]), ("python", &[])]
    };
    for (cmd, args) in candidates {
        if let Ok(output) = Command::new(cmd).args(*args).arg("--version").output() {
            if output.status.success() {
                return Some(cmd.to_string());
            }
        }
    }
    None
}

/// 校验项目解释器路径；失效则自动重探测并更新数据库。
/// 返回 "ok" / "recovered" / "missing"。
#[tauri::command]
pub async fn validate_project_interpreter(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<InterpreterValidation, String> {
    let (project_path, interpreter) = crate::db::get_project_interpreter(&app, project_id).await?;

    if let Some(interp) = interpreter {
        if Path::new(&interp).is_file() {
            return Ok(InterpreterValidation {
                status: "ok".to_string(),
                interpreter_path: Some(interp),
            });
        }
    }

    let project = PathBuf::from(&project_path);
    if let Some(found) = find_interpreter_for_project(&project) {
        crate::db::update_project_interpreter(&app, project_id, &found).await?;
        return Ok(InterpreterValidation {
            status: "recovered".to_string(),
            interpreter_path: Some(found),
        });
    }

    Ok(InterpreterValidation {
        status: "missing".to_string(),
        interpreter_path: None,
    })
}

// ============================================
