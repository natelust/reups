extern crate petgraph;

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
        let table = db.get_table_from_tag(&name.to_string(), vec![&String::from("current")]);
        let mut dep_graph = db::graph::Graph::new(&db);
        dep_graph.add_or_update_product(String::from(name),
                                        db::graph::NodeType::Required);
        if let Some(deps) = table.unwrap().inexact {
            for (k, v) in deps.required.iter() {
                dep_graph.add_or_update_product(k.clone(),
                                                db::graph::NodeType::Required);
                let _ = dep_graph.connect_products(&name.to_string(), &k, v.clone());
            }
        }
        //let connections = dep_graph.product_versions(&"afw".to_string());
        //println!("{:?}", connections);
        println!("{:?}", dep_graph);
    }   
}

