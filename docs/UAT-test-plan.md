# Testmate — User Acceptance Test Plan (Functional Requirements)

| Field | Value |
|---|---|
| System under test | **Testmate** — automated testing workbench for small Python projects (Tauri v2 desktop application) |
| Document version | 1.0 |
| Date | ________ |
| Basis | Table 6.1 (FR001–FR013) of the FYP report, Chapter 6; `docs/FR-NFR-Compliance-Report.md`; `docs/user-manual.md` |
| Scope | Functional requirements only (FR001–FR013). Non-functional requirements (NFR001–NFR008) are out of scope of this document. |
| Tester (name / role) | ________ |
| Acceptance approver (name / role) | ________ |

---

## 1. Introduction

### 1.1 Purpose

This document defines the User Acceptance Test (UAT) procedure for Testmate. Its purpose is to verify, from the perspective of the end user (a developer working on a small Python project), that the application fulfils the thirteen functional requirements FR001–FR013 defined in Table 6.1 of the FYP report, and to provide the evidence needed for formal acceptance.

### 1.2 Scope

The UAT covers the complete functional workflow of the application:

- Project import and environment preparation (FR001–FR003);
- Test execution and result visualisation (FR004–FR006);
- Regression suites (FR007);
- Automated test generation (FR008–FR010);
- Coverage analysis and reporting (FR011–FR013).

Each requirement is exercised by one or more executable test cases. Both happy-path and key negative scenarios are included. The tester records the actual result of every case and the overall acceptance decision is made against the criteria in Section 5.

### 1.3 References

| Ref | Document |
|---|---|
| [T6.1] | FYP Report, Chapter 6, Table 6.1 — Functional requirement traceability (FR001–FR013) |
| [COMP] | `docs/FR-NFR-Compliance-Report.md` — per-requirement code-level evidence |
| [MAN] | `docs/user-manual.md` — Testmate user manual (operating steps) |
| [CH6] | `docs/chapter6-draft-2026-08-29.md` — Chapter 6 draft (implementation & testing) |

---

## 2. Entry Criteria

UAT may begin only when **all** of the following hold:

1. The application build under test is the accepted build (version agreed with the development team) and launches without startup errors.
2. The dashboard loads and previously imported projects, if any, appear correctly.
3. A test project is available (see Section 3.2).
4. The required runtimes are installed: Python ≥ 3.10, and a working virtual environment for the test project containing pytest, coverage.py and Pynguin (see Section 3.1).
5. All backend unit tests pass (`cargo test`) and the frontend passes `vue-tsc` type checking and ESLint — evidence of development-side testing before acceptance.

---

## 3. Test Environment and Test Data

### 3.1 Environment Requirements

| Item | Requirement |
|---|---|
| Operating system | Windows 10/11 (64-bit) or Linux x86_64 (the primary acceptance platform is agreed with the stakeholders) |
| Python | 3.10 or later (Pynguin requires ≥ 3.10) |
| Test project venv | Virtual environment with `pytest`, `coverage.py`, `pynguin` installed |
| Screen / input | 1280×720 or larger; mouse and keyboard |
| Network | Not required (all engines run locally) |

### 3.2 Test Data — Demo Project

The acceptance runs are executed against a small Python demo project (referred to as the *demo project* in the FYP report, Section 6.2.4). It must:

- contain at least three source modules of under 200 lines each (e.g. `cart`, `pricing`, `utils`), plus a `__init__.py` package layout;
- contain a `tests/` directory with at least one hand-written test file, including **one passing**, **one failing-or-xfail**, and **one skipped** test case so that all result states can be verified;
- include a `requirements.txt` (pytest, coverage.py, pynguin) and a `.venv` virtual environment.

> If the demo project is not present on the acceptance machine, the tester should recreate it from the description above (or use any small Python project meeting the same criteria). Record the project path used: ________

---

## 4. Functional UAT Test Cases

### 4.1 Test Case Notation

| Column | Meaning |
|---|---|
| Case ID | Unique identifier `UAT-FRxxx-nn` |
| Requirement | The functional requirement (Table 6.1 [T6.1]) under test |
| Priority | P0 = mandatory for acceptance; P1 = should pass, blocking only if combined with other P1 failures; P2 = nice-to-have |
| Test Steps | Concrete user actions, in order |
| Expected Result | The observable behaviour that constitutes a pass |
| Actual Result | Filled in by the tester during execution |
| Status | Pass / Fail / Blocked / Not Executed (tester fills in) |

### 4.2 Project Import and Environment (FR001–FR003)

| Case ID | Requirement | Priority | Preconditions | Test Steps | Expected Result | Actual Result | Status |
|---|---|---|---|---|---|---|---|
| UAT-FR001-01 | FR001 Import a local Python project directory | P0 | Testmate launched; demo project directory exists on disk | 1) Click **Import Project** in the sidebar. 2) In the import wizard, select the demo project directory. 3) Complete the wizard (accept the detected environment). 4) Click **Import Project** to save. 5) Check the dashboard project list. | Wizard validates the directory and completes; the project appears in the dashboard list with its environment status; no error is shown. | | |
| UAT-FR001-02 | FR001 (negative) | P1 | An empty or non-Python folder is available on disk | 1) Open the import wizard. 2) Select a folder that contains no `.py`, `requirements.txt`, `pyproject.toml`, `setup.py` or `Pipfile`. 3) Attempt to proceed with the import. | A friendly error is shown (e.g. "No Python files found…"); the project is **not** added to the dashboard. | | |
| UAT-FR001-03 | FR001 (negative — duplicate) | P1 | UAT-FR001-01 completed (demo project already imported) | 1) Open the import wizard again. 2) Select the **same** demo project directory. 3) Attempt to save the import. | The duplicate path is rejected with a clear message; no duplicate entry appears in the dashboard list. | | |
| UAT-FR002-01 | FR002 Automatically detect the virtual environment | P0 | Demo project with a `.venv` virtual environment | 1) Import the demo project (wizard). 2) Observe the environment status shown in the wizard and on the dashboard. | The `.venv` environment is detected automatically (`.venv`/`venv` searched, cross-platform paths); the detected Python version is displayed; environment status is "ready". | | |
| UAT-FR002-02 | FR002 (fallback / create) | P1 | A project **without** a venv; global Python ≥ 3.10 installed | 1) Import such a project. 2) Observe the detection result. 3) If the environment is not ready, click **Create .venv**. | Without `.venv`/`venv` the detection falls back to the global Python; **Create .venv** creates a virtual environment and the status becomes ready. | | |
| UAT-FR003-01 | FR003 Check dependencies and prompt installation | P0 | Demo project imported; its venv is missing at least one of pytest / coverage.py / pynguin | 1) Run the dependency check (wizard / environment panel). 2) Observe the list of missing packages. 3) Click the install action (e.g. **Install Missing**). 4) Watch the installation progress. | Missing packages are listed; installation proceeds with per-package progress events (`install_step`); a completion message is shown; re-running the check reports all dependencies satisfied. | | |
| UAT-FR003-02 | FR003 (all present) | P2 | Demo project venv already contains pytest, coverage.py and pynguin | 1) Re-run the dependency check. | All dependencies are reported satisfied; no install prompt is shown. | | |

### 4.3 Test Execution and Results (FR004–FR006)

| Case ID | Requirement | Priority | Preconditions | Test Steps | Expected Result | Actual Result | Status |
|---|---|---|---|---|---|---|---|
| UAT-FR004-01 | FR004 Select test files/folders and execute | P0 | Demo project imported; pytest installed; `tests/` contains test files | 1) Open the **Execute** view. 2) Set the scope to **All tests**. 3) Click **Run**. 4) Wait for completion. | All collected tests are executed; summary cards show the totals; individual case results are listed. | | |
| UAT-FR004-02 | FR004 (single file) | P1 | Same as UAT-FR004-01 | 1) Set the scope to a **single test file** and select one file. 2) Click **Run**. 3) Wait for completion. | Only the tests of the selected file are executed; result counts match that file's contents. | | |
| UAT-FR004-03 | FR004 (single cases) | P1 | Same as UAT-FR004-01 | 1) Set the scope to **single test cases** and multi-select several cases (ideally from two different files). 2) Click **Run**. 3) Wait for completion. | Only the selected cases are executed; other cases are not run; counts match the selection. | | |
| UAT-FR005-01 | FR005 Stream real-time execution logs | P0 | A suite with at least 3 test cases (or one slow test) is runnable | 1) Start a test run. 2) While it is running, watch the terminal panel. 3) Interact with the window (scroll the log, click **Stop**). | pytest output streams into the terminal panel progressively **before** the run finishes (lines appear within ~500 ms of being produced); the UI never freezes; **Stop** terminates the run (SIGTERM, then SIGKILL after 2 s). | | |
| UAT-FR006-01 | FR006 Parse and visualise passed/failed/skipped results | P0 | Demo project contains passing, failing (or xfail) and skipped cases | 1) Run the full suite. 2) Inspect the summary cards and the per-case list. 3) Expand a failed case. 4) Use **Rerun Failed**. | Passed / failed / skipped counts match the actual outcome; a failed case can be expanded to show its error message and copied; **Rerun Failed** re-runs only the failed cases. | | |
| UAT-FR006-02 | FR006 (skipped ≠ passed regression check) | P2 | A test marked `@pytest.mark.skip` exists in the suite | 1) Run a suite containing the skipped test. 2) Check the summary count and the per-case badge. | The skipped test is reported as **Skipped** (never as Passed) in the summary, the case list and the saved history (regression check for the fixed skipped-case parsing defect). | | |

### 4.4 Regression Suites (FR007)

| Case ID | Requirement | Priority | Preconditions | Test Steps | Expected Result | Actual Result | Status |
|---|---|---|---|---|---|---|---|
| UAT-FR007-01 | FR007 Save and rerun execution parameters as a regression suite | P1 | Execute view ready with a valid scope/argument selection | 1) Set a scope and (optionally) custom pytest arguments. 2) Click **Save as Suite** and give it a name. 3) Verify the suite appears in the suite list. 4) Click one-click **rerun** of the suite. 5) Open the history entry. | The suite is saved with its selection and arguments; one-click rerun executes the same configuration; the saved run is tagged `REGRESSION` in history (distinct from `MANUAL` runs). | | |
| UAT-FR007-02 | FR007 (delete) | P2 | At least one saved suite exists | 1) Delete a saved suite. | The suite is removed from the list and can no longer be rerun. | | |

### 4.5 Test Generation (FR008–FR010)

| Case ID | Requirement | Priority | Preconditions | Test Steps | Expected Result | Actual Result | Status |
|---|---|---|---|---|---|---|---|
| UAT-FR008-01 | FR008 Select a target module to trigger generation | P0 | Pynguin installed in the project venv; demo project source files present | 1) Open the **Generate** view. 2) Verify the source-file tree excludes venv, `tests/`, config and build-artifact directories. 3) Select one or more modules (e.g. `cart`). 4) Start generation. 5) Watch per-module progress and logs. | Selection triggers a Pynguin subprocess; progress and logs stream to the interface; the resulting test files are listed with test counts and status; the run can be cancelled. | | |
| UAT-FR009-01 | FR009 Output generated tests without modifying source | P0 | UAT-FR008-01 completed for at least one module | 1) Note the output folder (default `tests/generated`). 2) Check the generated files (including `__init__.py` initialisation). 3) Verify the original source files are unchanged (e.g. `git status` or file timestamps). | Generated tests are written only into the configured output folder; the output directory is initialised as a package; the original source files are **not** modified. | | |
| UAT-FR009-02 | FR009 (configurable output folder) | P2 | Generate view ready | 1) Change the output folder to a custom relative path. 2) Generate one module. | Generated tests are written to the custom output folder instead of the default. | | |
| UAT-FR010-01 | FR010 Configure advanced generation settings | P1 | Generate view ready; Pynguin installed | 1) Set algorithm = **DynaMOSA**, maximum search time = 60 s, seed = a fixed value (e.g. 123), assertion generation = on. 2) Generate the same module twice with the same seed. | The settings are passed to Pynguin (visible in the log); running twice with the same seed reproduces the same result; generated tests contain assertions. | | |

### 4.6 Coverage Analysis and Reporting (FR011–FR013)

| Case ID | Requirement | Priority | Preconditions | Test Steps | Expected Result | Actual Result | Status |
|---|---|---|---|---|---|---|---|
| UAT-FR011-01 | FR011 One-click coverage collection after execution | P0 | A test run completed with at least one passing test | 1) After the run, answer the coverage prompt by confirming. 2) Observe that coverage analysis starts and the view changes. | Coverage analysis starts automatically from the post-run summary and navigates to the Coverage view; the summary shows **both** statement coverage and branch coverage (branch measurement enabled). | | |
| UAT-FR012-01 | FR012 Show overall coverage and highlight lines | P0 | Coverage data collected (UAT-FR011-01) | 1) Open the **Coverage** view. 2) Read the summary cards. 3) Select a source file. 4) Inspect the code viewer. 5) Use the function filter. | Overall statement and branch percentages are shown as summary cards; the per-file list is sortable; the code viewer highlights covered / missing / excluded / non-executable lines in four distinct colours with a legend; selecting a function narrows the view and shows its statistics. | | |
| UAT-FR013-01 | FR013 Filter by file | P1 | Coverage data collected | 1) Use the file filters (>80% / 50–80% / <50% / all) and the coverage sort. | The file list filters and sorts according to the selected threshold. | | |
| UAT-FR013-02 | FR013 Filter by function | P1 | Coverage data collected; a source file with ≥ 2 functions selected | 1) In the code viewer, pick a function from the function dropdown. | The viewer narrows to the selected function's line range and displays its covered/total statistics. | | |
| UAT-FR013-03 | FR013 Export reports (JSON/CSV) | P1 | Coverage data collected | 1) Click **Export** and choose JSON; confirm via the save dialog. 2) Repeat with CSV. 3) Open both exported files. | Both files are written to the chosen locations; they open correctly and contain per-file coverage data; any export error is shown as a friendly message. | | |

---

## 5. Acceptance Criteria

The build is accepted when **all** of the following hold:

1. **All P0 cases** (UAT-FR001-01, UAT-FR002-01, UAT-FR003-01, UAT-FR004-01, UAT-FR005-01, UAT-FR006-01, UAT-FR008-01, UAT-FR009-01, UAT-FR011-01, UAT-FR012-01) are executed and **Pass**.
2. No open defect of severity **Critical** or **High** remains (see defect log, Section 6). Medium/Low defects are acceptable if a workaround exists and is agreed with the stakeholders.
3. P1/P2 cases that fail are individually assessed; a failed P1 case blocks acceptance only if no workaround can be agreed.
4. The results of the executed cases are recorded in this document, and the document is signed by the tester and the acceptance approver.

---

## 6. Defect Log

| Defect ID | Related Case | Description | Severity (Critical/High/Medium/Low) | Reported by / Date | Status (Open/Fixed/Deferred/Won't-fix) | Resolution / Workaround |
|---|---|---|---|---|---|---|
| | | | | | | |

---

## 7. Sign-off

| Role | Name | Signature | Date |
|---|---|---|---|
| Tester | | | |
| Acceptance Approver | | | |
| Development Representative | | | |

---

## 8. Test Case Summary

| FR (Table 6.1) | Requirement (abbreviated) | Cases | Priority mix | Result (Pass / Fail / Blocked / N/E) |
|---|---|---|---|---|
| FR001 | Import a local Python project directory | UAT-FR001-01 … 03 | P0×1, P1×2 | |
| FR002 | Automatically detect the virtual environment | UAT-FR002-01 … 02 | P0×1, P1×1 | |
| FR003 | Check dependencies and prompt installation | UAT-FR003-01 … 02 | P0×1, P2×1 | |
| FR004 | Select test files/folders (and single cases) and execute | UAT-FR004-01 … 03 | P0×1, P1×2 | |
| FR005 | Stream real-time execution logs | UAT-FR005-01 | P0×1 | |
| FR006 | Parse and visualise passed/failed/skipped results | UAT-FR006-01 … 02 | P0×1, P2×1 | |
| FR007 | Save and rerun execution parameters as a regression suite | UAT-FR007-01 … 02 | P1×1, P2×1 | |
| FR008 | Select a target module to trigger generation | UAT-FR008-01 | P0×1 | |
| FR009 | Output generated tests without modifying source | UAT-FR009-01 … 02 | P0×1, P2×1 | |
| FR010 | Configure advanced generation settings | UAT-FR010-01 | P1×1 | |
| FR011 | One-click coverage collection after execution | UAT-FR011-01 | P0×1 | |
| FR012 | Show overall coverage and highlight lines | UAT-FR012-01 | P0×1 | |
| FR013 | Filter by file/function and export reports | UAT-FR013-01 … 03 | P1×3 | |
| **Total** | **13 requirements** | **24 cases** | **P0×10, P1×9, P2×5** | |

---

*This plan is derived from Table 6.1 of the FYP report (Chapter 6) and the FR/NFR compliance report (`docs/FR-NFR-Compliance-Report.md`); operating steps follow the user manual (`docs/user-manual.md`).*
