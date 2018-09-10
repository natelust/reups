/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

pub use clap::{ArgMatches};
use clap::{Arg,App, SubCommand};

fn build_setup<'a, 'b>() -> App<'a, 'b> {
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
                               .long("inexact"));
}

fn build_list<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("list")
                          .arg(Arg::with_name("product")
                               .help("Name of product to list (optional)")
                               .index(1))
                          .arg(Arg::with_name("setup")
                               .help("List only setup products")
                               .short("s")
                               .long("setup"))
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
                               .conflicts_with("onlyTags"));
}

fn build_prep<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("prep");
}

pub fn parse_args<'a> () -> ArgMatches<'a> {
    let matches = App::new("Rust Eups")
                           .author("Nate Lust")
                           .about("Dynamic environment management")
                           .version(crate_version!())
                           .subcommand(build_setup())
                           .subcommand(build_prep())
                           .subcommand(build_list()).get_matches();
    return matches;
}

