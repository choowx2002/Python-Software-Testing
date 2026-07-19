// src/utils/db.ts
import  Database  from '@tauri-apps/plugin-sql'

// 数据库连接字符串
const DB_URL = 'sqlite:pytest_auto.db'

// 单例数据库连接
let dbInstance: Database | null = null

/**
 * 获取数据库连接（单例模式）
 */
export async function getDatabase(): Promise<Database> {
  if (!dbInstance) {
    dbInstance = await Database.load(DB_URL)
  }
  return dbInstance
}

/**
 * 初始化数据库表结构
 * 使用 CREATE TABLE IF NOT EXISTS 保证幂等性
 */
export async function initializeDatabase(): Promise<void> {
  const db = await getDatabase()

  // 建表 SQL（对应 FYP Report Table 5.1 - 5.4）
  const initSQL = `
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
      execution_id INTEGER UNIQUE NOT NULL,
      statement_coverage REAL NOT NULL,
      branch_coverage REAL NOT NULL,
      cache_file_path TEXT NOT NULL,
      file_format VARCHAR(20) DEFAULT 'JSON',
      generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (execution_id) REFERENCES test_execution_history(id) ON DELETE CASCADE
    );

    -- 创建索引优化查询性能
    CREATE INDEX IF NOT EXISTS idx_project_id ON test_execution_history(project_id);
    CREATE INDEX IF NOT EXISTS idx_execution_id ON coverage_results(execution_id);
  `

  try {
    await db.execute(initSQL)
    console.log('[DB Init] ✅ Database tables initialized successfully')
  } catch (error) {
    console.error('[DB Init] ❌ Failed to initialize database:', error)
    throw error
  }
}

/**
 * 关闭数据库连接（应用退出时调用）
 */
export async function closeDatabase(): Promise<void> {
  if (dbInstance) {
    await dbInstance.close()
    dbInstance = null
    console.log('[DB] Database connection closed')
  }
}