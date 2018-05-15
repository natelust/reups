extern crate reups;

fn main() {
    let args = reups::parse_args();

    match args.subcommand() {
        ("setup", Some(m)) => {
            /*
            if m.is_present("deps") {
                println!("ignore deps");
            }
            let product = m.value_of("product");
            let db = reups::DB::new(None, None, None);
            if let Some(name) = product {
                println!("Setting up product {}", name);
                db.product_versions(&name.to_string());
            }
            */
            reups::setup_command(m, &args);
        },
        _ => println!("{}",args.usage()),
    }
}
