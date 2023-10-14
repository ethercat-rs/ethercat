# Changelog

## v0.3.1 (2023-10-14)

- Bump MSRV to 1.63.0
- Add `Master::sync_reference_clock_to` (PR #46)

## v0.3.0 (2023-04-05)

- Bump MSRV to 1.58.1 and bindgen to 0.63 (PR #42)
- Update `pregenerated-bindings` to commit c022ddbc of the master code

## v0.2.4 (2023-02-24)

- Fix compilation on ARM (PR #40)

## v0.2.3 (2022-12-03)

- Add `Master::foe_read` and `Master::foe_write` methods (PR #35)

## v0.2.2 (2021-03-27)

- Implement `SdoData` for floating point types
- Add bindings to distributed-clock related APIs on `Master`

## v0.2.1 (2020-11-21)

- Use `AlState` from `ethercat-types`

## v0.2.0 (2020-11-02)

- Move to [ethercat-rs](https://github.com/ethercat-rs) GitHub organization
- Move `ethercat-plc` into [separate repository](https://github.com/ethercat-rs/ethercat-plc)
- Move some common data structures into separate crate [`ethercat-types`](https://github.com/ethercat-rs/ethercat-types)
- BREAKING: `Master::reserve` does not open an instance anymore, use `Master::open` instead
- BREAKING: Rename a lot of fields and types from `index` to `idx`
- BREAKING: Rename `PdoInfo` to `PdoCfg`
- BREAKING: Split some fields from `SyncInfo` into `SyncCfg`
- BREAKING: Refactor & change `Master::config_pdos` to `Master::config_sm_pdos`
- BREAKING: Use specital `Error` enum instead of `io::Error`
- BRAKING: `Master::sdo_download` has now a `complet_access` parameter
- Add `Master::master_count`
- Add `Master::get_sdo` & `Master::get_sdo_entry`
- Add `Master::get_pdo` & `Master::get_pdo_entry`
- Add `Master::get_sync`
- Add `Master::request_state`
- Add `Master::dict_upload`
- Add some basic log messages
- Add `info`, `sdo`, `cyclic-data` examples
- Add `pregenerated-bindings` feature to compile the crate with pregenerated bindings (CAUTION: this might lead to problems)
- Add `sncn` feature to compile with synapticon `v1.5.2-sncn-11` fork
- Auto generate ioctl numbers from master header
- Derive some more implementations like `Clone`, `Copy`, etc. for some datastructures
- Refactoring & dependency updates

## v0.1.3 (2019-10-12)

- Relicense under MIT/Apache 2.0
- Add definition for Beckhoff EL1502
- Update dependencies

## v0.1.1 (2019-04-03)

- Fix ioctls for 32 Bit systems
- Add motor controller (EL7047) demo
- Minor fixes & dependency updates

## v0.1.0 (2019-03-01)

- Initial release
