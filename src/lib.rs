extern crate libc;
extern crate time;
extern crate memmap;
extern crate ethercat_sys as ec;

pub mod master;
pub mod types;
pub mod image;
pub mod plc;

pub use self::types::Result;
pub use self::master::{Master, Domain, SlaveConfig};
