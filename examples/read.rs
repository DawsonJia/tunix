use std::io::Read;
use tunix::Configuration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Configuration::default();

    #[cfg(target_os = "macos")]
    config.address_with_peer("10.0.0.9", 32, "10.0.0.10");
    #[cfg(not(target_os = "macos"))]
    config.address("10.0.0.9", 24);
    config.address("fd00::9", 64).up();

    #[cfg(target_os = "linux")]
    let config = config.platform(|tun_conf| {
        tun_conf.packet_information(true);
    });

    let mut dev = config.build().unwrap();
    let mut buf = [0u8; 4096];

    loop {
        let n = dev.read(&mut buf)?;
        println!("read {} bytes", n);
        println!("{:?}", &buf[..n]);
    }
}
