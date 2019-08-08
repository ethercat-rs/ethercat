// Part of ethercat-rs. Copyright 2018-2019 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

//! Wrap an EtherCAT master and slave configuration and provide a PLC-like
//! environment for cyclic task execution.

use std::{thread, time::Duration, marker::PhantomData};
use time::precise_time_ns;
use byteorder::{ByteOrder, NativeEndian as NE};
use crossbeam_channel::{Sender, Receiver};
use mlzlog;
use log::*;

use ethercat::*;

use crate::image::{ProcessImage, ExternImage};
use crate::server::{Server, Request, Response};

#[derive(Default)]
pub struct PlcBuilder {
    name: String,
    master_id: Option<u32>,
    cycle_freq: Option<u32>,
    server_addr: Option<String>,
    logfile_base: Option<String>,
    debug_logging: bool,
}

impl PlcBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            .. Self::default()
        }
    }

    pub fn master_id(mut self, id: u32) -> Self {
        self.master_id = Some(id);
        self
    }

    pub fn cycle_freq(mut self, freq: u32) -> Self {
        self.cycle_freq = Some(freq);
        self
    }

    pub fn with_server(mut self, addr: impl Into<String>) -> Self {
        self.server_addr = Some(addr.into());
        self
    }

    pub fn logging_cfg(mut self, logfile_base: Option<String>, debug_logging: bool) -> Self {
        self.logfile_base = logfile_base;
        self.debug_logging = debug_logging;
        self
    }

    pub fn build<P: ProcessImage, E: ExternImage>(self) -> Result<Plc<P, E>> {
        mlzlog::init(self.logfile_base, &self.name, false, self.debug_logging, true)?;

        let channels = if let Some(addr) = self.server_addr {
            let (srv, r, w) = Server::new();
            srv.start(&addr)?;
            Some((r, w))
        } else {
            None
        };

        let mut master = Master::reserve(self.master_id.unwrap_or(0))?;
        let domain = master.create_domain()?;

        debug!("PLC: EtherCAT master opened");

        // XXX
        // master.sdo_download(1, SdoIndex::new(0x1011, 1), &0x64616F6Cu32)?;
        // master.sdo_download(2, SdoIndex::new(0x1011, 1), &0x64616F6Cu32)?;

        let slave_ids = P::get_slave_ids();
        let slave_pdos = P::get_slave_pdos();
        let slave_regs = P::get_slave_regs();
        let slave_sdos = P::get_slave_sdos();
        for (i, (((id, pdos), regs), sdos)) in slave_ids.into_iter()
                                                        .zip(slave_pdos)
                                                        .zip(slave_regs)
                                                        .zip(slave_sdos)
                                                        .enumerate()
        {
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

            for (sdo_index, data) in sdos {
                config.add_sdo(sdo_index, &*data)?;
            }

            let cfg_index = config.index();
            drop(config);

            // ensure that the slave is actually present
            if master.get_config_info(cfg_index)?.slave_position.is_none() {
                panic!("slave {} does not match config", i);
            }
        }

        info!("PLC: EtherCAT slaves configured");

        let domain_size = master.domain(domain).size()?;
        if domain_size != P::size() {
            panic!("size: {} != {}", domain_size, P::size());
        }

        master.activate()?;
        info!("PLC: EtherCAT master activated");

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
    server: Option<(Receiver<Request>, Sender<Response>)>,
    _types: (PhantomData<P>, PhantomData<E>),
}

const BASE: usize = 0x3000;

impl<P: ProcessImage, E: ExternImage> Plc<P, E> {
    pub fn run<F>(&mut self, mut cycle_fn: F)
    where F: FnMut(&mut P, &mut E)
    {
        let mut ext = E::default();
        let mut cycle_start = precise_time_ns();

        loop {
            // process data exchange + logic
            if let Err(e) = self.single_cycle(&mut cycle_fn, &mut ext) {
                // XXX: logging unconditionally here is bad, could repeat endlessly
                warn!("error in cycle: {}", e);
            }

            // external data exchange via modbus
            if let Some((r, w)) = self.server.as_mut() {
                while let Ok(mut req) = r.try_recv() {
                    debug!("PLC got request: {:?}", req);
                    let data = ext.cast();
                    let resp = if req.addr < BASE || req.addr + req.count > BASE + E::size()/2 {
                        Response::Error(req, 2)
                    } else {
                        let from = 2 * (req.addr - BASE);
                        let to = from + 2 * req.count;
                        if let Some(ref mut values) = req.write {
                            // write request
                            NE::write_u16_into(values, &mut data[from..to]);
                            let values = req.write.take().unwrap();
                            Response::Ok(req, values)
                        } else {
                            // read request
                            let mut values = vec![0; req.count];
                            NE::read_u16_into(&data[from..to], &mut values);
                            Response::Ok(req, values)
                        }
                    };
                    debug!("PLC response: {:?}", resp);
                    if let Err(e) = w.send(resp) {
                        warn!("could not send back response: {}", e);
                    }
                }
            }

            // wait until next cycle
            let now = precise_time_ns();
            cycle_start += self.sleep;
            if cycle_start > now {
                thread::sleep(Duration::from_nanos(cycle_start - now));
            }
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
