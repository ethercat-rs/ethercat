// Part of ethercat-rs. Copyright 2018-2020 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use crate::{ec, types::*};
use num_traits::cast::FromPrimitive;
use std::{
    collections::HashMap,
    convert::TryFrom,
    ffi::CStr,
    fs::{File, OpenOptions},
    io,
    os::{raw::c_ulong, unix::io::AsRawFd},
};

macro_rules! ioctl {
    ($m:expr, $f:expr) => { ioctl!($m, $f,) };
    ($m:expr, $f:expr, $($arg:tt)*) => {{
        let res = unsafe { $f($m.file.as_raw_fd(), $($arg)*) };
        if res < 0 { Err(Error::Io(io::Error::last_os_error())) } else { Ok(res) }
    }}
}

/// An EtherCAT master.
pub struct Master {
    file: File,
    map: Option<memmap::MmapMut>,
    domains: HashMap<DomainIdx, DomainDataPlacement>,
}

pub struct Domain<'m> {
    master: &'m Master,
    idx: DomainIdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterAccess {
    ReadOnly,
    ReadWrite,
}

impl Master {
    pub fn open(idx: MasterIdx, access: MasterAccess) -> Result<Self> {
        let devpath = format!("/dev/EtherCAT{}", idx);
        log::debug!("Open EtherCAT Master {}", devpath);
        let file = OpenOptions::new()
            .read(true)
            .write(access == MasterAccess::ReadWrite)
            .open(&devpath)?;
        let mut module_info = ec::ec_ioctl_module_t::default();
        let master = Master {
            file,
            map: None,
            domains: HashMap::new(),
        };
        ioctl!(master, ec::ioctl::MODULE, &mut module_info)?;
        if module_info.ioctl_version_magic != ec::EC_IOCTL_VERSION_MAGIC {
            return Err(Error::KernelModule(
                ec::EC_IOCTL_VERSION_MAGIC,
                module_info.ioctl_version_magic,
            ));
        }
        Ok(master)
    }

    pub fn master_count() -> Result<usize> {
        let master = Self::open(0, MasterAccess::ReadOnly)?;
        let mut module_info = ec::ec_ioctl_module_t::default();
        ioctl!(master, ec::ioctl::MODULE, &mut module_info)?;
        Ok(module_info.master_count as usize)
    }

    pub fn reserve(&self) -> Result<()> {
        log::debug!("Reserve EtherCAT Master");
        ioctl!(self, ec::ioctl::REQUEST)?;
        Ok(())
    }

    pub fn create_domain(&self) -> Result<DomainIdx> {
        Ok((ioctl!(self, ec::ioctl::CREATE_DOMAIN)? as usize).into())
    }

    pub const fn domain(&self, idx: DomainIdx) -> Domain {
        Domain::new(idx, self)
    }

    pub fn domain_data(&mut self, idx: DomainIdx) -> Result<&mut [u8]> {
        let p = self
            .domain_data_placement(idx)
            .map_err(|_| Error::NoDomain)?;
        let data = self.map.as_mut().ok_or_else(|| Error::NotActivated)?;
        Ok(&mut data[p.offset..p.offset + p.size])
    }

    fn domain_data_placement(&mut self, idx: DomainIdx) -> Result<DomainDataPlacement> {
        Ok(match self.domains.get(&idx) {
            None => {
                let d_idx =
                    c_ulong::try_from(idx).map_err(|_| Error::DomainIdx(usize::from(idx)))?;
                let offset = ioctl!(self, ec::ioctl::DOMAIN_OFFSET, d_idx)? as usize;
                let size = ioctl!(self, ec::ioctl::DOMAIN_SIZE, d_idx)? as usize;
                let meta_data = DomainDataPlacement { offset, size };
                self.domains.insert(idx, meta_data);
                meta_data
            }
            Some(d) => *d,
        })
    }

    pub fn activate(&mut self) -> Result<()> {
        log::debug!("Activate EtherCAT Master");
        let mut data = ec::ec_ioctl_master_activate_t::default();
        ioctl!(self, ec::ioctl::ACTIVATE, &mut data)?;

        self.map = unsafe {
            memmap::MmapOptions::new()
                .len(data.process_data_size as usize)
                .map_mut(&self.file)
                .map(Some)?
        };
        self.map.as_mut().ok_or_else(|| Error::NotActivated)?[0] = 0;
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<()> {
        log::debug!("Deactivate EtherCAT Master");
        ioctl!(self, ec::ioctl::DEACTIVATE)?;
        self.domains.clear();
        self.map = None;
        Ok(())
    }

    pub fn set_send_interval(&mut self, interval_us: usize) -> Result<()> {
        ioctl!(self, ec::ioctl::SET_SEND_INTERVAL, &interval_us).map(|_| ())
    }

    pub fn send(&mut self) -> Result<usize> {
        let mut sent = 0;
        ioctl!(self, ec::ioctl::SEND, &mut sent as *mut _ as c_ulong)?;
        Ok(sent)
    }

    pub fn receive(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::RECEIVE).map(|_| ())
    }

    pub fn reset(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::RESET).map(|_| ())
    }

    pub fn state(&self) -> Result<MasterState> {
        let mut data = ec::ec_master_state_t::default();
        ioctl!(self, ec::ioctl::MASTER_STATE, &mut data)?;
        Ok(MasterState {
            slaves_responding: data.slaves_responding,
            al_states: data.al_states() as u8,
            link_up: data.link_up() != 0,
        })
    }

    pub fn link_state(&self, dev_idx: u32) -> Result<MasterState> {
        let mut state = ec::ec_master_link_state_t::default();
        let mut data = ec::ec_ioctl_link_state_t {
            dev_idx,
            state: &mut state,
        };
        ioctl!(self, ec::ioctl::MASTER_LINK_STATE, &mut data)?;
        Ok(MasterState {
            slaves_responding: state.slaves_responding,
            al_states: state.al_states() as u8,
            link_up: state.link_up() != 0,
        })
    }

    pub fn get_info(&self) -> Result<MasterInfo> {
        let mut data = ec::ec_ioctl_master_t::default();
        ioctl!(self, ec::ioctl::MASTER, &mut data)?;
        let ec::ec_ioctl_master_t {
            slave_count,
            devices,
            scan_busy,
            app_time,
            ..
        } = data;
        let first_device = devices.get(0).ok_or_else(|| Error::NoDevices)?;
        let link_up = first_device.link_state != 0;
        let scan_busy = scan_busy != 0;
        Ok(MasterInfo {
            slave_count,
            link_up,
            scan_busy,
            app_time,
        })
    }

    pub fn get_slave_info(&self, position: SlavePos) -> Result<SlaveInfo> {
        let mut data = ec::ec_ioctl_slave_t::default();
        data.position = u16::from(position);
        ioctl!(self, ec::ioctl::SLAVE, &mut data)?;
        let mut ports = [SlavePortInfo::default(); ec::EC_MAX_PORTS as usize];
        for (i, port) in ports.iter_mut().enumerate().take(ec::EC_MAX_PORTS as usize) {
            port.desc = match data.ports[i].desc {
                ec::EC_PORT_NOT_IMPLEMENTED => SlavePortType::NotImplemented,
                ec::EC_PORT_NOT_CONFIGURED => SlavePortType::NotConfigured,
                ec::EC_PORT_EBUS => SlavePortType::EBus,
                ec::EC_PORT_MII => SlavePortType::MII,
                x => panic!("invalid port type {}", x),
            };
            port.link = SlavePortLink {
                link_up: data.ports[i].link.link_up != 0,
                loop_closed: data.ports[i].link.loop_closed != 0,
                signal_detected: data.ports[i].link.signal_detected != 0,
            };
            port.receive_time = data.ports[i].receive_time;
            port.next_slave = data.ports[i].next_slave;
            port.delay_to_next_dc = data.ports[i].delay_to_next_dc;
        }
        Ok(SlaveInfo {
            name: unsafe {
                CStr::from_ptr(data.name.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            },
            ring_pos: data.position,
            id: SlaveId {
                vendor_id: data.vendor_id,
                product_code: data.product_code,
            },
            rev: SlaveRev {
                revision_number: data.revision_number,
                serial_number: data.serial_number,
            },
            alias: data.alias,
            current_on_ebus: data.current_on_ebus,
            al_state: AlState::try_from(data.al_state)
                .map_err(|_| Error::InvalidAlState(data.al_state))?,
            error_flag: data.error_flag,
            sync_count: data.sync_count,
            sdo_count: data.sdo_count,
            ports,
        })
    }

    pub fn get_config_info(&self, idx: SlaveConfigIdx) -> Result<ConfigInfo> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = idx;
        ioctl!(self, ec::ioctl::CONFIG, &mut data)?;
        let id = SlaveId {
            vendor_id: data.vendor_id,
            product_code: data.product_code,
        };
        let slave_position = if data.slave_position == -1 {
            None
        } else {
            Some(SlavePos::from(data.slave_position as u16))
        };
        Ok(ConfigInfo {
            alias: data.alias,
            position: data.position,
            id,
            slave_position,
            sdo_count: data.sdo_count,
            idn_count: data.idn_count,
        })
    }

    pub fn configure_slave(&mut self, addr: SlaveAddr, expected: SlaveId) -> Result<SlaveConfig> {
        log::debug!("Configure slave {:?}", addr);
        let mut data = ec::ec_ioctl_config_t::default();
        let (alias, pos) = addr.as_pair();
        data.alias = alias;
        data.position = pos;
        data.vendor_id = expected.vendor_id;
        data.product_code = expected.product_code;
        ioctl!(self, ec::ioctl::CREATE_SLAVE_CONFIG, &mut data)?;
        Ok(SlaveConfig {
            master: self,
            idx: data.config_index,
        })
    }

    pub fn get_sdo(&mut self, slave_pos: SlavePos, sdo_pos: SdoPos) -> Result<SdoInfo> {
        let mut sdo = ec::ec_ioctl_slave_sdo_t::default();
        sdo.slave_position = u16::from(slave_pos);
        sdo.sdo_position = u16::from(sdo_pos);
        ioctl!(self, ec::ioctl::SLAVE_SDO, &mut sdo)?;
        #[cfg(feature = "sncn")]
        {
            Ok(SdoInfo {
                pos: SdoPos::from(sdo.sdo_position),
                idx: Idx::from(sdo.sdo_index),
                max_sub_idx: SubIdx::from(sdo.max_subindex),
                object_code: Some(sdo.object_code),
                name: c_array_to_string(sdo.name.as_ptr()),
            })
        }
        #[cfg(not(feature = "sncn"))]
        {
            Ok(SdoInfo {
                pos: SdoPos::from(sdo.sdo_position),
                idx: Idx::from(sdo.sdo_index),
                max_sub_idx: SubIdx::from(sdo.max_subindex),
                object_code: None,
                name: c_array_to_string(sdo.name.as_ptr()),
            })
        }
    }

    pub fn get_sdo_entry(
        &mut self,
        slave_pos: SlavePos,
        addr: SdoEntryAddr,
    ) -> Result<SdoEntryInfo> {
        let mut entry = ec::ec_ioctl_slave_sdo_entry_t::default();
        entry.slave_position = u16::from(slave_pos);
        let (spec, sub) = match addr {
            SdoEntryAddr::ByPos(pos, sub) => ((u16::from(pos) as i32) * -1, sub),
            SdoEntryAddr::ByIdx(idx) => (u16::from(idx.idx) as i32, idx.sub_idx),
        };
        entry.sdo_spec = spec;
        entry.sdo_entry_subindex = u8::from(sub);
        ioctl!(self, ec::ioctl::SLAVE_SDO_ENTRY, &mut entry)?;
        Ok(SdoEntryInfo {
            data_type: DataType::from_u16(entry.data_type).unwrap_or_else(|| {
                let fallback = DataType::Raw;
                log::warn!(
                    "Slave {} / SDO {}: Unknown data type (type value: {:X}): use '{:?}' as fallback",
                    u16::from(slave_pos),
                    match addr {
                        SdoEntryAddr::ByPos(pos, sub) => format!("{:?} {:?} ", pos, sub),
                        SdoEntryAddr::ByIdx(idx) =>
                            format!("{:X}:{}", u16::from(idx.idx), u8::from(idx.sub_idx)),
                    },
                    entry.data_type,
                    fallback
                );
                fallback
            }),
            bit_len: entry.bit_length,
            access: get_sdo_entry_access(entry.read_access, entry.write_access),
            description: c_array_to_string(entry.description.as_ptr()),
        })
    }

    pub fn sdo_download<T>(
        &mut self,
        position: SlavePos,
        sdo_idx: SdoIdx,
        complete_access: bool,
        data: &T,
    ) -> Result<()>
    where
        T: SdoData + ?Sized,
    {
        #[cfg(feature = "sncn")]
        let data_ptr = data.data_ptr();

        #[cfg(not(feature = "sncn"))]
        let data_ptr = data.data_ptr() as *mut u8;

        let mut data = ec::ec_ioctl_slave_sdo_download_t {
            slave_position: u16::from(position),
            sdo_index: u16::from(sdo_idx.idx),
            sdo_entry_subindex: u8::from(sdo_idx.sub_idx),
            complete_access: if complete_access { 1 } else { 0 },
            data_size: data.data_size() as u64,
            data: data_ptr,
            abort_code: 0,
        };
        ioctl!(self, ec::ioctl::SLAVE_SDO_DOWNLOAD, &mut data).map(|_| ())
    }

    pub fn sdo_upload<'t>(
        &self,
        position: SlavePos,
        sdo_idx: SdoIdx,
        complete_access: bool,
        target: &'t mut [u8],
    ) -> Result<&'t mut [u8]> {
        let slave_position = u16::from(position);
        let sdo_index = u16::from(sdo_idx.idx);
        let sdo_entry_subindex = u8::from(sdo_idx.sub_idx);
        let target_size = target.len() as u64;
        let data_size = 0;
        let abort_code = 0;

        #[cfg(not(feature = "sncn"))]
        let mut data = ec::ec_ioctl_slave_sdo_upload_t {
            slave_position,
            sdo_index,
            sdo_entry_subindex,
            target_size,
            target: target.as_mut_ptr(),
            data_size,
            abort_code,
        };

        #[cfg(feature = "sncn")]
        let mut data = ec::ec_ioctl_slave_sdo_upload_t {
            slave_position,
            sdo_index,
            sdo_entry_subindex,
            target_size,
            target: target.as_mut_ptr(),
            data_size,
            abort_code,
            complete_access: if complete_access { 1 } else { 0 },
        };

        ioctl!(self, ec::ioctl::SLAVE_SDO_UPLOAD, &mut data)?;
        Ok(&mut target[..data.data_size as usize])
    }

    pub fn get_pdo(
        &mut self,
        slave_pos: SlavePos,
        sync_index: SmIdx,
        pdo_position: PdoPos,
    ) -> Result<PdoInfo> {
        let mut pdo = ec::ec_ioctl_slave_sync_pdo_t::default();
        pdo.slave_position = u16::from(slave_pos);
        pdo.sync_index = u8::from(sync_index) as u32;
        pdo.pdo_pos = u8::from(pdo_position) as u32;
        ioctl!(self, ec::ioctl::SLAVE_SYNC_PDO, &mut pdo)?;
        Ok(PdoInfo {
            sm: SmIdx::from(pdo.sync_index as u8),
            pos: PdoPos::from(pdo.pdo_pos as u8),
            idx: Idx::from(pdo.index),
            entry_count: pdo.entry_count,
            name: c_array_to_string(pdo.name.as_ptr()),
        })
    }

    pub fn get_pdo_entry(
        &mut self,
        slave_pos: SlavePos,
        sync_index: SmIdx,
        pdo_pos: PdoPos,
        entry_pos: PdoEntryPos,
    ) -> Result<PdoEntryInfo> {
        let mut entry = ec::ec_ioctl_slave_sync_pdo_entry_t::default();
        entry.slave_position = u16::from(slave_pos);
        entry.sync_index = u8::from(sync_index) as u32;
        entry.pdo_pos = u8::from(pdo_pos) as u32;
        entry.entry_pos = u8::from(entry_pos) as u32;
        ioctl!(self, ec::ioctl::SLAVE_SYNC_PDO_ENTRY, &mut entry)?;
        Ok(PdoEntryInfo {
            pos: PdoEntryPos::from(entry.pdo_pos as u8),
            entry_idx: PdoEntryIdx {
                idx: Idx::from(entry.index),
                sub_idx: SubIdx::from(entry.subindex),
            },
            bit_len: entry.bit_length,
            name: c_array_to_string(entry.name.as_ptr()),
        })
    }

    pub fn get_sync(&mut self, slave_pos: SlavePos, sm: SmIdx) -> Result<SmInfo> {
        let mut sync = ec::ec_ioctl_slave_sync_t::default();
        sync.slave_position = u16::from(slave_pos);
        sync.sync_index = u8::from(sm) as u32;
        ioctl!(self, ec::ioctl::SLAVE_SYNC, &mut sync)?;
        Ok(SmInfo {
            idx: SmIdx::from(sync.sync_index as u8),
            start_addr: sync.physical_start_address,
            default_size: sync.default_size,
            control_register: sync.control_register,
            enable: sync.enable == 1,
            pdo_count: sync.pdo_count,
        })
    }

    pub fn request_state(&mut self, slave_pos: SlavePos, state: AlState) -> Result<()> {
        let mut data = ec::ec_ioctl_slave_state_t::default();
        data.slave_position = u16::from(slave_pos);
        data.al_state = state as u8;
        ioctl!(self, ec::ioctl::SLAVE_STATE, &mut data)?;
        Ok(())
    }

    #[cfg(feature = "sncn")]
    pub fn dict_upload(&mut self, slave_pos: SlavePos) -> Result<()> {
        let mut data = ec::ec_ioctl_slave_dict_upload_t::default();
        data.slave_position = u16::from(slave_pos);
        ioctl!(self, ec::ioctl::SLAVE_DICT_UPLOAD, &mut data)?;
        Ok(())
    }

    // XXX missing: write_idn, read_idn,
    // application_time, sync_reference_clock, sync_slave_clocks,
    // reference_clock_time, sync_monitor_queue, sync_monitor_process
}

fn c_array_to_string(data: *const i8) -> String {
    unsafe { CStr::from_ptr(data).to_string_lossy().into_owned() }
}

#[test]
fn test_c_array_to_string() {
    let arr: [i8; 64] = [0_i8; 64];
    assert_eq!(c_array_to_string(arr.as_ptr()), "");

    let mut arr: [i8; 64] = [0_i8; 64];
    [80_i8, 114, 111, 100, 117, 99, 116, 32, 99, 111, 100, 101]
        .iter()
        .enumerate()
        .for_each(|(idx, v)| {
            arr[idx] = *v;
        });
    assert_eq!(c_array_to_string(arr.as_ptr()), "Product code");
}

pub struct SlaveConfig<'m> {
    master: &'m Master,
    idx: SlaveConfigIdx,
}

impl<'m> SlaveConfig<'m> {
    pub const fn index(&self) -> SlaveConfigIdx {
        self.idx
    }

    pub fn state(&self) -> Result<SlaveConfigState> {
        let mut state = ec::ec_slave_config_state_t::default();
        let mut data = ec::ec_ioctl_sc_state_t {
            config_index: self.idx,
            state: &mut state,
        };
        ioctl!(self.master, ec::ioctl::SC_STATE, &mut data)?;
        let al_state_u8 = state.al_state() as u8;
        Ok(SlaveConfigState {
            online: state.online() != 0,
            operational: state.operational() != 0,
            al_state: AlState::try_from(al_state_u8)
                .map_err(|_| Error::InvalidAlState(al_state_u8))?,
        })
    }

    /// Configure PDOs of a specifc Sync Manager
    pub fn config_sm_pdos(&mut self, sm_cfg: SmCfg, pdo_cfgs: &[PdoCfg]) -> Result<()> {
        self.config_sync_manager(&sm_cfg)?;
        self.clear_pdo_assignments(sm_cfg.idx)?;
        for pdo_cfg in &*pdo_cfgs {
            self.add_pdo_assignment(sm_cfg.idx, pdo_cfg.idx)?;
            if !pdo_cfg.entries.is_empty() {
                self.clear_pdo_mapping(pdo_cfg.idx)?;
                for entry in &pdo_cfg.entries {
                    self.add_pdo_mapping(pdo_cfg.idx, entry)?;
                }
            }
        }
        Ok(())
    }

    pub fn config_watchdog(&mut self, divider: u16, intervals: u16) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        data.watchdog_divider = divider;
        data.watchdog_intervals = intervals;
        ioctl!(self.master, ec::ioctl::SC_WATCHDOG, &data).map(|_| ())
    }

    #[cfg(feature = "sncn")]
    pub fn config_overlapping_pdos(&mut self, allow: bool) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        data.allow_overlapping_pdos = allow as u8;
        ioctl!(self.master, ec::ioctl::SC_OVERLAPPING_IO, &data).map(|_| ())
    }

    pub fn config_sync_manager(&mut self, cfg: &SmCfg) -> Result<()> {
        log::debug!("Configure Sync Manager: {:?}", cfg);
        if u8::from(cfg.idx) >= ec::EC_MAX_SYNC_MANAGERS as u8 {
            return Err(Error::SmIdxTooLarge);
        }
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        let ix = u8::from(cfg.idx) as usize;
        data.syncs[ix].dir = cfg.direction as u32;
        data.syncs[ix].watchdog_mode = cfg.watchdog_mode as u32;
        data.syncs[ix].config_this = 1;
        ioctl!(self.master, ec::ioctl::SC_SYNC, &data).map(|_| ())
    }

    pub fn clear_pdo_assignments(&mut self, sync_idx: SmIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_PDOS, &data).map(|_| ())
    }

    pub fn add_pdo_assignment(&mut self, sync_idx: SmIdx, pdo_idx: PdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        data.index = u16::from(pdo_idx);
        ioctl!(self.master, ec::ioctl::SC_ADD_PDO, &data).map(|_| ())
    }

    pub fn clear_pdo_mapping(&mut self, pdo_idx: PdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.index = u16::from(pdo_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_ENTRIES, &data).map(|_| ())
    }

    pub fn add_pdo_mapping(&mut self, pdo_index: PdoIdx, entry: &PdoEntryInfo) -> Result<()> {
        let data = ec::ec_ioctl_add_pdo_entry_t {
            config_index: self.idx,
            pdo_index: u16::from(pdo_index),
            entry_index: u16::from(entry.entry_idx.idx),
            entry_subindex: u8::from(entry.entry_idx.sub_idx),
            entry_bit_length: entry.bit_len,
        };
        ioctl!(self.master, ec::ioctl::SC_ADD_ENTRY, &data).map(|_| ())
    }

    pub fn register_pdo_entry(&mut self, index: PdoEntryIdx, domain: DomainIdx) -> Result<Offset> {
        let mut data = ec::ec_ioctl_reg_pdo_entry_t {
            config_index: self.idx,
            entry_index: u16::from(index.idx),
            entry_subindex: u8::from(index.sub_idx),
            domain_index: u32::try_from(domain)
                .map_err(|_| Error::DomainIdx(usize::from(domain)))?,
            bit_position: 0,
        };
        let byte = ioctl!(self.master, ec::ioctl::SC_REG_PDO_ENTRY, &mut data)?;
        Ok(Offset {
            byte: byte as usize,
            bit: data.bit_position,
        })
    }

    pub fn register_pdo_entry_by_position(
        &mut self,
        sync_index: SmIdx,
        pdo_pos: u32,
        entry_pos: u32,
        domain: DomainIdx,
    ) -> Result<Offset> {
        let mut data = ec::ec_ioctl_reg_pdo_pos_t {
            config_index: self.idx,
            sync_index: u8::from(sync_index) as u32,
            pdo_pos,
            entry_pos,
            domain_index: u32::try_from(domain)
                .map_err(|_| Error::DomainIdx(usize::from(domain)))?,
            bit_position: 0,
        };
        let byte = ioctl!(self.master, ec::ioctl::SC_REG_PDO_POS, &mut data)?;
        Ok(Offset {
            byte: byte as usize,
            bit: data.bit_position,
        })
    }

    pub fn config_dc(
        &mut self,
        assign_activate: u16,
        sync0_cycle_time: u32,
        sync0_shift_time: i32,
        sync1_cycle_time: u32,
        sync1_shift_time: i32,
    ) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        data.dc_assign_activate = assign_activate;
        data.dc_sync[0].cycle_time = sync0_cycle_time;
        data.dc_sync[0].shift_time = sync0_shift_time;
        data.dc_sync[1].cycle_time = sync1_cycle_time;
        data.dc_sync[1].shift_time = sync1_shift_time;
        ioctl!(self.master, ec::ioctl::SC_DC, &data).map(|_| ())
    }

    pub fn add_sdo<T>(&mut self, index: SdoIdx, data: &T) -> Result<()>
    where
        T: SdoData + ?Sized,
    {
        let data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.idx,
            index: u16::from(index.idx),
            subindex: u8::from(index.sub_idx),
            data: data.data_ptr(),
            size: data.data_size() as u64,
            complete_access: 0,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &data).map(|_| ())
    }

    pub fn add_complete_sdo(&mut self, index: SdoIdx, data: &[u8]) -> Result<()> {
        let data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.idx,
            index: u16::from(index.idx),
            subindex: u8::from(index.sub_idx),
            data: data.as_ptr(),
            size: data.len() as u64,
            complete_access: 1,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &data).map(|_| ())
    }

    pub fn config_idn(
        &mut self,
        drive_no: u8,
        idn: u16,
        al_state: AlState,
        data: &[u8],
    ) -> Result<()> {
        let data = ec::ec_ioctl_sc_idn_t {
            config_index: self.idx,
            drive_no,
            idn,
            al_state: al_state as u32,
            data: data.as_ptr(),
            size: data.len() as u64,
        };
        ioctl!(self.master, ec::ioctl::SC_IDN, &data).map(|_| ())
    }

    pub fn set_emerg_size(&mut self, elements: u64) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        data.size = elements;
        ioctl!(self.master, ec::ioctl::SC_EMERG_SIZE, &data).map(|_| ())
    }

    pub fn pop_emerg(&mut self, target: &mut [u8]) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        data.target = target.as_mut_ptr();
        ioctl!(self.master, ec::ioctl::SC_EMERG_POP, &mut data).map(|_| ())
    }

    pub fn clear_emerg(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        ioctl!(self.master, ec::ioctl::SC_EMERG_CLEAR, &data).map(|_| ())
    }

    pub fn emerg_overruns(&mut self) -> Result<i32> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        ioctl!(self.master, ec::ioctl::SC_EMERG_OVERRUNS, &mut data)?;
        Ok(data.overruns)
    }

    // XXX missing: create_sdo_request, create_reg_request, create_voe_handler
}

impl<'m> Domain<'m> {
    pub const fn new(idx: DomainIdx, master: &'m Master) -> Self {
        Self { idx, master }
    }

    pub fn size(&self) -> Result<usize> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_SIZE,
            c_ulong::try_from(self.idx).map_err(|_| Error::DomainIdx(usize::from(self.idx)))?
        )
        .map(|v| v as usize)
    }

    pub fn state(&self) -> Result<DomainState> {
        let mut state = ec::ec_domain_state_t::default();
        let mut data = ec::ec_ioctl_domain_state_t {
            domain_index: u32::try_from(self.idx)
                .map_err(|_| Error::DomainIdx(usize::from(self.idx)))?,
            state: &mut state,
        };
        ioctl!(self.master, ec::ioctl::DOMAIN_STATE, &mut data)?;
        Ok(DomainState {
            working_counter: state.working_counter,
            redundancy_active: state.redundancy_active != 0,
            wc_state: WcState::from(state.wc_state),
        })
    }

    pub fn process(&mut self) -> Result<()> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_PROCESS,
            c_ulong::from(usize::from(self.idx) as u64)
        )
        .map(|_| ())
    }

    pub fn queue(&mut self) -> Result<()> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_QUEUE,
            c_ulong::try_from(self.idx).map_err(|_| Error::DomainIdx(usize::from(self.idx)))?
        )
        .map(|_| ())
    }
}
