// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.


//! 	This crate provide an API that wraps IOCTL calls to the EthercCAT master kernel module developped by IgH/Etherlab.
//! 	
//! 	EtherCAT is an Ethernet-based fieldbus system, originally invented by Beckhoff GmbH but now used by numerous providers of automation related hardware. The IgH master lets you provide an EtherCAT master on a Linux machine without specialized hardware.
//! 	
//! 	This crate mainly features struct [Master] - this struct is the entry point to the ethercat master kernel module, it exposes all its functions


use ethercat_sys as ec;

mod convert;
mod master;
mod types;

pub use self::{
    master::{Domain, Master, MasterAccess, SlaveConfig},
    types::*,
};
