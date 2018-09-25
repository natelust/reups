/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

extern crate reups_lib;

use reups_lib as reups;

fn main() {
    let args = reups::parse_args();

    match args.subcommand() {
        ("setup", Some(m)) => {
            reups::setup_command(m, &args);
        },
        ("prep", Some(_)) => {
            println!("{}", reups::build_prep_string());
        },
        ("list", Some(m)) => {
            reups::list_command(m, &args);
        },
        ("completions", Some(m)) => {
            reups::write_completions_stdout(m.value_of("shell").unwrap());
        },
        _ => println!("{}",args.usage()),
    }
}
