/*
 * Copyright (c) 2019, Joyent, Inc.
 */

use clap::{crate_version, value_t, App, Arg, ArgMatches};
use std::env;
use std::path::PathBuf;

static ABOUT: &'static str = "Muppet is an HTTP loadbalancer (haproxy) and \
                              small daemon that interacts with ZooKeeper via \
                              registrar. The muppet daemon will update the \
                              loadbalancer with new configuration as hosts \
                              come and go from the given service name.";

pub struct Opts<'l> {
    matches: ArgMatches<'l>,
    default_path: PathBuf,
}

impl<'l> Opts<'l> {
    pub fn parse<'a, 'b>(app: String) -> Opts<'a> {
        // set up default config path
        let current_dir = env::current_dir().unwrap();
        let default_path: PathBuf = [current_dir, PathBuf::from("etc/config.json")]
            .iter()
            .collect();

        let matches = App::new(app)
            .about(ABOUT)
            .version(crate_version!())
            .arg(
                Arg::with_name("file")
                    .help("Configuration file")
                    .short("f")
                    .long("file")
                    .takes_value(true)
                    .required(false),
            )
            .arg(
                Arg::with_name("verbose")
                    .help("Verbose output. Use multiple times for more verbose.")
                    .short("v")
                    .long("verbose")
                    .multiple(true)
                    .takes_value(false)
                    .required(false),
            )
            .get_matches();
        Opts {
            matches,
            default_path,
        }
    }

    pub fn get_config_path(&self) -> PathBuf {
        let matches = &self.matches;
        let default_path = &self.default_path;
        value_t!(matches, "file", PathBuf).unwrap_or(default_path.to_path_buf())
    }

    pub fn get_verbose_count(&self) -> u64 {
        self.matches.occurrences_of("verbose")
    }

    pub fn crate_version(&self) -> &'l str {
        crate_version!()
    }
}
