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

fn build_prep<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("prep");
}

pub fn parse_args<'a> () -> ArgMatches<'a> {
    let matches = App::new("Rust Eups")
                           .author("Nate Lust")
                           .about("Dynamic environment management")
                           .version(crate_version!())
                           .subcommand(build_setup())
                           .subcommand(build_prep()).get_matches();
    return matches;
}

