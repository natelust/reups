/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

#[doc(no_inline)]
pub use clap::ArgMatches;
use clap::{App, Arg, SubCommand};

/**
 * Builds and returns the sub command struct, containing all the options for the setup command
*/
pub fn build_setup<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("setup")
        .arg(Arg::with_name("product").help("Product to setup").index(1))
        .arg(
            Arg::with_name("just")
                .help("ignore dependncies")
                .short("j")
                .long("just"),
        )
        .arg(
            Arg::with_name("relative")
                .help("setup relative path")
                .short("r")
                .long("relative")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("keep")
                .help("keep exsisting setup products")
                .short("k")
                .long("keep"),
        )
        .arg(
            Arg::with_name("tag")
                .help("specify one or more tags to look up for products, evaluated left to right")
                .short("t")
                .long("tag")
                .multiple(true)
                .number_of_values(1)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("inexact")
                .help("Run setup with Inexact versions as specified in the table files")
                .short("E")
                .long("inexact"),
        );
}

/**
 * Builds and returns the sub command struct, containing all the options for the list command
 */
fn build_list<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("list")
                          .arg(Arg::with_name("product")
                               .help("Name of product to list (optional)")
                               .index(1)
                               .conflicts_with_all(&["setup", "local", "sources"]))
                          .arg(Arg::with_name("setup")
                               .help("List only setup products")
                               .short("s")
                               .long("setup")
                               .conflicts_with_all(&["product", "local", "sources"]))
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
                               .conflicts_with_all(&["product", "setup", "sources"]))
                          .arg(Arg::with_name("sources")
                               .help("List identifier and path of all the sources that went into the database")
                               .long("sources")
                               .conflicts_with_all(&["product", "setup", "local"]));
}

/**
 * Builds and returns the sub command struct, containing all the options for the completions command.
 */
fn build_completions<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("completions")
        .about("Creates auto completeion scripts for given shell")
        .arg(
            Arg::with_name("shell")
                .required(true)
                .possible_values(&["bash", "fish", "zsh", "elvish"])
                .help("Shell to create completions for"),
        );
}

/**
 * Builds the completions for the sub command env. This allows the reups commands run in one
 * shell to be recorded and replayed in another shell
 */
fn build_env<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("env")
        .about("Save or restore an existing reups environment")
        .arg(
            Arg::with_name("command")
                .required(true)
                .possible_values(&["save", "restore", "delete", "list"])
                .help("Action to take for a given environment, to restore you most likely want to use the rrestore shell function"),
        )
        .arg(
            Arg::with_name("name")
                .required(false)
                .help("Optional name to save/restore"),
        );
}

/**
 * Builds cli interface for the subcommand declare. This allows new products to be declared to the
 * database.
 **/
fn build_declare<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("declare")
        .about("Declare a new product to the reups database. All paths are expanded unless relative is set, in which case paths are assumed to be relative to database path")
        .arg(
            Arg::with_name("product")
                .required(true)
                .help("Product name"),
        )
        .arg(
            Arg::with_name("version")
                .required(true)
                .help("Version name/number to assign to product"),
        )
        .arg(
            Arg::with_name("path")
                .required(true)
                .help("Path to directory of product to declare")
                .short("r")
                .long("root")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tag")
                .required(false)
                .help("Tag to assign to product")
                .short("t")
                .long("tag")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("source")
                .required(false)
                .help("Database source to declare to, list with reups list --sources")
                .long("source"),
        )
        .arg(
            Arg::with_name("ident")
                .required(false)
                .help("Unique identifier to assign to product")
                .long("ident")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("relative")
                .required(false)
                .help("Set this to allow declaring relative paths, otherwise paths are expanded")
                .long("relative")
                .takes_value(false),
        );
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
        .version(clap::crate_version!())
        .arg(
            Arg::with_name("verbose")
                .global(true)
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Sets the level of verbosity, multiple occurances increases verbosity"),
        )
        .arg(
            Arg::with_name("database")
                .global(true)
                .short("Z")
                .long("database")
                .takes_value(true)
                .help("Colon-separated list of paths to database to use"),
        )
        .arg(
            Arg::with_name("nouser")
                .global(true)
                .short("U")
                .long("nouser")
                .help("Disable loading database from standard user locations"),
        )
        .arg(
            Arg::with_name("nosys")
                .global(true)
                .short("S")
                .long("nosys")
                .help("Disable loading database found in system environment variables"),
        )
        .subcommand(build_setup())
        .subcommand(build_prep())
        .subcommand(build_list())
        .subcommand(build_completions())
        .subcommand(build_env())
        .subcommand(build_declare())
}

/**
 * This is the main argument parser for the reups program. It parses the arguments from the command
 * line into a `ArgMatches` object containing all the supplied options.
 */
pub fn parse_args<'a>() -> ArgMatches<'a> {
    let matches = build_cli().get_matches();
    return matches;
}
