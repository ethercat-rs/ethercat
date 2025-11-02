//! Example: Read an int8 SDO from a slave
//!
//! This example demonstrates a simple SDO read of a single signed byte
//! (int8) from a slave device. It opens the EtherCAT master (index 0) with
//! read/write access, requests SDO 0x6061 subindex 0 from the first slave
//! (slave position 0) and prints the interpreted signed value.
//!
//! Usage
//! ```text
//! # From the workspace root
//! cargo run --example sdo
//! ```
use ethercat::{Master, MasterAccess, SdoIdx, SlavePos};

pub fn main() -> Result<(), std::io::Error> {
    // Initialize the logger to see info messages from the library.
    env_logger::init();

    log::info!("Opening EtherCAT master...");
    // Open the first master (index 0) with read/write access. Adjust the index
    // if your setup uses a different master interface.
    let master = Master::open(0, MasterAccess::ReadWrite)?;

    // Read from the first slave (position 0) and SDO index 0x6061, subindex 0.
    let slave_pos = SlavePos::from(0);
    let sdo_idx = SdoIdx::new(0x6061, 0);

    // Read an i8 SDO value (1 byte) from the slave
    log::info!("Reading SDO 0x6061:0 [int8]...");
    // Create a buffer to hold the data returned by the SDO upload.
    let mut buf = [0u8; 1];
    // Perform the read operation (make the slave "upload" the SDO to the master)
    let data = master.sdo_upload(slave_pos, sdo_idx, false, &mut buf)?;
    if data.len() >= 1 {
        // Interpret the single returned byte as a signed 8-bit integer
        // (two's complement).
        let val = data[0] as i8;
        println!("SDO 0x{:X}/{} read int8 = {}", 0x6061, 1, val);
    } else {
        println!("SDO read returned {} bytes", data.len());
    }

    Ok(())
}
