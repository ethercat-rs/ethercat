use crate::{Error, Master, SdoEntryAddr, SdoEntryInfo, SdoIdx, SdoInfo, SdoPos, SlavePos, SubIdx};
use std::collections::HashMap;

type Result<T> = std::result::Result<T, Error>;

pub fn slave_sdos(
    master: &mut Master,
    slave_pos: SlavePos,
) -> Result<HashMap<SdoIdx, SdoEntryInfo>> {
    let slave = master.get_slave_info(slave_pos)?;
    let sdo_positions = (0..slave.sdo_count).into_iter().map(SdoPos::from);
    let mut res = HashMap::new();
    for sdo_pos in sdo_positions {
        let SdoInfo {
            idx, max_sub_idx, ..
        } = master.get_sdo(slave_pos, sdo_pos)?;
        let sdo_idxs = (1..=u8::from(max_sub_idx))
            .map(SubIdx::from)
            .map(|sub_idx| SdoIdx { idx, sub_idx });
        for sdo_idx in sdo_idxs {
            let entry = master.get_sdo_entry(slave_pos, SdoEntryAddr::ByIdx(sdo_idx))?;
            res.insert(sdo_idx, entry);
        }
    }
    Ok(res)
}
