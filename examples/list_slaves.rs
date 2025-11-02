//! EtherCAT master example: List slaves
//!
//! Opens master 0 (read-only), queries slave info for positions 0..31 and
//! prints discovered slaves.
use ethercat::{Master, SlavePos};
use std::{io};

pub fn main() -> Result<(), io::Error> {
    // initialize logging
    env_logger::init();

    // open the first available master interface
    let master = Master::open(0, ethercat::MasterAccess::ReadOnly)?;

    log::info!("Master opened. Attempting to list slaves (positions 0..31)...");

    let mut found = 0u32;
    for i in 0u16..=31 {
        let pos = SlavePos::from(i);
        match master.get_slave_info(pos) {
            Ok(info) => {
                println!("Slave pos {i}: {info:?}");
                found += 1;
            }
            Err(_) => {
                // ignore errors for positions without a slave
            }
        }
    }

    println!("Discovered {found} slaves (checked positions 0..31)");

    // keep program alive if user wants to extend it interactively
    Ok(())
}
