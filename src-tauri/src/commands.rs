// src-tauri/src/commands.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

// ============================================
// Data Structures
// ============================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvDetectionResult {
    pub python_path: Option<String>,
    pub python_version: Option<String>,
    pub venv_path: Option<String>,
    pub venv_activated: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub installed: Vec<String>,
    pub failed: Vec<String>,
    pub logs: String,
}

// ============================================
// Command 1: Detect Python Environment
// ============================================
#[tauri::command]
pub async fn detect_python_env(project_path: String) -> Result<EnvDetectionResult, String> {
    let path = PathBuf::from(&project_path);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid project directory".to_string());
    }

    let mut result = EnvDetectionResult {
        python_path: None,
        python_version: None,
        venv_path: None,
        venv_activated: false,
        dependencies: vec![],
    };

    // 1. Detect virtual environment (try common names)
    let venv_candidates = ["venv", ".venv", "env", ".env"];
    for candidate in &venv_candidates {
        let venv_dir = path.join(candidate);
        if venv_dir.exists() {
            result.venv_path = Some(venv_dir.to_string_lossy().to_string());
            
            // Find python executable inside venv
            let python_in_venv = if cfg!(target_os = "windows") {
                venv_dir.join("Scripts").join("python.exe")
            } else {
                venv_dir.join("bin").join("python")
            };
            
            if python_in_venv.exists() {
                result.python_path = Some(python_in_venv.to_string_lossy().to_string());
                result.venv_activated = true;
                break;
            }
        }
    }

    // 2. Fallback to system Python if no venv found
    if result.python_path.is_none() {
        let system_python = if cfg!(target_os = "windows") {
            "python"
        } else {
            "python3"
        };
        
        if let Ok(output) = Command::new(system_python).arg("--version").output() {
            if output.status.success() {
                result.python_path = Some(system_python.to_string());
            }
        }
    }

    // 3. Get Python version
    if let Some(python_path) = &result.python_path {
        if let Ok(output) = Command::new(python_path).arg("--version").output() {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                result.python_version = Some(version_str);
            }
        }
    }

    // 4. Check required dependencies
    let required_deps = ["pytest", "coverage", "pynguin"];
    if let Some(python_path) = &result.python_path {
        for dep in &required_deps {
            let mut dep_status = DependencyStatus {
                name: dep.to_string(),
                installed: false,
                version: None,
            };
            
            // Use pip show to check if package is installed
            if let Ok(output) = Command::new(python_path)
                .args(["-m", "pip", "show", dep])
                .output()
            {
                if output.status.success() {
                    dep_status.installed = true;
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse version from "Version: x.y.z"
                    for line in stdout.lines() {
                        if line.starts_with("Version:") {
                            dep_status.version = Some(line.trim_start_matches("Version:").trim().to_string());
                            break;
                        }
                    }
                }
            }
            
            result.dependencies.push(dep_status);
        }
    }

    Ok(result)
}

// ============================================
// Command 2: Install Missing Dependencies
// ============================================
#[tauri::command]
pub async fn install_dependencies(
    python_path: String,
    packages: Vec<String>,
) -> Result<InstallResult, String> {
    if packages.is_empty() {
        return Ok(InstallResult {
            success: true,
            installed: vec![],
            failed: vec![],
            logs: "No packages to install".to_string(),
        });
    }

    let mut installed = vec![];
    let mut failed = vec![];
    let mut all_logs = String::new();

    for package in &packages {
        let output = Command::new(&python_path)
            .args(["-m", "pip", "install", package])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                installed.push(package.clone());
                all_logs.push_str(&format!("[OK] {} installed successfully\n", package));
            }
            Ok(out) => {
                failed.push(package.clone());
                let stderr = String::from_utf8_lossy(&out.stderr);
                all_logs.push_str(&format!("[FAIL] {} installation failed: {}\n", package, stderr));
            }
            Err(e) => {
                failed.push(package.clone());
                all_logs.push_str(&format!("[FAIL] {} error: {}\n", package, e));
            }
        }
    }

    Ok(InstallResult {
        success: failed.is_empty(),
        installed,
        failed,
        logs: all_logs,
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
    
    // Check if it looks like a Python project (has at least one .py file)
    let has_python_files = std::fs::read_dir(&path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .any(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "py")
        });
    
    if !has_python_files {
        return Err("No Python files (.py) found in the selected directory".to_string());
    }
    
    Ok(true)
}