use crate::shared_buffer::SharedBuffer;
use anyhow::Result;
use dns_lookup::lookup_host;
use std::sync::Arc;
use std::{fs::File, sync::Mutex};

pub fn consumer(shared: Arc<SharedBuffer<String>>, log_file: Arc<Mutex<File>>) -> Result<()> {
    loop {
        // Wait until buffer is not empty
        let value = {
            let mut buffer = shared.buffer.lock().unwrap();

            while (*buffer).is_empty() {
                if buffer.running_producers == 0 {
                    return Ok(());
                }
                buffer = shared.can_consume.wait(buffer).unwrap();
            }

            let value = (*buffer).pop();

            shared.can_produce.notify_one();

            value
        };

        if let Some(hostname) = value {
            // println!("Pulled {}", hostname);

            if let Ok(ip_list) = lookup_host(&hostname) {
                if let Some(ip) = ip_list.into_iter().find(|ip| match ip {
                    std::net::IpAddr::V4(_) => true,
                    _ => false,
                }) {
                    threadprintln!("Hostname: {} - ip: {}", hostname, ip.to_string());

                    let out = format!("{}, {}\n", hostname, ip.to_string()).into_bytes();
                    let mut file = log_file.lock().unwrap();
                    file.write_all(&out)?;
                }
            } else {
                threadprintln!("failed to lookup address information: Name or service not known");

                let out = format!("{}, NOT_RESOLVED\n", hostname).into_bytes();
                let mut file = log_file.lock().unwrap();
                file.write_all(&out)?;
            }
        }
    }
}
