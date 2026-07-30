use tauri_plugin_sql::{Builder, Migration, MigrationKind};

pub struct NewExecutionHistory {
    pub project_id: i64,
    pub execution_type: String,
    pub execution_status: String,
    pub command: String,
    pub total_tests: i32,
    pub passed: i32,
    pub failed: i32,
    pub skipped: i32,
    pub execution_time: f64,
}


fn save_test_execution_history(
    app: &tauri::AppHandle,
    project_id: i64,
    execution_type: &str,
    execution_status: &str,
    command: &str,
    total_tests: i64,
    passed: i64,
    failed: i64,
    skipped: i64,
    execution_time: f64,
) -> Result<(), String> {

    let db = app
        .state::<tauri_plugin_sql::DbInstances>()
        .get("sqlite:test.db")
        .ok_or("Database not initialized")?;

    db.execute(
        "
        INSERT INTO test_execution_history
        (
            project_id,
            execution_type,
            execution_status,
            command,
            total_tests,
            passed,
            failed,
            skipped,
            execution_time
        )
        VALUES
        (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ",
        vec![
            project_id.into(),
            execution_type.into(),
            execution_status.into(),
            command.into(),
            total_tests.into(),
            passed.into(),
            failed.into(),
            skipped.into(),
            execution_time.into(),
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}