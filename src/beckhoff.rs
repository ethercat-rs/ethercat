use crate::image::ProcessImage;
use crate::types::*;

#[repr(C, packed)]
pub struct EK1100 {}

impl ProcessImage for EK1100 {
    const SLAVE_COUNT: usize = 1;
    fn get_slave_ids() -> Vec<SlaveId> { vec![SlaveId::EK(1100)] }
}

#[repr(C, packed)]
pub struct EL1859 {
    pub input: u8,
    pub output: u8,
}

impl ProcessImage for EL1859 {
    const SLAVE_COUNT: usize = 1;
    fn get_slave_ids() -> Vec<SlaveId> { vec![SlaveId::EL(1859)] }
    fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> {
        vec![vec![
            (PdoEntryIndex { index: 0x6000, subindex: 1 }, Offset { byte: 0, bit: 0 }),
            (PdoEntryIndex { index: 0x7080, subindex: 1 }, Offset { byte: 1, bit: 0 }),
        ]]
    }
}

#[repr(C, packed)]
pub struct EL3104 {
    pub ch1_status: u16,
    pub ch1: i16,
    pub ch2_status: u16,
    pub ch2: i16,
    pub ch3_status: u16,
    pub ch3: i16,
    pub ch4_status: u16,
    pub ch4: i16,
}

impl ProcessImage for EL3104 {
    const SLAVE_COUNT: usize = 1;
    fn get_slave_ids() -> Vec<SlaveId> { vec![SlaveId::EL(3104)] }
    fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> {
        vec![vec![
            (PdoEntryIndex { index: 0x6000, subindex: 1 },  Offset { byte: 0,  bit: 0}),
            (PdoEntryIndex { index: 0x6000, subindex: 17 }, Offset { byte: 2,  bit: 0}),
            (PdoEntryIndex { index: 0x6010, subindex: 1 },  Offset { byte: 4,  bit: 0}),
            (PdoEntryIndex { index: 0x6010, subindex: 17 }, Offset { byte: 6,  bit: 0}),
            (PdoEntryIndex { index: 0x6020, subindex: 1 },  Offset { byte: 8,  bit: 0}),
            (PdoEntryIndex { index: 0x6020, subindex: 17 }, Offset { byte: 10, bit: 0}),
            (PdoEntryIndex { index: 0x6030, subindex: 1 },  Offset { byte: 12, bit: 0}),
            (PdoEntryIndex { index: 0x6030, subindex: 17 }, Offset { byte: 14, bit: 0}),
        ]]
    }
}

#[repr(C, packed)]
pub struct EL4132 {
    pub ch1: i16,
    pub ch2: i16,
}

impl ProcessImage for EL4132 {
    const SLAVE_COUNT: usize = 1;
    fn get_slave_ids() -> Vec<SlaveId> { vec![SlaveId::EL(4132)] }
    fn get_slave_regs() -> Vec<Vec<(PdoEntryIndex, Offset)>> {
        vec![vec![
            (PdoEntryIndex { index: 0x3001, subindex: 1 }, Offset { byte: 0, bit: 0}),
            (PdoEntryIndex { index: 0x3002, subindex: 1 }, Offset { byte: 2, bit: 0}),
        ]]
    }
}
