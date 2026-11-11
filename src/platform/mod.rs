#[cfg(target_os = "linux")]
mod linux {
    mod name;
    mod sys;
    pub mod tun;
}
#[cfg(target_os = "linux")]
pub use linux::tun;

#[cfg(target_os = "macos")]
mod macos {
    pub mod tun;
}
#[cfg(target_os = "macos")]
pub use macos::tun;

#[cfg(target_os = "windows")]
mod windows {
    pub mod tun;
}
#[cfg(target_os = "windows")]
pub use windows::tun;

#[cfg(unix)]
pub(crate) mod posix {
    pub mod fd;
    pub mod sys;
}
