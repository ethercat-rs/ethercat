//! # EtherCAT Master Count Example
//!
//! This example demonstrates how to use the `Master::master_count()` function
//! to determine the number of available EtherCAT masters in the system.
//!
//! The function checks for available EtherCAT master devices (typically
//! `/dev/EtherCAT0`, `/dev/EtherCAT1`, etc.) and returns the count.
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example master_count
//! ```
//!
//! ## Expected Output (for 1 available master)
//!
//! ```text
//! Number of EtherCAT masters available: 1
//! ```

use ethercat::Master;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = Master::master_count()?;
    println!("Number of EtherCAT masters available: {}", count);
    Ok(())
}
