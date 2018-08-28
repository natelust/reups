extern crate fnv;

use self::fnv::FnvHashMap;

use std::env;
use std::path::PathBuf;
use std::fs;
use std::process;

use db;
use table;
use argparse;

fn setup_table(product_table: &table::Table, env_vars :& mut FnvHashMap<String, String>, keep: bool) {
    // set the product directory
    let mut prod_dir_label = product_table.name.clone();
    prod_dir_label = prod_dir_label.replace(" ", "_");
    prod_dir_label = prod_dir_label.to_uppercase();
    prod_dir_label.push_str("_DIR");

    // get the current env var correspoinding to this prod dir
    let prod_dir_env = env::var(&prod_dir_label);

    // If told to keep existing products, and those products are in the env in some fashion return
    // immediately
    if keep && (env_vars.contains_key(&prod_dir_label) || prod_dir_env.is_ok()) {
        return
    }

    // add this product in to the environment map that is to be setup
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


        // if the prod_dir_env is not none, than this product should be removed from all
        // existing env var keys before being set again
        let mut start_pos = 0;
        let mut end_pos = 0;
        if let Ok(prod_text) = prod_dir_env.as_ref() {
            let start_pos_option = existing_var.find(prod_text.as_str());
            if let Some(tmp_start) = start_pos_option {
                start_pos = tmp_start;
                for (i, character) in existing_var[tmp_start..].chars().enumerate() {
                    let glob_index = tmp_start + i;
                    if character == ':' || glob_index == existing_var.len() {
                        end_pos = glob_index+1;
                        break;
                    }
                }
            }
            if end_pos != 0 {
                existing_var.replace_range(start_pos..end_pos, "");
            }
        }

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

fn get_table_path_from_input(input_path: & str) -> Option<table::Table> {
    let mut input_pathbuf = PathBuf::from(input_path);
    // check if the full path to the table file was given
    let mut table_path : Option<PathBuf> = None;
    let mut prod_dir: Option<PathBuf> = None;
    if input_pathbuf.is_file() {
        if let Some(extension) = input_pathbuf.extension() {
            // if this is true, then the input path is the table path
            if extension.to_str().unwrap() == "table" {
                table_path = Some(input_pathbuf.clone());
                // assumes this is {prod_dir}/ups/something.table
                let mut tmp_prod_dir = input_pathbuf.clone();
                tmp_prod_dir.pop();
                tmp_prod_dir.pop();
                prod_dir = Some(tmp_prod_dir);
            }
        }
    }
    else if input_pathbuf.is_dir() {
        // The supplied path is a directory, it should be checked if it is an ups directory
        // or a directory containing an ups directory
        let mut search_path : Option<PathBuf> = None;
        if input_pathbuf.ends_with("ups") {
            search_path = Some(input_pathbuf.clone());
        }
        input_pathbuf.push("ups");
        if input_pathbuf.is_dir() {
            search_path = Some(input_pathbuf);
        }
        // need to scan the search dir for the table file
        if !search_path.is_none() {
           for entry in fs::read_dir(&search_path.unwrap()).unwrap(){
               let entry = entry.unwrap();
               if let Some(extension) = entry.path().extension() {
                   if extension.to_str().unwrap() == "table"{
                       table_path = Some(entry.path());
                       let mut tmp_prod_dir = entry.path();
                       tmp_prod_dir.pop();
                       tmp_prod_dir.pop();
                       prod_dir = Some(tmp_prod_dir);
                   }
               }
           }
        }
    }
    if let Some(table_file) = table_path {
        let name = String::from(table_file.file_stem().unwrap().to_str().unwrap());
        Some(table::Table::new(name, table_file, prod_dir.unwrap()).unwrap())
    }
    else {
        return None
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

    let table_option = match (product, sub_args.value_of("relative")) {
        (Some(name), _) => {
            if !db.has_product(&name.to_string()) {
                eprintln!("Cannot find product `{}` to setup", name);
                process::exit(1);
            }
            let local_table = db.get_table_from_tag(&name.to_string(), tags.clone());
            local_table
        },
        (None, Some(path)) => {
            // specifying a directory of table file to setup manually implys that version type
            // should be set to Inexact
            let table = get_table_path_from_input(path);
            mode = table::VersionType::Inexact;
            table
        },
        _ => None
    };

    let keep = sub_args.is_present("keep");

    if let Some(table) = table_option {
        // If someone specified the just flag, don't look up any dependencies
        let mut deps :Option<db::graph::Graph> = None;
        if !sub_args.is_present("just") {
            let mut dep_graph = db::graph::Graph::new(&db);
            dep_graph.add_table(&table,
                                mode,
                                db::graph::NodeType::Required,
                                Some(&tags),
                                true);

            deps = Some(dep_graph);
        }
        // create a hashmap to hold all the environment variables to set
        let mut env_vars : FnvHashMap<String, String> = FnvHashMap::default();
        setup_table(&table, & mut env_vars, keep);

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
                setup_table(&node_table, & mut env_vars, keep);
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
    else {
        eprintln!("Error, no product to setup, please specify product or path to table with -r");
        process::exit(1);
    }
}

