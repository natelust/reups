/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2019*/

use clap::{App, Arg};
use reups_lib as reups;
use reups_lib::DBImpl;
use serde_json;
use std::path::PathBuf;

/**
 * This is a small application to covert posix (r)eups database sources into
 * a json based database source.
 *
 * Arguments
 * ---------
 * source - Path to input posix store
 * dest - Location to write output file
 *
 **/
fn main() {
    let app = App::new("Posix2Json")
        .author("Nate Lust")
        .about("Dumps a(n) (r)eups posix db source into a yaml format")
        .version("0.0.1")
        .arg(
            Arg::with_name("source")
                .help("Posix source path")
                .required(true),
        )
        .arg(
            Arg::with_name("dest")
                .help("Path to write output")
                .required(true),
        );
    let matches = app.get_matches();
    let source = reups::PosixDBImpl::new(
        PathBuf::from(matches.value_of("source").unwrap()),
        Some(&reups::DBLoadControl::All),
        None,
    );

    let jsondb = source
        .unwrap()
        .to_json(&PathBuf::from(matches.value_of("dest").unwrap()));
    let serialized = match serde_json::to_string_pretty(&jsondb) {
        Ok(x) => x,
        Err(e) => {
            reups::exit_with_message!(format!("Problem serializing to json, message {}", e));
        }
    };
    let result = std::fs::write(jsondb.get_location(), serialized.as_bytes());

    if result.is_err() {
        eprintln!("There was a problem writing the database out to json, file may not exist, or may be corrupt");
    }
}
