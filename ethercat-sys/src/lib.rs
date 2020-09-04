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

    // Command-line tool
    ioctl!(read      MODULE               with EC, 0x00; ec_ioctl_module_t);
    ioctl!(read      MASTER               with EC, 0x01; ec_ioctl_master_t);
    ioctl!(readwrite SLAVE                with EC, 0x02; ec_ioctl_slave_t);
    ioctl!(readwrite SLAVE_SYNC           with EC, 0x03; ec_ioctl_slave_sync_t);
    ioctl!(readwrite SLAVE_SYNC_PDO       with EC, 0x04; ec_ioctl_slave_sync_pdo_t);
    ioctl!(readwrite SLAVE_SYNC_PDO_ENTRY with EC, 0x05; ec_ioctl_slave_sync_pdo_entry_t);
    ioctl!(readwrite DOMAIN               with EC, 0x06; ec_ioctl_domain_t);
    ioctl!(readwrite DOMAIN_FMMU          with EC, 0x07; ec_ioctl_domain_fmmu_t);
    ioctl!(readwrite DOMAIN_DATA          with EC, 0x08; ec_ioctl_domain_data_t);
    ioctl!(none      MASTER_DEBUG         with EC, 0x09);
    ioctl!(none      MASTER_RESCAN        with EC, 0x0a);
    ioctl!(write     SLAVE_STATE          with EC, 0x0b; ec_ioctl_slave_state_t);
    ioctl!(readwrite SLAVE_SDO            with EC, 0x0c; ec_ioctl_slave_sdo_t);
    ioctl!(readwrite SLAVE_SDO_ENTRY      with EC, 0x0d; ec_ioctl_slave_sdo_entry_t);
    ioctl!(readwrite SLAVE_SDO_UPLOAD     with EC, 0x0e; ec_ioctl_slave_sdo_upload_t);
    ioctl!(readwrite SLAVE_SDO_DOWNLOAD   with EC, 0x0f; ec_ioctl_slave_sdo_download_t);
    ioctl!(readwrite SLAVE_SII_READ       with EC, 0x10; ec_ioctl_slave_sii_t);
    ioctl!(write     SLAVE_SII_WRITE      with EC, 0x11; ec_ioctl_slave_sii_t);
    ioctl!(readwrite SLAVE_REG_READ       with EC, 0x12; ec_ioctl_slave_reg_t);
    ioctl!(write     SLAVE_REG_WRITE      with EC, 0x13; ec_ioctl_slave_reg_t);
    ioctl!(readwrite SLAVE_FOE_READ       with EC, 0x14; ec_ioctl_slave_foe_t);
    ioctl!(write     SLAVE_FOE_WRITE      with EC, 0x15; ec_ioctl_slave_foe_t);
    ioctl!(readwrite SLAVE_SOE_READ       with EC, 0x16; ec_ioctl_slave_soe_read_t);
    ioctl!(readwrite SLAVE_SOE_WRITE      with EC, 0x17; ec_ioctl_slave_soe_write_t);
    ioctl!(write     SLAVE_EOE_IP_PARAM   with EC, 0x18; ec_ioctl_slave_eoe_ip_t);
    ioctl!(readwrite CONFIG               with EC, 0x19; ec_ioctl_config_t);
    ioctl!(readwrite CONFIG_PDO           with EC, 0x1a; ec_ioctl_config_pdo_t);
    ioctl!(readwrite CONFIG_PDO_ENTRY     with EC, 0x1b; ec_ioctl_config_pdo_entry_t);
    ioctl!(readwrite CONFIG_SDO           with EC, 0x1c; ec_ioctl_config_sdo_t);
    ioctl!(readwrite CONFIG_IDN           with EC, 0x1d; ec_ioctl_config_idn_t);
    ioctl!(readwrite EOE_HANDLER          with EC, 0x1e; ec_ioctl_eoe_handler_t);

    // Application interface
    ioctl!(none      REQUEST             with EC, 0x1f);
    ioctl!(none      CREATE_DOMAIN       with EC, 0x20);
    ioctl!(readwrite CREATE_SLAVE_CONFIG with EC, 0x21; ec_ioctl_config_t);
    ioctl!(write     SELECT_REF_CLOCK    with EC, 0x22; u32);
    ioctl!(read      ACTIVATE            with EC, 0x23; ec_ioctl_master_activate_t);
    ioctl!(none      DEACTIVATE          with EC, 0x24);
    ioctl!(arg       SEND                with EC, 0x25);
    ioctl!(none      RECEIVE             with EC, 0x26);
    ioctl!(read      MASTER_STATE        with EC, 0x27; ec_master_state_t);
    ioctl!(readwrite MASTER_LINK_STATE   with EC, 0x28; ec_ioctl_link_state_t);
    ioctl!(write     APP_TIME            with EC, 0x29; u64);
    ioctl!(none      SYNC_REF            with EC, 0x2a);
    ioctl!(write     SYNC_REF_TO         with EC, 0x2b; u64);
    ioctl!(none      SYNC_SLAVES         with EC, 0x2c);
    ioctl!(read      REF_CLOCK_TIME      with EC, 0x2d; u32);
    ioctl!(none      SYNC_MON_QUEUE      with EC, 0x2e);
    ioctl!(read      SYNC_MON_PROCESS    with EC, 0x2f; u32);
    ioctl!(none      RESET               with EC, 0x30);

    ioctl!(write     SC_SYNC             with EC, 0x31; ec_ioctl_config_t);
    ioctl!(write     SC_WATCHDOG         with EC, 0x32; ec_ioctl_config_t);
    ioctl!(write     SC_ADD_PDO          with EC, 0x33; ec_ioctl_config_pdo_t);
    ioctl!(write     SC_CLEAR_PDOS       with EC, 0x34; ec_ioctl_config_pdo_t);
    ioctl!(write     SC_ADD_ENTRY        with EC, 0x35; ec_ioctl_add_pdo_entry_t);
    ioctl!(write     SC_CLEAR_ENTRIES    with EC, 0x36; ec_ioctl_config_pdo_t);
    ioctl!(readwrite SC_REG_PDO_ENTRY    with EC, 0x37; ec_ioctl_reg_pdo_entry_t);
    ioctl!(readwrite SC_REG_PDO_POS      with EC, 0x38; ec_ioctl_reg_pdo_pos_t);
    ioctl!(write     SC_DC               with EC, 0x39; ec_ioctl_config_t);
    ioctl!(write     SC_SDO              with EC, 0x3a; ec_ioctl_sc_sdo_t);
    ioctl!(write     SC_EMERG_SIZE       with EC, 0x3b; ec_ioctl_sc_emerg_t);
    ioctl!(readwrite SC_EMERG_POP        with EC, 0x3c; ec_ioctl_sc_emerg_t);
    ioctl!(write     SC_EMERG_CLEAR      with EC, 0x3d; ec_ioctl_sc_emerg_t);
    ioctl!(readwrite SC_EMERG_OVERRUNS   with EC, 0x3e; ec_ioctl_sc_emerg_t);
    ioctl!(readwrite SC_SDO_REQUEST      with EC, 0x3f; ec_ioctl_sdo_request_t);
    ioctl!(readwrite SC_REG_REQUEST      with EC, 0x40; ec_ioctl_reg_request_t);
    ioctl!(readwrite SC_VOE              with EC, 0x41; ec_ioctl_voe_t);
    ioctl!(readwrite SC_STATE            with EC, 0x42; ec_ioctl_sc_state_t);
    ioctl!(write     SC_IDN              with EC, 0x43; ec_ioctl_sc_idn_t);

    ioctl!(arg       DOMAIN_SIZE         with EC, 0x44);
    ioctl!(arg       DOMAIN_OFFSET       with EC, 0x45);
    ioctl!(arg       DOMAIN_PROCESS      with EC, 0x46);
    ioctl!(arg       DOMAIN_QUEUE        with EC, 0x47);
    ioctl!(readwrite DOMAIN_STATE        with EC, 0x48;  ec_ioctl_domain_state_t);

    ioctl!(readwrite SDO_REQUEST_INDEX   with EC, 0x49;  ec_ioctl_sdo_request_t);
    ioctl!(readwrite SDO_REQUEST_TIMEOUT with EC, 0x4a;  ec_ioctl_sdo_request_t);
    ioctl!(readwrite SDO_REQUEST_STATE   with EC, 0x4b;  ec_ioctl_sdo_request_t);
    ioctl!(readwrite SDO_REQUEST_READ    with EC, 0x4c;  ec_ioctl_sdo_request_t);
    ioctl!(readwrite SDO_REQUEST_WRITE   with EC, 0x4d;  ec_ioctl_sdo_request_t);
    ioctl!(readwrite SDO_REQUEST_DATA    with EC, 0x4e;  ec_ioctl_sdo_request_t);

    ioctl!(readwrite REG_REQUEST_DATA    with EC, 0x4f;  ec_ioctl_reg_request_t);
    ioctl!(readwrite REG_REQUEST_STATE   with EC, 0x50;  ec_ioctl_reg_request_t);
    ioctl!(readwrite REG_REQUEST_WRITE   with EC, 0x51;  ec_ioctl_reg_request_t);
    ioctl!(readwrite REG_REQUEST_READ    with EC, 0x52;  ec_ioctl_reg_request_t);

    ioctl!(write     VOE_SEND_HEADER     with EC, 0x53;  ec_ioctl_voe_t);
    ioctl!(readwrite VOE_REC_HEADER      with EC, 0x54;  ec_ioctl_voe_t);
    ioctl!(write     VOE_READ            with EC, 0x55;  ec_ioctl_voe_t);
    ioctl!(write     VOE_READ_NOSYNC     with EC, 0x56;  ec_ioctl_voe_t);
    ioctl!(readwrite VOE_WRITE           with EC, 0x57;  ec_ioctl_voe_t);
    ioctl!(readwrite VOE_EXEC            with EC, 0x58;  ec_ioctl_voe_t);
    ioctl!(readwrite VOE_DATA            with EC, 0x59;  ec_ioctl_voe_t);

    ioctl!(write      SET_SEND_INTERVAL     with EC, 0x5a; usize);
    ioctl!(write      SC_OVERLAPPING_IO     with EC, 0x5b; ec_ioctl_config_t);
    ioctl!(write      SLAVE_REBOOT          with EC, 0x5c; ec_ioctl_slave_reboot_t);
    ioctl!(readwrite  SLAVE_REG_READWRITE   with EC, 0x5d; ec_ioctl_slave_reg_t);
    ioctl!(readwrite  REG_REQUEST_READWRITE with EC, 0x5e; ec_ioctl_reg_request_t);
    ioctl!(read       SETUP_DOMAIN_MEMORY   with EC, 0x60; ec_ioctl_master_activate_t);
    ioctl!(arg        DEACTIVATE_SLAVES     with EC, 0x61);

    // TODO: ioctl!(arg        64_REF_CLK_TIME_QUEUE with EC, 0x62);
    // TODO: ioctl!(read       64_REF_CLK_TIME       with EC, 0x63; u64);

    ioctl!(readwrite  SC_FOE_REQUEST        with EC, 0x64; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_FILE      with EC, 0x65; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_TIMEOUT   with EC, 0x66; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_STATE     with EC, 0x67; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_READ      with EC, 0x68; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_WRITE     with EC, 0x69; ec_ioctl_foe_request_t);
    ioctl!(readwrite  FOE_REQUEST_DATA      with EC, 0x6a; ec_ioctl_foe_request_t);
    ioctl!(write      RT_SLAVE_REQUESTS     with EC, 0x6b; u32);
    ioctl!(arg        EXEC_SLAVE_REQUESTS   with EC, 0x6c);
    ioctl!(arg        EOE_IS_OPEN           with EC, 0x6d);
    ioctl!(arg        EOE_PROCESS           with EC, 0x6e);
    ioctl!(arg        SEND_EXT              with EC, 0x6f);
    ioctl!(readwrite EOE_ADDIF              with EC, 0x70; ec_ioctl_eoe_if_t);
    ioctl!(readwrite EOE_DELIF              with EC, 0x71; ec_ioctl_eoe_if_t);
    ioctl!(readwrite PCAP_DATA              with EC, 0x72; ec_ioctl_pcap_data_t);

    // Gateway
    ioctl!(readwrite MBOX_GATEWAY           with EC, 0x73; ec_ioctl_mbox_gateway_t);
    ioctl!(write     SLAVE_DICT_UPLOAD      with EC, 0x7f; ec_ioctl_slave_dict_upload_t);
}
