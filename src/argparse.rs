extern crate clap;
pub use self::clap::{ArgMatches};
use self::clap::{Arg,App, SubCommand};

fn build_setup<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("setup")
                          .arg(Arg::with_name("product")
                               .help("Product to setup")
                               .index(1))
                          .arg(Arg::with_name("deps")
                               .help("ignore dependncies")
                               .short("j"))
                          .arg(Arg::with_name("relative")
                               .help("setup relative path")
                               .short("r"));
}

pub fn parse_args<'a> () -> ArgMatches<'a> {
    let matches = App::new("Rust Eups").version("0.1")
                           .author("Nate Lust")
                           .about("Dynamic environment management")
                           .subcommand(build_setup()).get_matches();
    return matches;
}

