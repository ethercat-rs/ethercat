use ethercat::*;
use ethercat_derive::SlaveProcessImage;
use crate::image::ProcessImage;

#[repr(C, packed)]
#[derive(SlaveProcessImage)]
pub struct EK1100 {}

#[repr(C, packed)]
#[derive(SlaveProcessImage)]
pub struct EL1859 {
    #[pdo(0x6000, 1)]
    pub input: u8,
    #[pdo(0x7080, 1)]
    pub output: u8,
}

#[repr(C, packed)]
#[derive(SlaveProcessImage)]
pub struct EL3104 {
    #[pdo(0x6000, 1)]
    pub ch1_status: u16,
    #[pdo(0x6000, 17)]
    pub ch1: i16,
    #[pdo(0x6010, 1)]
    pub ch2_status: u16,
    #[pdo(0x6010, 17)]
    pub ch2: i16,
    #[pdo(0x6020, 1)]
    pub ch3_status: u16,
    #[pdo(0x6020, 17)]
    pub ch3: i16,
    #[pdo(0x6030, 1)]
    pub ch4_status: u16,
    #[pdo(0x6030, 17)]
    pub ch4: i16,
}

#[repr(C, packed)]
#[derive(SlaveProcessImage)]
pub struct EL4132 {
    #[pdo(0x3001, 1)]
    pub ch1: i16,
    #[pdo(0x3002, 1)]
    pub ch2: i16,
}
