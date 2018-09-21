/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use std::io;
use argparse;

pub fn write_completions_stdout(shell : &str) {
    argparse::build_cli().gen_completions_to("reups",
                                             shell.parse().unwrap(),
                                             & mut io::stdout()
                                             );
}
