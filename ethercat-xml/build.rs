use std::collections::{HashMap, HashSet};
use std::{env, fs, io, str};
use std::borrow::Cow;
use std::io::Write;
use std::path::{Path, PathBuf};
use heck::SnakeCase;
use rayon::prelude::*;
use quick_xml::{Reader, events::{Event, BytesStart}};

type XmlReader = Reader<io::BufReader<fs::File>>;

fn parse_number(bytes: &[u8]) -> u32 {
    let s = str::from_utf8(bytes).unwrap();
    if s.starts_with("#x") {
        u32::from_str_radix(&s[2..], 16).unwrap()
    } else {
        s.parse().unwrap()
    }
}

fn get_attr<'a, 'b>(tag: &'a BytesStart<'a>, name: &'b [u8]) -> &'a [u8] {
    for attr in tag.attributes() {
        if let Ok(attr) = attr {
            if attr.key == name {
                if let Cow::Borrowed(value) = attr.value {
                    return value;
                }
            }
        }
    }
    &[]
}

fn get_tag_bytes(reader: &mut XmlReader) -> Vec<u8> {
    let mut buf = Vec::new();
    match reader.read_event(&mut buf) {
        Ok(Event::Text(bytes)) | Ok(Event::CData(bytes)) =>
            bytes.unescaped().unwrap().into_owned(),
        Ok(Event::End(_)) => Vec::new(),
        x => panic!("expected tag text: {:?}", x)
    }
}

fn get_tag_text(reader: &mut XmlReader) -> String {
    let mut buf = Vec::new();
    match reader.read_event(&mut buf) {
        Ok(Event::Text(bytes)) | Ok(Event::CData(bytes)) =>
            bytes.unescape_and_decode(reader).unwrap(),
        Ok(Event::End(_)) => String::new(),
        x => panic!("expected tag text: {:?}", x)
    }
}

fn skip_tag(starttag: &[u8], reader: &mut XmlReader) {
    let mut buf = Vec::new();
    let mut nest = 1;
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref tag)) if tag.name() == starttag => nest += 1,
            Ok(Event::End(ref tag)) if tag.name() == starttag => {
                nest -= 1;
                if nest == 0 {
                    return;
                }
            }
            Ok(Event::Eof) => panic!("unexpected eof"),
            _ => {}
        }
    }
}

#[derive(Default, Debug)]
struct PdoEntry {
    index: u16,
    subindex: u16,
    bit_len: u16,
    name: String,
}

#[derive(Default, Debug)]
struct Pdo {
    sm: u8,
    index: u16,
    name: String,
    excludes: Vec<u16>,
    entries: Vec<PdoEntry>,
}

#[derive(Default, Debug)]
struct Mapping {
    name: String,
    entries: Vec<(u8, Vec<u16>)>,  // Sm, [PdoIndex]
}

#[derive(Default, Debug)]
struct Device {
    group: String,
    name: String,
    desc: String,
    product: u32,
    revision: u32,
    hiding: Vec<u32>,
    mappings: Vec<Mapping>,
    tx_pdos: Vec<Pdo>,
    rx_pdos: Vec<Pdo>,
}

impl Device {
    fn process_mapping(&mut self, reader: &mut XmlReader) -> io::Result<()> {
        let mut buf = Vec::new();
        let mut map = Mapping::default();
        let mut sm = (0, Vec::new());
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref tag)) => match tag.name() {
                    b"Name" => map.name = get_tag_text(reader),
                    b"Sm" => {
                        let smno = get_attr(tag, b"No");
                        if !smno.is_empty() {
                            sm.0 = parse_number(&smno) as u8;
                        }
                    }
                    b"Pdo" => {
                        let index = get_tag_bytes(reader);
                        if !index.is_empty() {
                            sm.1.push(parse_number(&index) as u16);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(ref tag)) => match tag.name() {
                    b"AlternativeSmMapping" => {
                        self.mappings.push(map);
                        return Ok(());
                    }
                    b"Sm" => {
                        if !sm.1.is_empty() {
                            map.entries.push(sm);
                        }
                        sm = (0, Vec::new());
                    }
                    _ => {}
                },
                Ok(Event::Eof) => panic!("unexpected eof"),
                _ => {}
            }
        }
    }

    fn process_pdo_entry(&mut self, reader: &mut XmlReader, pdo: &mut Pdo) -> io::Result<()> {
        let mut buf = Vec::new();
        let mut entry = PdoEntry::default();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref tag)) => match tag.name() {
                    b"Name" => entry.name = get_tag_text(reader),
                    b"Index" => entry.index = parse_number(&get_tag_bytes(reader)) as u16,
                    b"SubIndex" => entry.subindex = parse_number(&get_tag_bytes(reader)) as u16,
                    b"BitLen" => entry.bit_len = parse_number(&get_tag_bytes(reader)) as u16,
                    _ => {}
                },
                Ok(Event::End(ref tag)) => match tag.name() {
                    b"Entry" => { pdo.entries.push(entry); return Ok(()) }
                    _ => {}
                }
                Ok(Event::Eof) => panic!("unexpected eof"),
                _ => {}
            }
        }
    }

    fn process_pdo(&mut self, reader: &mut XmlReader, sm: u8) -> io::Result<()> {
        let mut buf = Vec::new();
        let mut pdo = Pdo::default();
        pdo.sm = sm;
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref tag)) => match tag.name() {
                    b"Index" => pdo.index = parse_number(&get_tag_bytes(reader)) as u16,
                    b"Name" => pdo.name = get_tag_text(reader),
                    b"Exclude" => pdo.excludes.push(parse_number(&get_tag_bytes(reader)) as u16),
                    b"Entry" => self.process_pdo_entry(reader, &mut pdo)?,
                    _ => {}
                },
                Ok(Event::End(ref tag)) => match tag.name() {
                    b"TxPdo" => { self.tx_pdos.push(pdo); return Ok(()) }
                    b"RxPdo" => { self.rx_pdos.push(pdo); return Ok(()) }
                    _ => {}
                }
                Ok(Event::Eof) => panic!("unexpected eof"),
                _ => {}
            }
        }
    }

    fn process(mut self, reader: &mut XmlReader) -> io::Result<Self> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref tag)) => match tag.name() {
                    b"GroupType" => self.group = get_tag_text(reader).to_snake_case(),
                    b"Name" if get_attr(tag, b"LcId") == b"1033" => self.desc = get_tag_text(reader),
                    b"Type" => {
                        let product = get_attr(tag, b"ProductCode");
                        if !product.is_empty() {
                            self.product = parse_number(&product);
                            self.revision = parse_number(&get_attr(tag, b"RevisionNo"));
                        }
                        self.name = get_tag_text(reader);
                        // println!("found device: {:#x} {:#x} - {}",
                        //          self.product, self.revision, self.name);
                    },
                    b"HideType" => {
                        let revno = get_attr(tag, b"RevisionNo");
                        if !revno.is_empty() {
                            self.hiding.push(parse_number(&revno));
                        }
                    }
                    b"AlternativeSmMapping" => self.process_mapping(reader)?,
                    b"TxPdo" | b"RxPdo" => {
                        let sm = get_attr(tag, b"Sm");
                        let sm = if !sm.is_empty() { parse_number(&sm) as u8 } else { 255 };
                        self.process_pdo(reader, sm)?;
                    },
                    b"Profile" | b"Port" | b"ExecutionUnit" | b"SettingsTab" | b"Dc" |
                    b"Slots" | b"Electrical" =>
                        skip_tag(tag.name(), reader),
                    _ => {}
                }
                Ok(Event::End(tag)) => match tag.name() {
                    b"Device" => return Ok(self),
                    _ => {}
                }
                Ok(Event::Eof) => panic!("unexpected eof"),
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
                // Ok(ev) => { eprintln!("{:?}", ev); },
                Ok(_) => {}
            }
        }
    }
}

fn process(path: &Path) -> io::Result<Vec<Device>> {
    let fp = fs::File::open(path)?;
    let file = io::BufReader::new(fp);
    let mut reader = Reader::from_reader(file);
    reader.trim_text(true);
    reader.expand_empty_elements(true);
    let mut buf = Vec::new();
    let mut devices = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(tag)) => match tag.name() {
                // b"Vendor" => TODO: record vendor Id
                b"Device" => devices.push(Device::default().process(&mut reader)?),
                _ => {}
            }
            Ok(Event::Eof) => return Ok(devices),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            Ok(_) => {},
        }
    }
}

fn write(fp: &mut fs::File, seen: &mut HashSet<String>, dev: Device) -> io::Result<()> {
    // TODO: regex?
    let struct_name = dev.name.replace(&['-', '.', ' ', '/'][..], "_");

    if seen.contains(&struct_name) {
        return Ok(());
    }

    writeln!(fp, "#[repr(C, packed)]")?;
    writeln!(fp, "/// {}", dev.desc)?;
    writeln!(fp, "// revision: {:#x}", dev.revision)?;
    writeln!(fp, "pub struct {} {{", struct_name)?;
    writeln!(fp, "}}\n")?;

    writeln!(fp, "impl ProcessImage for {} {{", struct_name)?;
    writeln!(fp, "    const SLAVE_COUNT: usize = 1;")?;
    writeln!(fp, "    fn get_slave_ids() -> Vec<SlaveId> {{ vec![SlaveId {{ \
                  vendor_id: 2, product_code: {:#x} }}] }}",
             dev.product)?;
    writeln!(fp, "}}\n\n")?;

    seen.insert(struct_name);
    Ok(())
}

fn main() {
    let path = match env::var("BECKHOFF_XML_PATH") {
        Ok(path) => PathBuf::from(&path),
        Err(_) => PathBuf::from("../xml")
    };

    // collect XML filenames
    let paths = fs::read_dir(&path).unwrap().filter_map(|file| {
        let path = file.unwrap().path();
        if path.extension().map_or(false, |e| e == "xml") { Some(path) } else { None }
    }).collect::<Vec<_>>();

    // extract all device info from XMLs
    let devices = paths.into_par_iter().flat_map(|path| {
        println!("cargo:rerun-if-changed={}", path.display());
        eprintln!("processing {}", path.display());
        process(&path).unwrap_or_else(|e| {
            eprintln!("error in {}: {}", path.display(), e);
            vec![]
        })
    }).collect::<Vec<_>>();

    // construct the blacklist, from "hiding" declarations in XML
    let blacklist = devices.iter().flat_map(|dev| {
        dev.hiding.iter().map(|rev| (dev.product, *rev)).collect::<Vec<_>>()
    }).collect::<HashSet<_>>();

    // assemble a list of group names and prepare a map group -> devices
    let root = PathBuf::from(env::var("OUT_DIR").unwrap());
    let all_groups = devices.iter().map(|dev| &dev.group).collect::<HashSet<_>>();
    let mut all_groups = all_groups.into_iter().filter_map(|g| {
        // filter out empty groups and evaluation boards
        if g.is_empty() || g.starts_with("eva_board") {
            None
        } else {
            Some((g.to_owned(), (root.join(&format!("{}.rs", g)), vec![])))
        }
    }).collect::<HashMap<_, _>>();

    // move devices into the map by group
    for dev in devices {
        if let Some(group) = all_groups.get_mut(&dev.group) {
            group.1.push(dev);
        }
    }

    // create the main output file, it just references the group modules
    let mainfile = root.join("generated.rs");
    let mut fp = fs::File::create(&mainfile).unwrap();
    for group in all_groups.keys() {
        writeln!(fp, "mod {};\npub use self::{}::*;", group, group);
    }
    drop(fp);

    // create a module per group
    all_groups.into_par_iter().for_each(|(_, (groupfile, devices))| {
        let mut fp = fs::File::create(&groupfile).unwrap();
        writeln!(fp, "use ethercat::*;\nuse ethercat_plc::ProcessImage;\n").unwrap();
        let mut seen = HashSet::new();

        for dev in devices {
            if !blacklist.contains(&(dev.product, dev.revision)) {
                if dev.product != 0 {
                    write(&mut fp, &mut seen, dev).unwrap();
                }
            }
        }
    });
}
