//! Wrap an EtherCAT master and slave configuration and provide a PLC-like
//! environment for cyclic task execution.

use std::{thread, time::Duration, marker::PhantomData};
use time::precise_time_ns;
use byteorder::{ByteOrder, NativeEndian as NE};
use crossbeam_channel::{Sender, Receiver};
use mlzlog;

use crate::{Result, Master};
use crate::image::{ProcessImage, ExternImage};
use crate::types::*;
use crate::server::{Server, Request, Response};

#[derive(Default)]
pub struct PlcBuilder {
    master_id: Option<u32>,
    cycle_freq: Option<u32>,
    server: Option<String>,
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

    pub fn server(mut self, addr: impl Into<String>) -> Self {
        self.server = Some(addr.into());
        self
    }

    pub fn build<P: ProcessImage, E: ExternImage>(self) -> Result<Plc<P, E>> {
        // XXX options!
        mlzlog::init::<&str>(None, "plc", false, true, true)?;

        let channels = if let Some(addr) = self.server {
            let (srv, r, w) = Server::new();
            srv.start(&addr)?;
            Some((r, w))
        } else {
            None
        };

        let mut master = Master::reserve(self.master_id.unwrap_or(0))?;
        let domain = master.create_domain()?;

        let slave_ids = P::get_slave_ids();
        let slave_pdos = P::get_slave_pdos();
        let slave_regs = P::get_slave_regs();
        for (i, ((id, pdos), regs)) in slave_ids.into_iter().zip(slave_pdos).zip(slave_regs).enumerate() {
            let mut config = master.configure_slave(SlaveAddr::ByPos(i as u16), id)?;
            if let Some(pdos) = pdos {
                config.config_pdos(&pdos)?;
            }
            let mut first_byte = 0;
            for (j, (entry, mut expected_position)) in regs.into_iter().enumerate() {
                let pos = config.register_pdo_entry(entry, domain)?;
                if j == 0 {
                    if pos.bit != 0 {
                        panic!("first PDO of slave {} not byte-aligned", i);
                    }
                    first_byte = pos.byte;
                } else {
                    expected_position.byte += first_byte;
                    if pos != expected_position {
                        panic!("slave {} pdo {}: {:?} != {:?}", i, j, pos, expected_position);
                    }
                }
            }
            // XXX: SDOs etc.
        }

        // XXX: check actual slaves against configuration

        let domain_size = master.domain(domain).size()?;
        if domain_size != P::size() {
            panic!("size: {} != {}", domain_size, P::size());
        }

        master.activate()?;

        Ok(Plc {
            master: master,
            domain: domain,
            server: channels,
            sleep: 1000_000_000 / self.cycle_freq.unwrap_or(1000) as u64,
            _types: (PhantomData, PhantomData),
        })
    }
}


pub struct Plc<P, E> {
    master: Master,
    domain: DomainHandle,
    sleep:  u64,
    server: Option<(Receiver<(usize, Request)>, Sender<(usize, Response)>)>,
    _types: (PhantomData<P>, PhantomData<E>),
}

impl<P: ProcessImage, E: ExternImage> Plc<P, E> {
    pub fn run<F>(&mut self, mut cycle_fn: F)
    where F: FnMut(&mut P, &mut E)
    {
        let mut ext = E::default();
        let mut epoch = precise_time_ns();
        loop {
            if let Err(e) = self.single_cycle(&mut cycle_fn, &mut ext) {
                // XXX: logging unconditionally here is bad, could repeat endlessly
                warn!("error in cycle: {}", e);
            }

            if let Some((r, w)) = self.server.as_mut() {
                while let Some((id, req)) = r.try_recv() {
                    debug!("PLC got request from {}: {:?}", id, req);
                    let data = ext.cast();
                    let resp = match req {
                        Request::Read(tid, fc, addr, count) => {
                            if addr + count >= E::size()/2 {
                                Response::Error(tid, fc, 2)
                            } else {
                                let mut values = vec![0; count];
                                NE::read_u16_into(&data[addr*2..addr*2+count*2], &mut values);
                                Response::Ok(tid, fc, addr, values)
                            }
                        }
                        Request::Write(tid, fc, addr, values) => {
                            if addr + values.len() >= E::size()/2 {
                                Response::Error(tid, fc, 2)
                            } else {
                                NE::write_u16_into(&values, &mut data[addr*2..addr*2+values.len()*2]);
                                Response::Ok(tid, fc, addr, values)
                            }
                        }
                    };
                    debug!("PLC response: {:?}", resp);
                    w.send((id, resp));
                }
            }

            epoch += self.sleep;
            thread::sleep(Duration::from_nanos(epoch - precise_time_ns()));
        }
    }

    fn single_cycle<F>(&mut self, mut cycle_fn: F, ext: &mut E) -> Result<()>
    where F: FnMut(&mut P, &mut E)
    {
        self.master.receive()?;
        self.master.domain(self.domain).process()?;

        // XXX: check working counters periodically, etc.
        // println!("master state: {:?}", self.master.state());
        // println!("domain state: {:?}", self.master.domain(self.domain).state());

        let data = P::cast(self.master.domain_data(self.domain));
        cycle_fn(data, ext);

        self.master.domain(self.domain).queue()?;
        self.master.send()?;
        Ok(())
    }
}
