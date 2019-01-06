mod plc;
mod image;
mod server;

pub mod beckhoff;
pub mod mlz_spec;

pub use self::plc::{Plc, PlcBuilder};
pub use self::image::{ExternImage, ProcessImage};
pub use ethercat_derive::{ExternImage, ProcessImage, SlaveProcessImage};
