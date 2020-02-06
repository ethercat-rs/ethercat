// Part of ethercat-rs. Copyright 2018-2020 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[rustfmt::skip]
pub mod ioctl {
    use super::EC_IOCTL_TYPE as EC;
    use super::*;
    use nix::*;

    ioctl_read!     ( MODULE              , EC, 0x00, ec_ioctl_module_t);
    ioctl_read!     ( MASTER              , EC, 0x01, ec_ioctl_master_t);
    ioctl_readwrite!( SLAVE               , EC, 0x02, ec_ioctl_slave_t);
    ioctl_readwrite!( SLAVE_SYNC          , EC, 0x03, ec_ioctl_slave_sync_t);
    ioctl_readwrite!( SLAVE_SYNC_PDO      , EC, 0x04, ec_ioctl_slave_sync_pdo_t);
    ioctl_readwrite!( SLAVE_SYNC_PDO_ENTRY, EC, 0x05, ec_ioctl_slave_sync_pdo_entry_t);
    ioctl_readwrite!( DOMAIN              , EC, 0x06, ec_ioctl_domain_t);
    ioctl_readwrite!( DOMAIN_FMMU         , EC, 0x07, ec_ioctl_domain_fmmu_t);
    ioctl_readwrite!( DOMAIN_DATA         , EC, 0x08, ec_ioctl_domain_data_t);
    ioctl_none!     ( MASTER_DEBUG        , EC, 0x09);
    ioctl_none!     ( MASTER_RESCAN       , EC, 0x0a);
    ioctl_write_ptr!( SLAVE_STATE         , EC, 0x0b, ec_ioctl_slave_state_t);
    ioctl_readwrite!( SLAVE_SDO           , EC, 0x0c, ec_ioctl_slave_sdo_t);
    ioctl_readwrite!( SLAVE_SDO_ENTRY     , EC, 0x0d, ec_ioctl_slave_sdo_entry_t);
    ioctl_readwrite!( SLAVE_SDO_UPLOAD    , EC, 0x0e, ec_ioctl_slave_sdo_upload_t);
    ioctl_readwrite!( SLAVE_SDO_DOWNLOAD  , EC, 0x0f, ec_ioctl_slave_sdo_download_t);
    ioctl_readwrite!( SLAVE_SII_READ      , EC, 0x10, ec_ioctl_slave_sii_t);
    ioctl_write_ptr!( SLAVE_SII_WRITE     , EC, 0x11, ec_ioctl_slave_sii_t);
    ioctl_readwrite!( SLAVE_REG_READ      , EC, 0x12, ec_ioctl_slave_reg_t);
    ioctl_write_ptr!( SLAVE_REG_WRITE     , EC, 0x13, ec_ioctl_slave_reg_t);
    ioctl_readwrite!( SLAVE_FOE_READ      , EC, 0x14, ec_ioctl_slave_foe_t);
    ioctl_write_ptr!( SLAVE_FOE_WRITE     , EC, 0x15, ec_ioctl_slave_foe_t);
    ioctl_readwrite!( SLAVE_SOE_READ      , EC, 0x16, ec_ioctl_slave_soe_read_t);
    ioctl_readwrite!( SLAVE_SOE_WRITE     , EC, 0x17, ec_ioctl_slave_soe_write_t);
    ioctl_write_ptr!( SLAVE_EOE_IP_PARAM  , EC, 0x18, ec_ioctl_slave_eoe_ip_t);
    ioctl_readwrite!( CONFIG              , EC, 0x19, ec_ioctl_config_t);
    ioctl_readwrite!( CONFIG_PDO          , EC, 0x1a, ec_ioctl_config_pdo_t);
    ioctl_readwrite!( CONFIG_PDO_ENTR     , EC, 0x1b, ec_ioctl_config_pdo_entry_t);
    ioctl_readwrite!( CONFIG_SDO          , EC, 0x1c, ec_ioctl_config_sdo_t);
    ioctl_readwrite!( CONFIG_IDN          , EC, 0x1d, ec_ioctl_config_idn_t);
    ioctl_readwrite!( EOE_HANDLER         , EC, 0x1e, ec_ioctl_eoe_handler_t);
    ioctl_none!     ( REQUEST             , EC, 0x1f);
    ioctl_none!     ( CREATE_DOMAIN       , EC, 0x20);
    ioctl_readwrite!( CREATE_SLAVE_CONFIG , EC, 0x21, ec_ioctl_config_t);
    ioctl_write_ptr!( SELECT_REF_CLOCK    , EC, 0x22, u32);
    ioctl_read!     ( ACTIVATE            , EC, 0x23, ec_ioctl_master_activate_t);
    ioctl_none!     ( DEACTIVATE          , EC, 0x24);
    ioctl_write_int!( SEND                , EC, 0x25);
    ioctl_none!     ( RECEIVE             , EC, 0x26);
    ioctl_read!     ( MASTER_STATE        , EC, 0x27, ec_master_state_t);
    ioctl_readwrite!( MASTER_LINK_STATE   , EC, 0x28, ec_ioctl_link_state_t);
    ioctl_write_ptr!( APP_TIME            , EC, 0x29, ec_ioctl_app_time_t);
    ioctl_none!     ( SYNC_REF            , EC, 0x2a);
    ioctl_none!     ( SYNC_SLAVES         , EC, 0x2b);
    ioctl_read!     ( REF_CLOCK_TIME      , EC, 0x2c, u32);
    ioctl_none!     ( SYNC_MON_QUEUE      , EC, 0x2d);
    ioctl_read!     ( SYNC_MON_PROCESS    , EC, 0x2e, u32);
    ioctl_none!     ( RESET               , EC, 0x2f);
    ioctl_write_ptr!( SC_SYNC             , EC, 0x30, ec_ioctl_config_t);
    ioctl_write_ptr!( SC_WATCHDOG         , EC, 0x31, ec_ioctl_config_t);
    ioctl_write_ptr!( SC_ADD_PDO          , EC, 0x32, ec_ioctl_config_pdo_t);
    ioctl_write_ptr!( SC_CLEAR_PDOS       , EC, 0x33, ec_ioctl_config_pdo_t);
    ioctl_write_ptr!( SC_ADD_ENTRY        , EC, 0x34, ec_ioctl_add_pdo_entry_t);
    ioctl_write_ptr!( SC_CLEAR_ENTRIES    , EC, 0x35, ec_ioctl_config_pdo_t);
    ioctl_readwrite!( SC_REG_PDO_ENTRY    , EC, 0x36, ec_ioctl_reg_pdo_entry_t);
    ioctl_readwrite!( SC_REG_PDO_POS      , EC, 0x37, ec_ioctl_reg_pdo_pos_t);
    ioctl_write_ptr!( SC_DC               , EC, 0x38, ec_ioctl_config_t);
    ioctl_write_ptr!( SC_SDO              , EC, 0x39, ec_ioctl_sc_sdo_t);
    ioctl_write_ptr!( SC_EMERG_SIZE       , EC, 0x3a, ec_ioctl_sc_emerg_t);
    ioctl_readwrite!( SC_EMERG_POP        , EC, 0x3b, ec_ioctl_sc_emerg_t);
    ioctl_write_ptr!( SC_EMERG_CLEAR      , EC, 0x3c, ec_ioctl_sc_emerg_t);
    ioctl_readwrite!( SC_EMERG_OVERRUNS   , EC, 0x3d, ec_ioctl_sc_emerg_t);
    ioctl_readwrite!( SC_SDO_REQUEST      , EC, 0x3e, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SC_REG_REQUEST      , EC, 0x3f, ec_ioctl_reg_request_t);
    ioctl_readwrite!( SC_VOE              , EC, 0x40, ec_ioctl_voe_t);
    ioctl_readwrite!( SC_STATE            , EC, 0x41, ec_ioctl_sc_state_t);
    ioctl_write_ptr!( SC_IDN              , EC, 0x42, ec_ioctl_sc_idn_t);
    ioctl_write_int!( DOMAIN_SIZE         , EC, 0x43);
    ioctl_write_int!( DOMAIN_OFFSET       , EC, 0x44);
    ioctl_write_int!( DOMAIN_PROCESS      , EC, 0x45);
    ioctl_write_int!( DOMAIN_QUEUE        , EC, 0x46);
    ioctl_readwrite!( DOMAIN_STATE        , EC, 0x47, ec_ioctl_domain_state_t);
    ioctl_readwrite!( SDO_REQUEST_INDEX   , EC, 0x48, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SDO_REQUEST_TIMEOUT , EC, 0x49, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SDO_REQUEST_STATE   , EC, 0x4a, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SDO_REQUEST_READ    , EC, 0x4b, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SDO_REQUEST_WRITE   , EC, 0x4c, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( SDO_REQUEST_DATA    , EC, 0x4d, ec_ioctl_sdo_request_t);
    ioctl_readwrite!( REG_REQUEST_DATA    , EC, 0x4e, ec_ioctl_reg_request_t);
    ioctl_readwrite!( REG_REQUEST_STATE   , EC, 0x4f, ec_ioctl_reg_request_t);
    ioctl_readwrite!( REG_REQUEST_WRITE   , EC, 0x50, ec_ioctl_reg_request_t);
    ioctl_readwrite!( REG_REQUEST_READ    , EC, 0x51, ec_ioctl_reg_request_t);
    ioctl_write_ptr!( VOE_SEND_HEADER     , EC, 0x52, ec_ioctl_voe_t);
    ioctl_readwrite!( VOE_REC_HEADER      , EC, 0x53, ec_ioctl_voe_t);
    ioctl_write_ptr!( VOE_READ            , EC, 0x54, ec_ioctl_voe_t);
    ioctl_write_ptr!( VOE_READ_NOSYNC     , EC, 0x55, ec_ioctl_voe_t);
    ioctl_readwrite!( VOE_WRITE           , EC, 0x56, ec_ioctl_voe_t);
    ioctl_readwrite!( VOE_EXEC            , EC, 0x57, ec_ioctl_voe_t);
    ioctl_readwrite!( VOE_DATA            , EC, 0x58, ec_ioctl_voe_t);
    ioctl_write_ptr!( SET_SEND_INTERVAL   , EC, 0x59, usize);
    ioctl_write_ptr!( SC_OVERLAPPING_IO   , EC, 0x5a, ec_ioctl_config_t);
}
