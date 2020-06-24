use ethercat::{Master, MasterAccess};

pub fn main() -> Result<(), std::io::Error> {
    let master = Master::open(0, MasterAccess::ReadWrite)?;
    let info = master.get_info();
    println!("EtherCAT Master: {:#?}", info);
    Ok(())
}
