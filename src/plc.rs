//! Wrap an EtherCAT master and slave configuration and provide a PLC-like
//! environment for cyclic task execution.

use std::{thread, time::Duration, marker::PhantomData};
use time::precise_time_ns;

use crate::{Result, Master};
use crate::image::ProcessImage;
use crate::types::*;

#[derive(Default)]
pub struct PlcBuilder {
    master_id: Option<u32>,
    cycle_freq: Option<u32>,
}

impl PlcBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn master_id(mut self, id: u32) -> Self {
        self.master_id = Some(id);
        self
    }

    pub fn cycle_freq(mut self, freq: u32) -> Self {
        self.cycle_freq = Some(freq);
        self
    }

    pub fn build<T: ProcessImage>(self) -> Result<Plc<T>> {
        let mut master = Master::reserve(self.master_id.unwrap_or(0))?;
        let domain = master.create_domain()?;

        for i in 0..T::slave_count() {
            let mut config = master.configure_slave(SlaveAddr::ByPos(i as u16),
                                                    T::get_slave_id(i))?;
            if let Some(pdos) = T::get_slave_pdos(i) {
                config.config_pdos(pdos)?;
            }
            // XXX: SDOs etc.
            for (entry, expected_position) in T::get_slave_regs(i) {
                let pos = config.register_pdo_entry(*entry, domain)?;
                if &pos != expected_position {
                    panic!("slave {}: {:?} != {:?}", i, pos, expected_position);
                }
            }
        }

        let domain_size = master.domain(domain).size()?;
        if domain_size != T::size() {
            panic!("size: {} != {}", domain_size, T::size());
        }

        master.activate()?;

        Ok(Plc {
            master: master,
            domain: domain,
            sleep: 1000_000_000 / self.cycle_freq.unwrap_or(1000) as u64,
            image_type: PhantomData,
        })
    }
}


pub struct Plc<T> {
    master: Master,
    domain: DomainHandle,
    sleep: u64,
    image_type: PhantomData<T>,
}

impl<T: ProcessImage> Plc<T> {
    pub fn run<F>(&mut self, mut cycle_fn: F)
    where F: FnMut(&mut T)
    {
        let mut epoch = precise_time_ns();
        loop {
            if let Err(e) = self.single_cycle(&mut cycle_fn) {
                // XXX: bad!
                eprintln!("error in cycle: {}", e);
            }

            epoch += self.sleep;
            thread::sleep(Duration::from_nanos(epoch - precise_time_ns()));
        }
    }

    fn single_cycle<F>(&mut self, mut cycle_fn: F) -> Result<()>
    where F: FnMut(&mut T)
    {
        self.master.receive()?;
        self.master.domain(self.domain).process()?;

        // println!("domain state: {:?}", self.master.domain(self.domain).state());
        println!("master state: {:?}", self.master.state());
        // println!("slave state: {:?}", self.master.configure_slave(
            // SlaveAddr::ByPos(1), SlaveId::EL(1859)
        // ).map(|sc| sc.state()));

        let ddata = self.master.domain_data(self.domain);
        // println!("< {:?}", ddata);

        let data = T::cast(ddata);
        cycle_fn(data);

        // println!("> {:?}", ddata);

        self.master.domain(self.domain).queue()?;
        self.master.send()?;
        Ok(())
    }
}
