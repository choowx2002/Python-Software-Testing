mod commands;
mod db;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_test_files,
            commands::collect_test_cases,
            commands::run_tests,
            commands::cancel_run,
            commands::save_regression_suite,
            commands::list_regression_suites,
            commands::delete_regression_suite,
            commands::run_regression_suite,
            commands::check_coverage_installed,
            commands::run_coverage,
            commands::get_coverage_detail,
            commands::export_coverage_report,
            commands::detect_python_env,
            commands::install_dependencies,
            commands::validate_project_directory,
            commands::create_virtual_env,
            commands::clone_repository,
            commands::open_in_file_manager,
            commands::scan_source_files,
            commands::generate_tests,
            commands::open_file,
            commands::reveal_in_folder,
            commands::init_db,
            commands::get_projects,
            commands::add_project,
            commands::update_last_opened,
            commands::update_project_name,
            commands::delete_project,
            commands::save_execution_history,
            commands::save_coverage_result,
            commands::count_execution_history,
            commands::get_global_stats,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            app_handle.state::<state::AppState>().kill_all();
        }
    });
}