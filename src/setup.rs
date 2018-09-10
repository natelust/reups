/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use fnv::FnvHashMap;

use std::env;
use std::path::PathBuf;
use std::fs;

use db;
use table;
use argparse;

#[cfg(target_os = "macos")]
static SYSTEM_OS: &str = "Darwin64";
#[cfg(target_os = "linux")]
static SYSTEM_OS: &str = "Linux64";

fn setup_table(product_version : &String, product_table: &table::Table, env_vars :& mut FnvHashMap<String, String>, keep: bool, flavor : &String, db_path: PathBuf) {
    // set the setup env var
    let mut setup_var = String::from("SETUP_");
    // set the product directory
    let mut prod_dir_label = product_table.name.clone();
    prod_dir_label = prod_dir_label.replace(" ", "_");
    prod_dir_label = prod_dir_label.to_uppercase();
    setup_var.push_str(prod_dir_label.as_str());
    prod_dir_label.push_str("_DIR");

    // get the current env var correspoinding to this prod dir
    let prod_dir_env = env::var(&prod_dir_label);

    // If told to keep existing products, and those products are in the env in some fashion return
    // immediately
    if keep && (env_vars.contains_key(&prod_dir_label) || prod_dir_env.is_ok()) {
        return
    }

    // add this product in to the environment map that is to be setup
    let mut setup_string_vec = vec![product_table.name.clone(),
                                    product_version.clone()];

    // if there is no flavor use the system os as platform
    setup_string_vec.push("-f".to_string());
    if flavor.is_empty() {
        setup_string_vec.push(SYSTEM_OS.to_string());
    }
    else{
        setup_string_vec.push(flavor.clone());
    }

    // Set db dir to none if there is no db dir (local setup)
    setup_string_vec.push("-Z".to_string());
    if db_path.to_str().unwrap().is_empty() {
        setup_string_vec.push("\\(none\\)".to_string());
    }
    else {
        setup_string_vec.push(db_path.to_str().unwrap().to_string().replace("ups_db",""));
    }
    env_vars.insert(prod_dir_label, String::from(product_table.product_dir.to_str().unwrap()));
    env_vars.insert(setup_var, setup_string_vec.join("\\ "));

    // iterate over all environment variables, values in the supplied table
    for (k, v) in product_table.env_var.iter(){
        // look up the specific env var specified in the table in the env_vars hashmap passed into
        // this function. If there is no existing variable in the hash map, check the environment
        // that was present when the program was executed. If it is found no where, return None
        // to mark there is no existing variables.
        let mut existing_var = match env_vars.get(k) {
            Some(existing) => existing.clone(),
            None => {
                match env::var(k) {
                    Ok(r) => r,
                    Err(_) => String::from("")
                }
            }
        };


        // if the prod_dir_env is not none, then the value of this variable should be removed from all
        // existing env var values before being set again, to prevent the variable from growing out
        // of control
        // 
        // Variables to mark the start and end position of where the prod_dir_env value is found in
        // the value of the environment variable (k). AKA LD_LIBRARY_PATH is a long string, find
        // the location of the substring corresponding to the value of prod_dir_env
        let mut start_pos = 0;
        let mut end_pos = 0;
        // Check if there was a current value set in the environment
        if let Ok(prod_text) = prod_dir_env.as_ref() {
            // Find the start position of the text
            let start_pos_option = existing_var.find(prod_text.as_str());
            // check if a start position was found
            if let Some(tmp_start) = start_pos_option {
                start_pos = tmp_start;
                // iterate character by character until either a : or the end of the string is
                // encountered. If one is found, get the end point plus one (+1 so that the
                // character is encluded in the subsiquent removal, as the end point in that
                // function call is not inclusive)
                for (i, character) in existing_var[tmp_start..].chars().enumerate() {
                    let glob_index = tmp_start + i;
                    if character == ':' || glob_index == existing_var.len() {
                        end_pos = glob_index+1;
                        break;
                    }
                }
            }
            // If an end point was found that means the string was found and has bounds.
            // Replace the range of the string with an empty str
            if end_pos != 0 {
                existing_var.replace_range(start_pos..end_pos, "");
            }
        }

        // check the action type and appropriately add the new value onto the env variable
        // under investigation in this loop
        let output_var = match v {
            (table::EnvActionType::Prepend, var) => {
                [var.clone(), existing_var].join(":")
            },
            (table::EnvActionType::Append, var) => {
                [existing_var, var.clone()].join(":")
            }
        };

        // Add the altered string back into the hash map of all env vars
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
        let table_file = table_file.canonicalize().unwrap();
        let prod_dir = prod_dir.unwrap().canonicalize().unwrap();
        let name = String::from(table_file.file_stem().unwrap().to_str().unwrap());
        Some(table::Table::new(name, table_file, prod_dir).unwrap())
    }
    else {
        return None
    }
}

pub fn setup_command(sub_args: & argparse::ArgMatches, _main_args: & argparse::ArgMatches){
    // Here we will process any of the global arguments in the future but for now there is
    // nothing so we do nothing but create the database. The global arguments might affect
    // construction in the future
    let db = db::DB::new(None, None, None, None);

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
                exit_with_message!(format!("Cannot find product `{}` to setup", name));
            }
            let local_table = db.get_table_from_tag(&name.to_string(), tags.clone());
            let versions = db.get_versions_from_tag(&name.to_string(), tags.clone());
            let mut version = String::from("");
            match versions.last() {
                Some(v) => {version = v.clone();},
                None => ()
            }
            (local_table, version)
        },
        (None, Some(path)) => {
            // specifying a directory of table file to setup manually implies that version type
            // should be set to Inexact
            let table = get_table_path_from_input(path);
            let mut version = String::from("");
            if table.is_some() {
                let mut tmp = String::from("LOCAL:");
                tmp.push_str(table.as_ref().unwrap().path.to_str().unwrap());
                version = tmp
            }
            mode = table::VersionType::Inexact;
            (table, version)
        },
        _ => (None, String::from(""))
    };

    let keep = sub_args.is_present("keep");

    if let (Some(table), version) = table_option {
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
        let flavors = db.get_flavors_from_version(&table.name, &version);
        let flavor = match flavors.last(){
            Some(flav) => flav.clone(),
            None => String::from("")
        };

        let db_dirs = db.get_db_directories();
        // This works because there are 2 dbs, the first is always the system one
        // the second is always the user db. and we always want to take the entry from
        // the end if it exists, in this case that is the flavor entry
        let db_path = match flavors.len() {
            1 => db_dirs[0].clone(),
            2 => db_dirs[1].clone(),
            _ => PathBuf::from("") // Needed to satisfy rust matching

        };

        setup_table(&version, &table, & mut env_vars, keep, &flavor, db_path);

        if let Some(dependencies) = deps {
            // Skip the root node, as it is what is setup
            for node in dependencies.iter().skip(1){
                let name = dependencies.get_name(node);
                let versions = dependencies.product_versions(&name);
                // right now we find the largest version from the graph and set that up, as it is
                // easiest, but it could be wrong and this code should be thought through more.
                // FINDME
                let mut largest_version = versions.iter().max().unwrap().clone().clone();
                let mut node_table_option : Option<table::Table>;
                if largest_version.as_str() != "" {
                    node_table_option = db.get_table_from_version(&name, &largest_version);
                }
                else {
                    node_table_option = db.get_table_from_tag(&name, tags.clone());
                    let versions = db.get_versions_from_tag(&name, tags.clone());
                    match versions.last() {
                        Some(v) => {largest_version = v.clone();},
                        None => ()
                    }
                }
                match (node_table_option, dependencies.is_optional(&name)) {
                    (Some(node_table), _) => {
                        let flavors = db.get_flavors_from_version(&node_table.name, &largest_version);
                        let flavor = match flavors.last() {
                            Some(flav) => flav.clone(),
                            None => String::from("")
                        };
                        // This works because there are 2 dbs, the first is always the system one
                        // the second is always the user db. and we always want to take the entry from
                        // the end if it exists, in this case that is the flavor entry
                        let db_path = match flavors.len() {
                            1 => db_dirs[0].clone(),
                            2 => db_dirs[1].clone(),
                            _ => PathBuf::from("") // Needed to satisfy rust matching
                        };
                        setup_table(&largest_version, &node_table, & mut env_vars, keep, &flavor, db_path)
                    },
                    (None, true) => continue,
                    (None, false) => {
                        exit_with_message!(format!("Cannot find any acceptable table for {}", &name));
                    }
                }
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
        exit_with_message!("Error, no product to setup, please specify product or path to table with -r");
    }
}

