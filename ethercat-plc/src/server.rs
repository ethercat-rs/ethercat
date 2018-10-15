//! Modbus server allowing access to the PLC "memory" variables.

use std::collections::BTreeMap;
use std::io::{Result, Read, Write, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::thread;
use byteorder::{ByteOrder, BE};
use crossbeam_channel::{unbounded, Sender, Receiver};


#[derive(Debug)]
pub(crate) struct Request {
    pub hid: usize,
    pub tid: u16,
    pub fc: u8,
    pub addr: usize,
    pub count: usize,
    pub write: Option<Vec<u16>>,
}

#[derive(Debug)]
pub(crate) enum Response {
    Ok(Request, Vec<u16>),
    Error(Request, u8),
}

enum HandlerEvent {
    Request(Request),
    New((usize, Sender<Response>)),
    Finished(usize),
}

struct Handler {
    hid:      usize,
    client:   TcpStream,
    requests: Sender<HandlerEvent>,
}

pub struct Server {
    to_plc:   Sender<Request>,
    from_plc: Receiver<Response>,
}

impl Handler {
    pub fn new(client: TcpStream, hid: usize, requests: Sender<HandlerEvent>,
               replies: Receiver<Response>) -> Self
    {
        let send_client = client.try_clone().expect("could not clone socket");
        thread::spawn(move || Handler::sender(send_client, replies));
        Handler { client, hid, requests }
    }

    fn sender(mut client: TcpStream, replies: Receiver<Response>) {
        let mut buf = [0u8; 256];
        mlzlog::set_thread_prefix(format!("{} sender: ", client.peer_addr().unwrap()));

        for response in replies {
            debug!("sending response: {:?}", response);
            let count = match response {
                Response::Ok(req, values) => {
                    BE::write_u16(&mut buf, req.tid);
                    buf[7] = req.fc;
                    match req.fc {
                        3 | 4 => {
                            let nbytes = 2 * values.len();
                            buf[8] = nbytes as u8;
                            BE::write_u16_into(&values, &mut buf[9..9+nbytes]);
                            9 + nbytes
                        }
                        6 => {
                            BE::write_u16(&mut buf[8..], req.addr as u16);
                            BE::write_u16(&mut buf[10..], values[0]);
                            12
                        }
                        16 => {
                            BE::write_u16(&mut buf[8..], req.addr as u16);
                            BE::write_u16(&mut buf[10..], values.len() as u16);
                            12
                        }
                        x => panic!("impossible function code {}", x)
                    }
                }
                Response::Error(req, ec) => {
                    BE::write_u16(&mut buf, req.tid);
                    buf[7] = req.fc | 0x80;
                    buf[8] = ec;
                    9
                }
            };
            BE::write_u16(&mut buf[4..], (count - 6) as u16);
            if let Err(err) = client.write_all(&buf[..count]) {
                warn!("write error: {}", err);
                break;
            }
        }
    }

    fn handle(mut self) {
        let mut headbuf = [0u8; 8];
        let mut bodybuf = [0u8; 250];  // max frame size is 255
        let mut errbuf  = [0, 0, 0, 0, 0, 9, 0, 0, 0];

        mlzlog::set_thread_prefix(format!("{}: ", self.client.peer_addr().unwrap()));
        info!("connection accepted");

        'outer: loop {
            if let Err(err) = self.client.read_exact(&mut headbuf) {
                if err.kind() != ErrorKind::UnexpectedEof {
                    warn!("error reading request head: {}", err);
                }
                break;
            }
            if &headbuf[2..4] != &[0, 0] {
                warn!("protocol ID mismatch: {:?}", headbuf);
                break;
            }
            let tid = BE::read_u16(&headbuf);
            let data_len = BE::read_u16(&headbuf[4..6]) as usize;
            if let Err(err) = self.client.read_exact(&mut bodybuf[..data_len - 2]) {
                warn!("error reading request body: {}", err);
                break;
            }
            if headbuf[6] != 0 {
                warn!("invalid slave {}", headbuf[6]);
                continue;
            }
            let fc = headbuf[7];
            let req = match fc {
                3 | 4 => {
                    if data_len != 6 {
                        warn!("invalid data length for fc {}", fc);
                        continue;
                    }
                    let addr = BE::read_u16(&bodybuf[..2]) as usize;
                    let count = BE::read_u16(&bodybuf[2..4]) as usize;
                    Request { hid: self.hid, tid, fc, addr, count, write: None }
                }
                6 => {
                    if data_len != 6 {
                        warn!("invalid data length for fc {}", fc);
                        continue;
                    }
                    let addr = BE::read_u16(&bodybuf[..2]) as usize;
                    let value = BE::read_u16(&bodybuf[2..4]);
                    Request { hid: self.hid, tid, fc, addr, count: 1, write: Some(vec![value]) }
                }
                16 => {
                    if data_len < 7 {
                        warn!("insufficient data length for fc {}", fc);
                        continue;
                    }
                    let addr = BE::read_u16(&bodybuf[..2]) as usize;
                    let bytecount = bodybuf[4] as usize;
                    if data_len != 7 + bytecount {
                        warn!("invalid data length for fc {}", fc);
                        continue;
                    }
                    let mut values = vec![0; bytecount / 2];
                    BE::read_u16_into(&bodybuf[5..5+bytecount], &mut values);
                    Request { hid: self.hid, tid, fc, addr, count: values.len(), write: Some(values) }
                }
                _ => {
                    warn!("unknown function code {}", fc);
                    BE::write_u16(&mut errbuf, tid);
                    errbuf[7] = fc | 0x80;
                    errbuf[8] = 1;
                    if let Err(err) = self.client.write_all(&errbuf) {
                        warn!("error writing error response: {}", err);
                        break;
                    }
                    continue;
                }
            };
            debug!("got request: {:?}", req);
            self.requests.send(HandlerEvent::Request(req));
        }
        info!("connection closed");
        self.requests.send(HandlerEvent::Finished(self.hid));
    }
}

impl Server {
    pub(crate) fn new() -> (Self, Receiver<Request>, Sender<Response>) {
        let (w_to_plc, r_to_plc) = unbounded();
        let (w_from_plc, r_from_plc) = unbounded();
        (Server { to_plc: w_to_plc, from_plc: r_from_plc }, r_to_plc, w_from_plc)
    }

    /// Listen for connections on the TCP socket and spawn handlers for it.
    fn tcp_listener(tcp_sock: TcpListener, handler_sender: Sender<HandlerEvent>) {
        mlzlog::set_thread_prefix("Modbus: ".into());

        info!("listening on {}", tcp_sock.local_addr().unwrap());
        let mut handler_id = 0;

        while let Ok((stream, _)) = tcp_sock.accept() {
            let (w_rep, r_rep) = unbounded();
            let w_req = handler_sender.clone();
            handler_id += 1;
            w_req.send(HandlerEvent::New((handler_id, w_rep)));
            thread::spawn(move || Handler::new(stream, handler_id, w_req, r_rep).handle());
        }
    }

    fn dispatcher(self, r_clients: Receiver<HandlerEvent>) {
        mlzlog::set_thread_prefix("Dispatcher: ".into());

        let mut handlers = BTreeMap::new();

        for event in r_clients {
            match event {
                HandlerEvent::New((id, chan)) => {
                    handlers.insert(id, chan);
                }
                HandlerEvent::Finished(id) => {
                    handlers.remove(&id);
                }
                HandlerEvent::Request(req) => {
                    let hid = req.hid;
                    self.to_plc.send(req);
                    let resp = self.from_plc.recv().unwrap();
                    handlers[&hid].send(resp);
                }
            }
        }
    }

    pub fn start(self, addr: &str) -> Result<()> {
        let (w_clients, r_clients) = unbounded();
        let tcp_sock = TcpListener::bind(addr)?;

        thread::spawn(move || Server::tcp_listener(tcp_sock, w_clients));
        thread::spawn(move || Server::dispatcher(self, r_clients));

        Ok(())
    }
}
