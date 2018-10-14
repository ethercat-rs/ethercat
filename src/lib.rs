#[macro_use]
extern crate log;
extern crate libc;
extern crate time;
extern crate memmap;
extern crate mlzlog;
extern crate byteorder;
extern crate crossbeam_channel;
extern crate ethercat_sys as ec;

mod master;
mod image;
pub mod types;
mod plc;
mod server;

pub mod beckhoff;

pub use self::types::Result;
pub use self::master::{Master, Domain, SlaveConfig};
pub use self::image::{ExternImage, ProcessImage};
pub use self::plc::{Plc, PlcBuilder};
