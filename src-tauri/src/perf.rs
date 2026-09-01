//! Lightweight performance instrumentation used to collect NFR003 / NFR004
//! evidence for the evaluation chapter of the thesis.
//!
//! * Enabled **only** when the `TESTMATE_PERF=1` environment variable is set,
//!   so normal usage carries no overhead beyond one atomic load per emitted
//!   event.
//! * NFR003 — UI blocking / log latency: every per-line `emit(...)` call on the
//!   output streaming loops (`test-output` / `coverage-output` /
//!   `generation-output`) is timed with `tokio::time::Instant`; the max,
//!   average and count of the emit dispatch duration are tracked. Under a heavy
//!   log stream this duration includes the synchronous IPC serialisation and
//!   the WebView2 send path — i.e. the segment of the UI pipeline that blocks
//!   while one log line is pushed — which is the value reported as "UI thread
//!   blocking time". The same timing doubles as the "log latency" of the
//!   backend streaming pipeline.
//! * NFR004 — memory: the RSS of the application process itself is sampled
//!   every second with the `sysinfo` crate and the running peak is tracked.
//!
//! Results are printed to stdout every 5 s and appended to a JSONL file under
//! the system temp directory (`testmate-perf/perf-metrics-<ts>.jsonl`), which
//! can later be re-plotted or quoted in the report.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

const SNAPSHOT_EVERY: Duration = Duration::from_secs(5);

static ENABLED: OnceLock<bool> = OnceLock::new();

/// True when `TESTMATE_PERF=1` is present in the environment.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("TESTMATE_PERF").as_deref() == Ok("1"))
}

// --- NFR003: emit dispatch timing (microseconds) ----------------------------

static EMIT_COUNT: AtomicU64 = AtomicU64::new(0);
static EMIT_SUM_US: AtomicU64 = AtomicU64::new(0);
static EMIT_MAX_US: AtomicU64 = AtomicU64::new(0);

/// Record the duration of one per-line event `emit(...)` call.
#[inline]
pub fn record_emit(elapsed: Duration) {
    if !enabled() {
        return;
    }
    let us = elapsed.as_micros() as u64;
    EMIT_COUNT.fetch_add(1, Ordering::Relaxed);
    EMIT_SUM_US.fetch_add(us, Ordering::Relaxed);
    EMIT_MAX_US.fetch_max(us, Ordering::Relaxed);
}

// --- NFR004: process RSS sampling (MB) ---------------------------------------

static CURRENT_RSS_MB: AtomicU64 = AtomicU64::new(0);
static PEAK_RSS_MB: AtomicU64 = AtomicU64::new(0);

fn record_memory(rss_bytes: u64) {
    let mb = rss_bytes / (1024 * 1024);
    CURRENT_RSS_MB.store(mb, Ordering::Relaxed);
    PEAK_RSS_MB.fetch_max(mb, Ordering::Relaxed);
}

// --- reporting ----------------------------------------------------------------

fn snapshot_line(elapsed: Duration) -> String {
    let count = EMIT_COUNT.load(Ordering::Relaxed);
    let sum = EMIT_SUM_US.load(Ordering::Relaxed);
    let max = EMIT_MAX_US.load(Ordering::Relaxed);
    let avg_us = if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    };
    format!(
        "[perf] t={:6.1}s rss={:>4}MB peak={:>4}MB | emit: n={} avg={:.2}ms max={:.2}ms",
        elapsed.as_secs_f64(),
        CURRENT_RSS_MB.load(Ordering::Relaxed),
        PEAK_RSS_MB.load(Ordering::Relaxed),
        count,
        avg_us / 1000.0,
        max as f64 / 1000.0,
    )
}

fn jsonl_line(elapsed: Duration) -> String {
    let count = EMIT_COUNT.load(Ordering::Relaxed);
    let sum = EMIT_SUM_US.load(Ordering::Relaxed);
    let max = EMIT_MAX_US.load(Ordering::Relaxed);
    let avg_us = if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    };
    format!(
        "{{\"t_s\":{:.1},\"rss_mb\":{},\"peak_mb\":{},\"emit_count\":{},\"emit_avg_ms\":{:.3},\"emit_max_ms\":{:.3}}}",
        elapsed.as_secs_f64(),
        CURRENT_RSS_MB.load(Ordering::Relaxed),
        PEAK_RSS_MB.load(Ordering::Relaxed),
        count,
        avg_us / 1000.0,
        max as f64 / 1000.0,
    )
}

/// Spawn the background sampler: 1 s RSS sampling, 5 s stdout + JSONL snapshot.
/// No-op unless `TESTMATE_PERF=1` is set.
pub fn start_memory_sampler() {
    if !enabled() {
        return;
    }

    let dir = std::env::temp_dir().join("testmate-perf");
    let _ = std::fs::create_dir_all(&dir);
    let unix_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let file_path = dir.join(format!("perf-metrics-{}.jsonl", unix_ts));
    println!("[perf] TESTMATE_PERF=1 — instrumentation enabled");
    println!("[perf] metrics file: {}", file_path.display());

    tauri::async_runtime::spawn(async move {
        let mut sys = sysinfo::System::new();
        let pid = sysinfo::Pid::from_u32(std::process::id());
        let start = tokio::time::Instant::now();
        let mut last_snapshot = tokio::time::Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            let _ = sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            if let Some(process) = sys.process(pid) {
                record_memory(process.memory());
            }
            if last_snapshot.elapsed() >= SNAPSHOT_EVERY {
                last_snapshot = tokio::time::Instant::now();
                let elapsed = start.elapsed();
                println!("{}", snapshot_line(elapsed));
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)
                {
                    use std::io::Write;
                    let _ = writeln!(f, "{}", jsonl_line(elapsed));
                }
            }
        }
    });
}
