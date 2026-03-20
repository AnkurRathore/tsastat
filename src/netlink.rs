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
    seq_num: u32, // Added sequence number field
}

impl TaskstatsClient {
    pub fn new() -> Result<Self> {
        let mut sock = NlSocketHandle::connect(NlFamily::Generic, None, &[])
            .context("Failed to connect to Generic Netlink")?;

        // Uses neli's built-in discovery!
        let family_id = sock
            .resolve_genl_family("TASKSTATS")
            .context("Could not find TASKSTATS. Is delay accounting enabled?")?;

        Ok(Self {
            sock,
            family_id,
            seq_num: 1,
        })
    }

    /// Fetches the raw TaskStats for a given PID
    pub fn get_stats(&mut self, target_pid: u32) -> Result<TaskStats> {
        //increment the sequence number for this request
        self.seq_num += 1;

        let mut attrs = GenlBuffer::new();
        attrs.push(Nlattr::new(
            false,
            false,
            TASKSTATS_CMD_ATTR_PID,
            target_pid,
        )?);

        let genlhdr = Genlmsghdr::new(TASKSTATS_CMD_GET, TASKSTATS_GENL_VERSION, attrs);
        let flags = NlmFFlags::new(&[NlmF::Request]);

        let nlhdr = Nlmsghdr::new(
            None,
            self.family_id,
            flags,
            Some(self.seq_num),
            None,
            NlPayload::Payload(genlhdr),
        );

        self.sock.send(nlhdr).context("Failed to send request")?;

        // 2. Read the response
        let mut found_stats = None;

        for response in self.sock.iter::<u16, Genlmsghdr<u8, u16>>(false) {
            let msg = match response {
                Ok(m) => m,
                Err(_) => continue, // Skip malformed packets
            };

            // Ignore any ghost packets left over in the kernel buffer
            if msg.nl_seq != self.seq_num {
                continue;
            }

            match msg.nl_payload {
                NlPayload::Payload(genl_msg) => {
                    let handle = genl_msg.get_attr_handle();
                    for attr in handle.get_attrs() {
                        if u16::from(attr.nla_type.nla_type) == TASKSTATS_TYPE_AGGR_PID {
                            let nested = attr.nla_payload.as_ref();
                            let mut offset = 0;

                            while offset < nested.len() {
                                let len = u16::from_ne_bytes([nested[offset], nested[offset + 1]])
                                    as usize;
                                let typ =
                                    u16::from_ne_bytes([nested[offset + 2], nested[offset + 3]]);

                                if typ == TASKSTATS_TYPE_STATS {
                                    let data_ptr = nested[offset + 4..].as_ptr();
                                    let stats: TaskStats = unsafe {
                                        std::ptr::read_unaligned(data_ptr as *const TaskStats)
                                    };
                                    found_stats = Some(stats);
                                }
                                offset += (len + 3) & !3;
                            }
                        }
                    }
                    // Break out of iteration once we found our payload
                    break;
                }
                NlPayload::Err(_) | NlPayload::Ack(_) | NlPayload::Empty => {
                    // Stop listening if we hit an error or end of message
                    break;
                }
            }
        }

        found_stats.context("TaskStats not found for this TID")
    }
}
