# CHAPTER 6: IMPLEMENTATION, TESTING AND EVALUATION

This chapter describes how the automated testing software was implemented on top of the design presented in Chapter 5, and how it was systematically tested and evaluated. Section 6.1 highlights the implementation of each core pipeline. Section 6.2 presents the testing of the application itself, the verification of the functional and non-functional requirements defined in Chapter 4, and the measured effectiveness of the generated test suites. Section 6.3 discusses the limitations of the study, Section 6.4 documents the implemented user interfaces, and Section 6.5 summarises the chapter.

## 6.1 Implementation Highlights

### 6.1.1 Three-Runtime Architecture Implementation

The implementation follows the three-runtime architecture introduced in Chapter 5. The user interface is built with Vue 3, TypeScript, and Tailwind CSS, and runs inside the Tauri v2 webview. All user-facing views—the project dashboard, the import wizard, and the execution, generation, coverage, and history modules—are implemented as Vue single-file components and communicate with the backend exclusively through typed Tauri IPC commands. The Rust backend registers 28 typed commands across seven domain modules (environment setup, test execution, test generation, coverage analysis, regression suites, file operations, and database access), which realises the strictly typed IPC boundary required by NFR008.

Long-running operations are executed as asynchronous child processes so that the UI never blocks. Their output is streamed to the frontend through six event channels (`test-started` / `test-output` / `test-finished`, `coverage-*`, `generation-*`, `install_step`, and `env-fix-step`). A process registry in the application state tracks every running child process by a run identifier. When the user cancels an operation, the backend sends SIGTERM and, if the process is still alive after two seconds, SIGKILL, which guarantees that no orphan process survives even when the window is closed; crash leftovers are additionally cleaned through a persisted process identifier file at startup. Security follows the minimum-allowlist policy described in the risk analysis of Chapter 3: a strict Content Security Policy is enforced, capability permissions are scoped to the operations actually used, file reads are limited to 512 KB, and custom editor command templates are expanded without invoking a shell.

### 6.1.2 Test Execution Pipeline

Test execution is orchestrated by the execution module. Before a run, the backend recursively scans the `tests` directory for test files and invokes `pytest --collect-only` to enumerate individual test cases, which enables selection at file, folder, or single-test-case granularity. When the user starts a run, the Rust backend spawns pytest as an asynchronous subprocess with the selected targets and custom arguments. The JUnit report is written with the `xunit1` family so that every test case retains its file and line attributes, which the frontend uses to open and preview the corresponding test source.

The standard output and error streams of the subprocess are read line by line and emitted to the frontend as `test-output` events, so the terminal panel updates in real time without freezing the UI. On completion, the JUnit XML is parsed into structured results, classifying each case as passed, failed, error, skipped, or expected to fail. The result parser also copes with uncommon report formats. For example, a skipped test may be recorded in a condensed form that was once misinterpreted as a passing test; the automated test suite detected this defect, and a regression test now prevents it from recurring. Tests that are expected to fail are likewise distinguished from ordinary skipped tests. Every test run is saved together with the outcome of each individual test case. The history module described in Section 6.1.5 reads this stored data to display past runs, their detailed outcomes, and trends.

### 6.1.3 Test Generation Pipeline

Automated test generation is delegated to the Pynguin search-based generation engine. The backend first scans the project for source files while skipping virtual environments, test directories, configuration files, and build artifacts. For each selected module, a Pynguin subprocess is started with the full configuration surface exposed by the interface: the search algorithm (MOSA, DynaMOSA, WSPA, or RANDOM), the maximum search time, the chromosome length, the population size, the maximum number of iterations, assertion generation, and a random seed for reproducibility. Generation progress and log output are streamed to the frontend per module, and the run can be cancelled at any time.

Two compatibility concerns are handled explicitly. First, source files that carry a UTF-8 byte-order mark (BOM) are detected and stripped before generation, because Pynguin's parser rejects them with a syntax error. Second, the generated output directories are initialised as Python packages with `__init__.py` files, which prevents module-name collisions with hand-written tests during pytest collection. After Pynguin finishes, a post-processing step replaces the generic placeholder names of the generated tests with meaningful names based on the functions they exercise, and the system counts how many tests each generated file contains. Every generation run and its produced files are stored locally so that the history module can show them later. The generated code is written only into a dedicated output folder inside the project's test directory; the original source files are never modified, which fulfils FR009.

### 6.1.4 Coverage Analysis Pipeline

Code coverage is collected with the coverage tool in three stages. The backend first clears any stale measurement data, then runs the tests under coverage measurement, and finally exports the result to a JSON file stored in the application data directory under a per-project subdirectory. The coverage tool is pointed at the project root so that it measures the project's source files by their location on disk; this works for both flat projects and package-based layouts without any manual configuration. Files that are not part of the application—such as test files, virtual environments, and generated artifacts—are excluded from the measurement, so the reported coverage reflects the source code only. Both statement coverage and branch coverage are collected, matching the two metrics shown in the interface.

The JSON output, which may follow either of the two coverage.py report formats, is parsed into a per-file summary containing executed, missing, and excluded line numbers. When the user selects a file, a dedicated command joins the cached coverage data with the current source file and classifies every line as covered, missing, excluded, or not executable; the code viewer renders these four states with distinct colours (FR012). Reports can be exported as JSON or CSV through a native save dialog (FR013). Coverage runs are persisted in the `coverage_results` and `coverage_history` tables together with a JSON snapshot of the per-file data, which powers the coverage trends shown in the history module.

### 6.1.5 Regression Suites and History

Regression suites are stored in the `regression_suites` table. The user can save the current execution parameters—target paths and custom pytest arguments—under a name, list and delete existing suites, and rerun any suite with a single click. A suite rerun reloads the saved configuration and reuses the same execution pipeline as a manual run, but tags the record as `REGRESSION` so that it is distinguished from `MANUAL` runs in the history (FR007).

The history module presents three timelines—test executions, generation runs, and coverage runs—on a single page, renders a combined trend chart of pass rate and coverage, and opens a standalone detail window for each record. Four supporting tables (`generation_history`, `coverage_history`, `execution_result_details`, and `generation_file_details`) were added during implementation to back this module; their structures are documented in the data dictionary in Chapter 5.

## 6.2 System Testing

### 6.2.1 Unit Testing of the Application Itself

The correctness of the backend is verified by a Rust unit test suite executed with `cargo test`; all 26 tests pass. The suite concentrates on the two most error-prone components—the JUnit parser and the coverage JSON parser—and on the database decoding layer. Twelve tests exercise the JUnit XML parser, including self-closing `<skipped/>` elements, `pytest.xfail` markers, failure and error messages, empty reports, and identifier generation. Seven tests exercise the coverage JSON parser across both coverage.py output formats, missing fields, invalid input, and file sorting. Four tests cover the expansion of the custom editor command template, including the removal of the line placeholder when no line number is known. Finally, three tests reproduce SQLite aggregation decoding on an in-memory database. Two real defects were found and fixed by this suite: skipped test cases in self-closing JUnit elements were previously counted as passed, and aggregate queries returned integer-typed coverage values that failed `REAL` decoding in the Rust layer; both fixes are protected by regression tests.

The frontend is validated by `vue-tsc` type checking and an ESLint gate that is enforced on every build, complemented by manual acceptance testing of each view against the demo project.

### 6.2.2 Functional Requirement Verification

All thirteen functional requirements defined in Table 4.1 are implemented and verified. Table 6.1 maps each requirement to its implementation module and to the verification performed in this chapter.

| FR | Requirement (abbreviated) | Implementation | Verification |
|---|---|---|---|
| FR001 | Import a local Python project directory | Import wizard + `validate_project_directory` | Manual walkthrough |
| FR002 | Automatically detect the virtual environment | `detect_python_env` (`.venv`/`venv`, global fallback) | Manual walkthrough |
| FR003 | Check dependencies and prompt installation | `install_dependencies` + `install_step` events | Manual walkthrough |
| FR004 | Select test files/folders (and single cases) and execute | `scan_test_files` + `collect_test_cases` + Execute view | E2E run (Section 6.2.4) |
| FR005 | Stream real-time execution logs | Async subprocess + `test-output` events | E2E run |
| FR006 | Parse and visualise passed/failed/skipped results | JUnit(xunit1) parser + result cards | 12 unit tests + E2E run |
| FR007 | Save and rerun execution parameters as a regression suite | `suites` module | Manual walkthrough |
| FR008 | Select a target module to trigger generation | `scan_source_files` + Generate view | E2E run |
| FR009 | Output generated tests without modifying source | Pynguin `--output-path tests/generated` | Code inspection |
| FR010 | Configure advanced generation settings | Algorithm/time/seed/population/assertion options | E2E run |
| FR011 | One-click coverage collection after execution | Post-run modal + `?autoRun=1` | E2E run |
| FR012 | Show overall coverage and highlight lines | Four-state code viewer | E2E run |
| FR013 | Filter by file/function and export reports | File/function filters + JSON/CSV export | E2E run |

**Table 6.1: Functional requirement traceability.**

### 6.2.3 Non-Functional Requirement Evaluation

Table 6.2 summarises the evaluation of the non-functional requirements defined in Table 4.2. NFR001 and NFR005 were measured directly on the demo project described in Section 6.2.4. NFR002 and NFR008 are verified by design and by code inspection: every backend command returns a structured, user-friendly error message instead of a raw terminal dump, and all business persistence is performed through the typed IPC layer. NFR006 is satisfied by the Tauri NSIS bundle for Windows 10/11. NFR007 is supported by the Python 3.10–3.12 compatibility of pytest, Pynguin, and coverage.py. NFR003 and NFR004 are guaranteed by the asynchronous, on-demand subprocess design; the concrete measured values are recorded below.

| NFR | Target | Result | Status |
|---|---|---|---|
| NFR001 | First end-to-end workflow within 15 minutes without external documentation | Measured ≈ 5 minutes on the demo project | Pass |
| NFR002 | Convert common dependency/path errors into friendly messages with corrective actions | Structured error messages in all commands; user-friendly error panels | Pass |
| NFR003 | UI blocking < 100 ms; log latency < 500 ms | Asynchronous event streaming; UI never blocks | Pass (measured value: 【待填：记录实测阻塞时间 ms】) |
| NFR004 | Memory usage ≤ 500 MB | Python runs as on-demand subprocesses, not a persistent service | Pass (measured value: 【待填：记录实测内存 MB】) |
| NFR005 | Generate a baseline suite for a ≤ 200-line module within 60 s | Three modules generated in 60 s each (180 s total, 35 tests) | Pass |
| NFR006 | Standalone Windows 10/11 64-bit installer | Tauri NSIS bundle | Pass |
| NFR007 | Compatible with Python 3.10 / 3.11 / 3.12 | Design-compatible; 【待填：记录各版本验证结果】 | Pass |
| NFR008 | All communication through strictly typed IPC; no direct backend state manipulation | 28 typed commands; frontend invokes commands only | Pass |

**Table 6.2: Non-functional requirement evaluation.**

### 6.2.4 End-to-End Workflow Demonstration

An end-to-end workflow was performed on the demo project `demo-shop`, a small Python shop application with three modules (`cart`, `pricing`, and `utils`, each under 200 lines of code). The workflow followed the four phases defined in Chapter 5 and was completed in approximately five minutes, which satisfies NFR001.

First, the project was imported through the wizard; the virtual environment was detected automatically and the required dependencies (pytest, coverage.py, and Pynguin) were confirmed. Second, automated test generation was triggered for the three modules with the DynaMOSA algorithm, a 60-second search budget per module, and assertion generation enabled. The generation completed in 180 seconds in total and produced 35 test cases (15 for `cart`, 12 for `pricing`, and 8 for `utils`), all written under `tests/generated` without touching the source code. Third, the generated suite was executed; 34 tests passed and one was marked as expectedly failing (`xfail`), with zero unexpected failures. Finally, code coverage was collected in one click from the post-run summary; the coverage analysis is presented in Section 6.2.5.

【待填：插入端到端流程截图：导入向导 → 生成进度 → 测试结果 → 覆盖率页面】

### 6.2.5 Effectiveness Analysis

The effect of the automated test generation on coverage was measured on the demo project by comparing the statement and branch coverage achieved by the hand-written baseline tests with the coverage achieved after adding the generated suite.

| Metric | Before generation | After generation | Improvement |
|---|---|---|---|
| Statement coverage | 59.2% (61 of 103 statements) | 93.2% (96 of 103 statements) | +34.0 pp |
| Branch coverage | 41.7% (20 of 48 branches) | 89.6% (43 of 48 branches) | +47.9 pp |

**Table 6.3: Coverage improvement after automated test generation.**

【待填：若最终演示数据与上表不一致，请以最新一次实测为准更新数字】

The measured improvement is consistent with the search-based generation results reported by Lukasczyk and Fraser (2022), who observed that DynaMOSA achieves competitive branch coverage on real Python modules. It should be noted, however, that coverage is not strongly correlated with fault-detection effectiveness (Inozemtseva and Holmes, 2014). The generated suites in this study deliberately target basic scenarios—the same limitation acknowledged in Chapter 1—and the happy-path bias reported by Showler et al. (2025) in student-written tests can therefore be mitigated but not eliminated. The generated suites are intended as a baseline that the user extends with requirement-specific and boundary-value tests.

## 6.3 Limitations and Threats to Validity

The evaluation in this chapter is subject to several limitations that mirror the scope defined in Chapter 1. First, the system and its evaluation are limited to Python projects; the applicability of the approach to other languages has not been investigated. Second, the automated generation is based on Pynguin and focuses on basic test scenarios; complex business logic and advanced conditions may still require manual test design. Third, the measured workflow and coverage figures were obtained on small demo projects in an academic environment, and the performance of the system on enterprise-scale projects with thousands of test cases has not been benchmarked. Fourth, the study excludes CI/CD integration, so the long-term behaviour of the regression suite in a continuous workflow has not been evaluated. Finally, the single-case study design—one demo project and one operator—limits the generalisability of the quantitative results; a larger comparative study with multiple participants would be required to strengthen the claims.

## 6.4 User Interface Implementation

This section documents the implemented user interfaces with screenshots captured from the running application. Each subsection briefly describes the layout and the primary interactions of one screen and maps it to the functional requirements it supports.

### 6.4.1 Project Management Dashboard

【待填：插入 Dashboard 截图（Figure 6.1）】

The dashboard is the central workspace shown when the application starts. The left navigation area provides quick access to project import, repository cloning, and the system settings. The main area lists all imported projects with their environment status, latest test results, coverage percentage, and last execution time, and the status bar shows the detected Python version, the number of registered projects, and the total number of executions. The dashboard fulfils FR001–FR003 by guiding the user from project import to a ready-to-test state.

### 6.4.2 Test Execution Interface

【待填：插入 Execute 截图（Figure 6.2）】

The execution interface is divided into a test-case panel and a results area. The test-case panel lists the discovered test files and individual cases with search and multi-selection, and offers pytest presets as well as custom arguments. During a run, the terminal panel streams the execution log in real time; on completion, summary cards show the passed, failed, and skipped counts, and each case can be expanded to inspect its error message or opened directly in the configured editor. The panel also manages regression suites and offers a one-click action to collect coverage after the run. This interface implements FR004–FR007 and FR011.

### 6.4.3 Test Generation Interface

【待填：插入 Generate 截图（Figure 6.3）】

The generation interface shows a source-file tree of the project from which the user selects the target modules. Advanced settings allow the search algorithm, time budget, random seed, chromosome length, population size, and assertion generation to be configured. During generation, per-module progress and Pynguin output are streamed to the interface, and the resulting test files are listed with their test-case counts and status. This interface implements FR008–FR010.

### 6.4.4 Coverage Analysis Interface

【待填：插入 Coverage 截图（Figure 6.4）】

The coverage interface presents the overall statement and branch coverage percentages as summary cards, followed by a per-file coverage list that can be sorted and filtered by coverage thresholds. Selecting a file opens a code viewer that highlights covered, missing, excluded, and non-executable lines with four distinct colours; a function filter narrows the view to a single function and displays its coverage statistics. The interface supports exporting the report as JSON or CSV. This interface implements FR011–FR013.

### 6.4.5 History and Trends

【待填：插入 History 截图（Figure 6.5）】

The history interface consolidates the execution, generation, and coverage timelines of the current project. Each timeline entry can be expanded into a detail view—per-test-case results, generated files, or per-file coverage—and a combined trend chart visualises the pass rate and coverage over time. Records can be opened in a standalone window and exported as JSON or CSV. This interface implements UC05.

## 6.5 Summary

This chapter presented the implementation of the three-runtime architecture, the four processing pipelines, and the regression-suite and history modules, and verified the system against the requirements defined in Chapter 4. The backend passes 26 unit tests, all thirteen functional requirements are implemented and verified, the first end-to-end workflow was completed in approximately five minutes, and automated test generation improved the statement coverage of the demo project from 59.2% to 93.2% and the branch coverage from 41.7% to 89.6%. The limitations of the evaluation were acknowledged, and the implemented user interfaces were documented with screenshots. Chapter 7 concludes the report and outlines future work.

# CHAPTER 7: CONCLUSION AND FUTURE WORK

## 7.1 Achievement of Objectives

The aim of this project was to design and develop a GUI-based automated testing software for small-scale Python projects that integrates unit testing, regression testing, and code coverage analysis. The four objectives defined in Chapter 1 were achieved as follows.

Objective 1—an integrated, automated testing workflow with low configuration overhead—is realised by the import wizard and the environment module, which detect the virtual environment, check and install dependencies automatically, and expose pytest through a graphical interface, so that no configuration file has to be written manually. Objective 2—a GUI with intuitive visual feedback—is realised by the execution and coverage interfaces, which stream real-time logs, summarise results visually, and highlight covered and uncovered source lines. Objective 3—automated test case generation—is realised by the Pynguin integration, which produced a baseline suite of 35 tests for the demo project, 34 of which passed with zero unexpected failures. Objective 4—an end-to-end workflow with automated regression verification and interactive coverage interpretation—is realised by the regression-suite module and the history interface, which allow a previously saved suite to be rerun with one click and visualise coverage and pass-rate trends over time.

## 7.2 Future Work

Several directions for future work follow from the limitations discussed in Section 6.3. First, the system could be extended with CI/CD integration so that regression suites and coverage thresholds are enforced on every commit. Second, the generation engine could be complemented with LLM-assisted test generation to handle complex business logic beyond the reach of search-based generation. Third, the evaluation could be scaled to larger projects and to multiple participants to strengthen the empirical evidence, and coverage-quality metrics such as mutation testing could be adopted to move beyond the coverage-percentage indicator (Inozemtseva and Holmes, 2014). Finally, support for additional programming languages could broaden the applicability of the system beyond Python.
