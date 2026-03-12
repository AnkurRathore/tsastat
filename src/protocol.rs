use std::mem;

// --- NETLINK CONSTANTS ---
pub const NETLINK_GENERIC: u16 = 16;
pub const NLM_F_REQUEST: u16 = 1;
pub const GENL_ID_CTRL: u16 = 16; // 0x10 - The Generic Netlink Controller
pub const CTRL_CMD_GETFAMILY: u8 = 3;
pub const CTRL_ATTR_FAMILY_NAME: u16 = 2;
pub const CTRL_ATTR_FAMILY_ID: u16 = 1;


// --- C Struct Definitions ---
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Nlmsghdr {
    pub nlmsg_len: u32,
    pub nlmsg_type: u16,
    pub nlmsg_flags: u16,
    pub nlmsg_seq: u32,
    pub nlmsg_pid: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Genlmsghdr {
    pub cmd: u8,
    pub version: u8,
    pub reserved: u16,
}

// Helper function to convert structs to raw byte slices for writing to sockets
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, mem::size_of::<T>())

}

// Netlink requires attributes to be padded to 4-byte boundaries
pub fn nla_align(len: usize) -> usize {
    (len + 3) & !3
}
