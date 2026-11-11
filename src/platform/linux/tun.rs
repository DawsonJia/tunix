use crate::{
    configuration::{Configuration, Layer},
    error::{Error, Result},
    interface::Interface,
    platform::posix::fd::Fd,
};
use libc::{c_int, c_short};
use routex::Handle;
use std::{
    ffi::CStr,
    io::{self, Read, Write},
    os::fd::{AsRawFd, RawFd},
    sync::{Arc, Mutex},
};

use super::{name::write_if_name, sys::*};

#[derive(Debug, Clone, Copy, Default)]
pub struct TunConf {
    pub(crate) packet_information: bool,
}

impl TunConf {
    pub fn packet_information(&mut self, value: bool) -> &mut Self {
        self.packet_information = value;
        self
    }
}

pub struct Queue {
    tun: Fd,
    pi_enabled: bool,
}

impl Queue {
    fn has_packet_information(&self) -> bool {
        self.pi_enabled
    }

    fn set_nonblocking(&self) -> io::Result<()> {
        self.tun.set_nonblocking(true)
    }

    fn cancel_nonblocking(&self) -> io::Result<()> {
        self.tun.set_nonblocking(false)
    }
}

impl AsRawFd for Queue {
    fn as_raw_fd(&self) -> RawFd {
        self.tun.as_raw_fd()
    }
}

pub struct Tun {
    // TODO: read the name off the queue with TUNGETIFF instead of keeping it: this copy is
    // stale once something else renames the interface, and `set_name` then fails. Not
    // `if_indextoname`, which answers for the namespace of the calling thread.
    name: Arc<Mutex<String>>,
    index: u32,
    handle: Handle,
    // The socket that renames the interface. As `handle`, it is of the network namespace
    // the interface was made in.
    ctl: Arc<Fd>,
    queue: Queue,
    layer: Layer,
}

impl Tun {
    pub fn new(config: &Configuration) -> Result<Self> {
        if config.queues.unwrap_or(1) > 1 {
            return Err(Error::InvalidQueuesNumber);
        }

        Tun::new_multi_queue(config)?
            .pop()
            .ok_or(Error::InvalidQueuesNumber)
    }

    pub fn new_multi_queue(config: &Configuration) -> Result<Vec<Self>> {
        let mut queues = Vec::new();
        let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };

        if let Some(name) = config.name.as_ref() {
            write_if_name(name, &mut ifr.ifr_name)?;
        };

        let tun_type: c_short = config.layer.into();
        let queue_nums = config.queues.unwrap_or(1);
        if queue_nums < 1 {
            return Err(Error::InvalidQueuesNumber);
        }

        let pi = config.platform.packet_information;
        ifr.ifr_ifru.ifru_flags = tun_type
            | if pi { 0 } else { libc::IFF_NO_PI as c_short }
            | if queue_nums > 1 {
                libc::IFF_MULTI_QUEUE as c_short
            } else {
                0
            };

        for _ in 0..queue_nums {
            let tun_fd = Fd::open(c"/dev/net/tun", libc::O_RDWR)?;

            unsafe {
                tunsetiff(
                    tun_fd.as_raw_fd(),
                    &mut ifr as *mut libc::ifreq as *mut c_int,
                )
            }?;

            queues.push(Queue {
                tun: tun_fd,
                pi_enabled: pi,
            });
        }

        // The kernel wrote the name it gave the interface back into the request.
        let name = unsafe {
            CStr::from_ptr(ifr.ifr_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };
        let index = routex::if_nametoindex(&name)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let name = Arc::new(Mutex::new(name));
        let ctl = Arc::new(Fd::socket(libc::AF_INET, libc::SOCK_DGRAM, 0)?);

        let mut tuns = queues
            .into_iter()
            .map(|queue| {
                Ok(Self {
                    name: name.clone(),
                    index,
                    handle: Handle::new()?,
                    ctl: ctl.clone(),
                    queue,
                    layer: config.layer,
                })
            })
            .collect::<Result<Vec<Self>>>()?;

        tuns[0].configure(config)?;

        Ok(tuns)
    }

    pub fn set_nonblocking(&self) -> io::Result<()> {
        self.queue.set_nonblocking()
    }

    pub fn cancel_nonblocking(&self) -> io::Result<()> {
        self.queue.cancel_nonblocking()
    }

    pub fn has_packet_information(&self) -> bool {
        self.queue.has_packet_information()
    }

    pub fn layer(&self) -> Layer {
        self.layer
    }
}

impl Interface for Tun {
    type Queue = Queue;

    fn name(&self) -> Result<String> {
        Ok(self.name.lock().unwrap().clone())
    }

    fn set_name(&mut self, new_name: &str) -> Result<()> {
        let mut name = self.name.lock().unwrap();
        let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
        write_if_name(&name, &mut ifr.ifr_name)?;
        unsafe { write_if_name(new_name, &mut ifr.ifr_ifru.ifru_newname)? };

        unsafe { siocsifname(self.ctl.as_raw_fd(), &ifr) }?;

        *name = new_name.into();

        Ok(())
    }

    fn index(&self) -> u32 {
        self.index
    }

    fn handle(&mut self) -> &mut Handle {
        &mut self.handle
    }

    fn queue(&mut self) -> &mut Self::Queue {
        &mut self.queue
    }
}

impl AsRawFd for Tun {
    fn as_raw_fd(&self) -> RawFd {
        self.queue.as_raw_fd()
    }
}

impl Read for Tun {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.queue.tun.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.queue.tun.read_vectored(bufs)
    }
}

impl Write for Tun {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.queue.tun.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.queue.tun.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.queue.tun.write_vectored(bufs)
    }
}

impl From<Layer> for c_short {
    fn from(value: Layer) -> Self {
        match value {
            Layer::L2 => libc::IFF_TAP as _,
            Layer::L3 => libc::IFF_TUN as _,
        }
    }
}
