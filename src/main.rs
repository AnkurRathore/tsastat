use anyhow::{Context, Result};
use neli::{
    consts::{nl::*, socket::NlFamily},
    genl::{Genlmsghdr, Nlattr},
    nl::{NlPayload, Nlmsghdr},
    socket::NlSocketHandle,
    types::GenlBuffer,
};
use std::process;
mod protocol;
use protocol::*;

// Taskstats constants from linux/taskstats.h
const TASKSTATS_GENL_VERSION: u8 = 1;
const TASKSTATS_CMD_GET: u8 = 1;
const TASKSTATS_CMD_ATTR_PID: u16 = 1;

fn main() -> Result<()> {
    println!("🔍 Initializing TSAS-STAT...");

    // 1. Open the Netlink Socket
    let mut sock = NlSocketHandle::connect(NlFamily::Generic, None, &[])
        .context("Failed to connect to Generic Netlink")?;

    // 2. Resolve the TASKSTATS Family ID dynamically
    let family_id = sock
        .resolve_genl_family("TASKSTATS")
        .context("Could not find TASKSTATS. Is delay accounting enabled?")?;

    println!("TASKSTATS Family ID resolved: {}", family_id);

    // 3. Construct the Request Payload: "Give me stats for PID X"
    let my_pid: u32 = process::id();
    let mut attrs = GenlBuffer::new();

    // Append the PID attribute. We tell the kernel we want data for `my_pid`
    attrs.push(Nlattr::new(
        false,
        false,
        TASKSTATS_CMD_ATTR_PID,
        my_pid,
    )?);

    // 4. Construct the Generic Netlink Header
    let genlhdr = Genlmsghdr::new(
        TASKSTATS_CMD_GET,
        TASKSTATS_GENL_VERSION,
        attrs,
    );

    // 5. Construct the Outer Netlink Header
    let flags = NlmFFlags::new(&[NlmF::Request]);
    let nlhdr = Nlmsghdr::new(
        None,
        family_id,
        flags,
        None,
        None,
        NlPayload::Payload(genlhdr),
    );

    // 6. Send the request
    sock.send(nlhdr).context("Failed to send request")?;
    println!("Sent request for PID: {}", my_pid);

    // 7. Receive the response using an Iterator
    println!("Waiting for Response...");
    
    for response in sock.iter::<u16, Genlmsghdr<u8, u16>>(false) {
        let msg = response.context("Error reading Netlink message")?;
        
        
        match msg.nl_payload {
            NlPayload::Payload(genl_msg) => {
                println!("Success: Received Taskstats Payload!");
                
                let handle = genl_msg.get_attr_handle();
                let attrs = handle.get_attrs();
                
                // Look at the outer attributes
                for attr in attrs {
                    let attr_type = u16::from(attr.nla_type.nla_type);
                    
                    // Is it the AGGR_PID wrapper? (Should be 4)
                    if attr_type == TASKSTATS_TYPE_AGGR_PID {
                        
                        let nested_payload = attr.nla_payload.as_ref();
                        let mut offset = 0;
                        
                        // Manual TLV parsing of the raw nested bytes
                        while offset < nested_payload.len() {
                            // Read standard Netlink Attribute Header (4 bytes)
                            let len = u16::from_ne_bytes([nested_payload[offset], nested_payload[offset+1]]) as usize;
                            let typ = u16::from_ne_bytes([nested_payload[offset+2], nested_payload[offset+3]]);
                            
                            // Is this the actual STATS payload? (Should be 3)
                            if typ == TASKSTATS_TYPE_STATS {
                                // Cast the raw bytes directly into our Rust C-Struct
                                let data_ptr = nested_payload[offset+4..].as_ptr();
                                let stats: TaskStats = unsafe { std::ptr::read_unaligned(data_ptr as *const TaskStats) };
                                
                                println!("--------------------------------------------------");
                                println!("THREAD STATE ANALYSIS FOR PID: {}", my_pid);
                                println!("--------------------------------------------------");
                                println!("CPU Execution Time: {} ms", stats.cpu_run_real_total / 1_000_000);
                                println!("CPU Wait (Delay):   {} ms", stats.cpu_delay_total / 1_000_000);
                                println!("Disk I/O Wait:      {} ms", stats.blkio_delay_total / 1_000_000);
                                println!("Swap (RAM) Wait:    {} ms", stats.swapin_delay_total / 1_000_000);
                                println!("--------------------------------------------------");
                            }
                            
                            // Align the length to a 4-byte boundary to find the next attribute
                            let aligned_len = (len + 3) & !3;
                            offset += aligned_len;
                        }
                    }
                }
                break; // Done parsing this message
            },
            NlPayload::Err(e) => {
                println!("Error: Kernel returned a Netlink Error: {:?}", e.error);
                break;
            },
            NlPayload::Ack(_) => {
                // Ignore acks
            },
            NlPayload::Empty => {
                // Ignore empty payloads
            }
        }
    }

    Ok(())
}