use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;

use crate::proc::NoConsole;

/// 全局应用状态：跟踪所有运行中的子进程（run_id → pid）
#[derive(Default)]
pub struct AppState {
    pub processes: Mutex<HashMap<String, u32>>,
    pub cancelled_runs: Mutex<HashMap<String, bool>>,
}

impl AppState {
    pub fn kill_all(&self) {
        if let Ok(mut processes) = self.processes.lock() {
            for pid in processes.values() {
                let _ = terminate_pid(*pid, true);
            }
            processes.clear();
        }
    }
}

/// 按 PID 终止进程：force=false 发送 SIGTERM（Windows 无优雅信号，直接终止）
pub fn terminate_pid(pid: u32, force: bool) -> bool {
    #[cfg(unix)]
    {
        let mut cmd = Command::new("kill");
        if force {
            cmd.arg("-9");
        }
        cmd.arg(pid.to_string());
        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    }
    #[cfg(windows)]
    {
        let mut cmd = Command::new("taskkill");
        cmd.no_console();
        if force {
            cmd.args(["/F", "/PID", &pid.to_string()]);
        } else {
            cmd.args(["/PID", &pid.to_string()]);
        }
        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    }
}

/// 检查 PID 对应的进程是否仍存活
pub fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .no_console()
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| !String::from_utf8_lossy(&o.stdout).contains("No tasks"))
            .unwrap_or(false)
    }
}