use db;
use argparse;

pub fn setup_command(sub_args: & argparse::ArgMatches, main_args: & argparse::ArgMatches){
    // Here we will process any of the global arguments in the future but for now there is
    // nothing so we do nothing but create the database. The global arguments might affect
    // construction in the future
    let db = db::DB::new(None, None, None);

    // We process local arguments here to set the state that will be used to setup a product
    if sub_args.is_present("deps") {
        println!("ignore deps");
    }   
    let product = sub_args.value_of("product");
    if let Some(name) = product {
        println!("Setting up product {}", name);
        //db.product_versions(&name.to_string());
        let version = db.get_table_from_tag(&name.to_string(), &String::from("current"));
        println!("version from current tag is {:?}", version.unwrap())
    }   
}
