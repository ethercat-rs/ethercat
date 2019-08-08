// Part of ethercat-rs. Copyright 2018-2019 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

mod plc;
mod image;
mod server;

pub mod beckhoff;
pub mod mlz_spec;

pub use self::plc::{Plc, PlcBuilder};
pub use self::image::{ExternImage, ProcessImage};
pub use ethercat_derive::{ExternImage, ProcessImage, SlaveProcessImage};
