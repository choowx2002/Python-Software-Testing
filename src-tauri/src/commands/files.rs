use std::path::Path;
use std::process::Command;
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
