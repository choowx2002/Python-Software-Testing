mod commands;
mod db;
mod perf;
mod proc;
mod state;

use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // NFR003/NFR004 取证：TESTMATE_PERF=1 时启动性能采样（无前端改动）
            crate::perf::start_memory_sampler();

            // 启动时清理上次崩溃残留的子进程（PID 文件）
            commands::cleanup_orphan_pids(app.handle());

            let show = MenuItem::with_id(app, "show", "显示 Testmate", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "隐藏到托盘", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&show, &hide, &PredefinedMenuItem::separator(app)?, &quit],
            )?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Testmate")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::MouseButtonState;
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
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
            commands::env::scan_missing_imports,
            commands::env::fix_python_env,
            commands::env::install_dependencies,
            commands::env::validate_project_directory,
            commands::env::create_virtual_env,
            commands::env::clone_repository,
            commands::env::validate_project_interpreter,
            commands::files::open_in_file_manager,
            commands::generation::check_generation_blockers,
            commands::generation::scan_source_files,
            commands::generation::generate_tests,
            commands::generation::check_python_bom,
            commands::generation::strip_python_bom,
            commands::files::open_file,
            commands::files::reveal_in_folder,
            commands::files::read_text_file,
            commands::files::save_text_file,
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
            commands::db_commands::save_execution_result_details,
            commands::db_commands::list_execution_result_details,
            commands::db_commands::get_execution_project_path,
            commands::db_commands::save_generation_file_details,
            commands::db_commands::list_generation_file_details,
            commands::db_commands::get_coverage_history_files,
            commands::db_commands::clear_all_history,
            commands::window::set_taskbar_progress,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            app_handle.state::<state::AppState>().kill_all();
            commands::clear_persisted_pids(&app_handle);
        }
    });
}