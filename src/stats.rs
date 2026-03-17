use crate::protocol::TaskStats;
use std::time::Duration;

#[derive(Debug, Default)]
pub struct ThreadMetrics {
    pub exec_percent: f64,
    pub cpu_wait_percent: f64,
    pub io_wait_percent: f64,
    pub swap_wait_percent: f64,
}

pub fn calculate_deltas(
    prev: &TaskStats,
    curr: &TaskStats,
    interval_ms: Duration,
) -> ThreadMetrics {
    //1ms = 1,000,000 ns
    let interval_ns = interval_ms.as_nanos() as f64;

    // Calculate deltas (B - A)
    let exec_delta = curr
        .cpu_run_real_total
        .saturating_sub(prev.cpu_run_real_total) as f64;
    let cpu_wait_delta = curr.cpu_delay_total.saturating_sub(prev.cpu_delay_total) as f64;
    let io_wait_delta = curr
        .blkio_delay_total
        .saturating_sub(prev.blkio_delay_total) as f64;
    let swap_wait_delta = curr
        .swapin_delay_total
        .saturating_sub(prev.swapin_delay_total) as f64;

    //convert to percentages of the interval
    // if a Thread spent 500ms executing during a 1000ms interval, that would be 50% CPU time
    ThreadMetrics {
        exec_percent: (exec_delta / interval_ns) * 100.0,
        cpu_wait_percent: (cpu_wait_delta / interval_ns) * 100.0,
        io_wait_percent: (io_wait_delta / interval_ns) * 100.0,
        swap_wait_percent: (swap_wait_delta / interval_ns) * 100.0,
    }
}
