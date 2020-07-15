// Part of ethercat-rs. Copyright 2018-2020 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use crate::{ec, types::*, Result};
use std::{
    collections::HashMap,
    convert::TryFrom,
    ffi::CStr,
    fs::{File, OpenOptions},
    io::{Error, ErrorKind},
    os::{raw::c_ulong, unix::io::AsRawFd},
};

macro_rules! ioctl {
    ($m:expr, $f:expr) => { ioctl!($m, $f,) };
    ($m:expr, $f:expr, $($arg:tt)*) => {{
        let res = unsafe { $f($m.file.as_raw_fd(), $($arg)*) };
        if res < 0 { Err(Error::last_os_error()) } else { Ok(res) }
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
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "module version mismatch: expected {}, found {}",
                    ec::EC_IOCTL_VERSION_MAGIC,
                    module_info.ioctl_version_magic
                ),
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
        ioctl!(self, ec::ioctl::REQUEST)?;
        Ok(())
    }

    pub fn create_domain(&self) -> Result<DomainIdx> {
        Ok((ioctl!(self, ec::ioctl::CREATE_DOMAIN)? as usize).into())
    }

    pub const fn domain(&self, idx: DomainIdx) -> Domain {
        Domain::new(idx, self)
    }

    pub fn domain_data(&mut self, idx: DomainIdx) -> &mut [u8] {
        let p = self
            .domain_data_placement(idx)
            .expect("Domain is not available");
        &mut self.map.as_mut().expect("Master is not activated")[p.offset..p.offset + p.size]
    }

    fn domain_data_placement(&mut self, idx: DomainIdx) -> Result<DomainDataPlacement> {
        Ok(match self.domains.get(&idx) {
            None => {
                let offset = ioctl!(
                    self,
                    ec::ioctl::DOMAIN_OFFSET,
                    c_ulong::try_from(idx).map_err(|e| Error::new(ErrorKind::Other, e))?
                )? as usize;
                let size = ioctl!(
                    self,
                    ec::ioctl::DOMAIN_SIZE,
                    c_ulong::try_from(idx).map_err(|e| Error::new(ErrorKind::Other, e))?
                )? as usize;
                let meta_data = DomainDataPlacement { offset, size };
                self.domains.insert(idx, meta_data);
                meta_data
            }
            Some(d) => *d,
        })
    }

    pub fn activate(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_master_activate_t::default();
        ioctl!(self, ec::ioctl::ACTIVATE, &mut data)?;

        self.map = unsafe {
            memmap::MmapOptions::new()
                .len(data.process_data_size as usize)
                .map_mut(&self.file)
                .map(Some)?
        };
        self.map.as_mut().unwrap()[0] = 0;
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<()> {
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
        let first_device = devices
            .get(0)
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No devices available"))?;
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
            al_state: AlState::from(data.al_state as u32),
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

    pub fn sdo_download<T>(&mut self, position: SlavePos, sdo_idx: SdoIdx, data: &T) -> Result<()>
    where
        T: SdoData + ?Sized,
    {
        let mut data = ec::ec_ioctl_slave_sdo_download_t {
            slave_position: u16::from(position),
            sdo_index: u16::from(sdo_idx.idx),
            sdo_entry_subindex: u8::from(sdo_idx.sub_idx),
            complete_access: 0,
            data_size: data.data_size() as u64,
            data: data.data_ptr(),
            abort_code: 0,
        };
        ioctl!(self, ec::ioctl::SLAVE_SDO_DOWNLOAD, &mut data).map(|_| ())
    }

    pub fn sdo_download_complete(
        &mut self,
        position: SlavePos,
        sdo_idx: SdoIdx,
        data: &[u8],
    ) -> Result<()> {
        let mut data = ec::ec_ioctl_slave_sdo_download_t {
            slave_position: u16::from(position),
            sdo_index: u16::from(sdo_idx.idx),
            sdo_entry_subindex: u8::from(sdo_idx.sub_idx),
            complete_access: 1,
            data_size: data.len() as u64,
            data: data.as_ptr(),
            abort_code: 0,
        };
        ioctl!(self, ec::ioctl::SLAVE_SDO_DOWNLOAD, &mut data).map(|_| ())
    }

    pub fn sdo_upload<'t>(
        &self,
        position: SlavePos,
        sdo_idx: SdoIdx,
        target: &'t mut [u8],
    ) -> Result<&'t mut [u8]> {
        let mut data = ec::ec_ioctl_slave_sdo_upload_t {
            slave_position: u16::from(position),
            sdo_index: u16::from(sdo_idx.idx),
            sdo_entry_subindex: u8::from(sdo_idx.sub_idx),
            target_size: target.len() as u64,
            target: target.as_mut_ptr(),
            data_size: 0,
            abort_code: 0,
        };
        ioctl!(self, ec::ioctl::SLAVE_SDO_UPLOAD, &mut data)?;
        Ok(&mut target[..data.data_size as usize])
    }

    // XXX missing: get_sync_manager, get_pdo, get_pdo_entry, write_idn, read_idn,
    // application_time, sync_reference_clock, sync_slave_clocks,
    // reference_clock_time, sync_monitor_queue, sync_monitor_process
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
        Ok(SlaveConfigState {
            online: state.online() != 0,
            operational: state.operational() != 0,
            al_state: AlState::from(state.al_state()),
        })
    }

    pub fn config_pdos(&mut self, info: &[SyncInfo]) -> Result<()> {
        for sm_info in info {
            self.config_sync_manager(sm_info)?;

            self.clear_pdo_assignments(sm_info.idx)?;
            for pdo_info in sm_info.pdos {
                self.add_pdo_assignment(sm_info.idx, pdo_info)?;

                if !pdo_info.entries.is_empty() {
                    self.clear_pdo_mapping(pdo_info.idx)?;
                    for entry in pdo_info.entries {
                        self.add_pdo_mapping(pdo_info.idx, *entry)?;
                    }
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

    pub fn config_overlapping_pdos(&mut self, allow: bool) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        data.allow_overlapping_pdos = allow as u8;
        ioctl!(self.master, ec::ioctl::SC_OVERLAPPING_IO, &data).map(|_| ())
    }

    pub fn config_sync_manager(&mut self, info: &SyncInfo) -> Result<()> {
        if u8::from(info.idx) >= ec::EC_MAX_SYNC_MANAGERS as u8 {
            return Err(Error::new(ErrorKind::Other, "sync manager index too large"));
        }
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.idx;
        let ix = u8::from(info.idx) as usize;
        data.syncs[ix].dir = info.direction as u32;
        data.syncs[ix].watchdog_mode = info.watchdog_mode as u32;
        data.syncs[ix].config_this = 1;
        ioctl!(self.master, ec::ioctl::SC_SYNC, &data).map(|_| ())
    }

    pub fn clear_pdo_assignments(&mut self, sync_idx: SmIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_PDOS, &data).map(|_| ())
    }

    pub fn add_pdo_assignment(&mut self, sync_idx: SmIdx, pdo: &PdoInfo) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        data.index = u16::from(pdo.idx);
        ioctl!(self.master, ec::ioctl::SC_ADD_PDO, &data).map(|_| ())
    }

    pub fn clear_pdo_mapping(&mut self, pdo_idx: PdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.index = u16::from(pdo_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_ENTRIES, &data).map(|_| ())
    }

    pub fn add_pdo_mapping(&mut self, pdo_index: PdoIdx, entry: PdoEntryInfo) -> Result<()> {
        let data = ec::ec_ioctl_add_pdo_entry_t {
            config_index: self.idx,
            pdo_index: u16::from(pdo_index),
            entry_index: u16::from(entry.idx.idx),
            entry_subindex: u8::from(entry.idx.sub_idx),
            entry_bit_length: entry.bit_length,
        };
        ioctl!(self.master, ec::ioctl::SC_ADD_ENTRY, &data).map(|_| ())
    }

    pub fn register_pdo_entry(&mut self, index: PdoEntryIdx, domain: DomainIdx) -> Result<Offset> {
        let mut data = ec::ec_ioctl_reg_pdo_entry_t {
            config_index: self.idx,
            entry_index: u16::from(index.idx),
            entry_subindex: u8::from(index.sub_idx),
            domain_index: u32::try_from(domain).map_err(|e| Error::new(ErrorKind::Other, e))?,
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
            domain_index: u32::try_from(domain).map_err(|e| Error::new(ErrorKind::Other, e))?,
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
            c_ulong::try_from(self.idx).map_err(|e| Error::new(ErrorKind::Other, e))?
        )
        .map(|v| v as usize)
    }

    pub fn state(&self) -> Result<DomainState> {
        let mut state = ec::ec_domain_state_t::default();
        let mut data = ec::ec_ioctl_domain_state_t {
            domain_index: u32::try_from(self.idx).map_err(|e| Error::new(ErrorKind::Other, e))?,
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
            c_ulong::try_from(self.idx).map_err(|e| Error::new(ErrorKind::Other, e))?
        )
        .map(|_| ())
    }
}
