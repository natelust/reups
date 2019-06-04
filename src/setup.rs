/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/*!
 Setup is the subcommand responsible for adding products to a users
 environment based on the options provided.
*/

use fnv::FnvHashMap;

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::argparse;
use crate::cogs;
use crate::db;
use crate::db::DBBuilderTrait;
use crate::logger;
use crate::table;

/// Given a product's version and table file, this function creates all the appropriate
/// environment variable entries given the supplied options.
///
/// * product_version: The version of the product being setup
/// * product_table: The table file object for the product being setup
/// * env_vars: HashMap of environment variables with keys equal to the variable name, and values
/// equal to the value of the variable.
/// * keep: bool that controls if this should overwirte a product which already exists in the environment or not
pub fn setup_table(
    product_version: &String,
    product_table: &table::Table,
    env_vars: &mut FnvHashMap<String, String>,
    keep: bool,
    flavor: &String,
    db_path: PathBuf,
    unsetup: bool,
) {
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
        return;
    }

    // add this product in to the environment map that is to be setup
    let mut setup_string_vec = vec![product_table.name.clone(), product_version.clone()];

    // if there is no flavor use the system os as platform
    setup_string_vec.push("-f".to_string());
    if flavor.is_empty() {
        setup_string_vec.push(cogs::SYSTEM_OS.to_string());
    } else {
        setup_string_vec.push(flavor.clone());
    }

    // Set db dir to none if there is no db dir (local setup)
    setup_string_vec.push("-Z".to_string());
    crate::debug!("Using database path: {}", db_path.to_str().unwrap());
    if db_path.to_str().unwrap().is_empty() {
        setup_string_vec.push("\\(none\\)".to_string());
    } else {
        setup_string_vec.push(db_path.to_str().unwrap().to_string().replace("ups_db", ""));
    }
    crate::info!(
        "Setting up: {:<25}Version: {}",
        product_table.name,
        product_version
    );
    if unsetup {
        let _ = env_vars.insert(prod_dir_label, "UNSET".to_string());
        let _ = env_vars.insert(setup_var, "UNSET".to_string());
    } else {
        env_vars.insert(
            prod_dir_label,
            String::from(product_table.product_dir.to_str().unwrap()),
        );
        env_vars.insert(setup_var, setup_string_vec.join("\\ "));
    }

    // iterate over all environment variables, values in the supplied table
    for (k, v) in product_table.env_var.iter() {
        // look up the specific env var specified in the table in the env_vars hashmap passed into
        // this function. If there is no existing variable in the hash map, check the environment
        // that was present when the program was executed. If it is found no where, return None
        // to mark there is no existing variables.
        let mut existing_var = match env_vars.get(k) {
            Some(existing) => existing.clone(),
            None => match env::var(k) {
                Ok(r) => r,
                Err(_) => String::from(""),
            },
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
                        end_pos = glob_index + 1;
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
        if !unsetup {
            // check the action type and appropriately add the new value onto the env variable
            // under investigation in this loop
            let output_var = match v {
                (table::EnvActionType::Prepend, var) => [var.clone(), existing_var].join(":"),
                (table::EnvActionType::Append, var) => [existing_var, var.clone()].join(":"),
                (table::EnvActionType::Set, var) => var.to_string(),
            };

            // Add the altered string back into the hash map of all env vars
            env_vars.insert(k.clone(), output_var);
        }
    }
}

/**
 * If tables are specified as a filesystem path, this function attempts to load and return the
 * table file.
 *
 * Valid input paths are the table file exactly, the path to the ups directory containing the
 * table, or the path to the directory containing the ups directory
 */
fn get_table_path_from_input(input_path: &str) -> Option<table::Table> {
    let mut input_pathbuf = PathBuf::from(input_path);
    // check if the full path to the table file was given
    let mut table_path: Option<PathBuf> = None;
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
    } else if input_pathbuf.is_dir() {
        // The supplied path is a directory, it should be checked if it is an ups directory
        // or a directory containing an ups directory
        let mut search_path: Option<PathBuf> = None;
        if input_pathbuf.ends_with("ups") {
            search_path = Some(input_pathbuf.clone());
        }
        input_pathbuf.push("ups");
        if input_pathbuf.is_dir() {
            search_path = Some(input_pathbuf);
        }
        // need to scan the search dir for the table file
        if !search_path.is_none() {
            for entry in fs::read_dir(&search_path.unwrap()).unwrap() {
                let entry = entry.unwrap();
                if let Some(extension) = entry.path().extension() {
                    if extension.to_str().unwrap() == "table" {
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
        Some(table::Table::from_file(name, table_file, prod_dir).unwrap())
    } else {
        return None;
    }
}

/** Function to ensure a supplied path is not a relative path
 *
 * * input - A string that represents a path to be normalized
 *
 * Returns a normalized path, or an error if the supplied input does not correspond to a file
 * system path, or there was some issue interacting with the file system.
 **/
fn normalize_path(input: String) -> Result<String, std::io::Error> {
    let tmp_path = PathBuf::from(input).canonicalize()?;
    let err = std::io::Error::new(std::io::ErrorKind::Other, "Problem normalizing Path");
    let tmp_string = tmp_path.to_str().ok_or(err)?;
    Ok(String::from(tmp_string))
}

/**
 * Gets the arguments used to invoke this subcommand from the command line, ensures all paths are
 * normalized, and formats these arguments into a single string
 **/
fn get_command_string() -> String {
    // marker to indicate the next argument is a path that should be normalized
    let mut marker = false;
    // String to accumulate the input arguments into
    let mut command_arg = String::new();
    // Make the switches to check a vector, so future switches can be added easily
    // This represents a switch where the following argument will be a path to be
    // normalized
    let switches = vec!["-r"];
    for arg in env::args() {
        let next_string: String = match marker {
            true => {
                // if the marker is set, normalize the current arg and return it, setting
                // marker to false
                marker = false;
                normalize_path(arg).unwrap()
            }
            false => {
                // The marker is not set, check if the current argument is a desired
                // switch and if so set marker so the next argument will be normalzied
                if switches.contains(&arg.as_str()) {
                    marker = true;
                }
                // return argument
                arg
            }
        };
        // push the current argument onto our accumulated string
        command_arg.push_str(format!("{} ", next_string.as_str()).as_str());
    }
    // pop off the trailing white space
    command_arg.pop();
    command_arg
}

/**
 * This function takes in arguments parsed from the command line, parses them for products to setup
 * and options to use during the setup, and sets up the specified product in the
 * environment.
 *
 * Because of the way environments work, this function itself actually only returns a string
 * containing all the environment variables to be setup. To actually have the variables added to
 * the environment, this command must be used in combination with the rsetup shell function.
 */
pub fn setup_command<W: Write>(
    sub_args: &argparse::ArgMatches,
    _main_args: &argparse::ArgMatches,
    writer: &mut W,
) -> Result<(), String> {
    let env_vars = make_setup_env_map(sub_args, None)?;
    // Process all the environment variables into a string to return
    let mut return_string = String::from("export ");
    let mut unset_string = String::from("");
    for (k, v) in env_vars {
        match v.as_str() {
            "UNSET" => unset_string.push_str(&format!("unset {} ", k)),
            _ => {
                return_string.push_str([k, v].join("=").as_str());
                return_string.push_str(" ");
            }
        }
    }
    if unset_string.chars().count() > 0 {
        return_string.push_str("; ");
        return_string.push_str(unset_string.as_str());
    }
    let _ = writer.write(format!("{}\n", return_string).as_bytes());
    Ok(())
}

pub fn make_setup_env_map(
    sub_args: &argparse::ArgMatches,
    db: Option<db::DB>,
) -> Result<FnvHashMap<String, String>, String> {
    // Here we will process any of the global arguments in the future but for now there is
    // nothing so we do nothing but create the database. The global arguments might affect
    // construction in the future
    logger::build_logger(sub_args, std::io::stderr());
    // if no db was passed in, create one from the sub_args
    let db = match db {
        Some(db) => db,
        None => db::DBBuilder::from_args(sub_args).build()?,
    };

    // We process local arguments here to set the state that will be used to setup a product
    // Create a vector for the tags to consider
    let current = &"current";
    let mut tags_str = vec![];
    let mut tags: Vec<&str> = vec![];
    if sub_args.is_present("tag") {
        for t in sub_args.values_of("tag").unwrap() {
            tags_str.push(t);
        }
        for t in tags_str.iter() {
            tags.push(t);
        }
    }
    // Always put the current tag
    tags.push(current);
    crate::info!("Using tags: {:?}", tags);

    let product = sub_args.value_of("product");
    // Get if the command should be run in exact or inexact mode
    let mut mode = table::VersionType::Exact;
    if sub_args.is_present("inexact") {
        mode = table::VersionType::Inexact;
    }

    // Match to determine if a product or relative path was given by the user
    let table_option = match (product, sub_args.value_of("relative")) {
        (Some(name), _) => {
            if !db.has_product(&name.to_string()) {
                exit_with_message!(format!("Cannot find product `{}` to setup", name));
            }
            let local_table = db.get_table_from_tag(name, &tags);
            let versions = db.get_versions_from_tag(&name.to_string(), &tags);
            let mut version = String::from("");
            match versions.first() {
                Some(v) => {
                    version = v.to_string();
                }
                None => (),
            }
            (local_table, version)
        }
        (None, Some(path)) => {
            // specifying a directory of table file to setup manually implies that version type
            // should be set to Inexact
            let table = get_table_path_from_input(path);
            let mut version = String::from("");
            if table.is_some() {
                let mut tmp = String::from("LOCAL:");
                tmp.push_str(
                    table
                        .as_ref()
                        .unwrap()
                        .path
                        .as_ref()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                );
                version = tmp
            }
            mode = table::VersionType::Inexact;
            (table, version)
        }
        _ => (None, String::from("")),
    };

    // Determine if the user wants existing dependencies to be kept in the environment
    // or replaced
    let keep = sub_args.is_present("keep");

    // If there is a valid table and version found, determine dependencies and setup
    // the product
    if let (Some(table), version) = table_option {
        // If someone specified the just flag, don't look up any dependencies
        let mut deps: Option<db::graph::Graph> = None;
        if !sub_args.is_present("just") {
            let mut dep_graph = db::graph::Graph::new();
            let mut dep_graph_db = dep_graph.make_db_helper(&db);
            dep_graph_db.add_table(
                &table,
                mode,
                db::graph::NodeType::Required,
                Some(&tags),
                true,
            );

            deps = Some(dep_graph);
        }
        // create a hashmap to hold all the environment variables to set
        let mut env_vars: FnvHashMap<String, String> = FnvHashMap::default();
        let flavors = db.get_flavors_from_version(&table.name, &version);
        let flavor = match flavors.last() {
            Some(flav) => flav.to_string(),
            None => String::from(""),
        };

        let db_path = db.get_database_path_from_version(&table.name, &version);

        // Keep should always be false for the first product to setup, as this is the
        // directory the user specified, so clearly they want to set it up.
        setup_table(
            &version,
            &table,
            &mut env_vars,
            false,
            &flavor,
            db_path,
            sub_args.is_present("unsetup"),
        );

        // If there are dependencies, then set them up as well
        if let Some(dependencies) = deps {
            // Skip the root node, as it is what is setup
            for node in dependencies.iter().skip(1) {
                let name = dependencies.get_name(node);
                let versions = dependencies.product_versions(&name);
                // right now we find the largest version from the graph and set that up, as it is
                // easiest, but it could be wrong and this code should be thought through more.
                // FINDME
                let mut largest_version = versions.iter().max().unwrap().clone().clone();
                let node_table_option: Option<table::Table>;
                if largest_version.as_str() != "" {
                    node_table_option = db.get_table_from_version(&name, &largest_version);
                } else {
                    node_table_option = db.get_table_from_tag(&name, &tags);
                    let versions = db.get_versions_from_tag(&name, &tags);
                    match versions.last() {
                        Some(v) => {
                            largest_version = v.to_string();
                        }
                        None => (),
                    }
                }
                match (node_table_option, dependencies.is_optional(&name)) {
                    (Some(node_table), _) => {
                        let flavors =
                            db.get_flavors_from_version(&node_table.name, &largest_version);
                        let flavor = match flavors.last() {
                            Some(flav) => flav.to_string(),
                            None => String::from(""),
                        };
                        let db_path =
                            db.get_database_path_from_version(&node_table.name, &largest_version);
                        setup_table(
                            &largest_version,
                            &node_table,
                            &mut env_vars,
                            keep,
                            &flavor,
                            db_path,
                            sub_args.is_present("unsetup"),
                        )
                    }
                    (None, true) => continue,
                    (None, false) => {
                        if env::var(String::from("SETUP_") + &name.to_uppercase()).is_ok() {
                            crate::warn!("Product {} could not be found in the database, resolving dependency using setup version", &name);
                            continue;
                        } else {
                            exit_with_message!(format!(
                                "Cannot find any acceptable table for {}",
                                &name
                            ));
                        }
                    }
                }
            }
        }

        // Add or update env var for reups history
        let current_reups_command = get_command_string();
        // If there is an existing reups history environment variable append to it
        // separating with a pipe character. else return a new string for the env
        // var. Both make sure the string to be set as an environment variable are
        // quoted so that all spaces are preserved
        let reups_history_string = match env::var("REUPS_HISTORY") {
            Ok(existing) => format!("\"{}|{}\"", existing, current_reups_command),
            _ => format!("\"{}\"", current_reups_command),
        };
        let reups_history_key = String::from("REUPS_HISTORY");
        // insert into the in memory map of environment variables to values
        env_vars.insert(reups_history_key, reups_history_string);
        Ok(env_vars)
    } else {
        return Err(
            "Error, no product to setup, please specify product or path to table with -r"
                .to_string(),
        );
    }
}
