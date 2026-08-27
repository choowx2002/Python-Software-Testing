#[tauri::command]
pub async fn set_taskbar_progress(window: tauri::Window, progress: u8, state: String) {
    #[cfg(target_os = "windows")]
    {
        let _ = window.set_progress_bar(progress as i32, match state.as_str() {
            "indeterminate" => tauri::ProgressBarState::Indeterminate,
            "error" => tauri::ProgressBarState::Error,
            "paused" => tauri::ProgressBarState::Paused,
            _ => tauri::ProgressBarState::Normal,
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (&window, progress, state);
    }
}