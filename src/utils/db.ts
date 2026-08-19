// src/utils/db.ts
import { invoke } from "@tauri-apps/api/core"
import Database from '@tauri-apps/plugin-sql'

// 数据库连接字符串
const DB_URL = 'sqlite:pytest_auto.db'

// 单例数据库连接
let dbInstance: Database | null = null

/**
 * 获取数据库连接（单例模式）
 *
 * 仅供调试页（DatabaseSchemaViewer）使用；业务持久化统一走 Rust 类型化命令（NFR008）。
 */
export async function getDatabase(): Promise<Database> {
  if (!dbInstance) {
    dbInstance = await Database.load(DB_URL)
  }
  return dbInstance
}

/**
 * 初始化数据库表结构（幂等）
 *
 * 建表与迁移逻辑已收编到 Rust（`init_db` 命令，见 src-tauri/src/db.rs），
 * 前端仅发起调用，满足论文 NFR008：前端不再直接执行裸 SQL 建表。
 */
export async function initializeDatabase(): Promise<void> {
  await invoke("init_db")
}
