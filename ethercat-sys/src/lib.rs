#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::os::raw::c_ulong;
const TYPE: c_ulong = (EC_IOCTL_TYPE as c_ulong) << _IOC_TYPESHIFT;

/// Define ioctls, which unfortunately bindgen cannot currently do.
macro_rules! ioctl {
    ($name:ident, $nr:expr) => {
        pub const $name: c_ulong = ($nr << _IOC_NRSHIFT) | TYPE;
    };
    ($name:ident, $nr:expr, r, $type:ident) => {
        pub const $name: c_ulong = ($nr << _IOC_NRSHIFT) | TYPE |
            (IOC_OUT as c_ulong) | (std::mem::size_of::<$type>() as c_ulong) << _IOC_SIZESHIFT;
    };
    ($name:ident, $nr:expr, w, $type:ident) => {
        pub const $name: c_ulong = ($nr << _IOC_NRSHIFT) | TYPE |
            (IOC_IN as c_ulong) | (std::mem::size_of::<$type>() as c_ulong) << _IOC_SIZESHIFT;
    };
    ($name:ident, $nr:expr, rw, $type:ident) => {
        pub const $name: c_ulong = ($nr << _IOC_NRSHIFT) | TYPE |
            (IOC_INOUT as c_ulong) | (std::mem::size_of::<$type>() as c_ulong) << _IOC_SIZESHIFT;
    };
}

ioctl!(EC_IOCTL_MODULE,               0x00, r,  ec_ioctl_module_t);
ioctl!(EC_IOCTL_MASTER,               0x01, r,  ec_ioctl_master_t);
ioctl!(EC_IOCTL_SLAVE,                0x02, rw, ec_ioctl_slave_t);
ioctl!(EC_IOCTL_SLAVE_SYNC,           0x03, rw, ec_ioctl_slave_sync_t);
ioctl!(EC_IOCTL_SLAVE_SYNC_PDO,       0x04, rw, ec_ioctl_slave_sync_pdo_t);
ioctl!(EC_IOCTL_SLAVE_SYNC_PDO_ENTRY, 0x05, rw, ec_ioctl_slave_sync_pdo_entry_t);
ioctl!(EC_IOCTL_DOMAIN,               0x06, rw, ec_ioctl_domain_t);
ioctl!(EC_IOCTL_DOMAIN_FMMU,          0x07, rw, ec_ioctl_domain_fmmu_t);
ioctl!(EC_IOCTL_DOMAIN_DATA,          0x08, rw, ec_ioctl_domain_data_t);
ioctl!(EC_IOCTL_MASTER_DEBUG,         0x09);
ioctl!(EC_IOCTL_MASTER_RESCAN,        0x0a);
ioctl!(EC_IOCTL_SLAVE_STATE,          0x0b, w,  ec_ioctl_slave_state_t);
ioctl!(EC_IOCTL_SLAVE_SDO,            0x0c, rw, ec_ioctl_slave_sdo_t);
ioctl!(EC_IOCTL_SLAVE_SDO_ENTRY,      0x0d, rw, ec_ioctl_slave_sdo_entry_t);
ioctl!(EC_IOCTL_SLAVE_SDO_UPLOAD,     0x0e, rw, ec_ioctl_slave_sdo_upload_t);
ioctl!(EC_IOCTL_SLAVE_SDO_DOWNLOAD,   0x0f, rw, ec_ioctl_slave_sdo_download_t);
ioctl!(EC_IOCTL_SLAVE_SII_READ,       0x10, rw, ec_ioctl_slave_sii_t);
ioctl!(EC_IOCTL_SLAVE_SII_WRITE,      0x11, w,  ec_ioctl_slave_sii_t);
ioctl!(EC_IOCTL_SLAVE_REG_READ,       0x12, rw, ec_ioctl_slave_reg_t);
ioctl!(EC_IOCTL_SLAVE_REG_WRITE,      0x13, w,  ec_ioctl_slave_reg_t);
ioctl!(EC_IOCTL_SLAVE_FOE_READ,       0x14, rw, ec_ioctl_slave_foe_t);
ioctl!(EC_IOCTL_SLAVE_FOE_WRITE,      0x15, w,  ec_ioctl_slave_foe_t);
ioctl!(EC_IOCTL_SLAVE_SOE_READ,       0x16, rw, ec_ioctl_slave_soe_read_t);
ioctl!(EC_IOCTL_SLAVE_SOE_WRITE,      0x17, rw, ec_ioctl_slave_soe_write_t);
ioctl!(EC_IOCTL_SLAVE_EOE_IP_PARAM,   0x18, w,  ec_ioctl_slave_eoe_ip_t);
ioctl!(EC_IOCTL_CONFIG,               0x19, rw, ec_ioctl_config_t);
ioctl!(EC_IOCTL_CONFIG_PDO,           0x1a, rw, ec_ioctl_config_pdo_t);
ioctl!(EC_IOCTL_CONFIG_PDO_ENTRY,     0x1b, rw, ec_ioctl_config_pdo_entry_t);
ioctl!(EC_IOCTL_CONFIG_SDO,           0x1c, rw, ec_ioctl_config_sdo_t);
ioctl!(EC_IOCTL_CONFIG_IDN,           0x1d, rw, ec_ioctl_config_idn_t);
ioctl!(EC_IOCTL_EOE_HANDLER,          0x1e, rw, ec_ioctl_eoe_handler_t);

ioctl!(EC_IOCTL_REQUEST,              0x1f);
ioctl!(EC_IOCTL_CREATE_DOMAIN,        0x20);
ioctl!(EC_IOCTL_CREATE_SLAVE_CONFIG,  0x21, rw, ec_ioctl_config_t);
ioctl!(EC_IOCTL_SELECT_REF_CLOCK,     0x22, w,  u32);
ioctl!(EC_IOCTL_ACTIVATE,             0x23, r,  ec_ioctl_master_activate_t);
ioctl!(EC_IOCTL_DEACTIVATE,           0x24);
ioctl!(EC_IOCTL_SEND,                 0x25);
ioctl!(EC_IOCTL_RECEIVE,              0x26);
ioctl!(EC_IOCTL_MASTER_STATE,         0x27, r,  ec_master_state_t);
ioctl!(EC_IOCTL_MASTER_LINK_STATE,    0x28, rw, ec_ioctl_link_state_t);
ioctl!(EC_IOCTL_APP_TIME,             0x29, w,  ec_ioctl_app_time_t);
ioctl!(EC_IOCTL_SYNC_REF,             0x2a);
ioctl!(EC_IOCTL_SYNC_SLAVES,          0x2b);
ioctl!(EC_IOCTL_REF_CLOCK_TIME,       0x2c, r,  u32);
ioctl!(EC_IOCTL_SYNC_MON_QUEUE,       0x2d);
ioctl!(EC_IOCTL_SYNC_MON_PROCESS,     0x2e, r,  u32);
ioctl!(EC_IOCTL_RESET,                0x2f);
ioctl!(EC_IOCTL_SC_SYNC,              0x30, w,  ec_ioctl_config_t);
ioctl!(EC_IOCTL_SC_WATCHDOG,          0x31, w,  ec_ioctl_config_t);
ioctl!(EC_IOCTL_SC_ADD_PDO,           0x32, w,  ec_ioctl_config_pdo_t);
ioctl!(EC_IOCTL_SC_CLEAR_PDOS,        0x33, w,  ec_ioctl_config_pdo_t);
ioctl!(EC_IOCTL_SC_ADD_ENTRY,         0x34, w,  ec_ioctl_add_pdo_entry_t);
ioctl!(EC_IOCTL_SC_CLEAR_ENTRIES,     0x35, w,  ec_ioctl_config_pdo_t);
ioctl!(EC_IOCTL_SC_REG_PDO_ENTRY,     0x36, rw, ec_ioctl_reg_pdo_entry_t);
ioctl!(EC_IOCTL_SC_REG_PDO_POS,       0x37, rw, ec_ioctl_reg_pdo_pos_t);
ioctl!(EC_IOCTL_SC_DC,                0x38, w,  ec_ioctl_config_t);
ioctl!(EC_IOCTL_SC_SDO,               0x39, w,  ec_ioctl_sc_sdo_t);
ioctl!(EC_IOCTL_SC_EMERG_SIZE,        0x3a, w,  ec_ioctl_sc_emerg_t);
ioctl!(EC_IOCTL_SC_EMERG_POP,         0x3b, rw, ec_ioctl_sc_emerg_t);
ioctl!(EC_IOCTL_SC_EMERG_CLEAR,       0x3c, w,  ec_ioctl_sc_emerg_t);
ioctl!(EC_IOCTL_SC_EMERG_OVERRUNS,    0x3d, rw, ec_ioctl_sc_emerg_t);
ioctl!(EC_IOCTL_SC_SDO_REQUEST,       0x3e, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SC_REG_REQUEST,       0x3f, rw, ec_ioctl_reg_request_t);
ioctl!(EC_IOCTL_SC_VOE,               0x40, rw, ec_ioctl_voe_t);
ioctl!(EC_IOCTL_SC_STATE,             0x41, rw, ec_ioctl_sc_state_t);
ioctl!(EC_IOCTL_SC_IDN,               0x42, w,  ec_ioctl_sc_idn_t);
ioctl!(EC_IOCTL_DOMAIN_SIZE,          0x43);
ioctl!(EC_IOCTL_DOMAIN_OFFSET,        0x44);
ioctl!(EC_IOCTL_DOMAIN_PROCESS,       0x45);
ioctl!(EC_IOCTL_DOMAIN_QUEUE,         0x46);
ioctl!(EC_IOCTL_DOMAIN_STATE,         0x47, rw, ec_ioctl_domain_state_t);
ioctl!(EC_IOCTL_SDO_REQUEST_INDEX,    0x48, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SDO_REQUEST_TIMEOUT,  0x49, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SDO_REQUEST_STATE,    0x4a, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SDO_REQUEST_READ,     0x4b, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SDO_REQUEST_WRITE,    0x4c, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_SDO_REQUEST_DATA,     0x4d, rw, ec_ioctl_sdo_request_t);
ioctl!(EC_IOCTL_REG_REQUEST_DATA,     0x4e, rw, ec_ioctl_reg_request_t);
ioctl!(EC_IOCTL_REG_REQUEST_STATE,    0x4f, rw, ec_ioctl_reg_request_t);
ioctl!(EC_IOCTL_REG_REQUEST_WRITE,    0x50, rw, ec_ioctl_reg_request_t);
ioctl!(EC_IOCTL_REG_REQUEST_READ,     0x51, rw, ec_ioctl_reg_request_t);
ioctl!(EC_IOCTL_VOE_SEND_HEADER,      0x52, w,  ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_REC_HEADER,       0x53, rw, ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_READ,             0x54, w,  ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_READ_NOSYNC,      0x55, w,  ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_WRITE,            0x56, rw, ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_EXEC,             0x57, rw, ec_ioctl_voe_t);
ioctl!(EC_IOCTL_VOE_DATA,             0x58, rw, ec_ioctl_voe_t);
ioctl!(EC_IOCTL_SET_SEND_INTERVAL,    0x59, w,  usize);
ioctl!(EC_IOCTL_SC_OVERLAPPING_IO,    0x5a, w,  ec_ioctl_config_t);
