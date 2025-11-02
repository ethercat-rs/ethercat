//! EtherCAT master example: List slaves
//!
//! Opens master 0 (read-only), queries slave info for positions 0..31 and
//! prints discovered slaves.
use ethercat::{Master, SlavePos};
use std::io;

pub fn main() -> Result<(), io::Error> {
    env_logger::init();

    // Open /dev/EtherCAT0 (the first EtherCAT master) in read-only mode
    // Prerequisites: You must configure the Etherlab EtherCAT master using `/etc/ethercat.conf`,
    // and have the master running (e.g. via `sudo systemctl start ethercat`).
    let master = Master::open(0, ethercat::MasterAccess::ReadOnly)?;

    log::info!("Master 0 opened. Attempting to list slaves (positions 0..31)...");

    let mut found = 0u32;
    for i in 0u16..=31 {
        let pos = SlavePos::from(i);
        match master.get_slave_info(pos) {
            Ok(info) => {
                println!("Slave pos {i}: {info:?}");
                found += 1;
            }
            Err(_) => {
                println!("Slave pos {i}: <no slave>");
            }
        }
    }

    println!("Discovered {found} slaves (checked positions 0..31)");

    Ok(())
}
