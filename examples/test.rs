extern crate ethercat;
#[macro_use]
extern crate ethercat_derive;

use ethercat::{PlcBuilder, ProcessImage, ExternImage};
use ethercat::beckhoff::*;

#[repr(C, packed)]
#[derive(ProcessImage)]
struct Image {
    coupler: EK1100,
    digital: EL1859,
    ana_in:  EL3104,
    ana_out: EL4132,
}

#[repr(C, packed)]
#[derive(ExternImage)]
struct Extern {
    magic: f32,
    offset: u16,
    cycle: i16,
    ana_in: i16,
}

impl Default for Extern {
    fn default() -> Self {
        Extern {
            magic: 2015.02,
            offset: 10,
            cycle: 0,
            ana_in: 0,
        }
    }
}

fn main() {
    let mut plc = PlcBuilder::new()
        .cycle_freq(100)
        .server("0.0.0.0:5020")
        .build::<Image, Extern>().unwrap();

    let mut blink = 6u8;
    let mut cycle = 0i16;

    plc.run(|data, ext| {
        blink = if blink == 6 { 9 } else { 6 };
        cycle = cycle.wrapping_add(1);

        ext.cycle = cycle;
        ext.ana_in = data.ana_in.ch1;

        data.digital.output = blink;
        data.ana_out.ch1 = cycle;
    });
}
