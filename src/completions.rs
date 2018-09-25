/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use argparse;
use std::io;

/**
 * The completions subcommand invokes this function with the shell variable
 * parsed from the command line. It is responsible for generating the
 * corresponding shell completion scripts for the supplied shell.
 *
 * This generates bindings for the main reups application, and also bindings
 * specifically for the rsetup subcommand.
 *
 * The resulting scripts are output to stdout so the user has the ability
 * to pipe them to the appropriate location.
 */
pub fn write_completions_stdout(shell: &str) {
    // Generate completions for the main reups program for all subcommands
    argparse::build_cli().gen_completions_to("reups", shell.parse().unwrap(), &mut io::stdout());
    // Generate conpletions for just the setup subcommand and bind these
    // to the rsetup string. This lets auto completion work for the rsetup
    // shell function
    argparse::build_setup().gen_completions_to("rsetup", shell.parse().unwrap(), &mut io::stdout());
}
