// Part of ethercat-rs. Copyright 2018-2019 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use ethercat_plc::{PlcBuilder, ProcessImage, ExternImage};
use ethercat_plc::beckhoff::*;
use ethercat_plc::mlz_spec::*;

const PLC_NAME:     &str = "testplc";
const PLC_VERSION:  &str = "v0.0.5beta";
const PLC_AUTHOR_1: &str = "some very long strings that I think";
const PLC_AUTHOR_2: &str = "won't really fit into the indexer";

const INDEXER_SIZE: u16 = std::mem::size_of::<Indexer>() as u16;
const INDEXER_OFFS: u16 = 6;


#[repr(C, packed)]
#[derive(ProcessImage)]
struct Image {
    coupler: EK1100,
    #[sdo(0x8010, 1, "750u16")]  // normal current 750 mA
    #[sdo(0x8010, 2, "250u16")]  // reduced current 250 mA
    #[sdo(0x8010, 3, "2400u16")] // supply is 24 V
    #[sdo(0x8010, 4, "1000u16")] // resistance is 10 Ohm
    #[sdo(0x8012, 8, "1u8")]     // feedback internal
    #[sdo(0x8012, 0x11, "7u8")]  // info data 1: velocity
    #[sdo(0x8012, 0x19, "13u8")] // info data 2: motor current
    motor:   EL7047_Position,
    dig_in:  EL1008,
    dig_out: EL2008,
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
#[derive(Default, ExternImage)]
struct Extern {
    magic: f32,
    offset: u16,
    indexer: Indexer,
    if_blink: DiscreteOutput,
    if_magnet: FlatOutput1,
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

    ext.magic = MAGIC;
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

fn fb_blink(data_in: &mut EL1008, data_out: &mut EL2008, iface: &mut DiscreteOutput) {
    match iface.status & 0xf000 {
        RESET => {
            data_out.output = 0;
            iface.target = 0;
            iface.status = IDLE;
        }
        IDLE | WARN => {
            iface.status = if iface.target == iface.value { IDLE } else { WARN };
        }
        START => {
            data_out.output = iface.target as u8;
            iface.status = IDLE;
        }
        _ => iface.status = ERROR,
    }

    iface.value = data_in.input as i16;
}

fn fb_magnet(inp: &mut EL3104, outp: &mut EL4132,
             iface: &mut FlatOutput1, vars: &mut MagnetVars) {
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

    let mut globals = Globals::default();
    globals.devices = vec![
        DeviceInfo { typcode: 0x1E03, name: "Blink", offset: 42, .. Default::default() },
        DeviceInfo { typcode: 0x3008, name: "Magnet", unit: 0x0007,
                     params: [0x3c, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                     aux: &["output disabled", "emergency shutdown"],
                     absmin: -15.0, absmax: 15.0, .. Default::default() },
    ];

    plc.run(|data, ext| {
        indexer(ext, &mut globals);
        fb_blink(&mut data.dig_in, &mut data.dig_out, &mut ext.if_blink);
        fb_magnet(&mut data.ana_in, &mut data.ana_out, &mut ext.if_magnet,
                  &mut globals.v_magnet);

        if data.motor.mot_status & 1 != 0 {
            data.motor.mot_control = 0x1;
        }
        if data.motor.mot_status & 2 != 0 {
            data.motor.mot_target = (globals.v_magnet.current * 10000.) as _;
        }
        // let info1 = data.motor.info_data1;
        // let info2 = data.motor.info_data2;
        // println!("st = {:#x}, id = {:#x}, {:#x}", data.motor.mot_status & 0xfff,
                 // info1, info2);
        println!("pos = {}", data.motor.mot_position & !0);
    });
}
