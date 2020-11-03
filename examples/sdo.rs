use ethercat::{AlState, Master, MasterAccess, SdoEntryAddr, SdoIdx, SdoPos, SlavePos, SubIdx};

pub fn main() -> Result<(), std::io::Error> {
    let slave_pos = SlavePos::from(0);
    let mut master = Master::open(0, MasterAccess::ReadWrite)?;
    master.request_state(slave_pos, AlState::Preop)?;
    #[cfg(feature = "sncn")]
    master.dict_upload(slave_pos)?;
    let sdo_count = master.get_slave_info(slave_pos)?.sdo_count;
    if sdo_count == 0 {
        println!("Could not find any SDOs");
        return Ok(());
    }
    for i in 0..sdo_count {
        let sdo_info = master.get_sdo(slave_pos, SdoPos::from(i))?;
        for j in 0..=u8::from(sdo_info.max_sub_idx) {
            let addr = SdoEntryAddr::ByIdx(SdoIdx {
                idx: sdo_info.idx,
                sub_idx: SubIdx::from(j),
            });
            let entry = master.get_sdo_entry(slave_pos, addr)?;
            println!("0x{:X}/{} = {:#?}", u16::from(sdo_info.idx), j, entry);
        }
    }
    Ok(())
}
