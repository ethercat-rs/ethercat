// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(not(feature = "pregenerated-bindings"))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(not(feature = "sncn"), feature = "pregenerated-bindings"))]
include!("bindings-v1.5-334c34cfd2e5.rs");

#[cfg(all(feature = "sncn", feature = "pregenerated-bindings"))]
include!("bindings-v1.5.2-sncn-11.rs");

use ioctl_sys::{io, ioc, ioctl, ior, iorw, iow};

pub mod ioctl {
    use super::EC_IOCTL_TYPE as EC;
    use super::*;

    #[cfg(not(feature = "pregenerated-bindings"))]
    include!(concat!(env!("OUT_DIR"), "/ioctls.rs"));

    #[cfg(all(not(feature = "sncn"), feature = "pregenerated-bindings"))]
    include!("ioctls-v1.5-334c34cfd2e5.rs");

    #[cfg(all(feature = "sncn", feature = "pregenerated-bindings"))]
    include!("ioctls-v1.5.2-sncn-11.rs");
}
