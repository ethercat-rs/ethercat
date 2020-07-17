use ethercat::{Master, MasterAccess, SdoEntryAddr, SdoIdx, SdoPos, SlavePos, SubIdx};

pub fn main() -> Result<(), std::io::Error> {
    let mut master = Master::open(0, MasterAccess::ReadOnly)?;
    let slave_pos = SlavePos::from(0);
    let slave = master.get_slave_info(slave_pos)?;
    for i in 0..slave.sdo_count {
        let sdo_info = master.get_sdo(slave_pos, SdoPos::from(i))?;
        for j in 0..u8::from(sdo_info.max_sub_idx) + 1 {
            let addr = SdoEntryAddr::ByIdx(SdoIdx {
                idx: sdo_info.idx,
                sub_idx: SubIdx::from(j),
            });
            let entry = master.get_sdo_entry(slave_pos, addr)?;
            println!("{:#?}", entry);
        }
    }
    Ok(())
}
