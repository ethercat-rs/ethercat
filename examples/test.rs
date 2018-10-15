extern crate ethercat_plc;
#[macro_use]
extern crate ethercat_derive;
extern crate byteorder;

use byteorder::{ByteOrder, NativeEndian as NE};
use ethercat_plc::{PlcBuilder, ProcessImage, ExternImage};
use ethercat_plc::beckhoff::*;

const PLC_NAME:     &str = "testplc";
const PLC_VERSION:  &str = "v0.0.5beta";
const PLC_AUTHOR_1: &str = "some very long strings that I think";
const PLC_AUTHOR_2: &str = "won't really fit into the indexer";

const INDEXER_SIZE: u16 = std::mem::size_of::<Indexer>() as u16;
const INDEXER_OFFS: u16 = 6;


const RESET: u16 = 0x0000;
const IDLE:  u16 = 0x1000;
const WARN:  u16 = 0x3000;
const START: u16 = 0x5000;
const BUSY:  u16 = 0x6000;
const STOP:  u16 = 0x7000;
const ERROR: u16 = 0x8000;

#[repr(C, packed)]
#[derive(ProcessImage)]
struct Image {
    coupler: EK1100,
    digital: EL1859,
    ana_in:  EL3104,
    ana_out: EL4132,
}

#[repr(C)]
#[derive(Default)]
struct Indexer {
    request: u16,
    data: [u16; 17],
}

#[repr(C)]
#[derive(Default)]
struct DiscOut {
    value:  i16,
    target: i16,
    status: u16,
}

#[repr(C)]
#[derive(Default)]
struct FlatOut1 {
    value:  f32,
    target: f32,
    status: u16,
    aux:    u16,
    param1: f32,
}

#[repr(C)]
#[derive(Default, ExternImage)]
struct Extern {
    magic: f32,
    offset: u16,
    indexer: Indexer,
    if_blink: DiscOut,
    if_magnet: FlatOut1,
}

#[derive(Default)]
struct MagnetVars {
    target: f32,
    start: f32,
    step: f32,
    current: f32,
    cycles: u32,
}

#[derive(Default)]
struct Globals {
    cycle: u16,
    indexer_is_init: bool,
    devices: Vec<DeviceInfo>,
    v_magnet: MagnetVars,
}

#[derive(Default)]
struct DeviceInfo {
    typcode: u16,
    size: u16,
    offset: u16,
    unit: u16,
    flags: u8,
    all_flags: u32,
    params: [u16; 16],
    name: &'static str,
    aux: &'static [&'static str],
    absmax: f32,
    absmin: f32,
}

fn copy_string(dst: &mut [u16], src: &str) {
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

fn copy_float(dst: &mut [u16], f: f32) {
    let mut buf = [0u8; 4];
    NE::write_f32(&mut buf, f);
    NE::read_u16_into(&buf, &mut dst[..2]);
}

fn indexer(ext: &mut Extern, globals: &mut Globals) {
    if !globals.indexer_is_init {
        let mut calc_offset = INDEXER_OFFS + INDEXER_SIZE;
        for dev in &mut globals.devices {
            dev.all_flags = (dev.flags as u32) << 24;
            for j in 0..8 {
                if dev.aux.len() > j && !dev.aux[j].is_empty() {
                    dev.all_flags |= 1 << j;
                }
            }
            if dev.size < (dev.typcode & 0xff) << 1 {
                dev.size = (dev.typcode & 0xff) << 1;
            }
            if dev.offset == 0 {
                dev.offset = calc_offset;
            } else {
                calc_offset = dev.offset;
            }
            calc_offset += dev.size;
        }
        globals.indexer_is_init = true;
    }

    ext.magic = 2015.02;
    ext.offset = INDEXER_OFFS;

    let devnum = ext.indexer.request as usize & 0xff;
    let infotype = (ext.indexer.request as usize >> 8) & 0x7f;

    let data = &mut ext.indexer.data;
    data.copy_from_slice(&[0; 17]);

    match devnum {
        0 => match infotype {
            0 => {
                data[..10].copy_from_slice(
                    &[0, INDEXER_SIZE, ext.offset, 0, 0, 0x8000, 0, 0, 0, 0]);
                copy_string(&mut data[10..], PLC_NAME);
            }
            1 => data[0] = INDEXER_SIZE,
            4 => copy_string(data, PLC_NAME),
            5 => copy_string(data, PLC_VERSION),
            6 => copy_string(data, PLC_AUTHOR_1),
            7 => copy_string(data, PLC_AUTHOR_2),
            _ => {}
        },
        n if n <= globals.devices.len() => {
            let dev = &globals.devices[n-1];
            match infotype {
                0 => {
                    data[..6].copy_from_slice(&[
                        dev.typcode, dev.size, dev.offset, dev.unit,
                        dev.all_flags as u16, (dev.all_flags >> 16) as u16,
                    ]);
                    copy_float(&mut data[6..], dev.absmin);
                    copy_float(&mut data[8..], dev.absmax);
                    copy_string(&mut data[10..], dev.name);
                }
                1 => data[0] = dev.size,
                2 => data[0] = dev.offset,
                3 => data[0] = dev.unit,
                4 => copy_string(data, dev.name),
                15 => data[..16].copy_from_slice(&dev.params),
                0x10 ..= 0x17 => copy_string(data, dev.aux.get(infotype-0x10).unwrap_or(&"")),
                _ => {}
            }
        },
        _ => {}
    }

    if infotype == 127 {
        data[0] = globals.cycle;
    }

    ext.indexer.request |= 0x8000;
    globals.cycle = globals.cycle.wrapping_add(1);
}

fn fb_blink(data: &mut EL1859, iface: &mut DiscOut) {
    match iface.status & 0xf000 {
        RESET => {
            data.output = 0;
            iface.target = 0;
            iface.status = IDLE;
        }
        IDLE | WARN => {
            iface.status = if iface.target == iface.value { IDLE } else { WARN };
        }
        START => {
            data.output = iface.target as u8;
            iface.status = IDLE;
        }
        _ => iface.status = ERROR,
    }

    iface.value = data.input as i16;
}

fn fb_magnet(inp: &mut EL3104, outp: &mut EL4132, iface: &mut FlatOut1, vars: &mut MagnetVars) {
    iface.target = iface.target.max(-15.0).min(15.0);
    iface.param1 = iface.param1.max(-10.0).min(10.0);

    const SLOPE: f32 = 2000.;

    match iface.status & 0xf000 {
        RESET => {
            iface.status = IDLE;
            iface.target = 0.;
        }
        IDLE | WARN => {}
        START => {
            vars.target = iface.target;
            vars.start = iface.value;
            vars.current = iface.value;
            vars.cycles = 0;
            vars.step = iface.param1 / 100.;
            if vars.target < vars.start {
                vars.step = -vars.step;
            }
            if (vars.current - vars.target).abs() <= vars.step.abs() {
                vars.current = vars.target;
                iface.status = IDLE;
            } else {
                iface.status = BUSY;
            }
        }
        BUSY => {
            vars.current = vars.start + (vars.cycles as f32) * vars.step;
            if (vars.current - vars.target).abs() <= vars.step.abs() {
                vars.current = vars.target;
                iface.status = IDLE;
            } else {
                vars.cycles += 1;
            }
        }
        STOP => {
            iface.status = IDLE;
        }
        ERROR => {
            iface.value = 0.;
        }
        _ => iface.status = ERROR,
    }

    outp.ch1 = (vars.current * SLOPE) as i16;
    iface.value = inp.ch1 as f32 / SLOPE;
}

fn main() {
    let mut plc = PlcBuilder::new("plc")
        .cycle_freq(100)
        .with_server("0.0.0.0:5020")
        .logging_cfg(None, false)
        .build::<Image, Extern>().unwrap();

    let mut global_instance = Globals::default();
    global_instance.devices = vec![
        DeviceInfo { typcode: 0x1E03, name: "Blink", offset: 42, .. Default::default() },
        DeviceInfo { typcode: 0x3008, name: "Magnet", unit: 0x0007,
                     params: [0x3c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                     aux: &["output disabled", "emergency shutdown"],
                     absmin: -15.0, absmax: 15.0, .. Default::default() },
    ];
    let globals = &mut global_instance;

    plc.run(|data, ext| {
        indexer(ext, globals);
        fb_blink(&mut data.digital, &mut ext.if_blink);
        fb_magnet(&mut data.ana_in, &mut data.ana_out, &mut ext.if_magnet, &mut globals.v_magnet);
    });
}
