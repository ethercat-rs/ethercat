use std::fs::{File, OpenOptions};
use std::ffi::CStr;
use std::io::{Error, ErrorKind};
use std::os::unix::io::AsRawFd;
use crate::ec;
use crate::Result;
use crate::types::*;

macro_rules! ioctl {
    ($m:expr, $f:expr) => { ioctl!($m, $f,) };
    ($m:expr, $f:expr, $($arg:tt)*) => {{
        let res = unsafe { $f($m.file.as_raw_fd(), $($arg)*) };
        if res < 0 { Err(Error::last_os_error()) } else { Ok(res) }
    }}
}

pub type MasterIndex = u32;
pub type DomainIndex = u32;
pub type SlaveConfigIndex = u32;

/// An EtherCAT master.
pub struct Master {
    file: File,
    map: Option<memmap::MmapMut>,
    domains: Vec<(DomainIndex, usize, usize)>,
}

pub struct Domain<'m> {
    master: &'m Master,
    index: DomainIndex,
}

#[derive(Clone, Copy)]
pub struct DomainHandle(usize);

impl Master {
    pub fn reserve(index: MasterIndex) -> Result<Self> {
        let devpath = format!("/dev/EtherCAT{}", index);
        let file = OpenOptions::new().read(true).write(true).open(&devpath)?;
        let mut module_info = ec::ec_ioctl_module_t {
            ioctl_version_magic: 0,
            master_count: 0,
        };
        let master = Master { file, map: None, domains: vec![] };
        ioctl!(master, ec::ioctl::MODULE, &mut module_info)?;
        if module_info.ioctl_version_magic != ec::EC_IOCTL_VERSION_MAGIC {
            Err(Error::new(ErrorKind::Other,
                           format!("module version mismatch: expected {}, found {}",
                                   ec::EC_IOCTL_VERSION_MAGIC,
                                   module_info.ioctl_version_magic)))
        } else {
            ioctl!(master, ec::ioctl::REQUEST)?;
            Ok(master)
        }
    }

    pub fn create_domain(&mut self) -> Result<DomainHandle> {
        let index = ioctl!(self, ec::ioctl::CREATE_DOMAIN).map(|v| v as DomainIndex)?;
        self.domains.push((index, 0, 0));
        Ok(DomainHandle(self.domains.len() - 1))
    }

    pub fn domain(&self, index: DomainHandle) -> Domain {
        Domain { master: self, index: self.domains[index.0].0 }
    }

    pub fn domain_data(&mut self, index: DomainHandle) -> &mut [u8] {
        let (ix, mut offset, mut size) = self.domains[index.0];
        if size == 0 {
            size = ioctl!(self, ec::ioctl::DOMAIN_SIZE, ix as u64).unwrap() as usize;
            offset = ioctl!(self, ec::ioctl::DOMAIN_OFFSET, ix as u64).unwrap() as usize;
            self.domains[index.0] = (ix, offset, size);
        }
        &mut self.map.as_mut().expect("master is not activated")[offset..offset+size]
    }

    pub fn activate(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_master_activate_t::default();
        ioctl!(self, ec::ioctl::ACTIVATE, &mut data)?;

        self.map = unsafe {
            memmap::MmapOptions::new()
                .len(data.process_data_size)
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

    pub fn set_send_interval(&mut self, mut interval_us: usize) -> Result<()> {
        ioctl!(self, ec::ioctl::SET_SEND_INTERVAL, &mut interval_us).map(|_| ())
    }

    pub fn send(&mut self) -> Result<usize> {
        let mut sent = 0;
        ioctl!(self, ec::ioctl::SEND, &mut sent as *mut _ as u64)?;
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
        Ok(MasterInfo {
            slave_count: data.slave_count,
            link_up: data.devices[0].link_state != 0,
            scan_busy: data.scan_busy != 0,
            app_time: data.app_time,
        })
    }

    pub fn get_slave_info(&self, position: u16) -> Result<SlaveInfo> {
        let mut data = ec::ec_ioctl_slave_t::default();
        data.position = position;
        ioctl!(self, ec::ioctl::SLAVE, &mut data)?;
        let mut ports = [SlavePortInfo::default(); ec::EC_MAX_PORTS as usize];
        for i in 0..ec::EC_MAX_PORTS as usize {
            ports[i].desc = match data.ports[i].desc {
                ec::EC_PORT_NOT_IMPLEMENTED => SlavePortType::NotImplemented,
                ec::EC_PORT_NOT_CONFIGURED => SlavePortType::NotConfigured,
                ec::EC_PORT_EBUS => SlavePortType::EBus,
                ec::EC_PORT_MII => SlavePortType::MII,
                x => panic!("invalid port type {}", x),
            };
            ports[i].link = SlavePortLink {
                link_up: data.ports[i].link.link_up != 0,
                loop_closed: data.ports[i].link.loop_closed != 0,
                signal_detected: data.ports[i].link.signal_detected != 0,
            };
            ports[i].receive_time = data.ports[i].receive_time;
            ports[i].next_slave = data.ports[i].next_slave;
            ports[i].delay_to_next_dc = data.ports[i].delay_to_next_dc;
        }
        Ok(SlaveInfo {
            name: unsafe { CStr::from_ptr(data.name.as_ptr()).to_string_lossy().into_owned() },
            ring_pos: data.position,
            id: SlaveId { vendor_id: data.vendor_id, product_code: data.product_code },
            rev: SlaveRev { revision_number: data.revision_number,
                            serial_number: data.serial_number },
            alias: data.alias,
            current_on_ebus: data.current_on_ebus,
            al_state: AlState::from(data.al_state as u32),
            error_flag: data.error_flag,
            sync_count: data.sync_count,
            sdo_count: data.sdo_count,
            ports
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
        Ok(SlaveConfig { master: self, index: data.config_index })
    }

    // XXX missing: get_sync_manager, get_pdo, get_pdo_entry,
    // sdo_download, sdo_download_complete, sdo_upload, write_idn, read_idn,
    // application_time, sync_reference_clock, sync_slave_clocks,
    // reference_clock_time, sync_monitor_queue, sync_monitor_process
}

pub struct SlaveConfig<'m> {
    master: &'m Master,
    index: SlaveConfigIndex,
}

impl<'m> SlaveConfig<'m> {
    pub fn get_state(&self) -> Result<SlaveConfigState> {
        let mut state = ec::ec_slave_config_state_t::default();
        let mut data = ec::ec_ioctl_sc_state_t { config_index: self.index, state: &mut state };
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

            self.clear_pdo_assignments(sm_info.index)?;
            for pdo_info in sm_info.pdos {
                self.add_pdo_assignment(sm_info.index, pdo_info)?;

                if !pdo_info.entries.is_empty() {
                    self.clear_pdo_mapping(pdo_info.index)?;
                    for entry in pdo_info.entries {
                        self.add_pdo_mapping(pdo_info.index, entry)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn config_watchdog(&mut self, divider: u16, intervals: u16) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.index;
        data.watchdog_divider = divider;
        data.watchdog_intervals = intervals;
        ioctl!(self.master, ec::ioctl::SC_WATCHDOG, &mut data).map(|_| ())
    }

    pub fn config_overlapping_pdos(&mut self, allow: bool) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.index;
        data.allow_overlapping_pdos = allow as u8;
        ioctl!(self.master, ec::ioctl::SC_OVERLAPPING_IO, &mut data).map(|_| ())
    }

    pub fn config_sync_manager(&mut self, info: &SyncInfo) -> Result<()> {
        if info.index >= ec::EC_MAX_SYNC_MANAGERS as u8 {
            return Err(Error::new(ErrorKind::Other, "sync manager index too large"));
        }

        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.index;
        let ix = info.index as usize;
        data.syncs[ix].dir = info.direction as u32;
        data.syncs[ix].watchdog_mode = info.watchdog_mode as u32;
        data.syncs[ix].config_this = 1;
        ioctl!(self.master, ec::ioctl::SC_SYNC, &mut data).map(|_| ())
    }

    pub fn clear_pdo_assignments(&mut self, sync_index: SmIndex) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.index;
        data.sync_index = sync_index;
        ioctl!(self.master, ec::ioctl::SC_CLEAR_PDOS, &mut data).map(|_| ())
    }

    pub fn add_pdo_assignment(&mut self, sync_index: SmIndex, pdo: &PdoInfo) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.index;
        data.sync_index = sync_index;
        data.index = pdo.index;
        ioctl!(self.master, ec::ioctl::SC_ADD_PDO, &mut data).map(|_| ())
    }

    pub fn clear_pdo_mapping(&mut self, pdo_index: PdoIndex) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.index;
        data.index = pdo_index;
        ioctl!(self.master, ec::ioctl::SC_CLEAR_ENTRIES, &mut data).map(|_| ())
    }

    pub fn add_pdo_mapping(&mut self, pdo_index: PdoIndex, entry: &PdoEntryInfo) -> Result<()> {
        let mut data = ec::ec_ioctl_add_pdo_entry_t {
            config_index: self.index,
            pdo_index,
            entry_index: entry.index.index,
            entry_subindex: entry.index.subindex,
            entry_bit_length: entry.bit_length,
        };
        ioctl!(self.master, ec::ioctl::SC_ADD_ENTRY, &mut data).map(|_| ())
    }

    pub fn register_pdo_entry(&mut self, index: PdoEntryIndex, domain: DomainHandle) -> Result<Position> {
        let mut data = ec::ec_ioctl_reg_pdo_entry_t {
            config_index: self.index,
            entry_index: index.index,
            entry_subindex: index.subindex,
            domain_index: self.master.domains[domain.0].0,
            bit_position: 0,
        };
        let byte = ioctl!(self.master, ec::ioctl::SC_REG_PDO_ENTRY, &mut data)?;
        Ok(Position { byte: byte as usize, bit: data.bit_position })
    }

    pub fn register_pdo_entry_by_position(&mut self, sync_index: SmIndex, pdo_pos: u32, entry_pos: u32,
                                          domain: DomainHandle) -> Result<Position> {
        let mut data = ec::ec_ioctl_reg_pdo_pos_t {
            config_index: self.index,
            sync_index: sync_index as u32,
            pdo_pos,
            entry_pos,
            domain_index: self.master.domains[domain.0].0,
            bit_position: 0,
        };
        let byte = ioctl!(self.master, ec::ioctl::SC_REG_PDO_POS, &mut data)?;
        Ok(Position { byte: byte as usize, bit: data.bit_position })
    }

    pub fn config_dc(&mut self, assign_activate: u16, sync0_cycle_time: u32, sync0_shift_time: i32,
                     sync1_cycle_time: u32, sync1_shift_time: i32) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        data.config_index = self.index;
        data.dc_assign_activate = assign_activate;
        data.dc_sync[0].cycle_time = sync0_cycle_time;
        data.dc_sync[0].shift_time = sync0_shift_time;
        data.dc_sync[1].cycle_time = sync1_cycle_time;
        data.dc_sync[1].shift_time = sync1_shift_time;
        ioctl!(self.master, ec::ioctl::SC_DC, &mut data).map(|_| ())
    }

    pub fn add_sdo<T: SdoData>(&mut self, index: SdoIndex, data: T) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.index,
            index: index.index,
            subindex: index.subindex,
            data: &data as *const _ as *const u8,
            size: std::mem::size_of::<T>(),
            complete_access: 0,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &mut data).map(|_| ())
    }

    pub fn add_complete_sdo(&mut self, index: SdoIndex, data: &[u8]) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.index,
            index: index.index,
            subindex: index.subindex,
            data: data.as_ptr(),
            size: data.len(),
            complete_access: 1,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &mut data).map(|_| ())
    }

    pub fn config_idn(&mut self, drive_no: u8, idn: u16, al_state: AlState, data: &[u8]) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_idn_t {
            config_index: self.index,
            drive_no,
            idn,
            al_state: al_state as u32,
            data: data.as_ptr(),
            size: data.len(),
        };
        ioctl!(self.master, ec::ioctl::SC_IDN, &mut data).map(|_| ())
    }

    pub fn set_emerg_size(&mut self, elements: usize) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.index;
        data.size = elements;
        ioctl!(self.master, ec::ioctl::SC_EMERG_SIZE, &mut data).map(|_| ())
    }

    pub fn pop_emerg(&mut self, target: &mut [u8]) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.index;
        data.target = target.as_mut_ptr();
        ioctl!(self.master, ec::ioctl::SC_EMERG_POP, &mut data).map(|_| ())
    }

    pub fn clear_emerg(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.index;
        ioctl!(self.master, ec::ioctl::SC_EMERG_CLEAR, &mut data).map(|_| ())
    }

    pub fn emerg_overruns(&mut self) -> Result<i32> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.index;
        ioctl!(self.master, ec::ioctl::SC_EMERG_OVERRUNS, &mut data)?;
        Ok(data.overruns)
    }

    // XXX missing: create_sdo_request, create_reg_request, create_voe_handler
}

impl<'m> Domain<'m> {
    pub fn size(&self) -> Result<usize> {
        ioctl!(self.master, ec::ioctl::DOMAIN_SIZE, self.index as u64).map(|v| v as usize)
    }

    pub fn state(&self) -> Result<DomainState> {
        let mut state = ec::ec_domain_state_t::default();
        let mut data = ec::ec_ioctl_domain_state_t {
            domain_index: self.index,
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
        ioctl!(self.master, ec::ioctl::DOMAIN_PROCESS, self.index as u64).map(|_| ())
    }

    pub fn queue(&mut self) -> Result<()> {
        ioctl!(self.master, ec::ioctl::DOMAIN_QUEUE, self.index as u64).map(|_| ())
    }
}
