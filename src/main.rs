/*
 * Copyright (c) 2019, Joyent, Inc.
 */

mod config;
mod opts;

use std::sync::Mutex;

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
    let options = opts::Opts::parse(APP.to_string());
    let mut config =
        config::Config::from_file(options.get_config_path()).expect("Failed to parse config file");
    // TODO have config populate untrusted as part of the construction
    // above (see config.rs for reasons)
    config
        .populate_untrusted_ips()
        .expect("Failed adding sdc nic ips to config");

    //TODO: Runtime log handling (Move this into config, so we can
    // just have config.get_log (e.g.)  By default slog makes the
    // decision on what log lines to include at compile time. There is
    // a way to do runtime selection though.
    match options.get_verbose_count() {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    let root_log = Logger::root(
        Mutex::new(slog_bunyan::default(std::io::stdout())).fuse(),
        o!("build-id" => options.crate_version()),
    );

    info!(root_log, "muppet has started");

    println!("config is {:?}", &config);
    let zk_result = zookeeper_session(&config);

    match zk_result {
        Ok(zk_session) => start_watch(&zk_session, &config),
        Err(_) => println!("Failed to connect to zk"),
    }
}
