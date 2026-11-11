use libc::{c_int, ifreq};

nix::ioctl_read_bad!(siocgifname, 0x8910, ifreq); // get iface name
nix::ioctl_write_ptr_bad!(siocsifname, 0x8923, ifreq); // set interface name

nix::ioctl_write_ptr!(tunsetiff, b'T', 202, c_int);
nix::ioctl_write_ptr!(tunsetpersist, b'T', 203, c_int);
nix::ioctl_write_ptr!(tunsetowner, b'T', 204, c_int);
nix::ioctl_write_ptr!(tunsetgroup, b'T', 205, c_int);
