// src-tauri/src/commands.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::Emitter;

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