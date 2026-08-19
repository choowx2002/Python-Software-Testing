use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use tauri::AppHandle;
use tauri::Manager;

/// 与前端 plugin-sql 共享的 SQLite 数据库文件：
/// plugin-sql 将 `sqlite:pytest_auto.db` 解析为 `{app_config_dir}/pytest_auto.db`。
fn db_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config directory: {}", e))?;
    Ok(dir.join("pytest_auto.db"))
}

pub async fn pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let path = db_path(app)?;
    SqlitePoolOptions::new()
        .max_connections(4)
        .connect(
            path.to_str()
                .ok_or_else(|| "Invalid database path".to_string())?,
        )
        .await
        .map_err(|e| format!("Failed to open database: {}", e))
}

// ============================================
// Schema (对应 FYP Report Table 5.1 - 5.4)
// ============================================

const INIT_SQL: &str = "
    -- 1. Projects Table
    CREATE TABLE IF NOT EXISTS projects (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name VARCHAR(255) NOT NULL,
      project_path TEXT NOT NULL UNIQUE,
      interpreter_path TEXT,
      status VARCHAR(20) DEFAULT 'active',
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      last_opened_at TIMESTAMP
    );

    -- 2. Regression Suites Table
    CREATE TABLE IF NOT EXISTS regression_suites (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      project_id INTEGER NOT NULL,
      suite_name VARCHAR(100) NOT NULL,
      target_paths TEXT NOT NULL,
      custom_params TEXT,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- 3. Test Execution History Table
    CREATE TABLE IF NOT EXISTS test_execution_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      project_id INTEGER NOT NULL,
      regression_suite_id INTEGER,
      execution_type VARCHAR(20) NOT NULL,
      execution_status VARCHAR(20) NOT NULL,
      command TEXT,
      total_tests INTEGER NOT NULL,
      passed INTEGER NOT NULL,
      failed INTEGER NOT NULL,
      skipped INTEGER NOT NULL,
      execution_time REAL NOT NULL,
      executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
      FOREIGN KEY (regression_suite_id) REFERENCES regression_suites(id) ON DELETE SET NULL
    );

    -- 4. Coverage Results Table
    CREATE TABLE IF NOT EXISTS coverage_results (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      project_id INTEGER NOT NULL,
      execution_id INTEGER,
      total_statement_coverage REAL NOT NULL,
      total_branch_coverage REAL,
      file_count INTEGER NOT NULL,
      covered_file_count INTEGER NOT NULL,
      detail_json_path TEXT,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (project_id) REFERENCES projects(id)
    );

    CREATE INDEX IF NOT EXISTS idx_project_id ON test_execution_history(project_id);
    CREATE INDEX IF NOT EXISTS idx_execution_id ON coverage_results(execution_id);
";

/// 初始化数据库表结构（幂等）。
/// 迁移：coverage_results 曾使用旧结构（cache_file_path 字段），检测到则先丢弃重建。
pub async fn init(app: &AppHandle) -> Result<(), String> {
    let db = pool(app).await?;

    let rows = sqlx::query(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'coverage_results'",
    )
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to inspect coverage_results table: {}", e))?;
    for row in rows {
        let sql: String = row.try_get("sql").unwrap_or_default();
        if sql.contains("cache_file_path") {
            sqlx::query("DROP TABLE coverage_results")
                .execute(&db)
                .await
                .map_err(|e| format!("Failed to migrate coverage_results table: {}", e))?;
        }
    }

    sqlx::query(INIT_SQL)
        .execute(&db)
        .await
        .map_err(|e| format!("Failed to initialize database tables: {}", e))?;

    Ok(())
}

// ============================================
// Projects
// ============================================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub interpreter_path: Option<String>,
    pub env_status: String,
    pub tests_passed: i64,
    pub tests_failed: i64,
    pub coverage: f64,
    pub last_run: Option<String>,
}

/// 项目列表（附带真实统计：执行历史 + 覆盖率）
pub async fn get_projects(app: &AppHandle) -> Result<Vec<ProjectRow>, String> {
    let db = pool(app).await?;

    let rows = sqlx::query(
        "SELECT
            p.id, p.name, p.project_path, p.interpreter_path, p.status,
            COALESCE((SELECT SUM(passed) FROM test_execution_history t WHERE t.project_id = p.id), 0) AS tests_passed,
            COALESCE((SELECT SUM(failed) FROM test_execution_history t WHERE t.project_id = p.id), 0) AS tests_failed,
            (SELECT MAX(executed_at) FROM test_execution_history t WHERE t.project_id = p.id) AS last_run,
            COALESCE((SELECT c.total_statement_coverage
                       FROM coverage_results c
                       WHERE c.project_id = p.id
                       ORDER BY c.id DESC LIMIT 1), 0) AS coverage
          FROM projects p
          ORDER BY p.created_at DESC",
    )
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to fetch projects: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(ProjectRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                path: row.try_get("project_path").map_err(|e| e.to_string())?,
                interpreter_path: row
                    .try_get("interpreter_path")
                    .map_err(|e| e.to_string())?,
                env_status: row.try_get("status").map_err(|e| e.to_string())?,
                tests_passed: row.try_get("tests_passed").map_err(|e| e.to_string())?,
                tests_failed: row.try_get("tests_failed").map_err(|e| e.to_string())?,
                coverage: row.try_get("coverage").map_err(|e| e.to_string())?,
                last_run: row.try_get("last_run").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// 新增项目（重名/重复路径拦截），返回新项目 id
pub async fn add_project(
    app: &AppHandle,
    name: &str,
    project_path: &str,
    interpreter_path: Option<String>,
) -> Result<i64, String> {
    let db = pool(app).await?;

    let existing = sqlx::query("SELECT id FROM projects WHERE project_path = ?")
        .bind(project_path)
        .fetch_optional(&db)
        .await
        .map_err(|e| format!("Failed to check existing project: {}", e))?;
    if existing.is_some() {
        return Err("This project has already been imported.".to_string());
    }

    let result = sqlx::query(
        "INSERT INTO projects (name, project_path, interpreter_path, status) VALUES (?, ?, ?, 'active')",
    )
    .bind(name)
    .bind(project_path)
    .bind(interpreter_path)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to add project: {}", e))?;

    Ok(result.last_insert_rowid())
}

pub async fn update_last_opened(app: &AppHandle, project_id: i64) -> Result<(), String> {
    let db = pool(app).await?;
    sqlx::query("UPDATE projects SET last_opened_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(project_id)
        .execute(&db)
        .await
        .map_err(|e| format!("Failed to update last opened: {}", e))?;
    Ok(())
}

pub async fn update_project_name(
    app: &AppHandle,
    project_id: i64,
    new_name: &str,
) -> Result<(), String> {
    let db = pool(app).await?;
    sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
        .bind(new_name)
        .bind(project_id)
        .execute(&db)
        .await
        .map_err(|e| format!("Failed to update project name: {}", e))?;
    Ok(())
}

/// 删除项目及其关联数据（coverage_results 无级联外键，需显式清理）
pub async fn delete_project(app: &AppHandle, project_id: i64) -> Result<(), String> {
    let db = pool(app).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    sqlx::query("DELETE FROM coverage_results WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete coverage results: {}", e))?;
    sqlx::query("DELETE FROM test_execution_history WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete execution history: {}", e))?;
    sqlx::query("DELETE FROM regression_suites WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete regression suites: {}", e))?;
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit delete: {}", e))?;
    Ok(())
}

// ============================================
// Test Execution History
// ============================================

/// 写入一次测试执行记录（execution_type 对齐论文 Table 5.2：MANUAL / REGRESSION），返回记录 id
pub async fn save_execution_history(
    app: &AppHandle,
    project_id: i64,
    execution_type: &str,
    regression_suite_id: Option<i64>,
    execution_status: &str,
    command: Option<String>,
    total_tests: i64,
    passed: i64,
    failed: i64,
    skipped: i64,
    execution_time: f64,
) -> Result<i64, String> {
    let db = pool(app).await?;
    let result = sqlx::query(
        "INSERT INTO test_execution_history
           (project_id, execution_type, regression_suite_id, execution_status, command,
            total_tests, passed, failed, skipped, execution_time)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project_id)
    .bind(execution_type)
    .bind(regression_suite_id)
    .bind(execution_status)
    .bind(command)
    .bind(total_tests)
    .bind(passed)
    .bind(failed)
    .bind(skipped)
    .bind(execution_time)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to save execution history: {}", e))?;

    Ok(result.last_insert_rowid())
}

// ============================================
// Coverage Results
// ============================================

pub async fn save_coverage_result(
    app: &AppHandle,
    project_id: i64,
    execution_id: Option<i64>,
    total_statement_coverage: f64,
    total_branch_coverage: Option<f64>,
    file_count: i64,
    covered_file_count: i64,
    detail_json_path: Option<String>,
) -> Result<(), String> {
    let db = pool(app).await?;
    sqlx::query(
        "INSERT INTO coverage_results
           (project_id, execution_id, total_statement_coverage, total_branch_coverage,
            file_count, covered_file_count, detail_json_path)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project_id)
    .bind(execution_id)
    .bind(total_statement_coverage)
    .bind(total_branch_coverage)
    .bind(file_count)
    .bind(covered_file_count)
    .bind(detail_json_path)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to save coverage result: {}", e))?;

    Ok(())
}
// ============================================
// Global Stats
// ============================================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalStatsRow {
    pub avg_pass_rate: f64,
    pub avg_coverage: f64,
}

pub async fn count_execution_history(app: &AppHandle) -> Result<i64, String> {
    let db = pool(app).await?;
    let row = sqlx::query("SELECT COUNT(*) AS total FROM test_execution_history")
        .fetch_one(&db)
        .await
        .map_err(|e| format!("Failed to count execution history: {}", e))?;
    let total: i64 = row.try_get("total").map_err(|e| e.to_string())?;
    Ok(total)
}

/// 全局统计：平均通过率、平均覆盖率（每个项目取最新一条覆盖率）
pub async fn get_global_stats(app: &AppHandle) -> Result<GlobalStatsRow, String> {
    let db = pool(app).await?;
    let row = sqlx::query(
        "SELECT
            COALESCE((SELECT AVG(
              CASE WHEN (passed + failed + skipped) > 0
                THEN passed * 100.0 / (passed + failed + skipped)
                ELSE NULL END
            ) FROM test_execution_history), 0) AS avg_pass_rate,
            COALESCE((SELECT AVG(c.total_statement_coverage)
              FROM coverage_results c
              WHERE c.id IN (SELECT MAX(id) FROM coverage_results GROUP BY project_id)), 0) AS avg_coverage",
    )
    .fetch_one(&db)
    .await
    .map_err(|e| format!("Failed to fetch global stats: {}", e))?;

    Ok(GlobalStatsRow {
        avg_pass_rate: row.try_get("avg_pass_rate").map_err(|e| e.to_string())?,
        avg_coverage: row.try_get("avg_coverage").map_err(|e| e.to_string())?,
    })
}
