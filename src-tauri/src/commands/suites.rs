use serde::{Deserialize, Serialize};
// Command: Regression Suites (FR007)
// ============================================

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegressionSuite {
    pub id: i64,
    pub project_id: i64,
    pub suite_name: String,
    pub target_paths: Vec<String>,
    pub custom_params: Option<Vec<String>>,
    pub created_at: String,
}

/// 将当前执行参数保存为回归套件（FR007）
#[tauri::command]
pub async fn save_regression_suite(
    app: tauri::AppHandle,
    project_id: i64,
    suite_name: String,
    target_paths: Vec<String>,
    custom_params: Option<Vec<String>>,
) -> Result<i64, String> {
    if suite_name.trim().is_empty() {
        return Err("Suite name must not be empty.".to_string());
    }
    let db = crate::db::pool(&app).await?;
    let targets = serde_json::to_string(&target_paths).map_err(|e| e.to_string())?;
    let params = custom_params.map(|p| serde_json::to_string(&p).unwrap_or_else(|_| "[]".into()));

    let result = sqlx::query(
        "INSERT INTO regression_suites (project_id, suite_name, target_paths, custom_params) VALUES (?, ?, ?, ?)",
    )
    .bind(project_id)
    .bind(suite_name.trim().to_string())
    .bind(targets)
    .bind(params)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to save regression suite: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// 列出某项目下的全部回归套件
#[tauri::command]
pub async fn list_regression_suites(
    app: tauri::AppHandle,
    project_id: i64,
) -> Result<Vec<RegressionSuite>, String> {
    use sqlx::Row;
    let db = crate::db::pool(&app).await?;
    let rows = sqlx::query(
        "SELECT id, project_id, suite_name, target_paths, custom_params, COALESCE(created_at, '') AS created_at FROM regression_suites WHERE project_id = ? ORDER BY id DESC",
    )
    .bind(project_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list regression suites: {}", e))?;

    rows.into_iter()
        .map(|row| {
            let id: i64 = row.try_get("id").map_err(|e| e.to_string())?;
            let project_id: i64 = row.try_get("project_id").map_err(|e| e.to_string())?;
            let suite_name: String = row.try_get("suite_name").map_err(|e| e.to_string())?;
            let targets: String = row.try_get("target_paths").map_err(|e| e.to_string())?;
            let params: Option<String> =
                row.try_get("custom_params").map_err(|e| e.to_string())?;
            let created_at: String = row.try_get("created_at").map_err(|e| e.to_string())?;

            Ok(RegressionSuite {
                id,
                project_id,
                suite_name,
                target_paths: serde_json::from_str(&targets).unwrap_or_default(),
                custom_params: params
                    .and_then(|p| serde_json::from_str(&p).ok())
                    .filter(|v: &Vec<String>| !v.is_empty()),
                created_at,
            })
        })
        .collect()
}

/// 删除回归套件
#[tauri::command]
pub async fn delete_regression_suite(
    app: tauri::AppHandle,
    suite_id: i64,
) -> Result<(), String> {
    let db = crate::db::pool(&app).await?;
    sqlx::query("DELETE FROM regression_suites WHERE id = ?")
        .bind(suite_id)
        .execute(&db)
        .await
        .map_err(|e| format!("Failed to delete regression suite: {}", e))?;
    Ok(())
}

/// 一键重跑回归套件：加载套件参数并复用 run_tests_core（FR007）
#[tauri::command]
pub async fn run_regression_suite(
    app: tauri::AppHandle,
    suite_id: i64,
) -> Result<String, String> {
    use sqlx::Row;
    let db = crate::db::pool(&app).await?;

    let row = sqlx::query(
        "SELECT project_id, target_paths, custom_params FROM regression_suites WHERE id = ?",
    )
    .bind(suite_id)
    .fetch_optional(&db)
    .await
    .map_err(|e| format!("Failed to load regression suite: {}", e))?
    .ok_or_else(|| "Regression suite not found.".to_string())?;

    let project_id: i64 = row.try_get("project_id").map_err(|e| e.to_string())?;
    let targets: String = row.try_get("target_paths").map_err(|e| e.to_string())?;
    let params: Option<String> = row.try_get("custom_params").map_err(|e| e.to_string())?;
    let target_paths: Vec<String> = serde_json::from_str(&targets).map_err(|e| e.to_string())?;
    let pytest_args: Vec<String> = params
        .and_then(|p| serde_json::from_str(&p).ok())
        .unwrap_or_default();

    let project_row = sqlx::query("SELECT project_path, interpreter_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| format!("Failed to load project: {}", e))?
        .ok_or_else(|| "Project not found for regression suite.".to_string())?;
    let project_path: String = project_row
        .try_get("project_path")
        .map_err(|e| e.to_string())?;
    let interpreter_path: Option<String> = project_row
        .try_get("interpreter_path")
        .map_err(|e| e.to_string())?;
    let interpreter_path = interpreter_path
        .ok_or_else(|| "Project has no Python interpreter configured.".to_string())?;

    crate::commands::execution::run_tests_core(
        app.clone(),
        project_id,
        project_path,
        interpreter_path,
        target_paths,
        pytest_args,
        Some(suite_id),
    )
    .await
}
