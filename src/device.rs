extern crate libusb;

use std::fs;
use std::io;
use std::net;
use std::path;
use std::time::Duration;

pub struct Usb<'a> {
    _vendor_id: u16,
    _product_id: u16,
    device_handle: libusb::DeviceHandle<'a>,
    write_endpoint: u8
}

fn find_device(context: libusb::Context, vendor_id: u16, product_id: u16) -> Option<(libusb::DeviceHandle<'static>, u8)> {
    for device in context.devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if device_desc.vendor_id() == vendor_id && device_desc.product_id() == product_id {
            match device.open() {
                Ok(handle) => match find_write_endpoint(device, device_desc) {
                    Some(address) => Some((handle, address)),
                    None => continue
                },
                Err(_) => continue
            };
        }
    }

    None
}

fn find_write_endpoint(device: libusb::Device, device_desc: libusb::DeviceDescriptor) -> Option<u8> {
    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue
        };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    if endpoint_desc.direction() == libusb::Direction::Out {
                        return Some(endpoint_desc.address());
                    }
                }
            }
        }
    }

    None
}

impl<'a> Usb<'a> {
    pub fn new(vendor_id: u16, product_id: u16) -> Usb<'a> {
        let context = libusb::Context::new().unwrap();

        match find_device(context, vendor_id, product_id) {
            Some((device_handle, address)) =>  {
                return Usb {
                    _vendor_id: vendor_id,
                    _product_id: product_id,
                    device_handle: device_handle,
                    write_endpoint: address
                }
            }
            None => panic!("could not find device {:04x}:{:04x}", vendor_id, product_id)
        }

    }
}

impl<'a> io::Write for Usb<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.device_handle.write_bulk(self.write_endpoint, buf, Duration::new(5, 0)) {
            Ok(n) => Ok(n),
            Err(_err) => Err(std::io::Error::new(std::io::ErrorKind::Other, "oh no!"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.device_handle.reset() {
            Ok(n) => Ok(n),
            Err(_err) => Err(std::io::Error::new(std::io::ErrorKind::Other, "oh no!"))
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
