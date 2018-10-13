extern crate ethercat;

use ethercat::plc::PlcBuilder;
use ethercat::image::ProcessImage;
use ethercat::types::*;

#[repr(packed)]
struct Simple {
    dig_in: u8,
    dig_out: u8,
}

impl ProcessImage for Simple {
    fn slave_count() -> usize { 2 }
    fn get_slave_id(slave: usize) -> SlaveId {
        match slave {
            0 => SlaveId::EK(1100),
            1 => SlaveId::EL(1859),
            _ => unreachable!()
        }
    }
    fn get_slave_pdos(_: usize) -> Option<&'static [SyncInfo<'static>]> {
        None
    }
    fn get_slave_sdos(_: usize) -> &'static [()] {
        &[]
    }
    fn get_slave_regs(slave: usize) -> &'static [PdoEntryIndex] {
        match slave {
            0 => &[],
            1 => &[
                PdoEntryIndex { index: 0x6000, subindex: 1},
                PdoEntryIndex { index: 0x7080, subindex: 1},
            ],
            _ => unreachable!()
        }
    }
}

fn main() {
    let mut plc = PlcBuilder::new().build::<Simple>().unwrap();

    let mut blink = 0x6;
    plc.run(|data| {
        std::thread::sleep(std::time::Duration::from_millis(10));
        println!("in: {}", data.dig_in);
        blink = if blink == 0x6 { 0x9 } else { 0x6 };
        data.dig_out = blink;
    });
}
