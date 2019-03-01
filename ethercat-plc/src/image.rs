// *****************************************************************************
//
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program; if not, write to the Free Software Foundation, Inc.,
// 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
//
// Module authors:
//   Georg Brandl <g.brandl@fz-juelich.de>
//
// *****************************************************************************

//! Tools to create a typesafe process image matching with possible slave PDOs.

use ethercat::*;

pub trait ProcessImage {
    // configuration APIs
    const SLAVE_COUNT: usize;
    fn get_slave_ids() -> Vec<SlaveId>;
    fn get_slave_pdos() -> Vec<Option<Vec<SyncInfo<'static>>>> { vec![None] }
    fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> { vec![vec![]] }
    fn get_slave_sdos() -> Vec<Vec<(SdoIndex, Box<dyn SdoData>)>> { vec![vec![]] }

    fn size() -> usize where Self: Sized {
        std::mem::size_of::<Self>()
    }

    fn cast(data: &mut [u8]) -> &mut Self where Self: Sized {
        unsafe { std::mem::transmute(data.as_mut_ptr()) }
    }
}

pub trait ExternImage : Default {
    fn size() -> usize where Self: Sized {
        std::mem::size_of::<Self>()
    }

    fn cast(&mut self) -> &mut [u8] where Self: Sized {
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, Self::size())
        }
    }
}
