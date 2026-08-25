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
            commands::execution::scan_test_files,
            commands::execution::collect_test_cases,
            commands::execution::run_tests,
            commands::execution::cancel_run,
            commands::suites::save_regression_suite,
            commands::suites::list_regression_suites,
            commands::suites::delete_regression_suite,
            commands::suites::run_regression_suite,
            commands::coverage::check_coverage_installed,
            commands::coverage::run_coverage,
            commands::coverage::get_coverage_detail,
            commands::coverage::export_coverage_report,
            commands::env::detect_python_env,
            commands::env::check_generation_env,
            commands::env::fix_python_env,
            commands::env::install_dependencies,
            commands::env::validate_project_directory,
            commands::env::create_virtual_env,
            commands::env::clone_repository,
            commands::files::open_in_file_manager,
            commands::generation::scan_source_files,
            commands::generation::generate_tests,
            commands::generation::check_python_bom,
            commands::generation::strip_python_bom,
            commands::files::open_file,
            commands::files::reveal_in_folder,
            commands::db_commands::init_db,
            commands::db_commands::get_projects,
            commands::db_commands::add_project,
            commands::db_commands::update_last_opened,
            commands::db_commands::update_project_name,
            commands::db_commands::delete_project,
            commands::db_commands::save_execution_history,
            commands::db_commands::save_coverage_result,
            commands::db_commands::count_execution_history,
            commands::db_commands::get_global_stats,
            commands::db_commands::list_execution_history,
            commands::db_commands::save_generation_history,
            commands::db_commands::list_generation_history,
            commands::db_commands::save_coverage_history,
            commands::db_commands::list_coverage_history,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            app_handle.state::<state::AppState>().kill_all();
        }
    });
}