// Part of ethercat-rs. Copyright 2018-2020 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use ioctl_sys::{io, ioc, ioctl, ior, iorw, iow};

pub mod ioctl {
    use super::EC_IOCTL_TYPE as EC;
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/ioctls.rs"));
}
