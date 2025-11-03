//! Example: Write an int8 SDO to a slave
//!
//! This example demonstrates a simple SDO write of a single unsigned byte
//! (u8) to a slave device. It opens the EtherCAT master (index 0) with
//! read/write access, and writes the value 0x06 to SDO 0x6060 subindex 0 on
//! the first slave (slave position 0).
//!
//! Usage
//! ```text
//! # From the workspace root
//! cargo run --example write_sdo
//! ```
use ethercat::{Master, MasterAccess, SdoIdx, SlavePos};

pub fn main() -> Result<(), std::io::Error> {
    // Initialize the logger to see info messages from the library.
    env_logger::init();

    log::info!("Opening EtherCAT master...");
    // Open the first master (index 0) with read/write access. Adjust the index
    // if your setup uses a different master interface.
    let mut master = Master::open(0, MasterAccess::ReadWrite)?;

    // Target the first slave (position 0) and SDO index 0x6060, subindex 0.
    let slave_pos = SlavePos::from(0);
    let sdo_idx = SdoIdx::new(0x6060, 0);

    // Write a single byte (u8) value 0x06 to the SDO
    let value: u8 = 0x06;
    log::info!("Writing SDO 0x6060:0 with value 0x06...");
    // Perform the download (write) operation. The API accepts any type that
    // implements `SdoData` (u8 is supported).
    master.sdo_download(slave_pos, sdo_idx, false, &value)?;

    println!("Wrote SDO 0x{:X}:{} = 0x{:02X}", 0x6060, 0, value);

    Ok(())
}
