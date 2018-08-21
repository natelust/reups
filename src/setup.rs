use std::env;
use std::collections::HashMap;

use db;
use table;
use argparse;

fn setup_table(product_version: &String, product_table: &table::Table, env_vars :& mut HashMap<String, String>) {
    // set the product directory
    let mut prod_dir_label = product_table.name.clone();
    prod_dir_label = prod_dir_label.replace(" ", "_");
    prod_dir_label = prod_dir_label.to_uppercase();
    prod_dir_label.push_str("_DIR");
    env_vars.insert(prod_dir_label, String::from(product_table.product_dir.to_str().unwrap()));

    for (k, v) in product_table.env_var.iter(){
        let mut existing_var = match env_vars.get(k) {
            Some(existing) => existing.clone(),
            None => {
                match env::var(k) {
                    Ok(r) => r,
                    Err(_) => String::from("")
                }
            }
        };

        let output_var = match v {
            (table::EnvActionType::Prepend, var) => {
                [var.clone(), existing_var].join(":")
            },
            (table::EnvActionType::Append, var) => {
                [existing_var, var.clone()].join(":")
            }
        };

        env_vars.insert(k.clone(), output_var);
    }

}

pub fn setup_command(sub_args: & argparse::ArgMatches, _main_args: & argparse::ArgMatches){
    // Here we will process any of the global arguments in the future but for now there is
    // nothing so we do nothing but create the database. The global arguments might affect
    // construction in the future
    let db = db::DB::new(None, None, None);

    // We process local arguments here to set the state that will be used to setup a product
    // Create a vector for the tags to consider
    let current = String::from("current");
    let mut tags_str = vec![];
    let mut tags = vec![];
    if sub_args.is_present("tag") {
        for t in sub_args.values_of("tag").unwrap() {
            tags_str.push(t.to_string());
        }
        for t in tags_str.iter() {
            tags.push(t);
        }
    }
    // Always put the current tag
    tags.push(& current);

    let product = sub_args.value_of("product");
    // Get if the command should be run in exact or inexact mode
    let mut mode = table::VersionType::Exact;
    if sub_args.is_present("inexact") {
        mode = table::VersionType::Inexact;
    }
    if let Some(name) = product {
        let table = db.get_table_from_tag(&name.to_string(), tags.clone());
        // If someone specified the just flag, don't look up any dependencies
        let mut deps :Option<db::graph::Graph> = None;
        if !sub_args.is_present("just") {
            let mut dep_graph = db::graph::Graph::new(&db);
            dep_graph.add_table(table.as_ref().unwrap(),
                                mode,
                                db::graph::NodeType::Required,
                                Some(&tags),
                                true);

            /*
            for node in dep_graph.iter().skip(1){
                let name = dep_graph.get_name(node);
                println!("{}, versions {:?}", &name, dep_graph.product_versions(&name));
            }
            */
            deps = Some(dep_graph);
        }
        let prod_versions = db.get_versions_from_tag(&name.to_string(), tags.clone());
        // create a hashmap to hold all the environment variables to set
        let mut env_vars : HashMap<String, String> = HashMap::new();
        setup_table(prod_versions[0].as_ref().unwrap(), &table.unwrap(), & mut env_vars);

        if let Some(dependencies) = deps {
            // Skip the root node, as it is what is setup
            for node in dependencies.iter().skip(1){
                let name = dependencies.get_name(node);
                let versions = dependencies.product_versions(&name);
                // right now we find the largest version from the graph and set that up, as it is
                // easiest, but it could be wrong and this code should be thought through more.
                // FINDME
                let largest_version = versions.iter().max().unwrap();
                let node_table = db.get_table_from_version(&name, &largest_version).unwrap();
                setup_table(&largest_version, &node_table, & mut env_vars);
            }
        }
        // Process all the environment variables into a string to return
        let mut return_string = String::from("export ");
        for (k, v) in env_vars {
            return_string.push_str([k, v].join("=").as_str());
            return_string.push_str(" ");
        }
        println!("{}", return_string);
    }   
}

