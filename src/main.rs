extern crate reups;

fn main() {
    let args = reups::parse_args();

    match args.subcommand() {
        ("setup", Some(m)) => {
            reups::setup_command(m, &args);
        },
        ("prep", Some(_)) => {
            println!(
"rsetup() {{
    eval $(reups setup \"$@\");
}}");
        }
        _ => println!("{}",args.usage()),
    }
}
