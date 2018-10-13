//! Wrap an EtherCAT master and slave configuration and provide a PLC-like
//! environment for cyclic task execution.

use std::marker::PhantomData;

use crate::{Result, Master};
use crate::image::ProcessImage;
use crate::types::*;

#[derive(Default)]
pub struct PlcBuilder {
    master_id: Option<u32>,
}

impl PlcBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn master_id(mut self, id: u32) -> Self {
        self.master_id = Some(id);
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
            for entry in T::get_slave_regs(i) {
                config.register_pdo_entry(*entry, domain)?;
            }
        }

        master.activate()?;

        Ok(Plc {
            master: master,
            domain: domain,
            image_type: PhantomData,
        })
    }
}


pub struct Plc<T> {
    master: Master,
    domain: DomainHandle,
    image_type: PhantomData<T>,
}

impl<T: ProcessImage> Plc<T> {
    pub fn run<F>(&mut self, mut cycle_fn: F)
    where F: FnMut(&mut T)
    {
        loop {
            if let Err(e) = self.single_cycle(&mut cycle_fn) {
                // XXX: bad!
                eprintln!("error in cycle: {}", e);
            }
        }
    }

    fn single_cycle<F>(&mut self, mut cycle_fn: F) -> Result<()>
    where F: FnMut(&mut T)
    {
        self.master.receive()?;
        self.master.domain(self.domain).process()?;
        let ddata = self.master.domain_data(self.domain);
        let data = T::cast(ddata);

        cycle_fn(data);

        println!("cyc: {:?}", ddata);

        self.master.domain(self.domain).queue()?;
        self.master.send()?;
        Ok(())
    }
}
