// Part of ethercat-rs. Copyright 2018-2019 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use ethercat_sys as ec;

mod master;
mod types;

pub use self::types::*;
pub use self::master::{Master, Domain, SlaveConfig};
