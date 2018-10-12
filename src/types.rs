use std::io;

pub type Error = io::Error;
pub type Result<T> = io::Result<T>;


/// An EtherCAT slave identification, consisting of vendor ID and product code.
pub struct SlaveId {
    pub vendor_id: u32,
    pub product_code: u32,
}


/// An EtherCAT slave, which is specified either by absolute position in the
/// ring or by offset from a given alias.
pub enum SlaveAddr {
    ByPos(u16),
    ByAlias(u16, u16)
}

impl SlaveAddr {
    pub(crate) fn as_pair(&self) -> (u16, u16) {
        match *self {
            SlaveAddr::ByPos(x) => (0, x),
            SlaveAddr::ByAlias(x, y) => (x, y),
        }
    }
}
