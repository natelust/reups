/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
* Copyright Nate Lust 2018*/


use log;
use std::boxed::Box;
use argparse;

pub struct Logger {
    log_level: log::LevelFilter,
    stderr: bool
}

impl Logger {
    pub fn new(log_level: log::LevelFilter, stderr: bool) -> Box<Logger>{
        Box::new(Logger {
            log_level,
            stderr
        })
    }
}

impl log::Log for Logger {

    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()){
            let message = format!("{}: {}", record.level(), record.args());
            match self.stderr {
                true => {
                    eprintln!("{}", message);
                },
                false => {
                    println!("{}", message);
                }
            }
        }
    }

    fn flush(&self) {}
}

pub fn build_logger(args: & argparse::ArgMatches, stderr: bool) {
    let level = match args.occurrences_of("verbose") {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace

    };
    log::set_boxed_logger(Logger::new(level, stderr)).unwrap();
    log::set_max_level(level)
}
