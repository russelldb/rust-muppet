/*
 * Copyright (c) 2019, Joyent, Inc.
 *
 *
 */

use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::IpAddr;
use std::path::Path;
use std::process::Command;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct SdcNic {
    ips: Option<HashSet<String>>,
    ip: Option<IpAddr>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MantaDomain(pub String);

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Config {
    name: MantaDomain,
    trustedIP: IpAddr,
    adminIPs: Option<HashSet<IpAddr>>,
    mantaIPs: Option<HashSet<IpAddr>>,
    untrustedIPs: Option<HashSet<IpAddr>>,
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

/// get sdc nic info from mdata-get to be added to config
pub fn get_sdc_nics() -> Result<HashSet<IpAddr>, Box<Error>> {
    // @TODO: use the logger from main
    let nics_json = get_nics_mdata()?;
    let sdc_nics: Vec<SdcNic> = serde_json::from_str(&nics_json)?;

    // mdata sdc:nics used to have only an `ip` property. That
    // was changed later to be an array of `ips` each with a
    // netmask suffix in cidr notation. This code then prefers
    // the `ips` array content, but fall backs to the `ip`
    // content if `ips` is not present. Massage the data here to
    // return just a Vec<IpAddr> as that is neater and fits
    // better
    let mut sdc_ips: Vec<IpAddr> = vec![];

    for nic in sdc_nics {
        match nic {
            SdcNic {
                ips: None,
                ip: Some(ip),
            } => sdc_ips.push(ip),
            SdcNic { ips: Some(ips), .. } => {
                for ip_str in ips {
                    // the data in the ips array is a string of
                    // ip/netmask like a cidr range,
                    // e.g. "10.0.0.10/24" we only want the host
                    // portion
                    let v: Vec<&str> = ip_str.split('/').collect();
                    if let Ok(ip) = v[0].parse::<IpAddr>() {
                        sdc_ips.push(ip);
                    } else {
                        println!("parse error on ip {} in 'ips'", ip_str);
                    }
                }
            }
            _ => println!("No ips for nic!"),
        }
    }

    let hs: HashSet<IpAddr> = sdc_ips.into_iter().collect();
    return Ok(hs);
}

impl Config {
    fn add_untrusted_ips(&mut self) -> Result<&mut Config, Box<Error>> {
        let mut sdc_ips = get_sdc_nics()?;

        if let Some(manta_ips) = &self.mantaIPs {
            sdc_ips = &sdc_ips - &manta_ips;
        }

        if let Some(admin_ips) = &self.adminIPs {
            sdc_ips = &sdc_ips - &admin_ips;
        }

        sdc_ips.remove(&self.trustedIP);

        if sdc_ips.is_empty() {
            self.untrustedIPs = None;
        } else {
            self.untrustedIPs = Some(sdc_ips);
        }

        return Ok(self);
    }

    pub fn get_untrusted_ips(&self) -> &Option<HashSet<IpAddr>> {
        return &self.untrustedIPs;
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `Config`.
        let mut c: Config = serde_json::from_reader(reader)?;

        c.add_untrusted_ips()?;

        Ok(c)
    }
}

/// call mdata-get sdc:nics and return the resulting JSON as a string
fn get_nics_mdata() -> Result<String, Box<Error>> {
    // @TODO: error handling/logging
    let output = Command::new("mdata-get").arg("sdc:nics").output()?;
    let data = String::from_utf8(output.stdout)?;
    return Ok(data);
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    /// Load a config from disk, and mock call mdata-get for untrusted
    /// IP addresses
    #[test]
    fn load_conf_and_untrusted() {
        let current_dir = env::current_dir().unwrap();
        let config_path: PathBuf = [current_dir, PathBuf::from("test/etc/lab.config.json")]
            .iter()
            .collect();

        let config =
            super::Config::from_file(config_path.as_path()).expect("Failed to parse config");

        let utip = config.get_untrusted_ips();
        assert!(utip.is_some());

        // FAILS @TODO mock the mdata-get bit
        match utip {
            Some(set) => {
                assert_eq!(set.len(), 1, "Config only has 1 untrusted IP");
            }
            None => (),
        }
    }
}
