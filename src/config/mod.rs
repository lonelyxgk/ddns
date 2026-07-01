use once_cell::sync::Lazy;
use std::io::Read;
use std::net::Ipv6Addr;
use std::sync::{OnceLock, RwLock};
use std::{fs::File, net::Ipv4Addr};

pub(crate) mod parse;

use parse::{Config, check};

pub const EXAMPLE_CONFIG: &str = include_str!("../../examples/config.toml");

static CONFIG: OnceLock<Config> = OnceLock::new();

pub(crate) static GIP: Lazy<RwLock<Gip>> = Lazy::new(|| {
    RwLock::new(Gip {
        ipv4: Ipv4Addr::new(0, 0, 0, 0),
        ipv6: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
    })
});

pub(crate) struct Gip {
    pub(crate) ipv4: Ipv4Addr,
    pub(crate) ipv6: Ipv6Addr,
}

pub(crate) fn get_config() -> &'static Config {
    CONFIG.get().expect("has not been initialized")
}

pub(crate) fn init_config(path: &str) -> anyhow::Result<()> {
    let mut file = File::open(path)?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let config = toml::from_str::<Config>(&buf)?;

    check(&config)?;

    CONFIG.set(config).expect("has been initialized");

    Ok(())
}
