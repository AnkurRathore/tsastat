mod netlink;
mod protocol;
mod stats;

use netlink::TaskstatsClient;
use std::{env, thread, time::Duration};

fn main() -> anyhow::Result<()> {
    // 1. Get Target PID from CLI args
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: sudo ./tsastat <PID>");
        std::process::exit(1);
    }
    let target_pid: u32 = args[1].parse().expect("Invalid PID");

    println!("Initializing TSAS-STAT for PID: {}", target_pid);
    let mut client = TaskstatsClient::new()?;
    println!("Connected to Netlink. Monitoring...\n");

    // Print Header
    println!("{:<8} | {:<10} | {:<10} | {:<10} | {:<10}", "PID", "EXEC %", "CPU WAIT %", "I/O WAIT %", "SWAP WAIT %");
    println!("{:-<60}", "");

    // 2. Initial Snapshot (A)
    let mut prev_stats = client.get_stats(target_pid)?;

    let interval_ms = 1000; // 1 second

    // 3. The Loop
    loop {
        thread::sleep(Duration::from_millis(interval_ms));

        // Snapshot B
        let curr_stats = match client.get_stats(target_pid) {
            Ok(s) => s,
            Err(_) => {
                println!("Process {} exited or disappeared.", target_pid);
                break;
            }
        };

        // 4. Calculate Deltas
        let metrics = stats::calculate_deltas(&prev_stats, &curr_stats, interval_ms);

        // 5. Print Row
        // Colorize if it's waiting a lot!
        let wait_str = if metrics.cpu_wait_percent > 10.0 {
             format!("{:.1}% ", metrics.cpu_wait_percent)
        } else {
             format!("{:.1}%", metrics.cpu_wait_percent)
        };

        println!(
            "{:<8} | {:<10.1}% | {:<10} | {:<10.1}% | {:<10.1}%",
            target_pid,
            metrics.exec_percent,
            wait_str,
            metrics.io_wait_percent,
            metrics.swap_wait_percent
        );

        // Advance
        prev_stats = curr_stats;
    }

    Ok(())
}