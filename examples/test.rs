extern crate ethercat;

use ethercat::plc::PlcBuilder;
use ethercat::image::{ProcessImage, ExternImage};
use ethercat::types::*;

#[repr(C, packed)]
#[derive(Default)]
struct Extern {
    magic: f32,
    offset: u16,
    cycle: i16,
    ana_in: i16,
}

impl ExternImage for Extern {}

#[repr(C, packed)]
struct Image {
    dig_in: u8,
    dig_out: u8,
    ana_in_1_sta: u16,
    ana_in_1_val: i16,
    ana_in_2_sta: u16,
    ana_in_2_val: i16,
    ana_out_1: i16,
    ana_out_2: i16,
}

const EL3104_SYNCS: &[SyncInfo] = &[SyncInfo::input(3, &[
    PdoInfo::default(0x1a00),
    PdoInfo::default(0x1a02),
])];

impl ProcessImage for Image {
    fn slave_count() -> usize { 4 }
    fn get_slave_id(slave: usize) -> SlaveId {
        match slave {
            0 => SlaveId::EK(1100),
            1 => SlaveId::EL(1859),
            2 => SlaveId::EL(3104),
            3 => SlaveId::EL(4132),
            _ => unreachable!()
        }
    }
    fn get_slave_pdos(slave: usize) -> Option<&'static [SyncInfo<'static>]> {
        match slave {
            2 => Some(EL3104_SYNCS),
            _ => None
        }
    }
    fn get_slave_regs(slave: usize) -> &'static [(PdoEntryIndex, Offset)] {
        match slave {
            0 => &[],
            1 => &[
                (PdoEntryIndex { index: 0x6000, subindex: 1 }, Offset { byte: 0, bit: 0 }),
                (PdoEntryIndex { index: 0x7080, subindex: 1 }, Offset { byte: 1, bit: 0 }),
            ],
            2 => &[
                (PdoEntryIndex { index: 0x6000, subindex: 1 }, Offset { byte: 2, bit: 0}),
                (PdoEntryIndex { index: 0x6000, subindex: 17 }, Offset { byte: 4, bit: 0}),
                (PdoEntryIndex { index: 0x6010, subindex: 1 }, Offset { byte: 6, bit: 0}),
                (PdoEntryIndex { index: 0x6010, subindex: 17 }, Offset { byte: 8, bit: 0}),
            ],
            3 => &[
                (PdoEntryIndex { index: 0x3001, subindex: 1 }, Offset { byte: 10, bit: 0}),
                (PdoEntryIndex { index: 0x3002, subindex: 1 }, Offset { byte: 12, bit: 0}),
            ],
            _ => unreachable!()
        }
    }
}

fn main() {
    let mut plc = PlcBuilder::new()
        .cycle_freq(100)
        .server("0.0.0.0:5020")
        .build::<Image, Extern>().unwrap();

    let mut blink = 6u8;
    let mut cycle = 0;
    plc.run(|data, ext| {
        ext.magic = 2015.02;
        ext.offset = 10;
        ext.cycle = cycle;
        ext.ana_in = data.ana_in_1_val; // cannot borrow
        blink = if blink == 6 { 9 } else { 6 };
        // blink = 1 - blink;
        data.dig_out = blink;
        cycle += 1;
        data.ana_out_1 = cycle;
    });
}
