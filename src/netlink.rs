use crate::protocol::*;
use anyhow::{Context, Result};
use byteorder::{NativeEndian, ReadBytesExt};
use netlink_sys::{protocols::NETLINK_GENERIC, Socket, SocketAddr};
use std::io::Cursor;
use std::mem;

pub struct TaskstatsClient {
    sock: Socket,
    pub family_id: u16,
}

impl TaskstatsClient {
    pub fn new() -> Result<Self> {
        // 1. Open the raw Netlink socket
        let mut sock = Socket::new(NETLINK_GENERIC).context("Failed to open netlink socket")?;
        
        // 2. Bind to Kernel (PID 0)
        let addr = SocketAddr::new(0, 0);
        sock.bind(&addr).context("Failed to bind socket")?;

        let mut client = Self { sock, family_id: 0 };
        
        // 3. Resolve the TASKSTATS dynamic ID
        client.family_id = client.resolve_family_id("TASKSTATS")?;
        
        Ok(client)
    }

    fn resolve_family_id(&mut self, family_name: &str) -> Result<u16> {
        // --- CONSTRUCT THE BINARY REQUEST ---
        let mut req = Vec::new();
        let name_bytes = family_name.as_bytes();
        let name_len = name_bytes.len() + 1; // +1 for null terminator
        
        let attr_len = mem::size_of::<NlAttr>() + name_len;
        let attr_pad = nla_align(attr_len) - attr_len;
        
        let total_len = mem::size_of::<NlMsgHdr>() 
                      + mem::size_of::<GenlMsgHdr>() 
                      + attr_len 
                      + attr_pad;

        let nl_hdr = NlMsgHdr {
            nlmsg_len: total_len as u32,
            nlmsg_type: GENL_ID_CTRL,
            nlmsg_flags: NLM_F_REQUEST,
            nlmsg_seq: 1,
            nlmsg_pid: std::process::id(),
        };

        let genl_hdr = GenlMsgHdr {
            cmd: CTRL_CMD_GETFAMILY,
            version: 1,
            reserved: 0,
        };

        let attr_hdr = NlAttr {
            nla_len: attr_len as u16,
            nla_type: CTRL_ATTR_FAMILY_NAME,
        };

        // Write structs to the buffer
        unsafe {
            req.extend_from_slice(any_as_u8_slice(&nl_hdr));
            req.extend_from_slice(any_as_u8_slice(&genl_hdr));
            req.extend_from_slice(any_as_u8_slice(&attr_hdr));
        }
        
        // Write the string + null terminator + padding
        req.extend_from_slice(name_bytes);
        req.push(0); // Null terminator
        for _ in 0..attr_pad { req.push(0); }

        // --- SEND TO KERNEL ---
        let addr = SocketAddr::new(0, 0);
        self.sock.send_to(&req, &addr, 0)?;

        // --- PARSE THE BINARY RESPONSE ---
        let mut buf = vec![0u8; 4096];
        let (len, _) = self.sock.recv_from(&mut buf, 0)?;
        let payload = &buf[..len];

        println!("Received {} bytes from Kernel", len);
        
        // THE HEX DUMP: Print the raw memory so we can decode it manually
        println!("--- HEX DUMP ---");
        for chunk in payload.chunks(16) {
            for byte in chunk {
                print!("{:02X} ", byte);
            }
            println!();
        }
        println!("----------------");

        // Parse through the nested TLV (Type-Length-Value) structure
        let mut cursor = Cursor::new(payload);
        
        // Skip outer headers
        cursor.set_position((mem::size_of::<NlMsgHdr>() + mem::size_of::<GenlMsgHdr>()) as u64);

        while (cursor.position() as usize) < len {
            let attr_len = cursor.read_u16::<NativeEndian>()?;
            let attr_type = cursor.read_u16::<NativeEndian>()?;

            // A valid Netlink attribute header is exactly 4 bytes.
            // If it's less than 4, we either hit zero-padding or a corrupt packet.
            if attr_len < 4 {
                break; // Exit the loop safely
            }

            if attr_type == CTRL_ATTR_FAMILY_ID {
                let id = cursor.read_u16::<NativeEndian>()?;
                return Ok(id);
            }

            // Skip to next attribute (handle padding)
            let advance = nla_align(attr_len as usize) - 4; // -4 for the NlAttr header we just read
            cursor.set_position(cursor.position() + advance as u64);
        }

        anyhow::bail!("Could not find TASKSTATS family ID. Is task delay accounting enabled in your kernel?")
    }

    /// Fetches the raw TaskStats for a given PID
    pub fn get_stats(&mut self, target_pid: u32) -> Result<TaskStats> {
        let mut attrs = GenlBuffer::new();
        attrs.push(Nlattr::new(false, false, TASKSTATS_CMD_ATTR_PID, target_pid)?);

        let genlhdr = Genlmsghdr::new(TASKSTATS_CMD_GET, TASKSTATS_GENL_VERSION, attrs);
        let flags = NlmFFlags::new(&[NlmF::Request]);
        let nlhdr = Nlmsghdr::new(None, self.family_id, flags, None, None, NlPayload::Payload(genlhdr));

        self.sock.send(nlhdr).context("Failed to send request")?;

        for response in self.sock.iter::<u16, Genlmsghdr<u8, u16>>(false) {
            let msg = response.context("Error reading Netlink message")?;
            
            if let NlPayload::Payload(genl_msg) = msg.nl_payload {
                let handle = genl_msg.get_attr_handle();
                for attr in handle.get_attrs() {
                    if u16::from(attr.nla_type.nla_type) == TASKSTATS_TYPE_AGGR_PID {
                        let nested_payload = attr.nla_payload.as_ref();
                        let mut offset = 0;
                        
                        while offset < nested_payload.len() {
                            let len = u16::from_ne_bytes([nested_payload[offset], nested_payload[offset+1]]) as usize;
                            let typ = u16::from_ne_bytes([nested_payload[offset+2], nested_payload[offset+3]]);
                            
                            if typ == TASKSTATS_TYPE_STATS {
                                let data_ptr = nested_payload[offset+4..].as_ptr();
                                // Safe copy of the misaligned struct
                                let stats: TaskStats = unsafe { std::ptr::read_unaligned(data_ptr as *const TaskStats) };
                                return Ok(stats);
                            }
                            
                            let aligned_len = (len + 3) & !3;
                            offset += aligned_len;
                        }
                    }
                }
            }
        }
        anyhow::bail!("Failed to parse TaskStats response")
    }
}