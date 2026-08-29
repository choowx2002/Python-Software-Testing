# Testmate FYP 论文 Ch.4 & Ch.5 — UML 图梳理与更新方案（2026-08-29）

> 依据：当前代码（git HEAD `6958681`）+ 论文现状（`docs/report-extract-clean.txt`）
> 用途：按此方案重画/更新 Ch.4（用例、活动、序列）与 Ch.5（架构、组件交互、ERD）的全部 UML 图
> 原则：**每个图都能在代码里找到对应函数**（避免画了代码里没有的流程）

---

## 0. 为什么需要重画：数据库 + 交互改动清单

| # | 改动 | 影响的图 |
|---|---|---|
| 1 | 数据库 4 表 → **8 表**（新增 generation_history、coverage_history、execution_result_details、generation_file_details，`db.rs` INIT_SQL） | ERD（5.6）、架构图（5.1） |
| 2 | **历史模块**：History.vue 时间线（执行/生成/覆盖率三类）+ 独立详情窗口（WebviewWindow `detail`）+ 趋势图（CombinedTrendChart）+ 导出 | 用例图、UC02/UC04 描述表、活动图 4.3/4.4/4.6、序列图 4.8/4.9/4.11、5.2–5.5 |
| 3 | **回归套件 UI**（Execute.vue 套件面板：保存/列表/一键重跑/删除，`suites.rs`） | 用例图、UC02、活动图 4.3、序列图 4.8 |
| 4 | **覆盖率一键触发**：执行完成弹窗 → `Coverage.vue?autoRun=1`（半自动，非全自动） | UC02 流程 10、UC04、5.2.4 描述 |
| 5 | **取消运行**：`cancel_run`（SIGTERM→2s→SIGKILL） | UC02/UC03 异常流、活动图 4.3/4.5 |
| 6 | **单用例粒度选择** + 搜索过滤 + Rerun Failed | UC02、活动图 4.3 |
| 7 | **Pynguin 高级参数**（算法/seed/染色体/种群/断言）+ BOM 自愈 + 语义化重命名 | UC03、活动图 4.5 |
| 8 | **Coverage 函数级过滤** + 四色行级高亮 + JSON/CSV 导出 | UC04、活动图 4.6 |
| 9 | Clone Repository（`env.rs::clone_repository`，Dashboard 入口） | UC01 可选流 |
| 10 | 设置页（语言切换、编辑器模板、清空历史 `clear_all_history`） | 用例图可选、UC05（若新增） |

---

## 1. 图清单总览（Table A）

### Chapter 4
| 图号 | 名称 | 类型 | 现状 | 处置 |
|---|---|---|---|---|
| Figure 4.1 | Use-Case Diagram | 用例图 | 4 个 UC | **小改**：建议新增 UC05（见 2.1） |
| Table 4.3 | Actors Identification | 表格 | 1 主 + 2 次 | **小改**：确认 Secondary Actor 列表 |
| Table 4.4 | UC01 Setup Project Environment | 用例描述 | 基本符合 | **小改**：补 Clone Repository / 自动建 venv |
| Table 4.5 | UC02 Execute Tests and View Results | 用例描述 | 缺新交互 | **更新**：单用例、取消、弹窗、历史（见 2.2） |
| Table 4.6 | UC03 Generate Unit Tests | 用例描述 | 缺新参数 | **更新**：高级参数、BOM、重命名（见 2.2） |
| Table 4.7 | UC04 Analyze Code Coverage | 用例描述 | 基本符合 | **小改**：autoRun 入口、历史对比（见 2.2） |
| Table 4.8（新增） | UC05 View History and Trends | 用例描述 | 无 | **新增**（若采纳 UC05） |
| Figure 4.2 | Activity: Setup Project Environment | 活动图 | 基本符合 | **小改**：补建 venv / 装依赖分支 |
| Figure 4.3 | Activity: Execute Tests | 活动图 | 缺取消/弹窗 | **更新**：见 2.3 |
| Figure 4.4 | Activity: View Test Results | 活动图 | 缺历史入库 | **更新**：见 2.3 |
| Figure 4.5 | Activity: Generate Unit Tests | 活动图 | 缺参数/BOM | **更新**：见 2.3 |
| Figure 4.6 | Activity: Analyze Code Coverage | 活动图 | 缺函数过滤/导出 | **更新**：见 2.3 |
| Figure 4.7（已采纳） | Activity: View History & Trends | 活动图 | 无 | **新增**（UC05 已采纳） |
| Figure 4.8–4.13 | Sequence ×6（Setup/Execute/View Results/Generate/Coverage/History） | 序列图 | 缺 DB 与事件 | **全部更新**：见 2.4 |

### Chapter 5
| 图号 | 名称 | 类型 | 现状 | 处置 |
|---|---|---|---|---|
| Figure 5.1 | High-Level Architecture | 架构图 | 三运行时 | **小改**：补 8 表、事件通道、历史模块 |
| Figure 5.2 | Sequence: Test Environment Setup | 序列图（组件级） | 缺 DB | **更新** |
| Figure 5.3 | Sequence: Automated Test Generation | 序列图（组件级） | 缺 DB/重命名 | **更新** |
| Figure 5.4 | Sequence: Test Execution | 序列图（组件级） | 缺事件/DB | **更新** |
| Figure 5.5 | Sequence: Automated Coverage Analysis | 序列图（组件级） | 缺 autoRun/DB | **更新** |
| Figure 5.6 | Entity-Relationship Diagram | ERD | 4 表 | **重画为 8 表**（内容见补丁文档 Section 3） |
| 可选 | Component Diagram | 组件图 | 无 | **可选加分**（见 3.4） |

> ✅ 已决定：**删除 Ch.5 §5.4 Prototype User Interface Design**（学院模板无强制要求），UI 截图移入 Ch.6 §6.4。
> 原 "Figure 5.6 重复编号（ERD 与 Dashboard 原型）" 问题随之消失，Ch.5 图号止于 5.6。

---

## 2. Chapter 4 逐图规格

### 2.1 Use-Case Diagram（Figure 4.1）

**参与者（Actors）**
- 主参与者：User（Student / Novice Developer）
- 次参与者：Python Execution Environment（pytest/Pynguin/coverage.py 子进程）、Local Storage（SQLite + JSON 缓存）

**用例清单（建议 5 个）**
| UC | 名称 | 说明 | 代码证据 |
|---|---|---|---|
| UC01 | Setup Project Environment | 导入/克隆项目、venv 检测、依赖安装 | `Import.vue`、`env.rs` |
| UC02 | Execute Tests and View Results | 选择用例/文件/套件执行、流式日志、结果可视化、保存套件、一键覆盖率 | `Execute.vue`、`execution.rs`、`suites.rs` |
| UC03 | Generate Unit Tests | 选模块、配置算法/时限等、Pynguin 生成 | `Generate.vue`、`generation.rs` |
| UC04 | Analyze Code Coverage | 行级高亮、函数/文件过滤、导出 | `Coverage.vue`、`coverage.rs` |
| **UC05（建议新增）** | **View History and Trends** | 浏览执行/生成/覆盖率历史、对比趋势、导出、详情窗口 | `History.vue`、`db_commands.rs`、`HistoryDetailContent.vue` |

**关系**
- UC01 «include» 检测环境、«include» 安装依赖；（可选）«extend» Clone Repository
- UC02 «include» Collect Coverage Data（一键触发）；«extend» Save as Regression Suite
- UC05 «extend» UC02（查看历史结果）、«extend» UC04（覆盖率历史对比）

**不新增 UC05 的备选方案**：把"查看历史"并入 UC02 的 Extend、"覆盖率历史对比"并入 UC04，保持 4 UC 不重排图号（代价：两个 UC 的 Extend 关系变多）。

### 2.2 用例描述表更新点（Table 4.4–4.7，新增 4.8）

**Table 4.4 UC01（小改）**
- 正常流补充：无 venv 时"系统自动创建 .venv"（`create_virtual_env`）
- 新增可选流：Clone Repository（Dashboard 入口 → `clone_repository` → git clone → 自动 `add_project`）

**Table 4.5 UC02（更新）**
- 正常流第 1 步：补"或选择单个测试用例"（`Execute.vue` 单用例粒度 + Shift 连选）
- 正常流第 10 步：**按补丁文档 Section 1 改为一键触发覆盖率**（弹窗 → autoRun）
- 新增子流：运行中取消（`cancel_run`，SIGTERM→2s→SIGKILL）
- 新增子流：Rerun Failed（`rerunFailedTests`，筛选 failed 结果重跑）
- 新增子流：执行结果持久化（`save_execution_history` + `save_execution_result_details`，8 表之一）
- Extend 补充：Save as Regression Suite 已有，可加"View History"（若不新增 UC05）

**Table 4.6 UC03（更新）**
- 正常流第 2 步：高级配置补全"算法（MOSA/DYNAMOSA/WSPA/RANDOM）、随机种子、染色体长度、种群大小、断言生成"（`Generate.vue`）
- 新增前置检查：环境健康检查（`check_generation_env`，Python/Pynguin 版本）
- 新增异常流：UTF-8 BOM 检测与修复（`check_python_bom` / `strip_python_bom`）
- 正常流第 9 步后补：生成文件语义化重命名（`scripts/rename_generated_tests.py`）+ 统计 test_case_count
- 新增子流：取消生成（`cancel_run`）

**Table 4.7 UC04（小改）**
- 触发方式补充：可从 Execute 页 `?autoRun=1` 自动进入并运行（`Coverage.vue` onMounted）
- 正常流第 5 步细化：文件过滤（>80%/50–80%/<50%）+ **函数级过滤**（前端正则提取 def/类方法）
- 导出细化：JSON / CSV 两种格式（`export_coverage_report`）
- 新增子流：覆盖率历史对比（`list_coverage_history` + 趋势图）

**Table 4.8 UC05（新增，若采纳）**
- 主参与者：User；次参与者：Local Storage
- 正常流：进入 History 页 → `list_execution_history`/`list_generation_history`/`list_coverage_history` 加载三类时间线 → 点条目查看明细（`list_execution_result_details`/`list_generation_file_details`/`get_coverage_history_files`）→ 打开文件定位行号（`open_file`）→ 趋势图（前端渲染）→ 导出（`save_text_file` json/csv）→ 独立详情窗口（WebviewWindow `detail`）

### 2.3 活动图规格（Figure 4.2–4.6）

**Figure 4.2 Setup Project Environment（小改）**
- 起点：用户选择项目目录
- 活动：校验目录（`validate_project_directory`）→ [无效] 报错返回
- 判定：检测环境（`detect_python_env`）→ [无 venv] 创建 .venv（`create_virtual_env`）→ [缺依赖] 安装依赖（`install_dependencies`，循环逐包）→ [就绪]
- 活动：保存项目（`add_project`，重复路径拦截）→ 终点：进入项目页
- （可选分支：Clone Repository → git clone → 同上保存）

**Figure 4.3 Execute Tests（更新）**
- 活动：扫描测试文件（`scan_test_files`）→ 收集用例（`collect_test_cases`）
- 活动：选择范围（全部/文件/单用例）+ 选择 pytest 预设/自定义参数
- 活动：启动 `run_tests` → **循环**：流式日志（事件 `test-output`）→ 进度更新
- 判定：用户取消？→ [是] `cancel_run`（SIGTERM→SIGKILL）→ 终止
- 活动：JUnit 解析 → `test-finished` → 结果汇总入库（`save_execution_history` + `save_execution_result_details`）
- 判定：失败用例？→ [有] 高亮失败 + 用户可 Rerun Failed
- 判定：弹窗操作 → [保存套件] `save_regression_suite` / [运行覆盖率] 跳转 `Coverage?autoRun=1` / [关闭]

**Figure 4.4 View Test Results（更新）**
- 活动：渲染结果卡片（passed/failed/skipped/xfailed 计数 + 逐用例状态）
- 活动：展开失败详情（error_message）/ 复制错误文本
- 活动：打开测试文件定位行号（`open_file`，可配自定义编辑器模板）
- 活动：查看历史（时间线 → 明细 → 独立详情窗口）← 新交互
- 活动：导出历史（`save_text_file` json/csv）

**Figure 4.5 Generate Unit Tests（更新）**
- 活动：扫描源文件（`scan_source_files`）→ 环境健康检查（`check_generation_env`）
- 判定：存在 BOM 文件？→ [是] 弹窗确认 → `strip_python_bom`
- 活动：选择模块 + 高级配置（算法/时限/seed/染色体/种群/断言/输出目录）
- 活动：启动 `generate_tests` → **循环（逐模块）**：Pynguin 子进程 → 事件 `generation-progress`/`generation-output`
- 判定：用户取消？→ [是] `cancel_run`
- 活动：生成后重命名脚本 + 统计 test_case_count → `generation-finished`
- 活动：入库（`save_generation_history` + `save_generation_file_details`）

**Figure 4.6 Analyze Code Coverage（更新）**
- 入口判定：手动进入 或 Execute 页 `?autoRun=1` 自动进入
- 活动：检查 coverage 安装（`check_coverage_installed`）→ [未装] 安装
- 活动：选择源文件/测试文件 → 启动 `run_coverage`（erase → run --branch → json）
- 活动：解析 JSON → 摘要卡片（语句%/分支%）→ 事件 `coverage-finished`
- 活动：点击文件 → `get_coverage_detail` → 四色行级高亮代码查看器
- 活动：文件过滤 / **函数级过滤**（前端正则）
- 活动：导出（`export_coverage_report` json/csv）
- 活动：入库（`save_coverage_result` + `save_coverage_history`）

**（已采纳）Figure 4.7 View History & Trends**
- 活动：进入 History 页 → 加载三类时间线 → 切换类型（执行/生成/覆盖率）→ 点条目 → 打开明细（可对比上一次覆盖率）→ 打开文件定位行号 → 导出 / 独立窗口

### 2.4 序列图规格（Figure 4.8–4.13，全部更新；4.13 为新增）

**通用生命线**：User、Vue Frontend、Rust Backend、Python Engine（pytest/Pynguin/coverage.py）、SQLite / File System

**Figure 4.7 Setup Project Environment（更新）**
```
User → Vue: 选择项目目录（对话框）
Vue → Rust: validate_project_directory(path) → 返回 校验结果
Vue → Rust: detect_python_env(path) → 返回 {pythonPath, pythonVersion, venvExists, dependencies}
alt 无 venv
    Vue → Rust: create_virtual_env(path)
    Rust → Python: python -m venv .venv
loop 缺失依赖
    Vue → Rust: install_dependencies(packages)
    Rust → Python: pip install <pkg>
    Rust --> Vue: 事件 install_step（逐包进度）
Vue → Rust: add_project(name, path, interpreterPath)
Rust → SQLite: INSERT projects
SQLite --> Rust: project id
Rust --> Vue: 返回 id → 跳转项目页
（可选）Dashboard 克隆流: Vue → Rust: clone_repository(url, dir) → Rust → git clone → 同上 add_project
```

**Figure 4.8 Execute Tests（更新）**
```
Vue → Rust: scan_test_files / collect_test_cases
Rust → Python: pytest --collect-only -q
Python --> Rust: 用例列表（:: 分隔）
Rust --> Vue: 用例树（可单选/连选）
User → Vue: 选择范围 + 参数 → 点击 Run
Vue → Rust: run_tests(projectId, path, interpreter, testCases, pytestArgs, regressionSuiteId?)
Rust → Python: 启动 pytest -v --junitxml=<tmp> (junit_family=xunit1)
loop 执行中
    Python --> Rust: stdout/stderr 逐行
    Rust --> Vue: 事件 test-output（流式终端）
alt 用户取消
    Vue → Rust: cancel_run(runId) → SIGTERM → (2s) → SIGKILL
Python --> Rust: 退出码 + JUnit XML
Rust: 解析 JUnit（quick-xml，含自闭合 <skipped/>）
Rust --> Vue: 事件 test-finished {passed, failed, skipped, results}
Rust → SQLite: INSERT test_execution_history + execution_result_details
alt 弹窗操作
    Vue → Rust: save_regression_suite(name, targets, params) → SQLite
    Vue --> Coverage.vue: ?autoRun=1（一键覆盖率）
```

**Figure 4.9 View Test Results（更新，并入历史）**
```
Rust --> Vue: test-finished（结果渲染）
User → Vue: 展开失败详情 / 复制错误
User → Vue: 打开文件定位行号
Vue → Rust: open_file(path, editor?, line?)
User → Vue: 进入 History 页
Vue → Rust: list_execution_history(projectId) → SQLite → 时间线
User → Vue: 点某条记录
Vue → Rust: list_execution_result_details(executionId) → 明细
Vue → Rust: get_execution_project_path(executionId)（独立窗口定位文件）
Vue → Rust: save_text_file(...)（导出 json/csv）
Vue → WebviewWindow: 打开 detail 窗口渲染 HistoryDetailContent
```

**Figure 4.10 Generate Unit Tests（更新）**
```
Vue → Rust: scan_source_files(projectPath) → 源码树（跳过 venv/tests 等）
Vue → Rust: check_generation_env(interpreter) → {pythonVersion, pynguinVersion}
alt 存在 BOM
    Vue → Rust: check_python_bom(files) → 弹窗 → strip_python_bom(files)
User → Vue: 选择模块 + 高级参数
Vue → Rust: generate_tests(projectId, path, interpreter, files, 算法/时限/seed/染色体/种群/断言/输出目录)
loop 逐模块
    Rust → Python: pynguin --module-name ... --algorithm ... --maximum-search-time ...
    Python --> Rust: 输出
    Rust --> Vue: 事件 generation-progress / generation-output
    Rust → Python: rename_generated_tests.py（语义化重命名）
alt 用户取消
    Vue → Rust: cancel_run(runId)
Rust: 扫描输出目录 → test_case_count
Rust --> Vue: 事件 generation-finished {generatedFiles}
Rust → SQLite: INSERT generation_history + generation_file_details
```

**Figure 4.11 Analyze Code Coverage（更新）**
```
入口: 手动 或 Execute 页 ?autoRun=1
Vue → Rust: check_coverage_installed(interpreter)
alt 未安装
    Vue → Rust: install_dependencies(["coverage"])
User → Vue: 选择源文件/测试文件 → 点击 Run Coverage
Vue → Rust: run_coverage(projectId, path, interpreter, testFiles, sourceDirs)
Rust → Python: coverage erase → coverage run --branch --source=<根> --omit=... -m pytest → coverage json -o <app_data>/<projectId>/coverage.json
Python --> Rust: JSON（format 1/2）
Rust: 解析 → 汇总
Rust --> Vue: 事件 coverage-finished {summary}
Rust → SQLite: save_coverage_result + save_coverage_history(+files_json)
Vue: 摘要卡片（语句%/分支%）
User → Vue: 点击文件
Vue → Rust: get_coverage_detail(filePath)
Rust: 读 coverage.json + 源码 → 行级四态（covered/missing/excluded/not-executable）
Rust --> Vue: FileCoverageDetail → 代码查看器高亮
User → Vue: 函数下拉过滤（前端正则提取 def/类方法）
User → Vue: 导出报告
Vue → Rust: export_coverage_report(format=json|csv) → 保存对话框 → 写文件
（历史对比）Vue → Rust: list_coverage_history(projectId) → 趋势图
```

**（已采纳）Figure 4.13 View History & Trends**
```
User → Vue: 进入 History 页
Vue → Rust: list_execution_history / list_generation_history / list_coverage_history
SQLite --> Rust --> Vue: 三类时间线
User → Vue: 切换类型 / 点条目
Vue → Rust: list_execution_result_details / list_generation_file_details / get_coverage_history_files
Vue: 渲染趋势图（CombinedTrendChart，纯前端 SVG）
User → Vue: 打开文件 / 导出 / 独立窗口
Vue → Rust: open_file / save_text_file
Vue → WebviewWindow: detail 窗口
```

---

## 3. Chapter 5 逐图规格

### 3.1 High-Level Architecture（Figure 5.1，小改）
- 保持三运行时分层：Vue 3（UI）→ Tauri IPC → Rust（编排中枢）→ Python 引擎（pytest/Pynguin/coverage.py）
- **补三处**：① 持久化层画 **8 张表**（SQLite）+ JSON 缓存（app_data/{projectId}/coverage.json）；② 事件通道标注（test-*/coverage-*/generation-* 等 6 类）；③ 前端模块补 History（时间线/趋势/详情窗口）
- 建议同时修正正文交叉引用：§5.1 的 "As illustrated in Figure 4.1" → Figure 5.1

### 3.2 组件交互序列图（Figure 5.2–5.5，更新）
- 与 Ch.4 序列图内容一致，**区别在生命线视角**：Ch.4 以 User 为主角（用户→系统），Ch.5 以组件为主角（Vue 组件 → Rust 核心 → Python 引擎 → SQLite/文件系统），去掉 User 生命线、把"事件"画成组件间异步消息
- 5.2 Test Environment Setup ← 对应 Ch.4 Figure 4.7
- 5.3 Automated Test Generation ← 对应 Ch.4 Figure 4.10
- 5.4 Test Execution ← 对应 Ch.4 Figure 4.8
- 5.5 Automated Coverage Analysis ← 对应 Ch.4 Figure 4.11（注意正文措辞按补丁文档改为一键触发）
- 每张图补 DB 写回（8 表相关插入）

### 3.3 ERD（Figure 5.6，重画为 8 表）
- 内容与补丁文档 `docs/thesis-sync-patch-2026-08-29.docx` Section 3 完全一致：9 条关系 + 8 实体
- 关键点：projects 为根实体 5 张子表；execution_result_details / generation_file_details 为明细表；regression_suites→test_execution_history 为 SET NULL；coverage_results.execution_id 为可选连接（无强 FK）

### 3.4 可选加分：UML Component Diagram（建议新增，Ch.5 或 Ch.6）
- 组件：Vue Frontend（views/components/stores）→ [Tauri IPC] → Rust Core（commands: env/execution/generation/coverage/suites/files/db；state 进程注册表）→ [stdio 子进程] → Python Engine（pytest/Pynguin/coverage.py）；Rust Core → [SQLite]（8 表）→ [File System]（coverage.json / running_pids.json）
- 答辩可讲解"28 个类型化 IPC 命令 + 6 类事件通道"（`lib.rs` 命令注册表）

---

## 4. 图号管理表（重画时对照，避免重复/跳号）

| 位置 | 图号 | 说明 |
|---|---|---|
| Ch.4 用例图 | Figure 4.1 | 保持 |
| Ch.4 活动图 | Figure 4.2–4.6 | 保持；**新增 History 活动图 → Figure 4.7** |
| Ch.4 序列图 | Figure 4.8–4.13 | **全部 +1 顺延**（原 4.7–4.11 → 4.8–4.12，新增 History 序列图 → 4.13） |
| Ch.5 架构图 | Figure 5.1 | 保持 |
| Ch.5 组件交互序列图 | Figure 5.2–5.5 | 保持 |
| Ch.5 ERD | Figure 5.6 | 保持（**删除重复的第二个 Figure 5.6**） |
| ~~Ch.5 原型~~ | **已删除 §5.4** | UI 真实截图移入 Ch.6 §6.4（每屏截图 + FR 标注，见 checklist A6/A7） |

> 已采纳 UC05 + 两张新图：Ch.4 活动图 4.2–4.6 保持、新增 4.7；序列图由 4.7–4.11 变为 **4.8–4.13**（6 张）。
> 若不想重排号，备选方案是把历史并入 UC02/UC04（见 2.1）。

---

## 5. 建议绘制顺序与工具

1. **ERD（5.6）** → 2. **用例图（4.1）+ 用例描述表（4.4–4.8）** → 3. **活动图（4.2–4.6）** → 4. **序列图（4.7–4.11）** → 5. **Ch.5 组件序列图（5.2–5.5）**（从 Ch.4 改视角）→ 6. **架构图（5.1）**
- 工具任选：draw.io（免费、可导出图片）、PlantUML（文本即图、可版本管理）、Visio/PowerPoint（直接贴 Word 风格一致）
- 每张图完成后核对：图中每个步骤都能在本文"代码证据"列找到对应函数

---

## 6. 下一步交付（可选）

我可以直接为你生成：
1. **PlantUML 源码**（每张图一个 `.puml` 文件：用例图 / 5 活动图 / 5+1 序列图 / ERD / 组件图），你用 PlantUML 或 VS Code 插件一键渲染导出 PNG/SVG 贴入 Word；
2. **Mermaid 源码**（贴到 mermaid.live 即可渲染，无需安装）；
3. **draw.io 可编辑文件**（.drawio，双击即可改）。

选一个格式，我就按本方案把全部图生成出来。
