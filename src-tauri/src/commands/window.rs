#[tauri::command]
pub async fn set_taskbar_progress(window: tauri::Window, progress: u8, state: String) {
    #[cfg(target_os = "windows")]
    {
        let status = match state.as_str() {
            "indeterminate" => tauri::window::ProgressBarStatus::Indeterminate,
            "error" => tauri::window::ProgressBarStatus::Error,
            "paused" => tauri::window::ProgressBarStatus::Paused,
            _ => tauri::window::ProgressBarStatus::Normal,
        };
        let _ = window.set_progress_bar(tauri::window::ProgressBarState {
            progress: Some(progress as u64),
            status: Some(status),
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (&window, progress, state);
    }
}