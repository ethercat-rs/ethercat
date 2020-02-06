pub fn main() -> Result<(), std::io::Error> {
    let master = ethercat::Master::reserve(0)?;
    let info = master.get_info();
    println!("EtherCAT Master: {:#?}", info);
    Ok(())
}
