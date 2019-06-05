/*
 * Copyright (c) 2019, Joyent, Inc.
 */

mod config;
mod opts;

use std::env;
use std::path::PathBuf;
use std::sync::Mutex;

use clap::{crate_version, value_t};
use config::Config;
use slog::{info, o, Drain, Logger};
use zookeeper::{ZkResult, ZooKeeper};

static APP: &'static str = "muppet";

fn zookeeper_session(c: &Config) -> ZkResult<ZooKeeper> {
    std::unimplemented!();
}

fn start_watch(z: &ZooKeeper, c: &Config) {
    std::unimplemented!();
}

fn main() {
    let matches = opts::parse(APP.to_string());
    let current_dir = env::current_dir().unwrap();
    let default_config: PathBuf = [current_dir, PathBuf::from("etc/config.json")]
        .iter()
        .collect();

    let config_path = value_t!(matches, "file", PathBuf).unwrap_or(default_config);
    let mut config =
        config::Config::from_file(config_path.as_path()).expect("Failed to parse config");
    let sdc_nics = config::get_nics_mdata().unwrap();
    config.add_untrusted_ips(&sdc_nics).unwrap();

    //TODO: Runtime log handling
    // By default slog makes the decision on what log lines to include at
    // compile time. There is a way to do runtime selection though.
    match matches.occurrences_of("verbose") {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    let root_log = Logger::root(
        Mutex::new(slog_bunyan::default(std::io::stdout())).fuse(),
        o!("build-id" => crate_version!()),
    );

    info!(root_log, "muppet has started");

    println!("config is {:?}", &config);
    let zk_result = zookeeper_session(&config);

    match zk_result {
        Ok(zk_session) => start_watch(&zk_session, &config),
        Err(_) => println!("Failed to connect to zk"),
    }
}
