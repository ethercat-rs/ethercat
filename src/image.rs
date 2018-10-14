//! Tools to create a typesafe process image matching with possible slave PDOs.

use crate::types::*;

pub trait ProcessImage {
    // configuration APIs
    fn slave_count() -> usize;
    fn get_slave_id(slave: usize) -> SlaveId;
    #[allow(unused_variables)]
    fn get_slave_pdos(slave: usize) -> Option<&'static [SyncInfo<'static>]> { None }
    fn get_slave_sdos(_slave: usize) -> &'static [()] { &[] }
    fn get_slave_regs(slave: usize) -> &'static [(PdoEntryIndex, Offset)];

    // data area
    fn size() -> usize
    where Self: Sized
    {
        std::mem::size_of::<Self>()
    }

    // cast
    fn cast(data: &mut [u8]) -> &mut Self
    where Self: Sized
    {
        unsafe { std::mem::transmute(data.as_mut_ptr()) }
    }
}

pub trait ExternImage : Default {
    fn cast(&mut self) -> &mut [u8]
    where Self: Sized
    {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, std::mem::size_of::<Self>())
        }
    }
}
