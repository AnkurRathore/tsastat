use crate::protocol::*;
use anyhow::{Context, Result};
use neli::{
    consts::{nl::*, socket::NlFamily},
    genl::{Genlmsghdr, Nlattr},
    nl::{NlPayload, Nlmsghdr},
    socket::NlSocketHandle,
    types::GenlBuffer,
};

pub struct TaskstatsClient {
    sock: NlSocketHandle,
    pub family_id: u16,
}

impl TaskstatsClient {
    pub fn new() -> Result<Self> {
        let mut sock = NlSocketHandle::connect(NlFamily::Generic, None, &[])
            .context("Failed to connect to Generic Netlink")?;

        // Uses neli's built-in discovery!
        let family_id = sock
            .resolve_genl_family("TASKSTATS")
            .context("Could not find TASKSTATS. Is delay accounting enabled?")?;

        Ok(Self { sock, family_id })
    }

    /// Fetches the raw TaskStats for a given PID
    pub fn get_stats(&mut self, target_pid: u32) -> Result<TaskStats> {
        // 1. Build the attributes using neli
        let mut attrs = GenlBuffer::new();
        attrs.push(Nlattr::new(
            false,
            false,
            TASKSTATS_CMD_ATTR_PID,
            target_pid,
        )?);

        // 2. Build the headers using neli
        let genlhdr = Genlmsghdr::new(TASKSTATS_CMD_GET, TASKSTATS_GENL_VERSION, attrs);
        let flags = NlmFFlags::new(&[NlmF::Request]);
        let nlhdr = Nlmsghdr::new(
            None,
            self.family_id,
            flags,
            None,
            None,
            NlPayload::Payload(genlhdr),
        );

        // 3. Send using neli
        self.sock.send(nlhdr).context("Failed to send request")?;

        // 4. Receive using neli
        for response in self.sock.iter::<u16, Genlmsghdr<u8, u16>>(false) {
            let msg = response.context("Error reading Netlink message")?;

            if let NlPayload::Payload(genl_msg) = msg.nl_payload {
                let handle = genl_msg.get_attr_handle();
                
                for attr in handle.get_attrs() {
                    if u16::from(attr.nla_type.nla_type) == TASKSTATS_TYPE_AGGR_PID {
                        
                        // 5. MANUAL PARSING: parse the raw bytes.
                        let nested_payload = attr.nla_payload.as_ref();
                        // Read head starts reading from the start of the payload
                        let mut offset = 0;

                        while offset < nested_payload.len() {
                            //1. Read the standard Netlink Attribute Header (4 bytes)
                            // Bytes 0-1: Length (including header), Bytes 2-3: Type
                            let len = u16::from_ne_bytes([
                                nested_payload[offset],
                                nested_payload[offset + 1],
                            ]) as usize;
                            let typ = u16::from_ne_bytes([
                                nested_payload[offset + 2],
                                nested_payload[offset + 3],
                            ]);

                            //2. Check if this is the STATS payload (Type 3)
                            if typ == TASKSTATS_TYPE_STATS {
                                // The actual TaskStats struct starts immediately after the 4-byte attribute header
                                let data_ptr = nested_payload[offset + 4..].as_ptr();
                                // Safely copy the C-struct from misaligned memory
                                let stats: TaskStats = unsafe {
                                    std::ptr::read_unaligned(data_ptr as *const TaskStats)
                                };
                                return Ok(stats);
                            }
                            //3. Move to the next attribute (length is aligned to 4 bytes)
                            // Netlink attributes are padded to 4 bytes, so we round up the length to the next multiple of 4
                            let aligned_len = (len + 3) & !3;

                            // Move the offset to the start of the next attribute
                            offset += aligned_len;
                        }
                    }
                }
            }
        }
        anyhow::bail!("Failed to parse TaskStats response. Process might have died.")
    }
}