# Testmate

> 面向 Python 小型项目的自动化测试工作台桌面应用。
> 整合 pytest、Pynguin 与 coverage.py，提供一键化的测试执行、自动生成与覆盖率分析能力。

## 功能概览

- **项目导入与环境检测** — 自动识别 Python 虚拟环境与项目依赖。
- **测试执行** — 调用 pytest 执行测试，实时流式日志推送，JUnit XML 结果解析。
- **测试生成** — 集成 Pynguin 搜索式测试生成引擎，支持多算法配置。
- **覆盖率分析** — 基于 coverage.py 的语句级覆盖率采集，行级源码高亮与 CSV/JSON 导出。
- **回归套件管理** — 持久化存储测试套件，支持历史对比与批量重跑。

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 GUI | Vue 3 + TypeScript + Tailwind CSS + Vite |
| 桌面壳 | Tauri v2 (Rust) |
| 持久化 | SQLite (Tauri SQL Plugin) |
| 测试引擎 | Python: pytest / Pynguin / coverage.py |

## 快速开始

### 环境要求

- **Node.js** >= 20 + **pnpm**
- **Rust** + **Cargo** (用于编译 Tauri 后端)
- **Python** >= 3.10 (用于运行测试引擎)

### 安装依赖

```bash
pnpm install
```

### 开发模式

```bash
pnpm tauri dev
```

### 生产构建

```bash
pnpm build:all
```

### 代码质量检查

```bash
pnpm type-check    # TypeScript 类型检查
pnpm lint          # ESLint 静态检查
pnpm format        # Prettier 格式化
pnpm test:rust     # Rust 后端单元测试
```

## 项目结构

```
.
├── src/                  # Vue 3 前端源码
├── src-tauri/src/        # Rust 后端源码
│   ├── commands.rs       # Tauri 命令集（正在拆分重构中）
│   └── main.rs           # 应用入口
├── docs/                 # 用户手册与对齐报告
├── public/               # 静态资源
└── ...
```

## 安全

本项目遵循 Tauri v2 安全最佳实践：
- 前端启用严格 CSP（内容安全策略）。
- Capability 权限按最小必要原则配置（`opener`、`shell`、`fs` 等均已收窄）。

## 许可证

MIT
