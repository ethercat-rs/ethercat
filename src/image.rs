//! Tools to create a typesafe process image matching with possible slave PDOs.

use crate::types::*;

pub trait ProcessImage {
    // configuration APIs
    const SLAVE_COUNT: usize;
    fn get_slave_ids() -> Vec<SlaveId>;
    fn get_slave_pdos() -> Vec<Option<Vec<SyncInfo<'static>>>> { vec![None] }
    fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> { vec![vec![]] }
    fn get_slave_sdos() -> Vec<Vec<()>> { vec![vec![]] }

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
