use ethercat as ec;
use std::{thread, time::Duration};

pub fn main() -> Result<(), std::io::Error> {
    let mut master = ec::Master::reserve(0)?;
    let domain_handle = master.create_domain()?;

    // ID of Weidmüller UR20 I/O coupler
    let slave_id = ec::SlaveId {
        vendor_id: 0x230,
        product_code: 0x4f911c30,
    };

    // Index of digital output 0 (UR20-RO-CO-255)
    let rx_entry_index_0 = ec::PdoEntryIndex {
        index: 0x7000,
        subindex: 1,
    };

    // Index of digital output 3 (UR20-RO-CO-255)
    let rx_entry_index_3 = ec::PdoEntryIndex {
        index: 0x7000,
        subindex: 4,
    };

    // Index of module state (UR20-RO-CO-255)
    let tx_entry_index = ec::PdoEntryIndex {
        index: 0x6000,
        subindex: 1,
    };

    // PDO entries of digital output 0 and 3
    let rx_pdo_entries = vec![
        ec::PdoEntryInfo {
            index: rx_entry_index_0,
            bit_length: 1,
        },
        ec::PdoEntryInfo {
            index: rx_entry_index_3,
            bit_length: 1,
        },
    ];

    // PDO entry of module state
    let tx_pdo_entries = vec![ec::PdoEntryInfo {
        index: tx_entry_index,
        bit_length: 8,
    }];

    // PDO of digital outputs 0 and 3
    let rx_pdo_info = ec::PdoInfo {
        index: 0x1600,
        entries: &rx_pdo_entries,
    };

    // PDO of module states
    let tx_pdo_info = ec::PdoInfo {
        index: 0x1A00,
        entries: &tx_pdo_entries,
    };

    let rx_pdos = vec![rx_pdo_info];
    let tx_pdos = vec![tx_pdo_info];

    // Sync masters
    let infos = vec![
        ec::SyncInfo::output(2, &rx_pdos),
        ec::SyncInfo::input(3, &tx_pdos),
    ];

    let mut config = master.configure_slave(ec::SlaveAddr::ByPos(0), slave_id)?;
    config.config_pdos(&infos)?;

    let pos = config.register_pdo_entry(rx_entry_index_0, domain_handle)?;
    println!(
        "Position of RX entry {:X}.{} is {:?}",
        rx_entry_index_0.index, rx_entry_index_0.subindex, pos
    );
    let pos = config.register_pdo_entry(rx_entry_index_3, domain_handle)?;
    println!(
        "Position of RX entry {:X}.{} is {:?}",
        rx_entry_index_3.index, rx_entry_index_3.subindex, pos
    );
    let pos = config.register_pdo_entry(tx_entry_index, domain_handle)?;
    println!(
        "Position of TX entry {:X}.{} is {:?}",
        tx_entry_index.index, tx_entry_index.subindex, pos
    );

    let cfg_index = config.index();
    let cfg_info = master.get_config_info(cfg_index)?;
    println!("Config info: {:#?}", cfg_info);
    if cfg_info.slave_position.is_none() {
        panic!("Unable to configure slave");
    }

    let info = master.get_info();
    println!("EtherCAT master: {:#?}", info);

    println!("Activate master");
    master.activate()?;

    loop {
        master.receive()?;
        master.domain(domain_handle).process()?;

        println!("Master state: {:?}", master.state());
        println!("Domain state: {:?}", master.domain(domain_handle).state());

        let data = master.domain_data(domain_handle);
        println!("Received data: {:?}", data);

        // Toggle output 3 (bit 1)
        data[0] ^= 0b_0000_0010;
        master.domain(domain_handle).queue()?;
        master.send()?;

        thread::sleep(Duration::from_millis(100));
    }
}
