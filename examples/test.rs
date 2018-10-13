extern crate ethercat;

use ethercat::Master;
use ethercat::types::*;

const SYNC_1859: &[SyncInfo] = &[
    SyncInfo { index: 0,
               direction: SyncDirection::Output,
               watchdog_mode: WatchdogMode::Default,
               pdos: &[
                   PdoInfo { index: 0x1608, entries: &[] },
                   PdoInfo { index: 0x1609, entries: &[] },
                   PdoInfo { index: 0x160a, entries: &[] },
                   PdoInfo { index: 0x160b, entries: &[] },
                   PdoInfo { index: 0x160c, entries: &[] },
                   PdoInfo { index: 0x160d, entries: &[] },
                   PdoInfo { index: 0x160e, entries: &[] },
                   PdoInfo { index: 0x160f, entries: &[] },
               ] },
    SyncInfo { index: 1,
               direction: SyncDirection::Input,
               watchdog_mode: WatchdogMode::Default,
               pdos: &[
                   PdoInfo { index: 0x1a00, entries: &[] },
                   PdoInfo { index: 0x1a01, entries: &[] },
                   PdoInfo { index: 0x1a02, entries: &[] },
                   PdoInfo { index: 0x1a03, entries: &[] },
                   PdoInfo { index: 0x1a04, entries: &[] },
                   PdoInfo { index: 0x1a05, entries: &[] },
                   PdoInfo { index: 0x1a06, entries: &[] },
                   PdoInfo { index: 0x1a07, entries: &[] },
               ] },
];


fn main() {
    let mut master = Master::reserve(0).unwrap();
    let domain = master.create_domain().unwrap();
    let mut sc1 = master.configure_slave(SlaveAddr::ByPos(1), SlaveId::EL(1859)).unwrap();
    sc1.config_pdos(SYNC_1859).unwrap();
    let ix_in = sc1.register_pdo_entry(PdoEntryIndex { index: 0x6010, subindex: 1}, domain).unwrap();
    let ix_out = sc1.register_pdo_entry(PdoEntryIndex { index: 0x7090, subindex: 1}, domain).unwrap();

    master.activate().unwrap();
    // XXX setprio -19, mlockall

    let mut blink = 0x6;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
        master.receive().unwrap();
        master.domain(domain).process().unwrap();
        let data = master.domain_data(domain);

        println!("in: {}", data[ix_in.byte]);
        blink = if blink == 0x6 { 0x9 } else { 0x6 };
        data[ix_out.byte] = blink;

        master.domain(domain).queue().unwrap();
        master.send().unwrap();
    }
}
