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
    ips: Option<Vec<String>>,
    ip: Option<IpAddr>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MantaDomain(pub String);

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    name: MantaDomain,
    // these camelcase(ish) names are a holdover from muppet and it's
    // json config
    #[serde(alias = "trustedIP")]
    trusted_ip: IpAddr,
    #[serde(alias = "adminIPS")]
    admin_ips: Option<HashSet<IpAddr>>,
    #[serde(alias = "mantaIPS")]
    manta_ips: Option<HashSet<IpAddr>>,
    // consistency (in naming) is a hobgoblin etc
    #[serde(alias = "untrustedIPs")]
    untrusted_ips: Option<HashSet<IpAddr>>,
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

/// call mdata-get sdc:nics and return the resulting JSON as a string
fn get_nics_mdata() -> Result<String, Box<Error>> {
    // @TODO: error handling/logging
    let output = Command::new("mdata-get").arg("sdc:nics").output()?;
    let data = String::from_utf8(output.stdout)?;
    return Ok(data);
}

/// parse sdc nic info (maybe from mdata-get)
fn parse_sdc_nics(nics_json: &str) -> Result<HashSet<IpAddr>, Box<Error>> {
    let sdc_nics: Vec<SdcNic> = serde_json::from_str(nics_json)?;

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
    /// update Config's internal untrustedIPs field with address from
    /// mdata-get sdc:nics you must call this after creating a Config
    /// with Config::from_file. NOTE: only if the existing config has
    /// no untrusted IPs
    pub fn populate_untrusted_ips(&mut self) -> Result<&mut Config, Box<Error>> {
        // the muppet.js code this is transposed from either reads
        // untrusted_ips from the Config OR from sdc:nics, with config
        // taking precedence.
        if let Some(_) = self.get_untrusted_ips() {
            return Ok(self);
        }

        let sdc_nics_json = get_nics_mdata()?;
        let sdc_ips = parse_sdc_nics(&sdc_nics_json)?;
        return self.add_untrusted_ips(sdc_ips);
    }

    /// Populate config.untrusted_ips from the given sdc_ips
    /// hashset. Only ips that are not in some other way configured
    /// are added as untrusted. This method overwrites existing
    /// configured untrusted ips (NOTE: there should be none, it's a
    /// private method used by `populate_untrusted_ips` above) to aid
    /// testability
    fn add_untrusted_ips(&mut self, sdc_ips: HashSet<IpAddr>) -> Result<&mut Config, Box<Error>> {
        let mut sdc_ips = sdc_ips;

        if let Some(manta_ips) = &self.manta_ips {
            sdc_ips = &sdc_ips - &manta_ips;
        }

        if let Some(admin_ips) = &self.admin_ips {
            sdc_ips = &sdc_ips - &admin_ips;
        }

        sdc_ips.remove(&self.trusted_ip);

        if sdc_ips.is_empty() {
            self.untrusted_ips = None;
        } else {
            self.untrusted_ips = Some(sdc_ips);
        }

        return Ok(self);
    }

    /// accessor for untrusted ips data member
    pub fn get_untrusted_ips(&self) -> &Option<HashSet<IpAddr>> {
        return &self.untrusted_ips;
    }

    /// accessor for manta ips data member
    pub fn get_manta_ips(&self) -> &Option<HashSet<IpAddr>> {
        return &self.manta_ips;
    }

    /// accessor for admin ips data member
    pub fn get_admin_ips(&self) -> &Option<HashSet<IpAddr>> {
        return &self.admin_ips;
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read the JSON contents of the file as an instance of `Config`.
        let c: Config = serde_json::from_reader(reader)?;

        // @TODO I'd like to then call populate_untrusted_ips here,
        // but unless I can mock it, I can't test it: investigate
        // mocking for now it means the caller MUST remember to
        // populate untrusted nics with a call to
        // populate_untrusted_ips
        Ok(c)
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
