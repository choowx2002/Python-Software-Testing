use serde::{Deserialize, Serialize};
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

    -- 5. Generation History Table（Pynguin 生成历史）
    CREATE TABLE IF NOT EXISTS generation_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      project_id INTEGER NOT NULL,
      generation_status VARCHAR(20) NOT NULL,
      total_files INTEGER NOT NULL DEFAULT 0,
      generated_files INTEGER NOT NULL DEFAULT 0,
      duration REAL NOT NULL DEFAULT 0,
      command TEXT,
      executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    -- 6. Coverage History Table（覆盖率运行历史）
    CREATE TABLE IF NOT EXISTS coverage_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      project_id INTEGER NOT NULL,
      coverage_status VARCHAR(20) NOT NULL,
      percent_covered REAL NOT NULL DEFAULT 0,
      total_statements INTEGER NOT NULL DEFAULT 0,
      covered_statements INTEGER NOT NULL DEFAULT 0,
      duration REAL NOT NULL DEFAULT 0,
      command TEXT,
      files_json TEXT,
      executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_gen_history_project ON generation_history(project_id);
    CREATE INDEX IF NOT EXISTS idx_cov_history_project ON coverage_history(project_id);

    -- 7. Execution Result Details Table（单次执行中每条测试用例的结果明细）
    CREATE TABLE IF NOT EXISTS execution_result_details (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      execution_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      file TEXT,
      status TEXT NOT NULL,
      duration REAL NOT NULL DEFAULT 0,
      error_message TEXT,
      FOREIGN KEY (execution_id) REFERENCES test_execution_history(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_exec_result_detail ON execution_result_details(execution_id);

    -- 8. Generation File Details Table（单次生成产生的测试文件明细）
    CREATE TABLE IF NOT EXISTS generation_file_details (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      generation_id INTEGER NOT NULL,
      name TEXT NOT NULL,
      relative_path TEXT,
      test_case_count INTEGER NOT NULL DEFAULT 0,
      status TEXT,
      FOREIGN KEY (generation_id) REFERENCES generation_history(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_gen_file_detail ON generation_file_details(generation_id);
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

    // 迁移：coverage_history 增加 files_json 列（历史版本无此列）
    let cols = sqlx::query("PRAGMA table_info(coverage_history)")
        .fetch_all(&db)
        .await
        .map_err(|e| format!("Failed to inspect coverage_history columns: {}", e))?;
    let has_files_json = cols
        .iter()
        .any(|row| row.try_get::<String, _>("name").unwrap_or_default() == "files_json");
    if !has_files_json {
        sqlx::query("ALTER TABLE coverage_history ADD COLUMN files_json TEXT")
            .execute(&db)
            .await
            .map_err(|e| format!("Failed to add files_json column: {}", e))?;
    }

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
            COALESCE((SELECT t.passed FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1), 0) AS tests_passed,
            COALESCE((SELECT t.failed FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1), 0) AS tests_failed,
            (SELECT t.executed_at FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1) AS last_run,
            CAST(COALESCE((SELECT c.total_statement_coverage
                       FROM coverage_results c
                       WHERE c.project_id = p.id
                       ORDER BY c.id DESC LIMIT 1), 0) AS REAL) AS coverage
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

/// 更新项目解释器路径（一键修复环境后写入新 venv 的 python）
pub async fn update_project_interpreter(
    app: &AppHandle,
    project_id: i64,
    interpreter_path: &str,
) -> Result<(), String> {
    let db = pool(app).await?;
    sqlx::query("UPDATE projects SET interpreter_path = ? WHERE id = ?")
        .bind(interpreter_path)
        .bind(project_id)
        .execute(&db)
        .await
        .map_err(|e| format!("Failed to update interpreter path: {}", e))?;
    Ok(())
}

/// 读取项目的路径与已存的解释器路径（用于启动校验）
pub async fn get_project_interpreter(
    app: &AppHandle,
    project_id: i64,
) -> Result<(String, Option<String>), String> {
    let db = pool(app).await?;
    let row = sqlx::query("SELECT project_path, interpreter_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| format!("Failed to load project interpreter: {}", e))?;
    let Some(row) = row else {
        return Err(format!("Project not found: {}", project_id));
    };
    let project_path: String = row.try_get("project_path").map_err(|e| e.to_string())?;
    let interpreter_path: Option<String> =
        row.try_get("interpreter_path").map_err(|e| e.to_string())?;
    Ok((project_path, interpreter_path))
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
    sqlx::query(
        "DELETE FROM execution_result_details
         WHERE execution_id IN (SELECT id FROM test_execution_history WHERE project_id = ?)",
    )
    .bind(project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete execution result details: {}", e))?;
    sqlx::query("DELETE FROM regression_suites WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete regression suites: {}", e))?;
    sqlx::query("DELETE FROM generation_history WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete generation history: {}", e))?;
    sqlx::query(
        "DELETE FROM generation_file_details
         WHERE generation_id IN (SELECT id FROM generation_history WHERE project_id = ?)",
    )
    .bind(project_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to delete generation file details: {}", e))?;
    sqlx::query("DELETE FROM coverage_history WHERE project_id = ?")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete coverage history: {}", e))?;
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
            CAST(COALESCE((SELECT AVG(
              CASE WHEN (passed + failed + skipped) > 0
                THEN passed * 100.0 / (passed + failed + skipped)
                ELSE NULL END
            ) FROM test_execution_history), 0) AS REAL) AS avg_pass_rate,
            CAST(COALESCE((SELECT AVG(c.total_statement_coverage)
              FROM coverage_results c
              WHERE c.id IN (SELECT MAX(id) FROM coverage_results GROUP BY project_id)), 0) AS REAL) AS avg_coverage",
    )
    .fetch_one(&db)
    .await
    .map_err(|e| format!("Failed to fetch global stats: {}", e))?;

    Ok(GlobalStatsRow {
        avg_pass_rate: row.try_get("avg_pass_rate").map_err(|e| e.to_string())?,
        avg_coverage: row.try_get("avg_coverage").map_err(|e| e.to_string())?,
    })
}

// ============================================
// History（执行 / 生成 / 覆盖率 三类运行历史）
// ============================================

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionHistoryRow {
    pub id: i64,
    pub project_id: i64,
    pub execution_type: String,
    pub regression_suite_id: Option<i64>,
    pub execution_status: String,
    pub command: Option<String>,
    pub total_tests: i64,
    pub passed: i64,
    pub failed: i64,
    pub skipped: i64,
    pub execution_time: f64,
    pub executed_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerationHistoryRow {
    pub id: i64,
    pub project_id: i64,
    pub generation_status: String,
    pub total_files: i64,
    pub generated_files: i64,
    pub duration: f64,
    pub command: Option<String>,
    pub executed_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoverageHistoryRow {
    pub id: i64,
    pub project_id: i64,
    pub coverage_status: String,
    pub percent_covered: f64,
    pub total_statements: i64,
    pub covered_statements: i64,
    pub duration: f64,
    pub command: Option<String>,
    pub executed_at: String,
}

/// 某项目的测试执行历史（最近 100 条）
pub async fn list_execution_history(
    app: &AppHandle,
    project_id: i64,
) -> Result<Vec<ExecutionHistoryRow>, String> {
    let db = pool(app).await?;
    let rows = sqlx::query(
        "SELECT id, project_id, execution_type, regression_suite_id, execution_status, command,
                total_tests, passed, failed, skipped, execution_time,
                COALESCE(executed_at, '') AS executed_at
         FROM test_execution_history
         WHERE project_id = ?
         ORDER BY id DESC LIMIT 100",
    )
    .bind(project_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list execution history: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(ExecutionHistoryRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                project_id: row.try_get("project_id").map_err(|e| e.to_string())?,
                execution_type: row.try_get("execution_type").map_err(|e| e.to_string())?,
                regression_suite_id: row
                    .try_get("regression_suite_id")
                    .map_err(|e| e.to_string())?,
                execution_status: row.try_get("execution_status").map_err(|e| e.to_string())?,
                command: row.try_get("command").map_err(|e| e.to_string())?,
                total_tests: row.try_get("total_tests").map_err(|e| e.to_string())?,
                passed: row.try_get("passed").map_err(|e| e.to_string())?,
                failed: row.try_get("failed").map_err(|e| e.to_string())?,
                skipped: row.try_get("skipped").map_err(|e| e.to_string())?,
                execution_time: row.try_get("execution_time").map_err(|e| e.to_string())?,
                executed_at: row.try_get("executed_at").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// 写入一次测试生成历史（Pynguin），返回记录 id
pub async fn save_generation_history(
    app: &AppHandle,
    project_id: i64,
    generation_status: &str,
    total_files: i64,
    generated_files: i64,
    duration: f64,
    command: Option<String>,
) -> Result<i64, String> {
    let db = pool(app).await?;
    let result = sqlx::query(
        "INSERT INTO generation_history
           (project_id, generation_status, total_files, generated_files, duration, command)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(project_id)
    .bind(generation_status)
    .bind(total_files)
    .bind(generated_files)
    .bind(duration)
    .bind(command)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to save generation history: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// 某项目的测试生成历史（最近 100 条）
pub async fn list_generation_history(
    app: &AppHandle,
    project_id: i64,
) -> Result<Vec<GenerationHistoryRow>, String> {
    let db = pool(app).await?;
    let rows = sqlx::query(
        "SELECT id, project_id, generation_status, total_files, generated_files, duration,
                command, COALESCE(executed_at, '') AS executed_at
         FROM generation_history
         WHERE project_id = ?
         ORDER BY id DESC LIMIT 100",
    )
    .bind(project_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list generation history: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(GenerationHistoryRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                project_id: row.try_get("project_id").map_err(|e| e.to_string())?,
                generation_status: row.try_get("generation_status").map_err(|e| e.to_string())?,
                total_files: row.try_get("total_files").map_err(|e| e.to_string())?,
                generated_files: row.try_get("generated_files").map_err(|e| e.to_string())?,
                duration: row.try_get("duration").map_err(|e| e.to_string())?,
                command: row.try_get("command").map_err(|e| e.to_string())?,
                executed_at: row.try_get("executed_at").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// 写入一次覆盖率运行历史，返回记录 id
pub async fn save_coverage_history(
    app: &AppHandle,
    project_id: i64,
    coverage_status: &str,
    percent_covered: f64,
    total_statements: i64,
    covered_statements: i64,
    duration: f64,
    command: Option<String>,
    files_json: Option<String>,
) -> Result<i64, String> {
    let db = pool(app).await?;
    let result = sqlx::query(
        "INSERT INTO coverage_history
           (project_id, coverage_status, percent_covered, total_statements,
            covered_statements, duration, command, files_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project_id)
    .bind(coverage_status)
    .bind(percent_covered)
    .bind(total_statements)
    .bind(covered_statements)
    .bind(duration)
    .bind(command)
    .bind(files_json)
    .execute(&db)
    .await
    .map_err(|e| format!("Failed to save coverage history: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// 某项目的覆盖率运行历史（最近 100 条）
pub async fn list_coverage_history(
    app: &AppHandle,
    project_id: i64,
) -> Result<Vec<CoverageHistoryRow>, String> {
    let db = pool(app).await?;
    let rows = sqlx::query(
        "SELECT id, project_id, coverage_status, percent_covered, total_statements,
                covered_statements, duration, command, COALESCE(executed_at, '') AS executed_at
         FROM coverage_history
         WHERE project_id = ?
         ORDER BY id DESC LIMIT 100",
    )
    .bind(project_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list coverage history: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(CoverageHistoryRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                project_id: row.try_get("project_id").map_err(|e| e.to_string())?,
                coverage_status: row.try_get("coverage_status").map_err(|e| e.to_string())?,
                percent_covered: row.try_get("percent_covered").map_err(|e| e.to_string())?,
                total_statements: row.try_get("total_statements").map_err(|e| e.to_string())?,
                covered_statements: row
                    .try_get("covered_statements")
                    .map_err(|e| e.to_string())?,
                duration: row.try_get("duration").map_err(|e| e.to_string())?,
                command: row.try_get("command").map_err(|e| e.to_string())?,
                executed_at: row.try_get("executed_at").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

// ============================================
// Execution / Generation 明细（历史详情）
// ============================================

/// 前端提交的单条测试结果（保存执行明细用）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionDetailPayload {
    pub name: String,
    pub file: Option<String>,
    pub status: String,
    pub duration: f64,
    pub error_message: Option<String>,
}

/// 执行结果明细行（返回前端）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResultDetailRow {
    pub id: i64,
    pub execution_id: i64,
    pub name: String,
    pub file: Option<String>,
    pub status: String,
    pub duration: f64,
    pub error_message: Option<String>,
}

/// 前端提交的单条生成文件（保存生成明细用）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationFilePayload {
    pub name: String,
    pub relative_path: Option<String>,
    pub test_case_count: u32,
    pub status: Option<String>,
}

/// 生成文件明细行（返回前端）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerationFileRow {
    pub id: i64,
    pub generation_id: i64,
    pub name: String,
    pub relative_path: Option<String>,
    pub test_case_count: i64,
    pub status: Option<String>,
}

/// 批量写入一次执行的所有测试结果明细
pub async fn save_execution_result_details(
    app: &AppHandle,
    execution_id: i64,
    results: &[ExecutionDetailPayload],
) -> Result<(), String> {
    let db = pool(app).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for r in results {
        sqlx::query(
            "INSERT INTO execution_result_details
               (execution_id, name, file, status, duration, error_message)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(execution_id)
        .bind(&r.name)
        .bind(&r.file)
        .bind(&r.status)
        .bind(r.duration)
        .bind(&r.error_message)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save execution result detail: {}", e))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit execution details: {}", e))?;
    Ok(())
}

/// 读取一次执行的所有测试结果明细
pub async fn list_execution_result_details(
    app: &AppHandle,
    execution_id: i64,
) -> Result<Vec<ExecutionResultDetailRow>, String> {
    let db = pool(app).await?;
    let rows = sqlx::query(
        "SELECT id, execution_id, name, file, status, duration, error_message
         FROM execution_result_details
         WHERE execution_id = ?",
    )
    .bind(execution_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list execution result details: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(ExecutionResultDetailRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                execution_id: row.try_get("execution_id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                file: row.try_get("file").map_err(|e| e.to_string())?,
                status: row.try_get("status").map_err(|e| e.to_string())?,
                duration: row.try_get("duration").map_err(|e| e.to_string())?,
                error_message: row.try_get("error_message").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// 批量写入一次生成产生的所有测试文件明细
pub async fn save_generation_file_details(
    app: &AppHandle,
    generation_id: i64,
    files: &[GenerationFilePayload],
) -> Result<(), String> {
    let db = pool(app).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for f in files {
        sqlx::query(
            "INSERT INTO generation_file_details
               (generation_id, name, relative_path, test_case_count, status)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(generation_id)
        .bind(&f.name)
        .bind(&f.relative_path)
        .bind(f.test_case_count as i64)
        .bind(&f.status)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save generation file detail: {}", e))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit generation details: {}", e))?;
    Ok(())
}

/// 读取一次生成的所有测试文件明细
pub async fn list_generation_file_details(
    app: &AppHandle,
    generation_id: i64,
) -> Result<Vec<GenerationFileRow>, String> {
    let db = pool(app).await?;
    let rows = sqlx::query(
        "SELECT id, generation_id, name, relative_path, test_case_count, status
         FROM generation_file_details
         WHERE generation_id = ?",
    )
    .bind(generation_id)
    .fetch_all(&db)
    .await
    .map_err(|e| format!("Failed to list generation file details: {}", e))?;

    rows.into_iter()
        .map(|row| {
            Ok(GenerationFileRow {
                id: row.try_get("id").map_err(|e| e.to_string())?,
                generation_id: row.try_get("generation_id").map_err(|e| e.to_string())?,
                name: row.try_get("name").map_err(|e| e.to_string())?,
                relative_path: row.try_get("relative_path").map_err(|e| e.to_string())?,
                test_case_count: row.try_get("test_case_count").map_err(|e| e.to_string())?,
                status: row.try_get("status").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

/// 读取一次覆盖率运行的文件级明细（来自 files_json 列）
pub async fn get_coverage_history_files(
    app: &AppHandle,
    history_id: i64,
) -> Result<Option<Vec<crate::commands::coverage::FileCoverage>>, String> {
    let db = pool(app).await?;
    let row = sqlx::query("SELECT files_json FROM coverage_history WHERE id = ?")
        .bind(history_id)
        .fetch_optional(&db)
        .await
        .map_err(|e| format!("Failed to load coverage history files: {}", e))?;

    let Some(row) = row else {
        return Ok(None);
    };
    let files_json: Option<String> = row.try_get("files_json").map_err(|e| e.to_string())?;
    let Some(json) = files_json else {
        return Ok(None);
    };
    if json.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&json)
        .map(Some)
        .map_err(|e| format!("Failed to parse coverage files: {}", e))
}

// ============================================
// Tests（回归测试：COALESCE 回退分支的 REAL 列解码）
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> SqlitePool {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to open in-memory sqlite");
        sqlx::query(INIT_SQL)
            .execute(&db)
            .await
            .expect("failed to init schema");
        db
    }

    /// 回归测试：get_projects 的 coverage 列在无覆盖率数据时（COALESCE 回退 0）
    /// 必须能按 f64 解码 —— 曾因 COALESCE(REAL, 0) 被 SQLite 声明为 INTEGER 而报错
    /// "Rust type f64 ... not compatible with SQL type INTEGER"。
    #[tokio::test]
    async fn get_projects_coverage_decodes_as_real_with_no_data() {
        let db = mem_db().await;
        sqlx::query(
            "INSERT INTO projects (name, project_path, interpreter_path, status) VALUES ('A', '/tmp/a', NULL, 'active')",
        )
        .execute(&db)
        .await
        .unwrap();

        // 与 db::get_projects 完全相同的 SELECT（含 CAST AS REAL 修复）
        let rows = sqlx::query(
            "SELECT
                p.id, p.name, p.project_path, p.interpreter_path, p.status,
                COALESCE((SELECT t.passed FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1), 0) AS tests_passed,
                COALESCE((SELECT t.failed FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1), 0) AS tests_failed,
                (SELECT t.executed_at FROM test_execution_history t WHERE t.project_id = p.id ORDER BY t.id DESC LIMIT 1) AS last_run,
                CAST(COALESCE((SELECT c.total_statement_coverage
                           FROM coverage_results c
                           WHERE c.project_id = p.id
                           ORDER BY c.id DESC LIMIT 1), 0) AS REAL) AS coverage
             FROM projects p
             ORDER BY p.created_at DESC",
        )
        .fetch_all(&db)
        .await
        .unwrap();

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        let coverage: f64 = row.try_get("coverage").expect("coverage must decode as f64");
        assert_eq!(coverage, 0.0);
        let tests_passed: i64 = row.try_get("tests_passed").unwrap();
        assert_eq!(tests_passed, 0);
        let last_run: Option<String> = row.try_get("last_run").unwrap();
        assert!(last_run.is_none());
    }

    /// get_projects 在有覆盖率记录时，coverage 列按 f64 返回真实百分比
    #[tokio::test]
    async fn get_projects_coverage_decodes_real_value() {
        let db = mem_db().await;
        sqlx::query(
            "INSERT INTO projects (name, project_path, interpreter_path, status) VALUES ('A', '/tmp/a', NULL, 'active')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO coverage_results (project_id, execution_id, total_statement_coverage, total_branch_coverage, file_count, covered_file_count, detail_json_path)
             VALUES (1, NULL, 87.5, 61.3, 4, 3, NULL)",
        )
        .execute(&db)
        .await
        .unwrap();

        let rows = sqlx::query(
            "SELECT CAST(COALESCE((SELECT c.total_statement_coverage
                       FROM coverage_results c
                       WHERE c.project_id = p.id
                       ORDER BY c.id DESC LIMIT 1), 0) AS REAL) AS coverage
              FROM projects p",
        )
        .fetch_all(&db)
        .await
        .unwrap();

        let coverage: f64 = rows[0].try_get("coverage").unwrap();
        assert!((coverage - 87.5).abs() < 1e-9);
    }

    /// get_global_stats 在无任何数据时，avg_pass_rate / avg_coverage 必须按 f64 解码
    #[tokio::test]
    async fn global_stats_decodes_as_real_with_no_data() {
        let db = mem_db().await;
        let row = sqlx::query(
            "SELECT
                CAST(COALESCE((SELECT AVG(
                  CASE WHEN (passed + failed + skipped) > 0
                    THEN passed * 100.0 / (passed + failed + skipped)
                    ELSE NULL END
                ) FROM test_execution_history), 0) AS REAL) AS avg_pass_rate,
                CAST(COALESCE((SELECT AVG(c.total_statement_coverage)
                  FROM coverage_results c
                  WHERE c.id IN (SELECT MAX(id) FROM coverage_results GROUP BY project_id)), 0) AS REAL) AS avg_coverage",
        )
        .fetch_one(&db)
        .await
        .unwrap();

        let avg_pass_rate: f64 = row
            .try_get("avg_pass_rate")
            .expect("avg_pass_rate must decode as f64");
        assert_eq!(avg_pass_rate, 0.0);
        let avg_coverage: f64 = row
            .try_get("avg_coverage")
            .expect("avg_coverage must decode as f64");
        assert_eq!(avg_coverage, 0.0);
    }
}

/// 清空所有历史记录（测试执行、生成、覆盖率、明细）
pub async fn clear_all_history(app: &AppHandle) -> Result<(), String> {
    let db = pool(app).await?;
    let mut tx = db
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    sqlx::query("DELETE FROM execution_result_details")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete execution result details: {}", e))?;

    sqlx::query("DELETE FROM generation_file_details")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete generation file details: {}", e))?;

    sqlx::query("DELETE FROM test_execution_history")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete execution history: {}", e))?;

    sqlx::query("DELETE FROM coverage_results")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete coverage results: {}", e))?;

    sqlx::query("DELETE FROM coverage_history")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete coverage history: {}", e))?;

    sqlx::query("DELETE FROM generation_history")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete generation history: {}", e))?;

    sqlx::query("DELETE FROM generation_file_details")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete generation file details: {}", e))?;

    sqlx::query("DELETE FROM regression_suites")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to delete regression suites: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit clear all history: {}", e))?;

    Ok(())
}
