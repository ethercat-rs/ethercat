use ethercat::{Master, MasterAccess};

fn main() -> Result<(), std::io::Error> {
    let mut master = Master::open(0, MasterAccess::ReadWrite)?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: foe-read <slave-position> <foe-name>");
        return Err(std::io::Error::other(
            "Not enough arguments",
        ));
    }
    let slave_idx: ethercat::SlavePos = args[1]
        .parse::<u16>()
        .map_err(std::io::Error::other)?
        .into();
    let foe_name = &args[2];
    let res = master.foe_read(slave_idx, foe_name)?;
    println!("FoE data: {:x?}, {} bytes", res, res.len());
    Ok(())
}
