use std::fs::File;
use std::io::{Error, ErrorKind};
use std::os::unix::io::AsRawFd;
use libc;
use crate::ec;
use crate::Result;
use crate::types::{SlaveAddr, SlaveId};

macro_rules! ioctl {
    ($m:expr, $nr:expr) => { ioctl!($m, $nr, 0 as *const i8) };
    ($m:expr, $nr:expr, @$arg:expr) => { ioctl!($m, $nr, $arg as *const i8) };
    ($m:expr, $nr:expr, $arg:expr) => {{
        let res = unsafe { libc::ioctl($m.file.as_raw_fd(), $nr, $arg as *const _) };
        if res < 0 { Err(Error::last_os_error()) } else { Ok(res) }
    }}
}

/// An EtherCAT master.
pub struct Master {
    file: File,
    domains: Vec<i32>,
}

pub struct Domain<'m> {
    master: &'m Master,
    index: i32,
}

impl Master {
    pub fn reserve(index: u32) -> Result<Self> {
        let devpath = format!("/dev/EtherCAT{}", index);
        let file = File::create(&devpath)?;
        let module_info = ec::ec_ioctl_module_t {
            ioctl_version_magic: 0,
            master_count: 0,
        };
        let master = Master { file, domains: vec![] };
        ioctl!(master, ec::EC_IOCTL_MODULE, &module_info)?;
        if module_info.ioctl_version_magic != ec::EC_IOCTL_VERSION_MAGIC {
            Err(Error::new(ErrorKind::Other,
                           format!("module version mismatch: expected {}, found {}",
                                   ec::EC_IOCTL_VERSION_MAGIC,
                                   module_info.ioctl_version_magic)))
        } else {
            ioctl!(master, ec::EC_IOCTL_REQUEST)?;
            Ok(master)
        }
    }

    pub fn create_domain(&mut self) -> Result<usize> {
        let index = ioctl!(self, ec::EC_IOCTL_CREATE_DOMAIN)?;
        self.domains.push(index);
        Ok(self.domains.len() - 1)
    }

    pub fn domain(&self, index: usize) -> Domain {
        Domain { master: self, index: self.domains[index] }
    }

    pub fn slave_config(&self, addr: SlaveAddr, expected: SlaveId) -> Result<()> {
        let mut data = ec::ec_ioctl_config_t::default();
        let (alias, pos) = addr.as_pair();
        data.alias = alias;
        data.position = pos;
        data.vendor_id = expected.vendor_id;
        data.product_code = expected.product_code;
        ioctl!(self, ec::EC_IOCTL_CREATE_SLAVE_CONFIG, &data)?;

        unimplemented!()
    }

}

// XXX add own type
pub type DomainState = ec::ec_domain_state_t;

impl<'m> Domain<'m> {
    pub fn size(&self) -> Result<usize> {
        ioctl!(self.master, ec::EC_IOCTL_DOMAIN_SIZE, @self.index).map(|v| v as usize)
    }

    pub fn data(&self) -> *mut u8 {
        unimplemented!()
    }

    pub fn state(&self) -> Result<DomainState> {
        let mut state = ec::ec_domain_state_t::default();
        let data = ec::ec_ioctl_domain_state_t { domain_index: self.index as u32,
                                                 state: &mut state };
        ioctl!(self.master, ec::EC_IOCTL_DOMAIN_STATE, &data)?;
        Ok(state)
    }

    pub fn process(&mut self) -> Result<()> {
        ioctl!(self.master, ec::EC_IOCTL_DOMAIN_PROCESS, @self.index).map(|_| ())
    }

    pub fn queue(&mut self) -> Result<()> {
        ioctl!(self.master, ec::EC_IOCTL_DOMAIN_QUEUE, @self.index).map(|_| ())
    }
}
