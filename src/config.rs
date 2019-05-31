/*
 * Copyright (c) 2019, Joyent, Inc.
 */

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::IpAddr;
use std::path::Path;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct MantaDomain(pub String);

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Config {
    name: MantaDomain,
    trustedIP: IpAddr,
    adminIPs: Option<Vec<IpAddr>>,
    mantaIPs: Option<Vec<IpAddr>>,
    untrustedIPs: Option<Vec<IpAddr>>,
    zookeeper: ZookeeperConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZookeeperConfig {
    servers: Vec<ZookeeperServer>,
    timeout: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZookeeperServer {
    host: String,
    port: u32,
}

impl Config {
    fn add_untrusted_ips(&mut self) {}

    fn add_untrusted_ip(&mut self, ip: IpAddr) {
        match self.untrustedIPs.as_mut() {
            Some(vec) => vec.push(ip),
            None => self.untrustedIPs = Some(vec![ip]),
        }
    }

    pub fn get_untrusted_ips(&self) -> &Option<Vec<IpAddr>> {
        return &self.untrustedIPs;
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `Config`.
        let mut c: Config = serde_json::from_reader(reader)?;

        c.add_untrusted_ips();

        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::path::PathBuf;

    /// Load a config from disk, and add an IP address to the
    /// untrustedIPs Vec. Simulates the start up process without
    /// calling out to mdata-get for sdc:nics
    #[test]
    fn load_conf_and_update() {
        let current_dir = env::current_dir().unwrap();
        let config_path: PathBuf = [current_dir, PathBuf::from("test/etc/config.json")]
            .iter()
            .collect();

        let untrusted_ip1 = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        let untrusted_ip2 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 11));

        let mut config =
            super::Config::from_file(config_path.as_path()).expect("Failed to parse config");

        assert!(config.get_untrusted_ips().is_some());

        let utip = config.get_untrusted_ips();
        assert!(utip.is_some());

        match utip {
            Some(vec) => {
                assert_eq!(vec.len(), 1, "Config only has 1 untrusted IP");
                assert_eq!(vec[0], untrusted_ip1);
            }
            None => (),
        }
        config.add_untrusted_ip(untrusted_ip2);

        let utip = config.get_untrusted_ips();
        assert!(utip.is_some());

        match utip {
            Some(vec) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[1], untrusted_ip2);
            }
            None => (),
        }
    }
}
