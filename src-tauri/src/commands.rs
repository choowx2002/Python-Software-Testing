use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::AppHandle;
use tauri::Manager;

// ============================================
// Data Structures
// ============================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvDetectionResult {
    pub python_path: Option<String>,
    pub python_version: Option<String>,
    pub venv_path: Option<String>,
    // 重命名: venv_activated -> venv_exists
    // 语义更准确：表示 Tauri 找到了可用的 venv 目录，可直接调用其 python，无需 shell activate
    pub venv_exists: bool,
    pub dependencies: Vec<DependencyStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DependencyStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PipPackage {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub installed: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InstallStep {
    pub package: String,
    pub status: String, // "starting", "success", "failed"
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFile {
    pub name: String,
    pub path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub file: String,
    pub class_name: Option<String>,
    pub line: Option<u32>,
}

// ============================================
pub mod env;
pub mod execution;
pub mod suites;
pub mod generation;
pub mod files;
pub mod coverage;
pub mod db_commands;

// ============================================
// Shared helpers used by multiple sub-modules
// ============================================

pub(crate) fn is_test_file(file_name: &str) -> bool {
    file_name.starts_with("test_") && file_name.ends_with(".py")
        || file_name.ends_with("_test.py")
}

pub(crate) fn build_python_path(project: &Path) -> String {
    let mut paths = vec![];

    if let Some(parent) = project.parent() {
        paths.push(parent.to_string_lossy().to_string());
    }

    if let Ok(existing) = std::env::var("PYTHONPATH") {
        if !existing.is_empty() {
            paths.push(existing);
        }
    }

    let separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    paths.join(separator)
}

pub(crate) fn register_run(app: &AppHandle, run_id: &str, pid: u32) {
    if let Ok(mut processes) = app.state::<AppState>().processes.lock() {
        processes.insert(run_id.to_string(), pid);
    }
}

pub(crate) fn unregister_run(app: &AppHandle, run_id: &str) {
    if let Ok(mut processes) = app.state::<AppState>().processes.lock() {
        processes.remove(run_id);
    }
    if let Ok(mut cancelled) = app.state::<AppState>().cancelled_runs.lock() {
        cancelled.remove(run_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // JUnit XML Parser Tests
    // ============================================

    #[test]
    fn test_parse_junit_single_pass() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_add" file="tests/test_math.py" time="0.01"/>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_add");
        assert_eq!(results[0].file, "tests/test_math.py");
        assert_eq!(results[0].status, "passed");
        assert_eq!(results[0].duration, 0.01);
        assert!(results[0].error_message.is_none());
    }

    #[test]
    fn test_parse_junit_with_failure() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_add" file="tests/test_math.py" time="0.01">
                <failure>assert 1 == 2</failure>
            </testcase>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "failed");
        assert!(results[0].error_message.is_some());
        assert_eq!(results[0].error_message.as_ref().unwrap(), "assert 1 == 2");
    }

    #[test]
    fn test_parse_junit_with_error() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_divide" file="tests/test_math.py" time="0.02">
                <error>ZeroDivisionError: division by zero</error>
            </testcase>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "error");
        assert!(results[0].error_message.is_some());
        assert!(results[0].error_message.as_ref().unwrap().contains("ZeroDivisionError"));
    }

    #[test]
    fn test_parse_junit_with_skipped() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_skip" file="tests/test_math.py" time="0.00">
                <skipped/>
            </testcase>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "skipped");
        assert!(results[0].error_message.is_none());
    }

    #[test]
    fn test_parse_junit_multiple_cases() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_pass" file="tests/test_a.py" time="0.01"/>
            <testcase name="test_fail" file="tests/test_a.py" time="0.02">
                <failure>failed</failure>
            </testcase>
            <testcase name="test_error" file="tests/test_b.py" time="0.03">
                <error>error</error>
            </testcase>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, "passed");
        assert_eq!(results[1].status, "failed");
        assert_eq!(results[2].status, "error");
    }

    #[test]
    fn test_parse_junit_empty_xml() {
        let xml = r#"<?xml version="1.0"?><testsuite></testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parse_junit_id_generation() {
        let xml = r#"<?xml version="1.0"?>
        <testsuite>
            <testcase name="test_add" file="tests/test_math.py" time="0.01"/>
            <testcase name="test_add" time="0.01"/>
        </testsuite>"#;
        let results = execution::parse_junit_results_from_str(xml).unwrap();
        assert_eq!(results[0].id, "tests/test_math.py::test_add");
        assert_eq!(results[1].id, "test_add");
    }

    // ============================================
    // Coverage JSON Parser Tests
    // ============================================

    #[test]
    fn test_parse_coverage_format1() {
        let json = r#"{
            "totals": {
                "percent_covered": 85.5,
                "num_statements": 100,
                "covered_lines": 85
            },
            "files": {
                "src/main.py": {
                    "percent_covered": 85.5,
                    "executed_lines": [1, 2, 3, 10],
                    "missing_lines": [4, 5],
                    "excluded_lines": []
                }
            }
        }"#;
        let (totals, files) = coverage::parse_coverage_json(json).unwrap();
        assert_eq!(totals.percent_covered, 85.5);
        assert_eq!(totals.total_statements, 100);
        assert_eq!(totals.covered_statements, 85);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.py");
        assert_eq!(files[0].percent_covered, 85.5);
        assert_eq!(files[0].executed_lines, vec![1, 2, 3, 10]);
        assert_eq!(files[0].missing_lines, vec![4, 5]);
        assert!(files[0].excluded_lines.is_empty());
    }

    #[test]
    fn test_parse_coverage_format2_with_summary() {
        let json = r#"{
            "totals": {
                "percent_covered": 60.0,
                "num_statements": 50,
                "covered_lines": 30
            },
            "files": {
                "src/utils.py": {
                    "summary": {
                        "percent_covered": 60.0
                    },
                    "executed_lines": [1, 2],
                    "missing_lines": [3, 4, 5],
                    "excluded_lines": [6]
                }
            }
        }"#;
        let (totals, files) = coverage::parse_coverage_json(json).unwrap();
        assert_eq!(totals.percent_covered, 60.0);
        assert_eq!(files[0].path, "src/utils.py");
        assert_eq!(files[0].percent_covered, 60.0);
        assert_eq!(files[0].executed_lines, vec![1, 2]);
        assert_eq!(files[0].missing_lines, vec![3, 4, 5]);
        assert_eq!(files[0].excluded_lines, vec![6]);
    }

    #[test]
    fn test_parse_coverage_multiple_files_sorted() {
        let json = r#"{
            "totals": {
                "percent_covered": 100.0,
                "num_statements": 10,
                "covered_lines": 10
            },
            "files": {
                "src/b.py": {
                    "executed_lines": [1],
                    "missing_lines": [],
                    "excluded_lines": []
                },
                "src/a.py": {
                    "executed_lines": [1],
                    "missing_lines": [],
                    "excluded_lines": []
                }
            }
        }"#;
        let (_totals, files) = coverage::parse_coverage_json(json).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/a.py");
        assert_eq!(files[1].path, "src/b.py");
    }

    #[test]
    fn test_parse_coverage_missing_fields() {
        let json = r#"{
            "totals": {
                "percent_covered": 0.0,
                "num_statements": 0,
                "covered_lines": 0
            },
            "files": {
                "src/empty.py": {}
            }
        }"#;
        let (totals, files) = coverage::parse_coverage_json(json).unwrap();
        assert_eq!(totals.percent_covered, 0.0);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/empty.py");
        assert_eq!(files[0].percent_covered, 0.0);
        assert!(files[0].executed_lines.is_empty());
        assert!(files[0].missing_lines.is_empty());
        assert!(files[0].excluded_lines.is_empty());
    }

    #[test]
    fn test_parse_coverage_invalid_json() {
        let result = coverage::parse_coverage_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_coverage_missing_totals() {
        let result = coverage::parse_coverage_json(r#"{"files": {}}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_coverage_missing_files() {
        let result = coverage::parse_coverage_json(r#"{"totals": {}}"#);
        assert!(result.is_err());
    }
}
