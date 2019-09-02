extern crate libusb;

use std::fs;
use std::io;
use std::net;
use std::path;
use std::time::Duration;
use std::vec::Vec;

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8
}

pub struct Usb<'a> {
    _vendor_id: u16,
    _product_id: u16,
    device_handle: libusb::DeviceHandle<'a>,
    write_endpoint: Endpoint,
    stream: Vec<u8>
}

fn find_print_endpoint(context: &mut libusb::Context) -> Option<(Endpoint, u16, u16)> {
    match find_print_device(context) {
        Some((device, device_desc)) => {
            match find_write_endpoint(device, device_desc) {
                Some((endpoint, vendor_id, product_id)) => Some((endpoint, vendor_id, product_id)),
                None => None
            }
        },
        None => None
    }
}

fn find_print_device(context: &mut libusb::Context) -> Option<(libusb::Device, libusb::DeviceDescriptor)> {
    for device in context.devices().unwrap().iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue
        };

        for n in 0..device_desc.num_configurations() {
            let config_desc = match device.config_descriptor(n) {
                Ok(c) => c,
                Err(_) => continue
            };

            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.class_code() == 7 {
                        return Some((device, device_desc));
                    }
                }
            }
        }
    }

    None
}

fn find_write_endpoint(device: libusb::Device, device_desc: libusb::DeviceDescriptor) -> Option<(Endpoint, u16, u16)> {
    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue
        };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    println!("endpoint: {:?} {:?}", endpoint_desc.direction(), endpoint_desc.transfer_type());
                    if endpoint_desc.direction() == libusb::Direction::Out {
                        println!("find writeable endpoint: {:?}", endpoint_desc.address());
                        return Some((
                            Endpoint {
                                config: config_desc.number(),
                                iface: interface_desc.interface_number(),
                                setting: interface_desc.setting_number(),
                                address: endpoint_desc.address()
                            },
                            device_desc.vendor_id(),
                            device_desc.product_id()
                        ));
                    }
                }
            }
        }
    }

    None
}

fn configure_endpoint(handle: &mut libusb::DeviceHandle, endpoint: &Endpoint) -> libusb::Result<()> {
    try!(handle.set_active_configuration(endpoint.config));
    try!(handle.claim_interface(endpoint.iface));
    try!(handle.set_alternate_setting(endpoint.iface, endpoint.setting));
    Ok(())
}

impl<'a> Usb<'a> {
    pub fn new(context: &'a mut libusb::Context, vendor_id: u16, product_id: u16) -> Usb<'a> {
        let empty_stream : Vec<u8> = Vec::new();
        match find_print_endpoint(context) {
            Some((endpoint, vendor_id, product_id)) => {
              let device_handle = context.open_device_with_vid_pid(vendor_id, product_id).unwrap();
              return Usb {
                  _vendor_id: vendor_id,
                  _product_id: product_id,
                  device_handle: device_handle,
                  write_endpoint: endpoint,
                  stream: empty_stream
              }
            },
            None => panic!("can't find print endpoint with device {:04x}:{:04x}", vendor_id, product_id)
        }
    }
}

impl<'a> io::Write for Usb<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.extend(buf.iter().cloned());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = self.device_handle.reset().unwrap();

        let handle = &mut self.device_handle;
        let endpoint = &self.write_endpoint;
        let empty_stream : Vec<u8> = Vec::new();

        match configure_endpoint(handle, endpoint) {
            Ok(_) => {
                match handle.write_bulk(endpoint.address, &self.stream.as_slice(), Duration::from_secs(10)) {
                    Ok(n) => {
                      println!("already write {} bytes!", n);
                      self.stream = empty_stream;
                      Ok(())
                    },
                    Err(err) => {
                      println!("error happened! {:?}", err);
                      self.stream = empty_stream;
                      Err(std::io::Error::new(std::io::ErrorKind::Other, "oh no!"))
                    }
                }
            },
            Err(err) => {
                println!("error happened! {:?}", err);
                self.stream = empty_stream;
                Err(std::io::Error::new(std::io::ErrorKind::Other, "oh no!"))
            }
        }
    }
}

pub struct Serial {}

#[derive(Debug)]
pub struct Network {
    _host: String,
    _port: u16,
    stream: net::TcpStream,
}

impl Network {
    pub fn new(host: &str, port: u16) -> Network {
        let stream = net::TcpStream::connect((host, port)).unwrap();
        Network {
            _host: host.to_string(),
            _port: port,
            stream,
        }
    }
}

impl io::Write for Network {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[derive(Debug)]
pub struct File<W> {
    fobj: W,
}

impl<W: io::Write> File<W> {
    pub fn from_path<P: AsRef<path::Path> + ToString>(path: P) -> File<fs::File> {
        let fobj = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        File { fobj }
    }

    pub fn from(fobj: W) -> File<W> {
        File { fobj }
    }
}

impl<W: io::Write> io::Write for File<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.fobj.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.fobj.flush()
    }
}
