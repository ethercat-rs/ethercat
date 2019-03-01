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

use byteorder::{ByteOrder, NativeEndian as NE};

pub const MAGIC: f32 = 2015.02;

pub const RESET: u16 = 0x0000;
pub const IDLE:  u16 = 0x1000;
pub const WARN:  u16 = 0x3000;
pub const START: u16 = 0x5000;
pub const BUSY:  u16 = 0x6000;
pub const STOP:  u16 = 0x7000;
pub const ERROR: u16 = 0x8000;

#[repr(C)]
#[derive(Default)]
pub struct DiscreteOutput {
    pub value:  i16,
    pub target: i16,
    pub status: u16,
}

#[repr(C)]
#[derive(Default)]
pub struct FlatOutput1 {
    pub value:  f32,
    pub target: f32,
    pub status: u16,
    pub aux:    u16,
    pub param1: f32,
}

pub fn copy_string(dst: &mut [u16], src: &str) {
    let mut nbytes = src.len().min(dst.len() * 2);
    let mut src_vec;
    let src = if nbytes % 2 == 1 {
        src_vec = src.to_string();
        src_vec.push('\0');
        nbytes += 1;
        &src_vec
    } else {
        src
    };
    NE::read_u16_into(&src[..nbytes].as_bytes(), &mut dst[..nbytes/2])
}

pub fn copy_float(dst: &mut [u16], f: f32) {
    let mut buf = [0u8; 4];
    NE::write_f32(&mut buf, f);
    NE::read_u16_into(&buf, &mut dst[..2]);
}
