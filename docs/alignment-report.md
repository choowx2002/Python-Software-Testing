# 论文草稿与 Tauri 项目对齐分析报告

> 分析对象：`docs/LKCFES-FYP Report-CHOO WEI XIANG(3).docx`（提取文本见 `docs/paper_extracted.txt`）
> 代码基线：`git 9155ff5`（"add pyguin basic UI and rs function."）
> 生成日期：2026-08-15

---

## 一、论文要点提取（Summary）

### 1. 核心方法论
- **产品定位**：面向 Python 小型项目的 GUI 自动化测试软件（Testmate），整合 **pytest**（执行）+ **Pynguin**（搜索式测试生成）+ **coverage.py**（覆盖率分析）三个核心库。
- **架构范式**（Ch.5）：Tauri v2 统一的三运行时解耦 —— Vue 3（纯 UI）/ Rust（业务逻辑与编排中枢，类型安全 IPC）/ Python（按需启动的隔离子进程引擎，通过 stdio 通信）。
- **异步事件驱动**：所有长任务（pytest 执行、Pynguin 生成）由 Rust 异步读子进程 stdout，以事件（event emission）流式推送到前端，保证 UI 不冻结。
- **混合持久化**（Ch.5.3）：SQLite 存结构化元数据（projects / test_execution_history / regression_suites / coverage_results 四表）；重型 JSON（覆盖率明细）存应用数据目录；每项目独立子目录 + Snowflake ID 隔离。

### 2. 功能需求（Table 4.1，FR001–FR013）
| ID | 需求 | 论文章节 |
|----|------|---------|
| FR001 | 导入本地 Python 项目目录 | UC01 |
| FR002 | 导入时自动检测 venv | UC01 |
| FR003 | 检查依赖并提示安装 | UC01 |
| FR004 | 选择指定测试文件/文件夹执行 | UC02 |
| FR005 | 执行日志实时流式推送前端 | UC02 |
| FR006 | 解析结果并可视化展示 passed/failed/skipped | UC02 |
| FR007 | 保存执行参数为回归套件，一键重跑 | UC02 Alt 9.1 |
| FR008 | 选择目标模块触发自动生成 | UC03 |
| FR009 | 生成代码输出到测试目录、不改动源码 | UC03 |
| FR010 | 高级生成配置（时限、目标函数等） | UC03 |
| FR011 | 测试成功后自动触发覆盖率采集 | UC02 流程 10 / Ch5 |
| FR012 | 展示总覆盖率并在代码查看器中高亮已/未覆盖行 | UC04 |
| FR013 | 按文件/函数过滤覆盖率、导出报告 | UC04 |

### 3. 关键术语
- 环境就绪状态：`active / missing / invalid`（Table 5.1）
- 执行类型：`MANUAL / REGRESSION`；执行状态：`SUCCESS / FAILED / ERROR / TIMEOUT`（Table 5.2）
- 模块四阶段：Test Environment Setup → Automated Test Generation → Test Execution → Automated Coverage Analysis（Ch.5.2）

---

## 二、代码清单（Code Inventory）

### 前端（Vue 3 + Pinia + vue-router + Tailwind）
| 文件 | 内容 |
|------|------|
| `src/views/Dashboard.vue` (371L) | 项目仪表盘：Quick Actions（Import/Clone/AI Test Generator）、项目表格（Environment / Last Test Result / Coverage / Last Run）、状态栏 |
| `src/views/projects/Import.vue` (434L) | 导入向导：select → detect → review → installing → save 五阶段 |
| `src/views/projects/Layout.vue` (461L) | 项目上下文壳：执行/生成/覆盖率三 Tab 导航 + 右侧项目概览 |
| `src/views/projects/[id]/Execute.vue` (1618L) | 测试执行：用例收集/选择、pytest 预设与自定义参数、实时日志、结果汇总、Rerun Failed |
| `src/views/projects/[id]/Generate.vue` (1418L) | Pynguin 生成：源码树选择、算法/时限/断言/种群等配置、进度、生成文件列表 |
| `src/views/projects/[id]/Coverage.vue` (1L) | **占位符 `<template>111</template>`** |
| `src/stores/projectStore.ts` | Pinia store（fetchProjects/updateLastOpened/updateProject/deleteProject） |
| `src/utils/db.ts` | plugin-sql 建表 SQL（四表 + 索引） |
| `src/helper/execute.ts` | pytest 参数解析器 |
| `src/components/` | TreeItem.vue（源码树）、DatabaseSchemaViewer.vue（调试用） |

### Rust 后端（`src-tauri/src/commands.rs`，1616 行单文件）
| Command | 对应论文 | 说明 |
|---------|---------|------|
| `validate_project_directory` | UC01 流程 3 | 校验目录含 .py/配置文件 |
| `detect_python_env` | FR002/FR003 | .venv/venv 检测→全局回退；requirements.txt 或硬编码依赖检查 |
| `create_virtual_env` | UI mock | `python -m venv .venv`，幂等校验 |
| `install_dependencies` | FR003 | 逐包 pip install，emit `install_step` |
| `scan_test_files` | FR004 | 递归扫 tests/ 下 test_*.py / *_test.py |
| `collect_test_cases` | FR004 | `pytest --collect-only -q` 解析用例 |
| `run_tests` | FR004–FR006 | 异步子进程跑 pytest，emit `test-started/test-output/test-finished`，解析 JUnit XML |
| `scan_source_files` | FR008 | 扫 .py 源文件（跳过 venv/tests/__init__/conftest） |
| `generate_tests` | FR008–FR010 | 逐模块调 Pynguin CLI，emit `generation-started/progress/output/finished` |
| `open_in_file_manager` / `open_file` / `reveal_in_folder` | — | 文件系统操作工具 |

事件通道：`test-started`、`test-output`、`test-finished`、`install_step`、`generation-*`。

### 配置
- `tauri.conf.json`：productName Testmate、v0.1.0、1280×600、`csp: null`、bundle targets "all"
- `capabilities/default.json`：core/dialog/opener/shell/fs/sql/store 全部 default，另加 `fs:allow-read`、`sql:allow-*`、`opener:allow-open-path "/**"`
- `Cargo.toml`：tauri 2 + 6 个官方插件 + tokio/uuid/quick-xml

---

## 三、对齐分析（Gap Analysis）

### ✅ 已对齐（论文描述且代码已实现）

| # | 论文依据 | 代码位置 | 说明 |
|---|---------|---------|------|
| 1 | FR001 导入项目 | `Import.vue` + `validate_project_directory` + dialog 插件 | 完整导入向导，含重名/已导入拦截 |
| 2 | FR002 自动检测 venv | `detect_python_env`（L74–230） | .venv/venv → 全局 python 回退，跨平台路径处理 |
| 3 | FR003 依赖检查与安装 | `detect_python_env` + `install_dependencies` + `install_step` 事件 | 支持 requirements.txt 解析与实时安装状态 |
| 4 | FR004 选择测试文件/用例执行 | `scan_test_files` + `collect_test_cases` + `Execute.vue` 三作用域（all/file/selected） | 超过论文要求：支持到单个用例粒度 |
| 5 | FR005 实时日志流 | `run_tests`（tokio 异步读 stdout/stderr）+ `test-output` 事件 + `Execute.vue` | 完全符合 UC02 流程 5–7 |
| 6 | FR006 结果可视化汇总 | JUnit XML 解析（`parse_junit_results`）+ passed/failed/skipped 卡片 + 失败详情展开 | 符合 UC02 流程 8–9 |
| 7 | FR008 选择模块生成 | `scan_source_files` + 源码树（TreeItem.vue） | 符合 UC03 流程 1 |
| 8 | FR009 输出到测试目录、不改源码 | `generate_tests`（`--output-path tests/generated` + `PYNGUIN_DANGER_AWARE=1`） | 符合 UC03 流程 9 |
| 9 | FR010 高级生成配置 | `Generate.vue`：算法 MOSA/DYNAMOSA/WSPA/RANDOM、时限、断言、seed、染色体长度、种群 | 超过论文要求（论文只提时限+目标函数） |
| 10 | Ch5 三运行时架构 + 事件驱动 | `lib.rs` 12 命令注册 + 6 类事件 | 架构与论文 Ch.5.1/5.2 一致 |
| 11 | 技术选型 pytest/Pynguin/coverage.py | 依赖检测列表（commands.rs L185） | 与 Ch.2 Table 2.3 一致 |
| 12 | Table 5.1–5.4 数据库四表 | `src/utils/db.ts` | 表结构、字段、外键、索引基本一致 |

### ⚠️ 待实现（论文提到、代码缺失）

| # | 论文依据 | 缺失内容 | 建议行动 |
|---|---------|---------|---------|
| 1 | **FR011**（UC02 流程 10、WBS 4.1） | 测试结束后**自动触发 coverage.py** 采集 statement/branch 覆盖率 | 新增 Rust 命令 `run_coverage`（或在 `run_tests` 尾部串联 coverage 子进程） |
| 2 | **FR012**（UC04、WBS 4.2） | 覆盖率展示 + **代码查看器高亮已/未覆盖行**；`Coverage.vue` 是空占位符 | 完整实现 Coverage 视图：读源码 + 行级高亮 |
| 3 | **FR013**（UC04 流程 5–7、WBS 4.3/4.4） | 按文件/函数过滤、**导出覆盖率报告** | 过滤控件 + 导出命令（写文件） |
| 4 | Ch5.3 持久化设计 | coverage JSON 缓存文件 + **每项目隔离子目录** | Rust 端写 `app_data_dir/<project>/coverage.json` |
| 5 | **FR007**（UC02 Alt 9.1、WBS 2.4） | 回归套件保存/一键运行 | 新增 `save_regression_suite` / `run_regression_suite` 命令 + 前端 UI |
| 6 | UI mock（Ch5.4） | Dashboard 的 **Last Test Result / Coverage / Last Run 列全是硬编码 0/"Never"**（`projectStore.ts` L53–56）；`total_runs` 恒为 0（Dashboard L366） | 改为 JOIN `test_execution_history` / `coverage_results` 的真实聚合查询 |
| 7 | UI mock（Ch5.4） | 状态栏 Python 版本**硬编码** "Python 3.13.2 (venv)"（Dashboard L357） | 从 `detect_python_env` 取真实版本 |
| 8 | UI mock（Ch5.4） | **Clone Repository** 按钮无任何功能 | 实现 git clone（shell 插件）或论文中删除该功能 |
| 9 | Ch3 风险表 / UC03 9.2 | 运行中**取消/终止**子进程（Execute/Generate 的 Stop 按钮均 disabled） | Rust 侧用 Tauri State 持有 `run_id → Child`，加 `cancel_run` 命令 |
| 10 | Ch3 WBS 4.1–4.4 | coverage 相关的 Rust 层代码**完全为 0**（grep 仅命中依赖名） | 同 #1–#4 |

### 🔄 不一致（代码与论文冲突）

| # | 论文描述 | 代码现状 | 影响 | 建议 |
|---|---------|---------|------|------|
| 1 | **NFR008**：前后端所有通信须为严格类型化 IPC，无直接后端状态操作 | 前端通过 `plugin-sql` **直接执行裸 SQL**（`utils/db.ts`、各视图内联 `db.execute`），绕过 Rust 类型层 | 违反论文自设 NFR；SQL 注入面、schema 漂移风险 | 将持久化收敛到 Rust 类型化命令（见第四部分建议） |
| 2 | Ch5.3：项目分配 **Snowflake ID** | SQLite `INTEGER PRIMARY KEY AUTOINCREMENT` | 与论文数据字典不符 | 改论文为 AUTOINCREMENT（或 Rust 端生成雪花 ID 再入库） |
| 3 | Table 5.2：`execution_type` 取值 `MANUAL`/`REGRESSION` | 实际写入硬编码 `'pytest'`（Execute.vue `saveExecutionToDb`），`regression_suite_id` 永远为 NULL | 数据字典与实现不符 | 论文改取值集合，或代码按触发来源写入 |
| 4 | Ch5.3：Rust 管理持久化层（"Rust backend persists..."） | 建表/插入全在前端 `main.ts`/视图内完成 | 与架构描述不符 | 见 NFR008 重构 |
| 5 | Ch3 风险表："Minimal allowlist configuration"（最小权限） | `csp: null`；capabilities 含 `shell:allow-execute`、`opener:allow-open-path "/**"`、`fs:allow-read` 全局开放 | 权限面与论文宣称的安全姿态不符 | 收紧权限；若 shell 未用则移除 |
| 6 | Table 5.1：`status` 取值 `active/missing/invalid` | 导入时一律写 `'active'`，且 Dashboard 渲染的 `Ready/Warning/Failed` 与 store 中 'active' 映射为灰色默认态 | 环境列无法反映真实 venv/依赖状态 | 导入后定期/进入项目时重跑 `detect_python_env` 刷新状态 |
| 7 | UI mock：状态栏显示"当前检测到的 Python 版本" | 硬编码字符串（见上表 #7） | 显示可能误导 | 接入真实检测 |

### 📝 待补充（代码已实现、论文未提及）

| # | 代码现状 | 论文建议补充位置 |
|---|---------|-----------------|
| 1 | `create_virtual_env` 命令（幂等校验 + 损坏 .venv 保护） | Ch.4 UC01 子流程（论文只有"手动覆写解释器路径"，无自动建 venv 细节） |
| 2 | Pynguin 高级参数：算法选择、seed、染色体长度、种群大小、断言生成开关 | Ch.4 Table 4.6 / FR010 只写"time limits and target functions" |
| 3 | 单用例粒度选择、搜索过滤、Rerun Failed、结果筛选、日志复制/清空 | Ch.4 UC02 描述可补充"按用例选择" |
| 4 | pytest 预设（Standard/Quick/Debug/Stop-on-Failure）+ 自定义参数 | Ch.4 UC02 / 界面 mock |
| 5 | `open_in_file_manager` / `open_file` / `reveal_in_folder` 三个文件操作命令 | Ch.5 组件交互可补一段"文件系统辅助能力" |
| 6 | `DatabaseSchemaViewer` 调试页（/debug/schema） | 可归入 Ch.3 开发工具或附录 |
| 7 | 依赖检测回退策略：无 requirements.txt 时硬编码 `pytest/coverage/pynguin` | Ch.4 UC01 子流程 4 |
| 8 | 生成结果状态分类（success/empty/failed）与 test_case_count 统计（`count_test_cases`） | Ch.4 UC03 流程 9 可补充"空结果警告"细节（论文 9.2 有提及超时/空输出，可呼应） |

---

## 四、行动建议（Action Items）

### Q1：优先改代码还是完善论文？
**先代码，后论文，按以下顺序：**

1. **立即做（代码）**：实现 Coverage 模块 —— 这是论文三大支柱（执行/生成/覆盖率）中唯一零实现的一项，`Coverage.vue` 占位符与 FR011–FR013 是完全空白。建议顺序：
   - Rust：新增 `run_coverage(project_path, interpreter_path, targets)` 命令 —— 复用 `run_tests` 的子进程模式，执行 `coverage run -m pytest ...` + `coverage json`，解析 JSON 为行级结构返回/缓存；
   - 在 `run_tests` 的 `test-finished` 之后由前端触发（或 Rust 自动串联）覆盖率采集，符合 UC02 流程 10；
   - 前端：实现 `Coverage.vue` —— 总覆盖率卡片（statement/branch）、文件树按覆盖率排序、代码查看器行级高亮（covered/uncovered/partial）、按文件/函数过滤、导出报告（复用 `reveal_in_folder` 模式写文件）；
   - 入库：写 `coverage_results`（此时四表才真正闭合）。
2. **次优先（代码）**：回归套件 FR007（表已建，补 UI + 2 个命令）；Dashboard 真实数据聚合（JOIN 查询替代硬编码 0）。
3. **论文修订（与代码同步）**：Snowflake→AUTOINCREMENT、execution_type 取值、Clone Repository 去留、补充第二/三节已实现细节。**不要在论文里写"已实现覆盖率可视化"直到代码完成**——目前论文 Ch.5 的 Coverage 描述（Figure 5.5 交互式代码查看器）与代码差距最大，答辩演示前必须闭环。

### Q2：需要优先重构的关键模块
1. **持久化层归属（最高优先）**：把 `utils/db.ts` 的裸 SQL 收进 Rust —— 新增 `db` 模块 + 一组类型化命令（`get_projects`、`save_execution`、`save_coverage`、`save_regression_suite` 等），前端只 `invoke`。这同时满足 NFR008、修复"不一致 #1/#4"，也为 Coverage 缓存写 JSON 提供统一入口。若时间紧，退而求其次：保留 plugin-sql 但在论文 NFR008 中补充"SQL 经官方 SQL 插件通道、非直接状态操作"的说明。
2. **`commands.rs` 拆分**：1616 行单文件已超出可维护边界；`src-tauri/src/commands/`、`src-tauri/src/execution/` 两个空目录已在（`git status` 显示为空目录），说明作者本意即分模块。建议：`commands/env.rs`（检测/建 venv/装依赖）、`commands/execution.rs`（pytest）、`commands/generation.rs`（Pynguin）、`commands/coverage.rs`（新增）、`commands/files.rs`（打开/显示）。与论文 Ch.5.2 四阶段一一对应，也便于论文引用代码位置。
3. **子进程生命周期管理**：Tauri State 持有 `HashMap<run_id, Child>`，实现取消/终止（论文 UC03 9.2"gracefully terminates without freezing"），并防止窗口关闭后孤儿进程。
4. **权限收敛**：`capabilities/default.json` 移除未使用的 `shell:allow-execute`、将 `opener`/`fs` 的 `/**` 收窄到项目目录范围；与论文风险表"Minimal allowlist"策略一致。

### Q3：公式/伪代码 ↔ 代码映射
论文**无公式与伪代码**（Ch.2.5 覆盖率定义是文字描述）。可对应的"算法级"实现如下，建议论文补充引用：

| 论文概念 | 代码实现位置 |
|---------|-------------|
| UC02 流程 4–6：pytest 子进程包装 + 实时日志解析 | `commands.rs` `run_tests`（L684 起）+ `Execute.vue` `parseRealtimeProgress`（正则 `^(.+?)::(.+?)\s+(PASSED|FAILED|SKIPPED|ERROR)`） |
| UC02 流程 8：结构化结果解析 | `commands.rs` `parse_junit_results` / `parse_testcase_attributes`（quick-xml 解析 `<testcase>/<failure>/<skipped>`） |
| UC03：Pynguin 搜索式生成调用 | `commands.rs` `generate_tests`（L1155 起：`--algorithm/--maximum-search-time/--population/--chromosome-length/--seed/--assertion-generation`）+ `Generate.vue` `estimatedTimeout = maxSearchTime × 文件数` |
| 生成文件质量统计 | `commands.rs` `count_test_cases`（`def test_` 行计数） |
| **（缺失）** coverage 采集与解析 | **无对应实现** —— 需新增 |

### Q4：Tauri 架构的模块划分建议
```
src-tauri/src/
├── main.rs / lib.rs            # 插件注册 + 命令总表（保持薄）
├── state.rs                    # AppState：run_id→Child 映射、每项目工作目录
├── commands/
│   ├── env.rs                  # detect/validate/create_venv/install_deps（对应 Setup 阶段）
│   ├── execution.rs            # scan/collect/run_tests/cancel（对应 Execute 阶段）
│   ├── generation.rs           # scan_source/generate_tests/cancel（对应 Generate 阶段）
│   ├── coverage.rs             # run_coverage/export_coverage（对应 Coverage 阶段，新增）
│   ├── suites.rs               # save/run/list regression suite（新增，FR007）
│   └── files.rs                # open/reveal 工具
├── db/
│   ├── mod.rs                  # 连接初始化 + 迁移
│   ├── projects.rs / history.rs / coverage.rs / suites.rs   # 类型化 CRUD
└── python/                     # 子进程参数构造 + 输出解析（junit/coverage json）
```
要点：
- 命令命名与论文 Ch.5.2 四阶段一一对应，评审时可直接对照；
- Python 解析器（JUnit/coverage JSON）独立成模块，便于单测；
- 事件名统一前缀（`test-*`/`generation-*`/`coverage-*`），与前端 listener 一一对应。

---

## 五、结论摘要
- **完成度**：环境设置 ✅ / 测试执行 ✅ / 测试生成 ✅ / **覆盖率分析 ❌（0%）** / 回归套件 ❌（仅建表）/ 仪表盘真实数据 ❌（硬编码）。
- **最大风险**：论文 Ch.5 与 FR011–013 对覆盖率模块有完整设计与界面描述，而代码侧是空占位符 —— 这是答辩演示与论文一致性上的最大缺口，必须最先补齐。
- **架构一致性**：核心 IPC 命令划分与论文模块吻合度高，事件流设计优秀；主要冲突集中在持久化层归属（NFR008）与少量数据字典细节，均为可快速修订项。
