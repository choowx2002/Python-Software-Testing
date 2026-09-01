use std::path::Path;
use std::process::Command;

use crate::proc::NoConsole;

/// 读取文本文件内容（带大小限制，用于生成测试预览）
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    const MAX_BYTES: u64 = 512 * 1024; // 512 KB
    let target = Path::new(&path);
    if !target.exists() {
        return Err(format!("File does not exist: {}", path));
    }
    let meta = std::fs::metadata(target)
        .map_err(|e| format!("Failed to read file metadata: {}", e))?;
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "File is too large to preview ({} bytes > {} bytes limit)",
            meta.len(),
            MAX_BYTES
        ));
    }
    std::fs::read_to_string(target)
        .map_err(|e| format!("Failed to read file: {}", e))
}

/// 通过保存对话框将内容写入文件（用于历史导出等）
#[tauri::command]
pub async fn save_text_file(
    app: tauri::AppHandle,
    default_file_name: String,
    content: String,
    filter_name: String,
    extensions: Vec<String>,
) -> Result<String, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let file_path = app
        .dialog()
        .file()
        .set_file_name(&default_file_name)
        .add_filter(&filter_name, &extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .blocking_save_file()
        .ok_or("Save dialog was cancelled")?;

    let target = match file_path {
        FilePath::Path(path) => path,
        FilePath::Url(url) => url
            .to_file_path()
            .map_err(|_| "Invalid file URL returned from save dialog".to_string())?,
    };

    std::fs::write(&target, content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(target.to_string_lossy().to_string())
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
/// 将编辑器命令模板（含 {path}/{line} 占位符）展开为参数序列。
///
/// - `{path}` 始终替换为文件绝对路径（含空格也保持为单个参数，不经过 shell）；
/// - `{line}` 已知时替换为行号；
/// - `{line}` 未知时按规则移除（`:12` / `+12` / `--line 12` / `-l 12` 一并去掉）。
fn expand_editor_template(template: &str, path: &str, line: Option<u32>) -> Vec<String> {
    const SENTINEL: &str = "__TESTMATE_PATH__";
    let mut cmd = template.replace("{path}", SENTINEL);
    match line {
        Some(n) => {
            cmd = cmd.replace("{line}", &n.to_string());
        }
        None => {
            cmd = cmd
                .replace(":{line}", "")
                .replace("+{line}", "")
                .replace("--line {line}", "")
                .replace("-l {line}", "")
                .replace("{line}", "");
        }
    }
    cmd.split_whitespace()
        .map(|token| token.replace(SENTINEL, path))
        .collect()
}

#[tauri::command]
pub async fn open_file(
    path: String,
    editor: Option<String>,
    line: Option<u32>,
) -> Result<(), String> {
    let target = Path::new(&path);

    if !target.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    // 用户配置了自定义编辑器：按模板展开并启动（不经过 shell）
    if let Some(template) = editor.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let tokens = expand_editor_template(template, &path, line);
        if tokens.is_empty() {
            return Err("Editor command template is empty".to_string());
        }
        let program = &tokens[0];
        let args = &tokens[1..];

        #[cfg(target_os = "linux")]
        {
            Command::new(program)
                .args(args)
                .spawn()
                .map_err(|e| format!("Failed to launch editor '{}': {}", program, e))?;
        }
        #[cfg(target_os = "macos")]
        {
            Command::new(program)
                .args(args)
                .spawn()
                .map_err(|e| format!("Failed to launch editor '{}': {}", program, e))?;
        }
        #[cfg(target_os = "windows")]
        {
            Command::new(program)
                .no_console()
                .args(args)
                .spawn()
                .map_err(|e| format!("Failed to launch editor '{}': {}", program, e))?;
        }

        return Ok(());
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
            .no_console()
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }

    Ok(())
}
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

#[cfg(test)]
mod tests {
    use super::expand_editor_template;

    #[test]
    fn expands_path_with_line() {
        let tokens = expand_editor_template(
            "code -g {path}:{line}",
            "/tmp/my dir/test_x.py",
            Some(12),
        );
        assert_eq!(tokens, vec!["code", "-g", "/tmp/my dir/test_x.py:12"]);
    }

    #[test]
    fn removes_line_placeholder_when_unknown() {
        let tokens = expand_editor_template(
            "code -g {path}:{line}",
            "/tmp/test_x.py",
            None,
        );
        assert_eq!(tokens, vec!["code", "-g", "/tmp/test_x.py"]);
    }

    #[test]
    fn removes_line_flag_when_unknown() {
        let tokens = expand_editor_template(
            "pycharm --line {line} {path}",
            "/tmp/test_x.py",
            None,
        );
        assert_eq!(tokens, vec!["pycharm", "/tmp/test_x.py"]);
    }

    #[test]
    fn expands_plus_line_syntax() {
        let tokens = expand_editor_template(
            "nvim +{line} {path}",
            "/tmp/test_x.py",
            Some(7),
        );
        assert_eq!(tokens, vec!["nvim", "+7", "/tmp/test_x.py"]);
    }
}
