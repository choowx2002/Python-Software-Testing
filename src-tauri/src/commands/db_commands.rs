// Command: Typed Persistence Layer (NFR008)
// ============================================

/// 初始化数据库表结构（幂等，含旧 schema 迁移）
#[tauri::command]
pub async fn init_db(app: tauri::AppHandle) -> Result<(), String> {
    crate::db::init(&app).await
}

/// 获取项目列表（附带执行历史 / 覆盖率聚合统计）
#[tauri::command]
pub async fn get_projects(
    app: tauri::AppHandle,
) -> Result<Vec<crate::db::ProjectRow>, String> {
    crate::db::get_projects(&app).await
}

/// 新增项目（重复路径拦截），返回项目 id
#[tauri::command]
pub async fn add_project(
    app: tauri::AppHandle,
    name: String,
    project_path: String,
    interpreter_path: Option<String>,
) -> Result<i64, String> {
    crate::db::add_project(&app, &name, &project_path, interpreter_path).await
}

#[tauri::command]
pub async fn update_last_opened(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<(), String> {
    crate::db::update_last_opened(&app, project_id).await
}

#[tauri::command]
pub async fn update_project_name(
    app: tauri::AppHandle,
    project_id: i64,
    new_name: String,
) -> Result<(), String> {
    crate::db::update_project_name(&app, project_id, &new_name).await
}

#[tauri::command]
pub async fn delete_project(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<(), String> {
    crate::db::delete_project(&app, project_id).await
}

/// 写入测试执行历史（execution_type: MANUAL / REGRESSION），返回记录 id
#[tauri::command]
pub async fn save_execution_history(
    app: tauri::AppHandle,
    project_id: i64,
    execution_type: String,
    regression_suite_id: Option<i64>,
    execution_status: String,
    command: Option<String>,
    total_tests: i64,
    passed: i64,
    failed: i64,
    skipped: i64,
    execution_time: f64,
) -> Result<i64, String> {
    crate::db::save_execution_history(
        &app,
        project_id,
        &execution_type,
        regression_suite_id,
        &execution_status,
        command,
        total_tests,
        passed,
        failed,
        skipped,
        execution_time,
    )
    .await
}

#[tauri::command]
pub async fn save_coverage_result(
    app: tauri::AppHandle,
    project_id: i64,
    execution_id: Option<i64>,
    total_statement_coverage: f64,
    total_branch_coverage: Option<f64>,
    file_count: i64,
    covered_file_count: i64,
    detail_json_path: Option<String>,
) -> Result<(), String> {
    crate::db::save_coverage_result(
        &app,
        project_id,
        execution_id,
        total_statement_coverage,
        total_branch_coverage,
        file_count,
        covered_file_count,
        detail_json_path,
    )
    .await
}

/// 全局执行总次数
#[tauri::command]
pub async fn count_execution_history(
    app: tauri::AppHandle,
) -> Result<i64, String> {
    crate::db::count_execution_history(&app).await
}

/// 全局统计：平均通过率 / 平均覆盖率
#[tauri::command]
pub async fn get_global_stats(
    app: tauri::AppHandle,
) -> Result<crate::db::GlobalStatsRow, String> {
    crate::db::get_global_stats(&app).await
}

/// 读取某项目的测试执行历史（最近 100 条）
#[tauri::command]
pub async fn list_execution_history(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<Vec<crate::db::ExecutionHistoryRow>, String> {
    crate::db::list_execution_history(&app, project_id).await
}

/// 写入一次测试生成历史（Pynguin），返回记录 id
#[tauri::command]
pub async fn save_generation_history(
    app: tauri::AppHandle,
    project_id: i64,
    generation_status: String,
    total_files: i64,
    generated_files: i64,
    duration: f64,
    command: Option<String>,
) -> Result<i64, String> {
    crate::db::save_generation_history(
        &app,
        project_id,
        &generation_status,
        total_files,
        generated_files,
        duration,
        command,
    )
    .await
}

/// 读取某项目的测试生成历史（最近 100 条）
#[tauri::command]
pub async fn list_generation_history(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<Vec<crate::db::GenerationHistoryRow>, String> {
    crate::db::list_generation_history(&app, project_id).await
}

/// 写入一次覆盖率运行历史，返回记录 id
#[tauri::command]
pub async fn save_coverage_history(
    app: tauri::AppHandle,
    project_id: i64,
    coverage_status: String,
    percent_covered: f64,
    total_statements: i64,
    covered_statements: i64,
    duration: f64,
    command: Option<String>,
) -> Result<i64, String> {
    crate::db::save_coverage_history(
        &app,
        project_id,
        &coverage_status,
        percent_covered,
        total_statements,
        covered_statements,
        duration,
        command,
    )
    .await
}

/// 读取某项目的覆盖率运行历史（最近 100 条）
#[tauri::command]
pub async fn list_coverage_history(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<Vec<crate::db::CoverageHistoryRow>, String> {
    crate::db::list_coverage_history(&app, project_id).await
}
