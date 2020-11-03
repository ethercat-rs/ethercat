use ethercat::{
    AlState, DomainIdx as DomainIndex, Idx, Master, MasterAccess, Offset, PdoCfg, PdoEntryIdx,
    PdoEntryIdx as PdoEntryIndex, PdoEntryInfo, PdoEntryPos, PdoIdx, SlaveAddr, SlaveId, SlavePos,
    SmCfg, SubIdx,
};
use ethercat_esi::EtherCatInfo;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{self, prelude::*},
    thread,
    time::Duration,
};

type BitLen = u8;

pub fn main() -> Result<(), io::Error> {
    env_logger::init();
    let args: Vec<_> = env::args().collect();
    let file_name = match args.len() {
        2 => &args[1],
        _ => {
            println!("usage: {} ESI-FILE", env!("CARGO_PKG_NAME"));
            return Ok(());
        }
    };

    log::debug!("Parse XML file {}", file_name);
    let mut esi_file = File::open(file_name)?;
    let mut esi_xml_string = String::new();
    esi_file.read_to_string(&mut esi_xml_string)?;
    let esi = EtherCatInfo::from_xml_str(&esi_xml_string)?;
    let (mut master, domain_idx, offsets) = init_master(&esi, 0_u32)?;
    for (s, o) in &offsets {
        log::info!("PDO offsets of Slave {}:", u16::from(*s));
        for (pdo, (bit_len, offset)) in o {
            log::info!(
                " - {:X}:{:X} - {:?}, bit length: {}",
                u16::from(pdo.idx),
                u8::from(pdo.sub_idx),
                offset,
                bit_len
            );
        }
    }
    let cycle_time = Duration::from_micros(50_000);
    master.activate()?;

    loop {
        master.receive()?;
        master.domain(domain_idx).process()?;
        master.domain(domain_idx).queue()?;
        master.send()?;
        let m_state = master.state()?;
        let d_state = master.domain(domain_idx).state();
        log::debug!("Master state: {:?}", m_state);
        log::debug!("Domain state: {:?}", d_state);
        if m_state.link_up && m_state.al_states == 8 {
            let raw_data = master.domain_data(domain_idx);
            log::debug!("{:?}", raw_data);
        }
        thread::sleep(cycle_time);
    }
}

pub fn init_master(
    esi: &EtherCatInfo,
    idx: u32,
) -> Result<
    (
        Master,
        DomainIndex,
        HashMap<SlavePos, HashMap<PdoEntryIndex, (BitLen, Offset)>>,
    ),
    io::Error,
> {
    let mut master = Master::open(idx, MasterAccess::ReadWrite)?;
    log::debug!("Reserve master");
    master.reserve()?;
    log::debug!("Create domain");
    let domain_idx = master.create_domain()?;
    let mut offsets: HashMap<SlavePos, HashMap<PdoEntryIndex, (u8, Offset)>> = HashMap::new();

    for (dev_nr, dev) in esi.description.devices.iter().enumerate() {
        let slave_pos = SlavePos::from(dev_nr as u16);
        log::debug!("Request PreOp state for {:?}", slave_pos);
        master.request_state(slave_pos, AlState::Preop)?;
        let slave_info = master.get_slave_info(slave_pos)?;
        log::info!("Found device {}:{:?}", dev.name, slave_info);
        let slave_addr = SlaveAddr::ByPos(dev_nr as u16);
        let slave_id = SlaveId {
            vendor_id: esi.vendor.id,
            product_code: dev.product_code,
        };
        let mut config = master.configure_slave(slave_addr, slave_id)?;
        let mut entry_offsets: HashMap<PdoEntryIndex, (u8, Offset)> = HashMap::new();

        let rx_pdos: Vec<PdoCfg> = dev
            .rx_pdo
            .iter()
            .map(|pdo| PdoCfg {
                idx: PdoIdx::from(pdo.index),
                entries: pdo
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| PdoEntryInfo {
                        entry_idx: PdoEntryIdx {
                            idx: Idx::from(e.index),
                            sub_idx: SubIdx::from(e.sub_index.unwrap_or(1) as u8),
                        },
                        bit_len: e.bit_len as u8,
                        name: e.name.clone().unwrap_or(String::new()),
                        pos: PdoEntryPos::from(i as u8),
                    })
                    .collect(),
            })
            .collect();

        let tx_pdos: Vec<PdoCfg> = dev
            .tx_pdo
            .iter()
            .map(|pdo| PdoCfg {
                idx: PdoIdx::from(pdo.index),
                entries: pdo
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| PdoEntryInfo {
                        entry_idx: PdoEntryIdx {
                            idx: Idx::from(e.index),
                            sub_idx: SubIdx::from(e.sub_index.unwrap_or(1) as u8),
                        },
                        bit_len: e.bit_len as u8,
                        name: e.name.clone().unwrap_or(String::new()),
                        pos: PdoEntryPos::from(i as u8),
                    })
                    .collect(),
            })
            .collect();

        let output = SmCfg::output(2.into());
        let input = SmCfg::input(3.into());

        config.config_sm_pdos(output, &rx_pdos)?;
        config.config_sm_pdos(input, &tx_pdos)?;

        for pdo in &rx_pdos {
            // Positions of RX PDO
            log::debug!("Positions of RX PDO 0x{:X}:", u16::from(pdo.idx));
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                entry_offsets.insert(entry.entry_idx, (entry.bit_len, offset));
            }
        }
        for pdo in &tx_pdos {
            // Positions of TX PDO
            log::debug!("Positions of TX PDO 0x{:X}:", u16::from(pdo.idx));
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                entry_offsets.insert(entry.entry_idx, (entry.bit_len, offset));
            }
        }

        let cfg_index = config.index();
        let cfg_info = master.get_config_info(cfg_index)?;
        log::info!("Config info: {:#?}", cfg_info);
        if cfg_info.slave_position.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unable to configure slave",
            ));
        }
        offsets.insert(slave_pos, entry_offsets);
    }
    Ok((master, domain_idx, offsets))
}
