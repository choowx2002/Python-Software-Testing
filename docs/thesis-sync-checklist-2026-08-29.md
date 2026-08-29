# 论文 ↔ 代码同步修改清单（可直接照做）

> 目标文件：`docs/LKCFES-FYP Report-CHOO WEI XIANG.docx`
> 依据：当前代码（git HEAD `6958681` + 8-29 未提交修改）、`docs/report-extract-clean.txt`（论文全文提取）
> 用法：在 Word 中按条目定位原文 → 替换为"改为"内容。所有代码证据均已核验。

---

## A. 必须修改（一致性硬伤 —— 答辩最容易被抓包的点）

### A1. §5.3.3 数据字典：Table 5.1–5.4 → 实际 8 张表 【最高优先，约 1 小时】

**现状问题**：论文只收录 4 张表；实现有 **8 张表**（`src-tauri/src/db.rs` INIT_SQL，L34–152）。且 Table 5.4（Coverage Results）列名、约束与实现**完全不同**；每张表的 id 描述都写 "Unique Snowflake identifier"（实现是 SQLite AUTOINCREMENT）。

**操作**：把 §5.3.3 的 Table 5.1–5.4 整体替换为下面 8 张表（格式沿用论文的 Field / Data Type / Constraints / Description 四列）：

**Table 5.1: Projects Table Structure.（修正版，实际：`project_path` 有 UNIQUE）**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each imported project. |
| name | VARCHAR(255) | NOT NULL | User-defined project name or detected folder name. |
| project_path | TEXT | NOT NULL, UNIQUE | Absolute local file system path of the Python project. Duplicate paths are rejected on import. |
| interpreter_path | TEXT | NULL | Path of the detected or manually configured Python virtual environment interpreter. |
| status | VARCHAR(20) | DEFAULT 'active' | Current project availability status (active, missing, invalid). |
| created_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | Timestamp when the project was imported into the system. |
| last_opened_at | TIMESTAMP | NULL | Latest timestamp when the user accessed the project. |

**Table 5.2: Test Execution History Table Structure.（修正版，标题改正拼写）**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each test execution record. |
| project_id | INTEGER | NOT NULL, FOREIGN KEY projects(id) ON DELETE CASCADE | Identifies the project associated with this test execution. |
| regression_suite_id | INTEGER | FOREIGN KEY regression_suites(id) ON DELETE SET NULL | Identifies the regression suite used. Null if manually triggered. |
| execution_type | VARCHAR(20) | NOT NULL | Defines the execution source (MANUAL or REGRESSION). |
| execution_status | VARCHAR(20) | NOT NULL | Final execution status (SUCCESS, FAILED, ERROR, TIMEOUT). |
| command | TEXT | NULL | The pytest command executed. |
| total_tests | INTEGER | NOT NULL | Total number of detected and executed test cases. |
| passed | INTEGER | NOT NULL | Number of successfully passed test cases. |
| failed | INTEGER | NOT NULL | Number of failed test cases. |
| skipped | INTEGER | NOT NULL | Number of skipped test cases. |
| execution_time | REAL | NOT NULL | Total execution duration in seconds. |
| executed_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | Date and time when the test execution started. |

**Table 5.3: Regression Suites Table Structure.（修正版，标题改正拼写）** — 原表内容与实现一致（7 字段），只需：① 标题 "Regression Suits" → "Regression Suites"；② id 描述 "Snowflake identifier" → "Unique identifier"；③ project_id 约束补 "NOT NULL, ... ON DELETE CASCADE"。

**Table 5.4: Coverage Results Table Structure.（重写 —— 原表 7 列全错）**

| Field Name | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each coverage result record. |
| project_id | INTEGER | NOT NULL, FOREIGN KEY projects(id) | Associates the coverage result with a specific Python project. |
| execution_id | INTEGER | NULL | Optional link to the test execution that produced this coverage data (not enforced as a foreign key). |
| total_statement_coverage | REAL | NOT NULL | Overall statement coverage percentage calculated from the executed tests. |
| total_branch_coverage | REAL | NULL | Overall branch coverage percentage (populated when branch collection is enabled). |
| file_count | INTEGER | NOT NULL | Number of source files included in the coverage measurement. |
| covered_file_count | INTEGER | NOT NULL | Number of source files with at least one executed statement. |
| detail_json_path | TEXT | NULL | Local path pointing to the JSON file containing detailed line-level coverage information. |
| created_at | DATETIME | DEFAULT CURRENT_TIMESTAMP | Timestamp when the coverage result was generated. |

**新增 Table 5.5: Generation History Table Structure.（实现表，论文缺失）**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each test generation record. |
| project_id | INTEGER | NOT NULL, FOREIGN KEY projects(id) ON DELETE CASCADE | Identifies the project for which tests were generated. |
| generation_status | VARCHAR(20) | NOT NULL | Final generation status (SUCCESS, FAILED, ...). |
| total_files | INTEGER | NOT NULL DEFAULT 0 | Number of source modules selected for generation. |
| generated_files | INTEGER | NOT NULL DEFAULT 0 | Number of test files actually produced. |
| duration | REAL | NOT NULL DEFAULT 0 | Total generation duration in seconds. |
| command | TEXT | NULL | The Pynguin command executed. |
| executed_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | Timestamp when the generation run started. |

**新增 Table 5.6: Coverage History Table Structure.**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each coverage run record. |
| project_id | INTEGER | NOT NULL, FOREIGN KEY projects(id) ON DELETE CASCADE | Identifies the project measured. |
| coverage_status | VARCHAR(20) | NOT NULL | Status of the coverage run (SUCCESS, FAILED). |
| percent_covered | REAL | NOT NULL DEFAULT 0 | Overall statement coverage percentage. |
| total_statements | INTEGER | NOT NULL DEFAULT 0 | Total number of executable statements measured. |
| covered_statements | INTEGER | NOT NULL DEFAULT 0 | Number of statements executed by the tests. |
| duration | REAL | NOT NULL DEFAULT 0 | Total coverage run duration in seconds. |
| command | TEXT | NULL | The coverage command executed. |
| files_json | TEXT | NULL | JSON snapshot of per-file line-level coverage for historical comparison. |
| executed_at | TIMESTAMP | DEFAULT CURRENT_TIMESTAMP | Timestamp when the coverage run started. |

**新增 Table 5.7: Execution Result Details Table Structure.**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each per-test-case detail record. |
| execution_id | INTEGER | NOT NULL, FOREIGN KEY test_execution_history(id) ON DELETE CASCADE | Links the detail to its parent test execution. |
| name | TEXT | NOT NULL | Name of the individual test case. |
| file | TEXT | NULL | Test file containing the test case. |
| status | TEXT | NOT NULL | Result status (passed, failed, error, skipped, xfailed). |
| duration | REAL | NOT NULL DEFAULT 0 | Execution time of this test case in seconds. |
| error_message | TEXT | NULL | Captured failure or error message. |
| line | INTEGER | NULL | Line number of the test case in its file. |
| skip_reason | TEXT | NULL | Reason for skipping, when the case was skipped. |

**新增 Table 5.8: Generation File Details Table Structure.**

| Field | Data Type | Constraints | Description |
|---|---|---|---|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | Unique identifier for each generated-file detail record. |
| generation_id | INTEGER | NOT NULL, FOREIGN KEY generation_history(id) ON DELETE CASCADE | Links the detail to its parent generation run. |
| name | TEXT | NOT NULL | Name of the generated test file. |
| relative_path | TEXT | NULL | Path of the generated file relative to the project root. |
| test_case_count | INTEGER | NOT NULL DEFAULT 0 | Number of test cases detected in the generated file. |
| status | TEXT | NULL | Generation result for this file (success, empty, failed). |

> 建议在 Table 5.8 后加一句说明："In addition to the four core tables described in the original design, four supporting tables (Tables 5.5–5.8) were introduced during implementation to persist generation history, coverage history, and per-execution detail records for the history module and trend visualization."

### A2. §5.3.2 ERD（Figure 5.6）：重画为 8 表关系图

**现状**：ERD 只含 4 表；关系描述（L901–905）只写 4 条。
**改为**（照实际 schema 的 FK 画）：
- projects 1—N test_execution_history（project_id，CASCADE）
- projects 1—N regression_suites（project_id，CASCADE）
- projects 1—N coverage_results（project_id）
- projects 1—N generation_history（project_id，CASCADE）
- projects 1—N coverage_history（project_id，CASCADE）
- regression_suites 1—N test_execution_history（regression_suite_id，SET NULL）
- test_execution_history 1—N execution_result_details（execution_id，CASCADE）
- generation_history 1—N generation_file_details（generation_id，CASCADE）
- test_execution_history 0..1—1 coverage_results（execution_id，无强约束，可选连接）
- 正文 L899 的段落相应改为："...The Coverage Results table stores overall coverage metrics mapped to the JSON cache files. In addition, Generation History and Coverage History tables record the running history of the generation and coverage modules, while Execution Result Details and Generation File Details store per-test-case and per-file granular records for the history view."

### A3. "自动触发覆盖率" 措辞 ×4 处（FR011 + UC02 + §5.2.4 + UC02 摘要）【半天】

实现现状：测试完成后**弹出结果弹窗 → 用户点"运行覆盖率"→ 跳转并自动执行**（`Execute.vue` `runCoverageFromPostRun` → `Coverage.vue?autoRun=1`），是"一键触发"而非全自动。**推荐改论文（不动代码）**：

1. **Table 4.1 FR011**（L580）：
   - 原文："The system shall automatically trigger the coverage tool immediately after a successful test execution to collect data."
   - 改为："The system shall provide one-click code coverage collection immediately after test execution completes, without requiring the user to reconfigure the coverage toolchain."
2. **§4.3.4 UC02 Brief Description**（L730）：
   - 原文："...Upon completion, it automatically triggers code coverage data collection and provides an optional workflow..."
   - 改为："...Upon completion, it offers a one-click action to trigger code coverage data collection and provides an optional workflow..."
3. **UC02 Normal Flow of Events 第 10 步**（L748）：
   - 原文："The system automatically triggers the coverage library in a new subprocess to collect statement and branch coverage data."
   - 改为："The system prompts the user with a post-run summary and offers a one-click action to trigger the coverage library in a new subprocess, collecting statement and branch coverage data."
4. **§5.2.4 Automated Coverage Analysis**（L893）：
   - 原文："After a successful test execution, the Rust backend automatically triggers coverage.py library in subprocess to collect statement and branch coverage data."
   - 改为："After a test execution completes, the Rust backend presents a post-run summary; upon the user's one-click confirmation, it invokes coverage.py in a subprocess to collect statement and branch coverage data. The raw coverage output is parsed into a structured JSON format by the Rust layer and cached in local storage, after which an event is emitted to the frontend to render an interactive code viewer, highlighting covered and uncovered lines."

### A4. 日期矛盾（封面 + 声明 + logbook）【一次改动】

- 封面 "May 2026"、声明 "Date: 22/04/2026"；而 git 开发记录为 **2026-07-19 ~ 08-29**（`git log`）。
- **必须**：把封面/声明日期改为与实际完成时间一致（你确认真实提交时间线后统一，例如 2026-09 或实际答辩季）；`docs/logbook.md` 目前只覆盖 8/10–8/22，**补 7/19–8/9**（git log 有真实提交记录可回填：c20dae0 起）。

### A5. 目录（TOC）页码全部为 "1"（占位符）

- 目录中所有条目页码都是 "1"。在 Word 中右键目录 → 更新域（Update Field），或重排后重新生成目录。

### A6. §5.4 Prototype User Interface Design：**整节删除，UI 内容移入 Ch.6**【已确认学院模板无强制要求】

- **现状问题**：5.4.1 与 5.4.2 标题均为 "Project Management Dashboard"（5.4.2 无正文直接进 REFERENCES）；Figure 5.6 编号重复（L900 ERD、L1061 Dashboard 原型）。
- **决定**：删除 §5.4 整节。理由：① 答辩更看重"实现与文档一致"，真实截图比线框原型更有说服力；② 删除后 Figure 5.6 重复编号自动消失，Ch.5 图号止于 5.6；③ UI 展示移入 Ch.6 新增的 §6.4 User Interface Implementation（见 A7）。
- **操作**：Word 中删除 §5.4 全部内容（含重复小节与原型图）；把 §5.4 现有的 Dashboard 功能分区描述（L1060–1068）改写后并入 Ch.6 §6.4.1；Ch.5 其余图号（5.1–5.6）不变。

### A7. 缺 Chapter 6（Implementation, Testing and Evaluation）与 Chapter 7（Conclusion）【2–3 天，最高优先】

报告止于 Ch.5 + REFERENCES。这是**交付物不完整**的最大问题。骨架与素材（全部来自你已有的实测，直接可用）：

```
CHAPTER 6: IMPLEMENTATION, TESTING AND EVALUATION
6.1 Implementation Highlights
    6.1.1 Three-Runtime Architecture Implementation（28 typed IPC commands, 6 event channels）
    6.1.2 Test Execution Pipeline（async subprocess + JUnit(xunit1) parsing）
    6.1.3 Pynguin Generation Pipeline（algorithms, BOM handling, rename script）
    6.1.4 Coverage Analysis Pipeline（--branch, JSON format 1/2, line-level viewer）
    6.1.5 Regression Suites and History（FR007 + 4 supporting tables）
6.2 System Testing
    6.2.1 Unit Testing of the Application Itself —— cargo test: 26 tests, all passed
          （JUnit parsing incl. self-closing <skipped/>, coverage JSON dual-format,
            DB REAL decoding regressions, editor template expansion）
    6.2.2 Functional Requirement Verification（FR001–FR013 traceability table, 13/13）
    6.2.3 Non-Functional Requirement Evaluation
          NFR001: first end-to-end workflow measured ≈ 5 minutes (target < 15 min)
          NFR003/004: async event streaming design; UI never blocks (record measured values)
          NFR005: 3 modules (< 200 lines each) generated in 60s each → 35 tests in 180s
    6.2.4 End-to-End Workflow Demonstration（demo project screenshots）
    6.2.5 Effectiveness Analysis
          覆盖前后对比：statement 59.2% → 93.2%（61→96/103），branch 41.7% → 89.6%（20→43/48）；
          生成套件执行：34 passed + 1 xfail，0 unexpected failure
6.3 Limitations and Threats to Validity（对照 Ch.1.5：语言泛化、复杂逻辑生成、规模、评测环境）
6.4 User Interface Implementation（原 §5.4 内容迁入；每屏真实截图 + 3–5 句说明 + FR 对应）
    6.4.1 Project Management Dashboard（截图；FR001–FR003；原 §5.4 描述改写）
    6.4.2 Test Execution Interface（截图：用例选择/流式日志/结果卡片/回归套件面板；FR004–FR007）
    6.4.3 Test Generation Interface（截图：源码树/算法配置/进度/生成文件；FR008–FR010）
    6.4.4 Coverage Analysis Interface（截图：语句/分支卡片 + 四色行级高亮 + 函数过滤 + 导出；FR011–FR013）
    6.4.5 History and Trends（截图：时间线/趋势图/详情窗口；UC05）
6.5 Summary
CHAPTER 7: CONCLUSION AND FUTURE WORK
7.1 Achievement of Objectives —— 逐一回应 Ch.1 §1.4 四个 Objectives（对照表见
     docs/fyp-final-review-2026-08-29.md §1，可直接改写为段落）
7.2 Future Work（CI/CD 集成、多语言支持、更大规模评测、LLM 辅助生成、可插拔生成器）
APPENDICES
Appendix A: 界面截图集（Dashboard / Import / Execute / Generate / Coverage / History）
Appendix B: cargo test 输出与需求追溯表
Appendix C: 用户手册（docs/user-manual.md 直接排版）
```

### A8. LIST OF SYMBOLS / ABBREVIATIONS 与 LIST OF APPENDICES 有目录无正文

- TOC 列了 "LIST OF SYMBOLS / ABBREVIATIONS"（ix）和 "LIST OF APPENDICES"（x），正文均无。
- 二选一：删除目录条目；或补内容（符号表列 pytest / Pynguin / coverage.py / IPC / JUnit / SQLite / TDD / GUI 等；附录见 A7）。

---

## B. 建议修改（准确性 / 加分项）

### B1. 笔误 ×4
- §2.6.1 标题 "Text Execution Library Comparison" → "**Test** Execution Library Comparison"
- Table 5.2 标题 "Text Execution History Table Structure" → "**Test** Execution History Table Structure"
- Table 5.3 标题 "Regression Suits Table Structure" → "Regression **Suites** Table Structure"
- §4.5 标题 "Sequences Diagrams" → "Sequence Diagrams"

### B2. §5.3.1 与数据字典中 "Snowflake ID" 措辞
- 实现是 SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`，无雪花算法。删除所有 "Snowflake" 字样，统一为 "unique identifier"（见 A1 各表）；§5.3.1 L897 "Each imported project is assigned a unique Snowflake ID" → "Each imported project is assigned a unique identifier by the embedded SQLite database".

### B3. UC02 / UC03 补充已实现细节（可选，答辩加分）
- UC02 流程第 1 步："selects a specific test file or folder" → 可加 "or an individual test case"（实现支持单用例粒度 + Shift 连选 + 搜索过滤 + Rerun Failed，`Execute.vue`）。
- UC03 流程第 2 步："configures advanced generation settings, such as time limits and target functions" → 可加 "and algorithm, random seed, chromosome length, population size, and assertion generation"（实现支持，`generation.rs`）。
- UC03 补充：生成文件会经过语义化重命名脚本处理（`scripts/rename_generated_tests.py`）——可加一句到流程第 9 步之后。

### B4. §5.1 交叉引用错误（需在 Word 里核对）
- L873 "As illustrated in **Figure 4.1**" —— 上下文是 Ch.5 架构，且紧跟其后的图是 "Figure 5.1: High-Level Architecture"，应为 **Figure 5.1**（若 Word 中 Figure 4.1 另有他图，则改为正确编号）。

### B5. 参考文献
- 9 条引用与正文引用点对应良好，无需增删；如需更扎实，可在 Ch.2 补 coverage.py 官方文档引用（可选）。

---

## C. 已一致、**不要动**的部分（避免白改）

- **Ch.1 Problem Statement / Objectives / Scope**：与实现完全吻合，不改。
- **Ch.4 Table 4.1 除 FR011 外的 12 条 FR、Table 4.2 全部 8 条 NFR**：与实现一致，不改。
- **§5.1 三运行时架构、§5.2.1 环境设置、§5.2.2 生成、§5.2.3 执行**：描述与代码一致，不改。
- **§5.3 混合存储（SQLite + JSON 缓存）总述**：与实现一致（`coverage.json` 存 app_data_dir/{project_id}/），不改。
- 用户手册 `docs/user-manual.md`、README：已与代码同步，不改。

---

*证据：db.rs INIT_SQL（8 表 + 索引）、get_projects/global_stats 聚合查询、execution.rs/suites.rs/coverage.rs/generation.rs 命令签名、Execute.vue/Coverage.vue 前端调用、git log（2026-07-19 ~ 08-29）、cargo test 26/26、lint/type-check/build 全绿。*
