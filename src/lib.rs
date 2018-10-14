#[macro_use]
extern crate log;
extern crate libc;
extern crate time;
extern crate memmap;
extern crate mlzlog;
extern crate byteorder;
extern crate crossbeam_channel;
extern crate ethercat_sys as ec;

pub mod master;
pub mod types;
pub mod image;
pub mod plc;
pub mod server;

pub use self::types::Result;
pub use self::master::{Master, Domain, SlaveConfig};
