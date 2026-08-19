mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_test_files,
            commands::collect_test_cases,
            commands::run_tests,
            commands::check_coverage_installed,
            commands::run_coverage,
            commands::get_coverage_detail,
            commands::export_coverage_report,
            commands::detect_python_env,
            commands::install_dependencies,
            commands::validate_project_directory,
            commands::create_virtual_env,
            commands::open_in_file_manager,
            commands::scan_source_files,
            commands::generate_tests,
            commands::open_file,
            commands::reveal_in_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
