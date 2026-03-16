// --- TASKSTATS PROTOCOL CONSTANTS ---
// These are defined in the Linux Kernel source: include/uapi/linux/taskstats.h

// The version of the taskstats struct we expect
pub const TASKSTATS_GENL_VERSION: u8 = 1;

// Commands we can send to the kernel
pub const TASKSTATS_CMD_GET: u8 = 1; // "Give me stats"

// Attributes we can send in our request
pub const TASKSTATS_CMD_ATTR_PID: u16 = 1; // "Here is the PID I want"

// Attributes the kernel sends back in the response
pub const TASKSTATS_TYPE_PID: u16 = 1;      // The PID the stats belong to
pub const TASKSTATS_TYPE_STATS: u16 = 3;    // The actual TaskStats binary struct
pub const TASKSTATS_TYPE_AGGR_PID: u16 = 4; // A wrapper attribute grouping PID and STATS together

// --- THE KERNEL C-STRUCT ---
// We use `repr(C)` so Rust automatically inserts the exact same invisible
// memory padding bytes that the Linux C compiler uses. 
// Without repr(C), Rust would reorder these fields to save space, and our 
// data pointer cast would read garbage memory.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TaskStats {
    pub version: u16,           // 2 bytes
    // Rust automatically inserts 2 bytes of padding here for C-alignment
    pub ac_exitcode: u32,       // 4 bytes
    pub ac_flag: u8,            // 1 byte
    pub ac_nice: u8,            // 1 byte
    // Rust automatically inserts 6 bytes of padding here so the next u64 aligns to 8 bytes
    
    // --- BASIC STATS ---
    pub cpu_count: u64,
    
    // --- DELAY ACCOUNTING METRICS (In Nanoseconds) ---
    pub cpu_delay_total: u64,   // Time spent waiting for the CPU scheduler
    pub blkio_count: u64,
    pub blkio_delay_total: u64, // Time spent waiting for Disk I/O (Synchronous Block I/O)
    pub swapin_count: u64,
    pub swapin_delay_total: u64,// Time spent waiting for RAM pages to be swapped in
    
    // --- EXECUTION METRICS (In Nanoseconds) ---
    pub cpu_run_real_total: u64,    // Actual time spent executing on the CPU
    pub cpu_run_virtual_total: u64, // Time executing + time spent in kernel on behalf of process

    // --- SAFETY BUFFER ---
    // The actual Linux taskstats struct has many more fields (memory, cgroups, etc)
    // and is over 300 bytes long. If we don't pad our Rust struct to be large enough, 
    // `std::ptr::read_unaligned` might read past the end of the memory boundary.
    // This 256-byte padding ensures our struct is safely large enough to capture the header.
    pub _padding: [u8; 256], 
}