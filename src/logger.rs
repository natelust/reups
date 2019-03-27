/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
* Copyright Nate Lust 2018*/

use crate::argparse;
use log;
use std::boxed::Box;
use std::io::Write;
use std::sync::Mutex;

/// Structure which is responsible processing input from the std log
/// api. It's members are the highest log level to output, and what
/// writer object that the logger should write out to.
pub struct Logger<W: Write> {
    log_level: log::LevelFilter,
    writer: Mutex<W>,
}

impl<W: Write> Logger<W> {
    /// Creates a new logger object. Arguments are the maximum level to log,
    /// and if the output should go to standard out or standard error.
    pub fn new(log_level: log::LevelFilter, writer: W) -> Box<Logger<W>> {
        Box::new(Logger {
            log_level,
            writer: Mutex::new(writer),
        })
    }
}

impl<W: Write + Send + Sync> log::Log for Logger<W> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}: {}\n", record.level(), record.args());
            let _ = self.writer.lock().unwrap().write(message.as_bytes());
        }
    }

    fn flush(&self) {}
}

/// Builds and initializes a logging object with options from the command line
/// and the stderr boolean which is governed by the context of the subcommand
/// that initiates the logger.
pub fn build_logger<W: Write + Sync + Send + 'static>(args: &argparse::ArgMatches, writer: W) {
    let level = match args.occurrences_of("verbose") {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    log::set_boxed_logger(Logger::new(level, writer)).unwrap();
    log::set_max_level(level)
}
