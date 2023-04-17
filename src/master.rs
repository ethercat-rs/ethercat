// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

#![allow(clippy::field_reassign_with_default)]

use crate::{convert, ec, types::*};
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
	/**
		Opens an EtherCAT master for userspace access.
		
		This function has to be the first function an application has to call to use EtherCAT. The function takes the index of the master as its argument.
		The first master has index 0, the n-th master has index n - 1. The number of masters has to be specified when loading the master module.
	*/
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

    /**
		Reserves an EtherCAT master for realtime operation.
		
		Before an application can use PDO/domain registration functions or SDO request functions on the master, it has to reserve one for exclusive use.
    */
    pub fn reserve(&self) -> Result<()> {
        log::debug!("Reserve EtherCAT Master");
        ioctl!(self, ec::ioctl::REQUEST)?;
        Ok(())
    }

    /**
		Creates a new process data domain.

		For process data exchange, at least one process data domain is needed. This method creates a new process data domain and returns a pointer to the new domain object. This object can be used for registering PDOs and exchanging them in cyclic operation.

		This method allocates memory and should be called in non-realtime context before [Self::activate].
    */
    pub fn create_domain(&self) -> Result<DomainIdx> {
        Ok((ioctl!(self, ec::ioctl::CREATE_DOMAIN)? as usize).into())
    }

    /**
		Return a helper to configure a domain
	*/
    pub const fn domain(&self, idx: DomainIdx) -> Domain {
        Domain::new(idx, self)
    }

    /**
		Returns the domain's process data.

		- In kernel context: If external memory was provided with `ecrt_domain_external_memory()`, the returned pointer will contain the address of that memory. Otherwise it will point to the internally allocated memory. In the latter case, this method may not be called before [Self::activate].
		- In userspace context: This method has to be called after [Self::activate] to get the mapped domain process data memory.
    */
    pub fn domain_data(&mut self, idx: DomainIdx) -> Result<&mut [u8]> {
        let p = self
            .domain_data_placement(idx)
            .map_err(|_| Error::NoDomain)?;
        let data = self.map.as_mut().ok_or_else(|| Error::NotActivated)?;
        Ok(&mut data[p.offset..p.offset + p.size])
    }

    /**
		Returns the current size and offset of the domain's process data. 
    */
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

    /**
		Finishes the configuration phase and prepares for cyclic operation.

		This function tells the master that the configuration phase is finished and the realtime operation will begin. The function allocates internal memory for the domains and calculates the logical FMMU addresses for domain members. It tells the master state machine that the bus configuration is now to be applied.

		- Attention
		
			After this function has been called, the realtime application is in charge of cyclically calling [Self::send] and [Self::receive] to ensure bus communication. Before calling this function, the master thread is responsible for that, so these functions may not be called! The method itself allocates memory and should not be called in realtime context.
    */
    pub fn activate(&mut self) -> Result<()> {
        log::debug!("Activate EtherCAT Master");
        let mut data = ec::ec_ioctl_master_activate_t::default();
        ioctl!(self, ec::ioctl::ACTIVATE, &mut data)?;

        self.map = unsafe {
            memmap::MmapOptions::new()
                .len(data.process_data_size)
                .map_mut(&self.file)
                .map(Some)?
        };
        self.map.as_mut().ok_or_else(|| Error::NotActivated)?[0] = 0;
        Ok(())
    }

    /**
		Deactivates the master.

		Removes the bus configuration. All objects created by [Self::create_domain], [Self::configure_slave], [Self::domain_data], [SlaveConfig::create_sdo_request] and [SlaveConfig::create_voe_handler] are freed, so pointers to them become invalid.

		This method should not be called in realtime context. 
	*/
    pub fn deactivate(&mut self) -> Result<()> {
        log::debug!("Deactivate EtherCAT Master");
        ioctl!(self, ec::ioctl::DEACTIVATE)?;
        self.domains.clear();
        self.map = None;
        Ok(())
    }

    /**
		Set interval between calls to ecrt_master_send().

		This information helps the master to decide, how much data can be appended to a frame by the master state machine. When the master is configured with –enable-hrtimers, this is used to calculate the scheduling of the master thread.
    */
    pub fn set_send_interval(&mut self, interval_us: usize) -> Result<()> {
        ioctl!(self, ec::ioctl::SET_SEND_INTERVAL, &interval_us).map(|_| ())
    }

    /**
		Sends all datagrams in the queue.

		This method takes all datagrams, that have been queued for transmission, puts them into frames, and passes them to the Ethernet device for sending.

		Has to be called cyclically by the application after [Self::activate] has returned.
    */
    pub fn send(&mut self) -> Result<usize> {
        let mut sent = 0;
        ioctl!(self, ec::ioctl::SEND, &mut sent as *mut _ as c_ulong)?;
        Ok(sent)
    }

    /**
		Fetches received frames from the hardware and processes the datagrams.

		Queries the network device for received frames by calling the interrupt service routine. Extracts received datagrams and dispatches the results to the datagram objects in the queue. Received datagrams, and the ones that timed out, will be marked, and dequeued.

		Has to be called cyclically by the realtime application after [Self::activate] has returned. 
    */
    pub fn receive(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::RECEIVE).map(|_| ())
    }

    /**
		Retry configuring slaves.

		Via this method, the application can tell the master to bring all slaves to OP state. In general, this is not necessary, because it is automatically done by the master. But with special slaves, that can be reconfigured by the vendor during runtime, it can be useful. 
    */
    pub fn reset(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::RESET).map(|_| ())
    }

    /**
		Reads the current master state.

		Stores the master state information in the given state structure.

		This method returns a global state. For the link-specific states in a redundant bus topology, use the [Self::link_state] method. 
    */
    pub fn state(&self) -> Result<MasterState> {
        let mut data = ec::ec_master_state_t::default();
        ioctl!(self, ec::ioctl::MASTER_STATE, &mut data)?;
        Ok(MasterState {
            slaves_responding: data.slaves_responding,
            al_states: data.al_states() as u8,
            link_up: data.link_up() != 0,
        })
    }

    /**
		Reads the current state of a redundant link.

		Stores the link state information in the given state structure.

		## Parameters
		
		- `dev_idx` -	Index of the device (0 = main device, 1 = first backup device, ...). 
    */
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

    /**
		Obtains master information. 
    */
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

    /**
		Obtains slave information.

		Tries to find the slave with the given ring position. The obtained information is stored in a structure. No memory is allocated on the heap in this function.
    */
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

    /**
		Obtains a slave configuration.

		Creates a slave configuration object for the given alias and position tuple and returns it. If a configuration with the same alias and position already exists, it will be re-used. In the latter case, the given vendor ID and product code are compared to the stored ones. On mismatch, an error message is raised and the function returns NULL.

		Slaves are addressed with the alias and position parameters.

		- If alias is zero, position is interpreted as the desired slave's ring position.
		- If alias is non-zero, it matches a slave with the given alias. In this case, position is interpreted as ring offset, starting from the aliased slave, so a position of zero means the aliased slave itself and a positive value matches the n-th slave behind the aliased one.

		If the slave with the given address is found during the bus configuration, its vendor ID and product code are matched against the given value. On mismatch, the slave is not configured and an error message is raised.

		If different slave configurations are pointing to the same slave during bus configuration, a warning is raised and only the first configuration is applied.

		This method allocates memory and should be called in non-realtime context before ecrt_master_activate().
    */
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

    /**
		Create a helper and start configuring the slave's PDOs
    */
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

    /** retreive informations about a given SDO */
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
                name: convert::c_array_to_string(sdo.name.as_ptr()),
            })
        }
    }

    /** retreive informations about a given SDO's entry */
    pub fn get_sdo_entry(
        &mut self,
        slave_pos: SlavePos,
        addr: SdoEntryAddr,
    ) -> Result<SdoEntryInfo> {
        let mut entry = ec::ec_ioctl_slave_sdo_entry_t::default();
        entry.slave_position = u16::from(slave_pos);
        let (spec, sub) = match addr {
            SdoEntryAddr::ByPos(pos, sub) => (-(u16::from(pos) as i32), sub),
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
            description: convert::c_array_to_string(entry.description.as_ptr()),
        })
    }

    /**
		Executes an SDO download request to write data to a slave.

		This request is processed by the master state machine. This method blocks, until the request has been processed and may not be called in realtime context.
    */
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
            data_size: data.data_size(),
            data: data_ptr,
            abort_code: 0,
        };
        ioctl!(self, ec::ioctl::SLAVE_SDO_DOWNLOAD, &mut data).map(|_| ())
    }

    /**
		Executes an SDO upload request to read data from a slave.

		This request is processed by the master state machine. This method blocks, until the request has been processed and may not be called in realtime context.
    */
    pub fn sdo_upload<'t>(
        &self,
        position: SlavePos,
        sdo_idx: SdoIdx,
        #[allow(unused_variables)] complete_access: bool,
        target: &'t mut [u8],
    ) -> Result<&'t mut [u8]> {
        let slave_position = u16::from(position);
        let sdo_index = u16::from(sdo_idx.idx);
        let sdo_entry_subindex = u8::from(sdo_idx.sub_idx);
        let target_size = target.len();
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
        Ok(&mut target[..data.data_size])
    }

    /**
		Returns information about a currently assigned PDO.
		
		Use [Self::get_pdo_entry] to get the PDO entry information.
		
		## Parameters
		
		- `slave_pos` - the slave position
		- `sync_index` - the sync manager index, must be less than `EC_MAX_SYNC_MANAGERS`
		- `pdo_position` - zero-based PDO position
    */
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
            name: convert::c_array_to_string(pdo.name.as_ptr()),
        })
    }

    /** 
		Returns information about a currently mapped PDO entry
		
		## Parameters
		
		- `slave_pos` - the slave position
		- `sync_index` - the sync manager index, must be less than `EC_MAX_SYNC_MANAGERS`
		- `pdo_position` - Zero-based PDO position
		- `entry_pos` - Zero-based PDO entry position
	*/
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
            name: convert::c_array_to_string(entry.name.as_ptr()),
        })
    }

    /**
		Returns the proposed configuration of a slave's sync manager.
		
		Fills a given ec_sync_info_t structure with the attributes of a sync manager. The \a pdos field of the return value is left empty. Use [Self::get_pdo] to get the PDO information.
    */
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

    /**
		Request a slave to switch to a state of communication and resets the error flag.
    */
    pub fn request_state(&mut self, slave_pos: SlavePos, state: AlState) -> Result<()> {
        let mut data = ec::ec_ioctl_slave_state_t::default();
        data.slave_position = u16::from(slave_pos);
        data.al_state = state as u8;
        ioctl!(self, ec::ioctl::SLAVE_STATE, &data)?;
        Ok(())
    }

    #[cfg(feature = "sncn")]
    pub fn dict_upload(&mut self, slave_pos: SlavePos) -> Result<()> {
        let mut data = ec::ec_ioctl_slave_dict_upload_t::default();
        data.slave_position = u16::from(slave_pos);
        ioctl!(self, ec::ioctl::SLAVE_DICT_UPLOAD, &mut data)?;
        Ok(())
    }

    /**
		Sets the application time.

		The master has to know the application's time when operating slaves with distributed clocks. The time is not incremented by the master itself, so this method has to be called cyclically.

		- Attention
		
			The time passed to this method is used to calculate the phase of the slaves' SYNC0/1 interrupts. It should be called constantly at the same point of the realtime cycle. So it is recommended to call it at the start of the calculations to avoid deviancies due to changing execution times.

		The time is used when setting the slaves' System Time Offset and Cyclic Operation Start Time registers and when synchronizing the DC reference clock to the application time via [Self::sync_reference_clock].

		The time is defined as nanoseconds from 2000-01-01 00:00.
    */
    pub fn set_application_time(&mut self, app_time: u64) -> Result<()> {
        ioctl!(self, ec::ioctl::APP_TIME, &app_time)?;
        Ok(())
    }

    /**
		Queues the DC reference clock drift compensation datagram for sending.

		The reference clock will by synchronized to the application time provided by the last call off [Self::application_time].
    */
    pub fn sync_reference_clock(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::SYNC_REF)?;
        Ok(())
    }

    /**
		Queues the DC clock drift compensation datagram for sending.

		All slave clocks synchronized to the reference clock. 
    */
    pub fn sync_slave_clocks(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::SYNC_SLAVES)?;
        Ok(())
    }

    /**
		Queues the DC synchrony monitoring datagram for sending.

		The datagram broadcast-reads all "System time difference" registers (0x092c) to get an upper estimation of the DC synchrony. The result can be checked with the ecrt_master_sync_monitor_process() method.
	*/
    pub fn sync_monitor_queue(&mut self) -> Result<()> {
        ioctl!(self, ec::ioctl::SYNC_MON_QUEUE)?;
        Ok(())
    }

    /**
		Processes the DC synchrony monitoring datagram.

		If the sync monitoring datagram was sent before with ecrt_master_sync_monitor_queue(), the result can be queried with this method.

		## Returns
		
		Upper estimation of the maximum time difference in ns. 
    */
    pub fn sync_monitor_process(&mut self) -> Result<u32> {
        let mut time = 0;
        ioctl!(self, ec::ioctl::SYNC_MON_PROCESS, &mut time)?;
        Ok(time)
    }

    /**
		Get the lower 32 bit of the reference clock system time.

		This method can be used to synchronize the master to the reference clock.

		The reference clock system time is queried via the ecrt_master_sync_slave_clocks() method, that reads the system time of the reference clock and writes it to the slave clocks (so be sure to call it cyclically to get valid data).

		## Attention
			
		The returned time is the system time of the reference clock minus the transmission delay of the reference clock.
    */
    pub fn get_reference_clock_time(&mut self) -> Result<u32> {
        let mut time = 0;
        ioctl!(self, ec::ioctl::REF_CLOCK_TIME, &mut time)?;
        Ok(time)
    }

    pub fn foe_read(&mut self, idx: SlavePos, name: &str) -> Result<Vec<u8>> {
        let file_name = convert::string_to_foe_name(name)?;
        // FIXME: this is the same as in the c-implementation. Should read in chunks instead of a
        // fixed size buffer. The ioctl-call in the master pre-allocates a 10000 byte buffer, so we
        // do the same here.
        const FOE_SIZE: usize = 10_000;
        let mut buf: Vec<u8> = vec![0; FOE_SIZE];
        let mut data = ec::ec_ioctl_slave_foe_t {
            slave_position: idx.into(),
            offset: 0,
            buffer_size: FOE_SIZE,
            buffer: buf.as_mut_ptr(),
            file_name,
            ..Default::default()
        };
        ioctl!(self, ec::ioctl::SLAVE_FOE_READ, &mut data)?;

        assert!(data.data_size <= FOE_SIZE);
        buf.truncate(data.data_size);
        Ok(buf)
    }

    pub fn foe_write(&mut self, idx: SlavePos, name: &str, data: &[u8]) -> Result<()> {
        let file_name = convert::string_to_foe_name(name)?;

        let buffer = data.as_ptr() as *mut _;
        let data = ec::ec_ioctl_slave_foe_t {
            slave_position: idx.into(),
            offset: 0,
            buffer_size: data.len(),
            buffer,
            file_name,
            ..Default::default()
        };
        ioctl!(self, ec::ioctl::SLAVE_FOE_WRITE, &data)?;

        Ok(())
    }

    // XXX missing: write_idn, read_idn
}

/**
	Helper to configure a slave's PDO's before master activation.
	
	This configuration has to be called in non-realtime context before [Master::activate].
*/
pub struct SlaveConfig<'m> {
    master: &'m Master,
    idx: SlaveConfigIdx,
}

impl<'m> SlaveConfig<'m> {
	/// slave index
    pub const fn index(&self) -> SlaveConfigIdx {
        self.idx
    }

    /**
		Outputs the state of the slave configuration.

		Stores the state information in the given state structure. The state information is updated by the master state machine, so it may take a few cycles, until it changes.
    */
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
        for pdo_cfg in pdo_cfgs {
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

    /**
		Configure a slave's watchdog times. 
		
		## Parameters
		
		- `divider` -	Number of 40 ns intervals (register 0x0400). Used as a base unit for all slave watchdogs^. If set to zero, the value is not written, so the default is used.
		- `intervals` -	Number of base intervals for sync manager watchdog (register 0x0420). If set to zero, the value is not written, so the default is used. 
    */
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

    /**
		Configure a sync manager.

		Sets the direction of a sync manager. This overrides the direction bits from the default control register from SII.
    */
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

    /**
		Clear a sync manager's PDO assignment.

		This can be called before assigning PDOs via [Self::add_pdo_assignment], to clear the default assignment of a sync manager.
    */
    pub fn clear_pdo_assignments(&mut self, sync_idx: SmIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_PDOS, &data).map(|_| ())
    }

	/**
		Add a PDO entry to the given PDO's mapping. 
	*/
    pub fn add_pdo_assignment(&mut self, sync_idx: SmIdx, pdo_idx: PdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.sync_index = u8::from(sync_idx);
        data.index = u16::from(pdo_idx);
        ioctl!(self.master, ec::ioctl::SC_ADD_PDO, &data).map(|_| ())
    }

    /**
		Clear the mapping of a given PDO.

		This can be called before mapping PDO entries via [Self::add_pdo_mapping], to clear the default mapping.
    */
    pub fn clear_pdo_mapping(&mut self, pdo_idx: PdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_config_pdo_t::default();
        data.config_index = self.idx;
        data.index = u16::from(pdo_idx);
        ioctl!(self.master, ec::ioctl::SC_CLEAR_ENTRIES, &data).map(|_| ())
    }

    /**
		Add a PDO entry to the given PDO's mapping.
    */
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

    /**
		Registers a PDO entry for process data exchange in a domain.

		Searches the assigned PDOs for the given PDO entry. An error is raised, if the given entry is not mapped. Otherwise, the corresponding sync manager and FMMU configurations are provided for slave configuration and the respective sync manager's assigned PDOs are appended to the given domain, if not already done. The offset of the requested PDO entry's data inside the domain's process data is returned. Optionally, the PDO entry bit position (0-7) can be retrieved via the bit_position output parameter. This pointer may be NULL, in this case an error is raised if the PDO entry does not byte-align.
    */
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

    /**
		Registers a PDO entry using its position.

		Similar to [Self::register_pdo_entry], but not using PDO indices but offsets in the PDO mapping, because PDO entry indices may not be unique inside a slave's PDO mapping. An error is raised, if one of the given positions is out of range.
    */
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

    /**
		Configure distributed clocks.

		Sets the AssignActivate word and the cycle and shift times for the sync signals.

		The AssignActivate word is vendor-specific and can be taken from the XML device description file (Device -> Dc -> AssignActivate). Set this to zero, if the slave shall be operated without distributed clocks (default).
    */
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

    /**
		Add an SDO configuration.

		An SDO configuration is stored in the slave configuration object and is downloaded to the slave whenever the slave is being configured by the master. This usually happens once on master activation, but can be repeated subsequently, for example after the slave's power supply failed.

		- Attention
		
			The SDOs for PDO assignment (0x1C10 - 0x1C2F) and PDO mapping (0x1600 - 0x17FF and 0x1A00 - 0x1BFF) should not be configured with this function, because they are part of the slave configuration done by the master. Please use the methods of [SlaveConfig].

		This is the generic function for adding an SDO configuration. Please note that the this function does not do any endianness correction.
    */
    pub fn add_sdo<T>(&mut self, index: SdoIdx, data: &T) -> Result<()>
    where
        T: SdoData + ?Sized,
    {
        let data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.idx,
            index: u16::from(index.idx),
            subindex: u8::from(index.sub_idx),
            data: data.data_ptr(),
            size: data.data_size(),
            complete_access: 0,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &data).map(|_| ())
    }

    /**
		Add configuration data for a complete SDO.

		The SDO data are transferred via CompleteAccess. Data for the first subindex (0) have to be included.

		See also [Self::add_sdo].
    */
    pub fn add_complete_sdo(&mut self, index: SdoIdx, data: &[u8]) -> Result<()> {
        let data = ec::ec_ioctl_sc_sdo_t {
            config_index: self.idx,
            index: u16::from(index.idx),
            subindex: u8::from(index.sub_idx),
            data: data.as_ptr(),
            size: data.len(),
            complete_access: 1,
        };
        ioctl!(self.master, ec::ioctl::SC_SDO, &data).map(|_| ())
    }

    /**
		Add an SoE IDN configuration.

		A configuration for a Sercos-over-EtherCAT IDN is stored in the slave configuration object and is written to the slave whenever the slave is being configured by the master. This usually happens once on master activation, but can be repeated subsequently, for example after the slave's power supply failed.

		The idn parameter can be separated into several sections:

		| idn section | content |
		|-------------|---------|
		| Bit 15      |  Standard data (0) or Product data (1)  |
		| Bit 14 - 12 |  Parameter set (0 - 7)  |
		| Bit 11 - 0  |  Data block number (0 - 4095)  |

		Please note that the this function does not do any endianness correction. Multi-byte data have to be passed in EtherCAT endianness (little-endian).
		
		## Parameters
		
		- `drive_no` -	Drive number.
		- `idn` -	SoE IDN.
		- `al_state` -	AL state in which to write the IDN (PREOP or SAFEOP).
		- `data` -	Pointer to the data.
    */
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
            size: data.len(),
        };
        ioctl!(self.master, ec::ioctl::SC_IDN, &data).map(|_| ())
    }

    /**
		Set the size of the CoE emergency ring buffer.

		The initial size is zero, so all messages will be dropped. This method can be called even after master activation, but it will clear the ring buffer!

		## Parameters
		
		- `elements` -	Number of records of the CoE emergency ring. 
    */
    pub fn set_emerg_size(&mut self, elements: u64) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        data.size = elements as usize;
        ioctl!(self.master, ec::ioctl::SC_EMERG_SIZE, &data).map(|_| ())
    }

    /**
		Read and remove one record from the CoE emergency ring buffer.

		A record consists of 8 bytes:

		| bytes | content |
		|------|---------|
		| Byte 0-1 | Error code (little endian)  |
		| Byte 2   | Error register |
		| Byte 3-7 | Data |
		
		## Parameters
		
		- `target` -	Pointer to target memory (at least `EC_COE_EMERGENCY_MSG_SIZE` bytes). 
    */
    pub fn pop_emerg(&mut self, target: &mut [u8]) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        data.target = target.as_mut_ptr();
        ioctl!(self.master, ec::ioctl::SC_EMERG_POP, &mut data).map(|_| ())
    }

    /**
		Clears CoE emergency ring buffer and the overrun counter. 
    */
    pub fn clear_emerg(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        ioctl!(self.master, ec::ioctl::SC_EMERG_CLEAR, &data).map(|_| ())
    }

    /**
		Read the number of CoE emergency overruns.

		The overrun counter will be incremented when a CoE emergency message could not be stored in the ring buffer and had to be dropped. Call ecrt_slave_config_emerg_clear() to reset the counter.
    */
    pub fn emerg_overruns(&mut self) -> Result<i32> {
        let mut data = ec::ec_ioctl_sc_emerg_t::default();
        data.config_index = self.idx;
        ioctl!(self.master, ec::ioctl::SC_EMERG_OVERRUNS, &mut data)?;
        Ok(data.overruns)
    }

    // TODO missing: create_sdo_request, create_reg_request, create_voe_handler
}

impl<'m> Domain<'m> {
    pub const fn new(idx: DomainIdx, master: &'m Master) -> Self {
        Self { idx, master }
    }

	/**
		Returns the current size of the domain's process data. 
	*/
    pub fn size(&self) -> Result<usize> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_SIZE,
            c_ulong::try_from(self.idx).map_err(|_| Error::DomainIdx(usize::from(self.idx)))?
        )
        .map(|v| v as usize)
    }

    /**
		Reads the state of a domain.

		Stores the domain state in the given state structure.

		Using this method, the process data exchange can be monitored in realtime. 
    */
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

    /**
		Determines the states of the domain's datagrams.

		Evaluates the working counters of the received datagrams and outputs statistics, if necessary. This must be called after [Master::receive] is expected to receive the domain datagrams in order to make [Self::state] return the result of the last process data exchange. 
    */
    pub fn process(&mut self) -> Result<()> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_PROCESS,
            usize::from(self.idx) as c_ulong
        )
        .map(|_| ())
    }

    /**
		(Re-)queues all domain datagrams in the master's datagram queue.

		Call this function to mark the domain's datagrams for exchanging at the next call of [Master::send]. 
    */
    pub fn queue(&mut self) -> Result<()> {
        ioctl!(
            self.master,
            ec::ioctl::DOMAIN_QUEUE,
            c_ulong::try_from(self.idx).map_err(|_| Error::DomainIdx(usize::from(self.idx)))?
        )
        .map(|_| ())
    }
}
