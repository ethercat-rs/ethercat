// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.


//! This crate provide an API that wraps IOCTL calls to the EthercCAT master kernel module developped by IgH/Etherlab.
//! 	
//! EtherCAT is an Ethernet-based fieldbus system, originally invented by Beckhoff GmbH but now used by numerous providers of automation related hardware. The IgH master lets you provide an EtherCAT master on a Linux machine without specialized hardware.
//! 	
//! This crate mainly features struct [Master] - this struct is the entry point to the ethercat master kernel module, it exposes all its functions
//!
//!	# typical cycle
//!
//! ```no_run
//! # use ethercat::{
//! #     AlState, DomainIdx as DomainIndex, Idx, Master, MasterAccess, Offset, PdoCfg, PdoEntryIdx,
//! #     PdoEntryIdx as PdoEntryIndex, PdoEntryInfo, PdoEntryPos, PdoIdx, SlaveAddr, SlaveId, SlavePos,
//! #     SmCfg, SubIdx, Result,
//! # };
//! # fn main() -> Result<()> {
//! #    let master_idx = 0;
//! #    let slave_pos = SlavePos::from(0);
//! #    let slave_addr = SlaveAddr::ByPos(0);
//! #    let slave_id = SlaveId {vendor_id: 0, product_code: 0};
//! #
//! 	// connecting to an ethercat master in the linux kernel
//! 	let mut master = Master::open(master_idx, MasterAccess::ReadWrite)?;
//! 	master.reserve()?;
//! 	// configure realtime transmissions
//! 	let domain_idx = master.create_domain()?;
//! 	let mut config = master.configure_slave(slave_addr, slave_id)?;
//! 	// ... perform some configuration
//! 	
//! 	// switch to operation and realtime mode
//! 	master.request_state(slave_pos, AlState::Op)?;
//! 	master.activate()?;
//! 	
//! 	loop {
//! 		// execute the transmission steps
//! 		master.receive()?;
//! 		master.domain(domain_idx).process()?;
//! 		master.domain(domain_idx).queue()?;
//! 		master.send()?;
//! 		
//! 		let raw_data = master.domain_data(domain_idx)?;
//! 		// ... do something with the process data
//! 	}
//! #
//! # }
//! ```


use ethercat_sys as ec;

mod convert;
mod master;
mod types;

pub use self::{
    master::{Domain, Master, MasterAccess, SlaveConfig},
    types::*,
};
