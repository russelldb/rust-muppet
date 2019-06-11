use super::*;
use std::collections::HashSet;
use std::env;
use std::net::IpAddr;
use std::path::PathBuf;

fn ips_to_hashset(mut ips: Vec<&str>) -> HashSet<IpAddr> {
    let hs: HashSet<IpAddr> = ips
        .iter_mut()
        .map(|e| e.parse::<IpAddr>().unwrap())
        .collect();
    return hs;
}

/// a config with untrusted IPs doesn't load more
#[test]
fn config_with_untrusted() {
    let current_dir = env::current_dir().unwrap();
    let config_path: PathBuf = [current_dir, PathBuf::from("test/etc/config.json")]
        .iter()
        .collect();

    let mut config =
        super::Config::from_file(config_path.as_path()).expect("Failed to parse config");

    let untrusted = match config.get_untrusted_ips() {
        None => HashSet::<IpAddr>::new(),
        Some(ips) => ips.clone(),
    };

    config.populate_untrusted_ips().expect("should be a no-op");

    let new_untrusted = match config.get_untrusted_ips() {
        None => HashSet::<IpAddr>::new(),
        Some(ips) => ips.clone(),
    };

    assert_eq!(untrusted.len(), 1, "expected a single configured ip");
    assert_eq!(untrusted, new_untrusted);
}

/// create a config, and some sdc nic ips, and verify that only those
/// ips that are not explictly configured in some other capacity, get
/// added as untrusted ips
#[test]
fn only_unconfigured_are_untrusted() {
    let admin_ip = "192.168.1.171";
    let manta_ip = "192.168.118.13";
    let localhost = "127.0.0.1";
    let untrusted_ip1 = "10.77.77.44";
    let untrusted_ip2 = "10.77.77.55";

    // don't load from disk, just make a config programatically
    let mut config = Config {
        name: MantaDomain(String::from("test")),
        trusted_ip: localhost.parse::<IpAddr>().unwrap(),
        admin_ips: Some(ips_to_hashset(vec![admin_ip])),
        manta_ips: Some(ips_to_hashset(vec![manta_ip])),
        untrusted_ips: None::<HashSet<IpAddr>>,
        zookeeper: ZookeeperConfig {
            servers: vec![ZookeeperServer {
                host: String::from("zkhost"),
                port: 9000,
            }],
            timeout: 1000,
        },
    };
    // don't parse sdc:nics from JSON, just make them programatically
    let sdc_ips = ips_to_hashset(vec![
        admin_ip,
        manta_ip,
        localhost,
        untrusted_ip1,
        untrusted_ip2,
    ]);

    config.add_untrusted_ips(sdc_ips).unwrap();

    let expected = ips_to_hashset(vec![untrusted_ip1, untrusted_ip2]);

    if let Some(configured_untrusted) = config.get_untrusted_ips() {
        assert_eq!(&expected, configured_untrusted);
    } else {
        assert!(false, "Expected some untrusted ips in config");
    }
}

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
    // find a way to declare them only once
    let expected: HashSet<IpAddr> =
        ips_to_hashset(vec!["192.168.1.171", "192.168.118.13", "10.77.77.44"]);

    // I'd rather mock get_nics_mdata() but for now use some test
    // data
    let sdc_ips = super::parse_sdc_nics(MIX_SDC_NICS_TEST_DATA).unwrap();
    config.add_untrusted_ips(sdc_ips).unwrap();

    let untrusted_ips = config.get_untrusted_ips();
    assert!(untrusted_ips.is_some());

    match untrusted_ips {
        Some(set) => {
            assert_eq!(set, &expected);
        }
        None => (),
    }

    if let Some(manta_ips) = config.get_manta_ips() {
        assert_eq!(manta_ips.len(), 1, "Expected a single manta ip");
    } else {
        assert!(false, "Expected a manta ip in config")
    }

    if let Some(admin_ips) = config.get_admin_ips() {
        assert_eq!(admin_ips.len(), 1, "Expected a single admin ip");
    } else {
        assert!(false, "Expected a admin ip in config")
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
