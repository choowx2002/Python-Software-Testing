# Testmate 毕业设计终审报告（Final Review）

> 审查对象：`docs/LKCFES-FYP Report-CHOO WEI XIANG.docx`（2026-08-28 版）+ 工作区全部代码（git HEAD `6958681` + 未提交修改，2026-08-29）
> 审查基准：报告 Ch.1 §1.3 Problem Statement（1.3.1–1.3.4）与 §1.4 四个 Objectives、Ch.4 Table 4.1（FR001–FR013）/ Table 4.2（NFR001–NFR008）
> 审查方式：静态代码审查（Rust 后端 + Vue 前端）+ 实测验证（`cargo test` / `pnpm lint` / `vue-tsc` / `vite build`）+ 报告逐章比对

---

## 0. 本次实测验证结果（2026-08-29 独立复现）

| 检查项 | 结果 |
|---|---|
| `cargo test`（Rust 后端单元测试） | ✅ 26 passed / 0 failed（JUnit 解析、coverage JSON 双格式、DB 聚合解码、编辑器模板展开） |
| `pnpm lint`（ESLint） | ✅ 0 errors / 0 warnings（8-28 审查时的 6 errors + 3 warnings 已全部修复） |
| `vue-tsc --noEmit`（TypeScript） | ✅ 0 errors |
| `vite build`（前端生产构建） | ✅ 成功（6.88s，产物在 dist/） |
| 功能模块 | 执行 / 生成 / 覆盖率 / 回归套件 / 历史 五条主链路代码完整，无占位符 |

---

## 1. 核心目标对齐检查（Objective Coverage Check）

### Objective 1 — 集成化自动测试工作流（单元测试 + 回归套件，降低配置开销）
**状态：✅ 已完全实现（超出要求）**

| 子目标 | 对应代码 | 评价 |
|---|---|---|
| 单元测试执行 | `execution.rs`：`scan_test_files` / `collect_test_cases` / `run_tests` / `cancel_run`；`Execute.vue` 三作用域（全部/文件/单用例） | 粒度超过论文要求（支持到单个用例 + Shift 连选 + Rerun Failed） |
| 回归测试套件 | `suites.rs`：保存 / 列表 / 删除 / `run_regression_suite` 一键重跑（复用 `run_tests_core`，`execution_type=REGRESSION`）；`Execute.vue` 套件面板 | 完整闭环，历史入库 |
| 配置开销降低 | `env.rs`：`detect_python_env`（.venv/venv→全局回退）/ `create_virtual_env` / `install_dependencies`；`Import.vue` 五阶段向导 | 用户无需手写 pytest.ini / .coveragerc / Pynguin CLI 参数，痛点 1.3.2 直接命中 |

### Objective 2 — GUI 简化配置 + 测试结果与覆盖率的直观可视化反馈
**状态：✅ 已完全实现**

- `Execute.vue`：流式日志终端（`test-output` 逐行事件）、passed/failed/skipped/xfailed 状态卡片、失败详情展开、错误面板 + 复制。
- `Coverage.vue`：总览卡片（语句% + 分支% 分离口径）、四色行级代码查看器（covered/missing/excluded/not-executable）、函数级下拉过滤（正则提取 def/类方法，7 组边界自测）。
- `Dashboard.vue` / `Overview.vue`：真实聚合数据（Rust 端 JOIN，非硬编码；8-15 基线中的硬编码问题已修复）。

### Objective 3 — 自动化测试用例生成（降低手工编写与知识门槛）
**状态：✅ 已完全实现（超出要求）**

- `generation.rs`：`scan_source_files`（跳过 19 类目录）+ `generate_tests`（Pynguin 全参数：MOSA/DYNAMOSA/WSPA/RANDOM、时限、染色体、种群、迭代数、断言、seed）+ BOM 自愈 + `scripts/rename_generated_tests.py` 语义化重命名。
- `Generate.vue`：环境健康检查（Python/Pynguin 版本）、进度、生成文件列表。
- 实测背书（FR-NFR 报告）：demo-shop 3 模块各 60s 共 180s 生成 35 个测试，34 passed + 1 xfail。直接回应痛点 1.3.3（Fraser & Arcuri 论据落地）。

### Objective 4 — 端到端回归验证 + 覆盖率交互式解读（消除 CLI 认知负担）
**状态：✅ 已完全实现**

- 回归自动化：`run_regression_suite` 一键重跑 + `execution_type` 区分 MANUAL/REGRESSION，入库可追溯。
- 覆盖率解读：行级高亮 + 函数过滤 + 文件过滤（>80% / 50–80% / <50%）+ JSON/CSV 导出（`export_coverage_report`）。
- 历史与趋势：`History.vue` 三类时间线（执行/生成/覆盖率）+ `CombinedTrendChart.vue` 趋势图 + 独立详情窗口（WebviewWindow label=detail）+ 历史导出。直接回应痛点 1.3.4（"no visual cue when a previously passing test has begun to fail"）。

### Problem 解决程度（对照 §1.3 四个子问题）

| 痛点 | 解决机制 | 结论 |
|---|---|---|
| 1.3.1 跳过测试 + 教育缺口 | 导入向导 + 环境自愈 + Overview 三步引导 + NFR001 实测 5 分钟全流程（<15 min） | ✅ 结构性降低准入门槛 |
| 1.3.2 工具链碎片化 / 配置开销 | 三引擎统一 GUI 编排，零手工配置 | ✅ 直接消除 |
| 1.3.3 手工写测试的知识壁垒 | Pynguin 搜索式生成（含 DynaMOSA），实测生成套件全通过 | ✅ 直接消除 |
| 1.3.4 回归不可见 + 覆盖率难解读 | 套件一键重跑 + 历史趋势 + 行级高亮 | ✅ 直接消除 |

**结论：代码 100% 覆盖 Problem 与四个 Objectives，且大多数维度超出论文承诺。**

---

## 2. 毕业作品成熟度评估

### 功能完整性（9/10）
- 核心流程全部跑通：导入 → 环境检测 → 依赖安装 → 测试收集 → 执行（流式日志 + JUnit 解析）→ 结果入库 → 覆盖率采集（statement + branch）→ 行级高亮 → 导出；回归套件 → 一键重跑 → 历史对比。
- 边界情况处理成熟：JUnit `<skipped/>` 自闭合空元素（曾有真实 bug，已被单测捕获并修复）、coverage.json format 1/2 双兼容、BOM 文件、Windows/Unix 路径、PYTHONPATH 包/非包布局（--source "." 解析）、256MB 文件读取上限、取消运行（SIGTERM→2s→SIGKILL）、崩溃孤儿进程清理、单实例。
- 唯一口径偏差：FR011 为"执行完成弹窗 → 一键跳转自动运行"（半自动），非论文字面的"automatically triggers"。

### 代码质量（8/10）
- 架构清晰：Rust 8 个命令模块（execution/coverage/generation/env/suites/files/db/window）+ db 层 + state；前端按域拆分 views/components/composables/stores；28 个类型化 IPC 命令、6 类事件通道。
- 命名规范、注释到位（含中文设计决策注释）、i18n en/zh 570 键对齐、严格 CSP + capability 最小化、编辑器模板不经 shell 展开。
- 工程自检：26 个 Rust 单测、lint/type-check/build 全绿。**8-28 审查的 6 个 lint 错误（含 2 个真实 bug：Ctrl+` 快捷键失效）已在 8-29 未提交修改中修复。**
- 残留小问题：`fix_python_env` 无确认删除 .venv 且硬编码 Windows/winget/Python 3.11；`validate_project_interpreter` 全局回退写入裸命令名（"python3"）导致 `run_tests` 的 `Path::exists()` 校验失败；前端无 vitest 单测；`commands.rs` 遗留 `#[allow(unused_variables)]`。

### 技术深度（8.5/10）
Tauri v2 多进程编排（tokio 异步子进程 + 事件流）、quick-xml JUnit/coverage JSON 双格式解析、Pynguin 全参数配置、分支覆盖率采集、进程生命周期治理、SQLite 迁移（DROP+ALTER+PRAGMA 探测）、独立 WebviewWindow 详情窗口 —— 对本科 FYP 属明显超出平均水准的技术含量，足以支撑学分。

---

## 3. 致命缺陷与必须修复项（Blockers）

**按答辩风险排序（代码层面无"不合格"级问题，风险集中在论文与 2 个演示翻车点）：**

### 🔴 B1 论文缺 Chapter 6（Implementation, Testing and Evaluation）与结论章 —— 交付物不完整
- 报告止于 Ch.5 + REFERENCES（已逐行核实全文）。UTAR FYP 硬性要求实现/测试/评估章节。
- **为什么必须改**：答辩评委无从看到系统验证证据；你已有全部素材（FR-NFR 报告：5 分钟全流程、35 tests、语句 59.2%→93.2%、分支 41.7%→89.6%、cargo test 26 项），写成正式章节即可。预估 2–3 天。

### 🔴 B2 数据字典 / ERD 与实现 schema 不一致（4 表 vs 8 表）
- 报告 Table 5.x 只收录 projects / regression_suites / test_execution_history / coverage_results；实现有 8 表（另有 generation_history / coverage_history / execution_result_details / generation_file_details），且 coverage_results 列名与约束完全不同（cache_file_path vs detail_json_path、execution_id 无 UNIQUE/FK）。
- **为什么必须改**：答辩最容易被当场抓包的点（"论文说 4 张表，代码里 8 张？"）。

### 🔴 B3 报告日期与开发记录矛盾
- 封面 May 2026、声明 22/04/2026，而 git 提交为 2026-07-19 ~ 08-29。若评审核对 git 历史，"报告先于开发"会引发真实性质疑。
- **必须改**：日期统一为实际提交时间；logbook 补 7/19–8/9 空白段。

### 🟡 B4 `fix_python_env` 演示翻车点（无确认删除 + 平台硬编码）
- `env.rs` 直接 `remove_dir_all(.venv)` 无确认弹窗；winget 仅 Windows，Linux/macOS 上会报 "Failed to run winget"。
- **为什么必须改**：评委现场点"一键修复环境"→ 误删用户 venv 或报错，观感极差。前端加 AppConfirmModal 确认 + 后端非 Windows 平台守卫。

### 🟡 B5 `validate_project_interpreter` 恢复路径隐患
- `find_interpreter_for_project` 全局回退返回裸命令名（"python3"），写库后 `run_tests` 的 `Path::exists()` 失败报 "Python interpreter does not exist"。
- **为什么必须改**：演示"删除 venv 后自动恢复"会翻车。改为解析绝对路径（`python3 -c "import sys; print(sys.executable)"`）。

### 🟡 B6 FR011 措辞与实现不符（改论文即可，推荐）
- 论文："system shall automatically trigger the coverage tool immediately after a successful test execution"；实现：完成弹窗 → 点击 → 跳转自动运行。改论文措辞为 "provide one-click coverage collection immediately after test execution" 即闭环。

### 🟢 B7 论文格式占位（半天可清）
TOC 页码全为 "1"；LIST OF SYMBOLS/APPENDICES 有目录无正文；5.4.1 与 5.4.2 标题重复（均为 "Project Management Dashboard"）；Figure 5.6 编号重复；笔误（"Text Execution"、"Regression Suits"、"Sequences Diagrams"）。

---

## 4. 文档同步建议（Documentation Sync）

文档确实落后于代码，以下章节需按当前代码更新/重写：

| 章节 | 现状 | 需更新为 |
|---|---|---|
| Ch.1 | Problem/Objectives 与代码吻合，无需改 | — |
| Ch.4 §4.2.1 FR011 | 措辞为全自动 | "一键覆盖率采集"（见 B6） |
| Ch.4 UC02/UC03 | 无单用例粒度、回归套件 UI、Pynguin 高级参数 | 补充：单用例选择、Rerun Failed、套件面板、算法/seed/断言配置 |
| Ch.5 §5.1 架构 | 三运行时架构与实现一致 ✅ | 可补 28 IPC 命令 + 6 事件通道清单（答辩加分） |
| Ch.5 §5.3.2/5.3.3 ERD + 数据字典 | 4 表，coverage_results 字段过期 | **重写为实际 8 表 schema**（见 B2）；删 "Snowflake ID" 措辞 |
| Ch.5 §5.4 原型图 | 线框 mockup | 替换为真实截图（Dashboard/Execute/Coverage/History 各一张） |
| 缺失 Ch.6 | 无 | 新增 Implementation / Testing / Evaluation，用 FR-NFR 报告实测数据 + cargo test 输出 + 截图 |
| 缺失 Ch.7 | 无 | 新增 Conclusion，**逐一回应四个 Objectives**（可用本报告 §1 的对照表） |
| 附录 | 无 | 截图集 / 数据表 / 用户手册（已有素材） |
| 日期 / logbook | May 2026 / 8-10~8-22 | 统一为实际开发时间，补 7-19~8-9 |

> 8-15 的 `alignment-report.md` 与 8-28 的 `code-review-report.md` 中的"缺失"项（覆盖率 0%、套件仅建表、Dashboard 硬编码、lint 6 errors）在 8-29 现状中均已闭环，两篇文档本身也已过期，可作为 Ch.6 素材但不可再引用为"现状"。

---

## 5. 最终结论与下一步行动

### 当前状态
- **代码**：✅ 可直接答辩（26 单测全绿、lint/type-check/build 干净、五大功能闭环、13/13 FR、8/8 NFR 架构合规）—— 修完 B4/B5 两个演示翻车点即为满分态。
- **论文**：❌ 需要大修 —— 缺 Ch.6/Ch.7、数据字典过期、日期矛盾。**"代码完成度高 + 论文缺评估章节"是当前唯一硬伤。**

### 最紧急的 3 件事
1. **写 Ch.6（系统测试与评估）+ Ch.7（结论）** —— 素材已齐（FR-NFR 报告实测数据 + 本报告 §1 目标对照表），2–3 天完成；这是"合格毕业作品"的最后一块拼图。
2. **同步论文与代码** —— 数据字典改为 8 表、FR011 措辞、报告日期与 logbook 对齐 git 历史、清 TOC 占位与重复编号（1 天内）。
3. **修 2 个演示翻车点 + 全流程彩排** —— `fix_python_env` 加确认弹窗与平台守卫、`validate_project_interpreter` 恢复绝对路径；用 demo 项目完整走一遍 导入→生成→执行→覆盖率→回归套件→历史趋势，录好关键画面。

### 答辩风险提示
- 最大送分点（务必主动展示）：cargo test 26 项、覆盖率 59.2%→93.2%（分支 41.7%→89.6%）、回归套件一键重跑、历史趋势图。
- 最大送命题（务必提前处理）：论文与代码不一致（B2/B3）、FR011 措辞（B6）、演示时误删 venv（B4）。

---

*审查证据：`cargo test` 26/26 ✅；`pnpm lint` 0/0 ✅；`vue-tsc --noEmit` 0 errors ✅；`vite build` ✅；git HEAD `6958681` + 7 个未提交 lint 修复文件（8-29）；报告全文提取比对（docs/report-extract-clean.txt）。*
