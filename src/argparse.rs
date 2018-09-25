/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

#[doc(no_inline)]
pub use clap::{ArgMatches};
use clap::{Arg,App, SubCommand};

/**
 * Builds and returns the sub command struct, containing all the options for the setup command
*/
pub fn build_setup<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("setup")
                          .arg(Arg::with_name("product")
                               .help("Product to setup")
                               .index(1))
                          .arg(Arg::with_name("just")
                               .help("ignore dependncies")
                               .short("j")
                               .long("just"))
                          .arg(Arg::with_name("relative")
                               .help("setup relative path")
                               .short("r")
                               .long("relative")
                               .takes_value(true))
                          .arg(Arg::with_name("keep")
                               .help("keep exsisting setup products")
                               .short("k")
                               .long("keep"))
                          .arg(Arg::with_name("tag")
                               .help("specify one or more tags to look up for products, evaluated left to right")
                               .short("t")
                               .long("tag")
                               .multiple(true)
                               .number_of_values(1)
                               .takes_value(true))
                          .arg(Arg::with_name("inexact")
                               .help("Run setup with Inexact versions as specified in the table files")
                               .short("E")
                               .long("inexact"))
                           .arg(Arg::with_name("verbose")
                                .short("v")
                                .long("verbose")
                                .multiple(true)
                                .help("Sets the level of verbosity, multiple occurances increases verbosity"));
}

/**
 * Builds and returns the sub command struct, containing all the options for the list command
 */
fn build_list<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("list")
                          .arg(Arg::with_name("product")
                               .help("Name of product to list (optional)")
                               .index(1)
                               .conflicts_with_all(&["setup", "local"]))
                          .arg(Arg::with_name("setup")
                               .help("List only setup products")
                               .short("s")
                               .long("setup")
                               .conflicts_with_all(&["product", "local"]))
                          .arg(Arg::with_name("tags")
                               .help("List only these tags (does not include current)")
                               .short("t")
                               .long("tags")
                               .multiple(true)
                               .number_of_values(1)
                               .takes_value(true))
                          .arg(Arg::with_name("onlyTags")
                               .help("Only list product & tags (faster than tags and versions, but does not indicate setup tag)")
                               .long("onlyTags"))
                          .arg(Arg::with_name("onlyVers")
                               .help("Only list product & versions (faster than tags and versions)")
                               .long("onlyVers")
                               .conflicts_with("onlyTags"))
                          .arg(Arg::with_name("local")
                               .help("Only list products that are setup as local products")
                               .short("l")
                               .long("local")
                               .conflicts_with_all(&["product", "setup"]));
}

/**
 * Builds and returns the sub command struct, containing all the options for the completions command.
 */
fn build_completions<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("completions")
                          .about("Creates auto completeion scripts for given shell")
                          .arg(Arg::with_name("shell")
                               .required(true)
                               .possible_values(&["bash", "fish", "zsh", "elvish"])
                               .help("Shell to create completions for"));
}

/**
 * Builds and returns the sub command struct, containing all the options for the prep command.
 *
 * Currently this is basically an empty command just to create the setup option, but in the future
 * this command may take optional arguments such as a configuration file to use in preping.
 */
fn build_prep<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("prep");
}

/**
 * This function is responsible for creating all the possible command line options and arguments for the main program, and each of the sub commands.
 */
pub fn build_cli() -> App<'static, 'static> {
    App::new("Rust Eups")
        .author("Nate Lust")
        .about("Dynamic environment management")
        .version(crate_version!())
        .subcommand(build_setup())
        .subcommand(build_prep())
        .subcommand(build_list())
        .subcommand(build_completions())
}

/**
 * This is the main argument parser for the reups program. It parses the arguments from the command
 * line into a `ArgMatches` object containing all the supplied options.
 */
pub fn parse_args<'a> () -> ArgMatches<'a> {
    let matches = build_cli().get_matches();
    return matches;
}

