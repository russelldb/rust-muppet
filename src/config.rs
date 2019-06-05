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

/// call mdata-get sdc:nics and return the resulting JSON as a string
pub fn get_nics_mdata() -> Result<String, Box<Error>> {
    // @TODO: error handling/logging
    let output = Command::new("mdata-get").arg("sdc:nics").output()?;
    let data = String::from_utf8(output.stdout)?;
    return Ok(data);
}

/// parse sdc nic info (maybe from mdata-get)
pub fn parse_sdc_nics(nics_json: &str) -> Result<HashSet<IpAddr>, Box<Error>> {
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
    /// mdata-get sdc:nics
    pub fn add_untrusted_ips(&mut self, nics_json: &str) -> Result<&mut Config, Box<Error>> {
        let mut sdc_ips = parse_sdc_nics(nics_json)?;

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

        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::env;
    use std::net::IpAddr;
    use std::path::PathBuf;

    /// Load a config from disk, load untrusted ips.
    #[test]
    fn load_conf_and_untrusted() {
        let current_dir = env::current_dir().unwrap();
        let config_path: PathBuf = [current_dir, PathBuf::from("test/etc/lab.config.json")]
            .iter()
            .collect();

        let mut config =
            super::Config::from_file(config_path.as_path()).expect("Failed to parse config");

        // these are the IPs in the test data json, would be better to
        // find a way to declare them once only
        let expected: HashSet<IpAddr> = vec!["192.168.1.171", "192.168.118.13", "10.77.77.44"]
            .iter_mut()
            .map(|e| e.parse::<IpAddr>().unwrap())
            .collect();

        // I'd rather mock get_nics_mdata() but for now use some test
        // data
        config.add_untrusted_ips(MIX_SDC_NICS_TEST_DATA).unwrap();

        let utip = config.get_untrusted_ips();
        assert!(utip.is_some());

        match utip {
            Some(set) => {
                assert_eq!(set.len(), 3, "Config has 3 untrusted IPs");
                assert_eq!(set, &expected);
            }
            None => (),
        }
    }

    /// Static test data JSON outputs for test, a nice mix of records
    /// with ips + ip, only ip, and no ips at all!
    static MIX_SDC_NICS_TEST_DATA: &'static str = r#"
    [
       {
          "ips" : [
             "192.168.1.171/24"
          ],
          "mac" : "90:b8:d0:22:26:65",
          "network_uuid" : "3f2b4e0d-6da6-4531-b018-a892e4c96b3c",
          "mtu" : 1500,
          "vlan_id" : 0,
          "interface" : "net0",
          "ip" : "192.168.1.171",
          "netmask" : "255.255.255.0",
          "nic_tag" : "admin"
       },
       {
          "netmask" : "255.255.255.0",
          "ip" : "192.168.118.13",
          "gateways" : [
             "192.168.118.1"
          ],
          "mac" : "90:b8:d0:8d:17:1d",
          "vlan_id" : 0,
          "nic_tag" : "external",
          "primary" : true,
          "interface" : "net1",
          "gateway" : "192.168.118.1",
          "network_uuid" : "c8854428-7a67-4a55-9f9c-9a9a8cbd4faf",
          "mtu" : 1500
         },
       {
          "ips" : [
             "10.77.77.44/24"
          ],
          "vlan_id" : 0,
          "mtu" : 1500,
          "network_uuid" : "6e4b3fab-96fc-463e-b8bc-6d483638bb2c",
          "mac" : "90:b8:d0:00:c0:aa",
          "interface" : "net2",
          "nic_tag" : "manta",
          "netmask" : "255.255.255.0"
       },
       {
          "mac" : "90:b8:d0:bd:4c:a6",
          "vlan_id" : 0,
          "netmask" : "255.255.255.0",
          "gateways" : [
             "10.66.66.2"
          ],
          "gateway" : "10.66.66.2",
          "network_uuid" : "78d3dda3-2f27-42cc-8a88-81eefde25121",
          "mtu" : 1500,
          "nic_tag" : "mantanat",
          "interface" : "net3"
       }
    ]"#;

}
