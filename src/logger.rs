/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
* Copyright Nate Lust 2018*/

use crate::argparse;
use log;
use std::boxed::Box;

/// Structure which is responsible processing input from the std log
/// api. It's members are the highest log level to output, and if
/// the output should be sent to stdandard error instead of standard out.
pub struct Logger {
    log_level: log::LevelFilter,
    stderr: bool,
}

impl Logger {
    /// Creates a new logger object. Arguments are the maximum level to log,
    /// and if the output should go to standard out or standard error.
    pub fn new(log_level: log::LevelFilter, stderr: bool) -> Box<Logger> {
        Box::new(Logger { log_level, stderr })
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}: {}", record.level(), record.args());
            match self.stderr {
                true => {
                    eprintln!("{}", message);
                }
                false => {
                    println!("{}", message);
                }
            }
        }
    }

    fn flush(&self) {}
}

/// Builds and initializes a logging object with options from the command line
/// and the stderr boolean which is governed by the context of the subcommand
/// that initiates the logger.
pub fn build_logger(args: &argparse::ArgMatches, stderr: bool) {
    let level = match args.occurrences_of("verbose") {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    log::set_boxed_logger(Logger::new(level, stderr)).unwrap();
    log::set_max_level(level)
}
