use super::Fd;
use crate::platform::posix::sys::cvt;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

#[test]
fn owns_cloexec_descriptor_and_reads_into_mutable_buffer() {
    let mut descriptors = [-1; 2];
    // SAFETY: descriptors has space for the two fds written by pipe.
    cvt(unsafe { libc::pipe(descriptors.as_mut_ptr()) }).unwrap();
    let mut reader = Fd::from_syscall(descriptors[0]).unwrap();
    let mut writer = Fd::from_syscall(descriptors[1]).unwrap();

    let flags = cvt(unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFD) }).unwrap();
    assert_ne!(flags & libc::FD_CLOEXEC, 0);

    writer.write_all(b"tunix").unwrap();
    let mut received = [0; 5];
    reader.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"tunix");
}
