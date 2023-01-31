# The `ethercat` crate

[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache2.0-blue.svg)](./LICENSE-APACHE)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![crates.io](http://meritbadge.herokuapp.com/ethercat)](https://crates.io/crates/ethercat)
[![docs](https://docs.rs/ethercat/badge.svg)](https://docs.rs/ethercat)

[Documentation](https://docs.rs/crate/ethercat/)

# About

The `ethercat` crate provides a Rust wrapper for the IgH/Etherlab
[EtherCAT Master for Linux](https://etherlab.org/en/ethercat/).

EtherCAT is an Ethernet-based fieldbus system, originally invented by Beckhoff
GmbH but now used by numerous providers of automation related hardware.
The IgH master lets you provide an EtherCAT master on a Linux machine without
specialized hardware.

# Building

In order to build the raw wrapper crate `ethercat-sys`, you need to set the
environment variable `ETHERCAT_PATH` to the location of a checkout of the IgH
Etherlab repository, *after running `configure` there*.

- The minimum tested Rust version is 1.34.2.
- The recommended EtherCAT source is: https://gitlab.com/etherlab.org/ethercat

Bindings have been pregenerated for revision `7fd991bec7` - if you activate the
feature `pregenerated-bindings`, you don't need the master code to build, but
the kernel modules must match that revision.

# Licensing

The Etherlab master provides Linux kernel modules under GPLv2 with an ioctl
based interface, and a userspace library under LGPLv2.1.  This crate does
not use the userspace library (which is mostly a simple wrapper around the
ioctls anyway) but rather communicates with the kernel modules through the
raw ioctls.

Therefore, we believe that the crate does not need to be GPLv2-licensed, and
are using the dual MIT/Apache-2 license commonly used for Rust crates.
