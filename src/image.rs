//! Tools to create a typesafe process image matching with possible slave PDOs.

use crate::types::*;

pub trait ProcessImage {
    // configuration APIs
    fn slave_count() -> usize;
    fn get_slave_id(slave: usize) -> SlaveId;
    fn get_slave_pdos(slave: usize) -> Option<&'static [SyncInfo<'static>]>;
    fn get_slave_sdos(slave: usize) -> &'static [()];
    fn get_slave_regs(slave: usize) -> &'static [PdoEntryIndex];

    // data area
    // fn size() -> usize;

    // cast
    fn cast(data: &mut [u8]) -> &mut Self
    where Self: Sized
    {
        unsafe { std::mem::transmute(data.as_mut_ptr()) }
    }
}
