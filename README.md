# tsastat (Thread State Analysis)

**A high-resolution Linux thread state analyzer built in Rust, powered by Kernel Delay Accounting (Taskstats) and raw Netlink sockets.**

![Status](https://img.shields.io/badge/status-active_development-orange)
![Platform](https://img.shields.io/badge/platform-linux-lightgrey)
![Rust](https://img.shields.io/badge/rust-2021-red)

## The Problem: The "100% CPU" Lie
Standard tools like `top` or `/proc/[pid]/stat` tell you if a process is using the CPU, but they are terrible at telling you *why* a process is slow when it isn't. 

If a database query takes 5 seconds, was the thread actually executing? Was it waiting in the run-queue because the CPU was saturated? Was it stalled on a page fault? Standard tools lack the granularity to answer these questions without expensive tracing.

## The Solution: Linux Delay Accounting
**`tsastat`** communicates directly with the Linux Kernel Scheduler via **Generic Netlink** to extract microsecond-precision Delay Accounting metrics. 

Instead of showing absolute usage, it calculates the **rolling percentage of time** a thread spends in specific states:
*   **EXEC:** Actively executing on the CPU.
*   **CPU WAIT:** Runnable, but waiting for the CPU scheduler (Saturation).
*   **I/O WAIT:** Blocked waiting for synchronous block I/O (Disk).
*   **SWAP WAIT:** Blocked waiting for memory paging.

## Under the Hood (Architecture)

This tool bypasses high-level wrappers to interface directly with the Linux ABI:
1.  **Dynamic Discovery:** Communicates with the `GENL_ID_CTRL` controller to dynamically resolve the `TASKSTATS` family ID at runtime.
2.  **Binary TLV Parsing:** Manually parses nested Type-Length-Value (TLV) attributes from the Netlink byte stream, enforcing strict 4-byte alignment rules.
3.  **Zero-Cost Deserialization:** Uses `#[repr(C)]` structs and `std::ptr::read_unaligned` to safely cast raw network buffers directly into Rust structures, avoiding memory allocation overhead.

## Usage

### Prerequisites
*   Linux Kernel with Task Delay Accounting enabled (`CONFIG_TASK_DELAY_ACCT=y`).
*   *(Usually enabled by default on Ubuntu/Debian, or via `sysctl kernel.task_delayacct=1`)*

### Build & Run
Because this tool queries the kernel scheduler directly via Netlink, it requires `root` privileges.

```bash
cargo build --release
sudo ./target/release/tsastat <PID>
```

### Example Output

```text
Initializing TSAS-STAT for PID: 19284
Connected to Netlink. Monitoring...

PID      | EXEC %     | CPU WAIT % | I/O WAIT % | SWAP WAIT %
------------------------------------------------------------
19284    | 66.7%      | 33.7%      | 0.0%       | 0.0%      
19284    | 49.8%      | 50.1%      | 0.0%       | 0.0%      
19284    | 47.3%      | 52.8%      | 0.0%       | 0.0%      
19284    | 43.7%      | 57.4%      | 0.0%       | 0.0%      
```
*(In this example, the process is heavily bottlenecked by CPU saturation, spending over 50% of its time just waiting to be scheduled).*

## Roadmap

- [x] Raw Netlink socket initialization and Family ID resolution.
- [x] Binary parsing of `taskstats` C-struct.
- [x] Rolling delta calculations for time-spent percentages.
- [ ] **TUI Dashboard:** Upgrade the CLI output to a `ratatui` terminal interface with historical sparklines.
- [ ] **Thread Discovery:** Auto-discover and list all TIDs (threads) belonging to the target PID.

## License
MIT License
