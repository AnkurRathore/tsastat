use std::mem;

// --- NETLINK CONSTANTS ---
pub const NETLINK_GENERIC: u16 = 16;
pub const NLM_F_REQUEST: u16 = 1;
pub const GENL_ID_CTRL: u16 = 16; // 0x10 - The Generic Netlink Controller
pub const CTRL_CMD_GETFAMILY: u8 = 3;
pub const CTRL_ATTR_FAMILY_NAME: u16 = 2;
pub const CTRL_ATTR_FAMILY_ID: u16 = 1;
// Constants for unpacking the nested attributes
pub const TASKSTATS_TYPE_PID: u16 = 1;
pub const TASKSTATS_TYPE_STATS: u16 = 3;
pub const TASKSTATS_TYPE_AGGR_PID: u16 = 4;

// The Linux Kernel Taskstats struct (Partial)
// We use repr(C) to guarantee exact memory alignment with the kernel.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TaskStats {
    pub version: u16,
    pub ac_exitcode: u32,
    pub ac_flag: u8,
    pub ac_nice: u8,
    pub cpu_count: u64,
    pub cpu_delay_total: u64,   // Nanoseconds waiting for CPU
    pub blkio_count: u64,
    pub blkio_delay_total: u64, // Nanoseconds waiting for Disk I/O
    pub swapin_count: u64,
    pub swapin_delay_total: u64,// Nanoseconds waiting for RAM (Swap)
    pub cpu_run_real_total: u64,// Nanoseconds actually executing
    pub cpu_run_virtual_total: u64,
    // The struct is ~328 bytes in newer kernels. 
    // We pad the rest so our pointer cast doesn't read out of bounds.
    pub _padding: [u8; 256], 
}

// --- C Struct Definitions ---
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NlMsgHdr {
    pub nlmsg_len: u32,
    pub nlmsg_type: u16,
    pub nlmsg_flags: u16,
    pub nlmsg_seq: u32,
    pub nlmsg_pid: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GenlMsgHdr {
    pub cmd: u8,
    pub version: u8,
    pub reserved: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NlAttr {
    pub nla_len: u16,
    pub nla_type: u16,
}

// Helper function to convert structs to raw byte slices for writing to sockets
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, mem::size_of::<T>())

}

// Netlink requires attributes to be padded to 4-byte boundaries
pub fn nla_align(len: usize) -> usize {
    (len + 3) & !3
}
