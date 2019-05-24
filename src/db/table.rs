/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use fnv::FnvHashMap;
use lazy_static;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path;

/**!
 A Table object is the in memory representation of a products table file.
*/

lazy_static::lazy_static! {
    // Regexes to capture information out of the text of a table file
    // captures exact dependency trees
    static ref EXACT: Regex = Regex::new(r"(?m)^\s*[^#](?P<type>etup(Optional|Required))[(](?P<product>[[:word:]]+?\b)\s+[-]j\s(?P<version>\S+?\b)[)]").unwrap();
    // captures inexact dependency trees
    static ref INEXACT: Regex = Regex::new(r"(?m)^\s*[^#](?P<type>etup(Optional|Required))[(](?P<product>[[:word:]]+?\b)(\)|\s+(?P<version>[^-\s]\S+?\b)\s\[?)").unwrap();


    // Finds variables to be prepended to an environment variable
    static ref ENV_PREPEND: Regex = Regex::new(r"(envPrepend|pathPrepend)[(](?P<var>.+?)[,]\s(?P<target>.+?)[)]").unwrap();
    // Finds variables to be appended to an environment variable
    static ref ENV_APPEND: Regex = Regex::new(r"(envAppend|pathAppend)[(](?P<var>.+?)[,]\s(?P<target>.+?)[)]").unwrap();
    static ref ENV_SET: Regex = Regex::new(r"(envSet)[(](?P<var>.+?)[,]\s(?P<target>.+?)[)]").unwrap();
}

/// VersionType is an enum that differentiates between dependency trees that have
/// explicit exact versions sepecified, or if specific versions will be determined
/// with tags.
#[derive(Clone)]
pub enum VersionType {
    Exact,
    Inexact,
}

/// Enum to describ the action of an environment variable, prepend or append to the
/// env var.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvActionType {
    Prepend,
    Append,
    Set,
}

/// Deps describes if a product is a required or optional dependency. Required
/// dependencies will cause the application to abort if they are not present
#[derive(Debug, Clone)]
pub struct Deps {
    pub required: FnvHashMap<String, String>,
    pub optional: FnvHashMap<String, String>,
}

/// Structure containing all the information about an on disk table file
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub path: Option<path::PathBuf>,
    pub product_dir: path::PathBuf,
    pub exact: Option<Deps>,
    pub inexact: Option<Deps>,
    pub env_var: FnvHashMap<String, (EnvActionType, String)>,
}

impl Table {
    /// Creates a new Table object given the product name to assign, the path to the
    /// table file, and the directory the product is located in
    pub fn from_file(
        name: String,
        path: path::PathBuf,
        prod_dir: path::PathBuf,
    ) -> Result<Table, io::Error> {
        // expand product path in case there are any relative links in the path
        let prod_dir = prod_dir
            .canonicalize()
            .expect("Problem getting full table path");
        let mut f = File::open(path.clone())?;
        crate::debug!("Opened file {}", path.to_str().unwrap());
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        crate::debug!("Read file {}", path.to_str().unwrap());
        // Get the exact mapping
        // Dereferencing and taking a reference is nesseary to cause the
        // lazy static object defined at the top to be evaluated and turned into
        // a proper static, this only happens at first dereference. These are
        // defined as statics because they will remain between different tables
        // being created
        let exact = Table::extract_setup(contents.as_str(), &*EXACT);
        crate::debug!("Table for {} contains exact dependencies {:?}", name, exact);
        // Get the inexact mapping
        let inexact = Table::extract_setup(contents.as_str(), &*INEXACT);
        crate::debug!(
            "Table for {} contains inexact dependencies {:?}",
            name,
            inexact
        );
        let mut env_var = FnvHashMap::default();
        let env_re_vec: Vec<&Regex> = vec![&*ENV_PREPEND, &*ENV_APPEND, &*ENV_SET];
        for (re, action) in env_re_vec.iter().zip(
            [
                EnvActionType::Prepend,
                EnvActionType::Append,
                EnvActionType::Set,
            ]
            .iter(),
        ) {
            for cap in re.captures_iter(contents.as_str()) {
                let var = String::from(&cap["var"]);
                let target = String::from(&cap["target"]);
                let final_target = target.replace("${PRODUCT_DIR}", prod_dir.to_str().unwrap());
                env_var.insert(var, (action.clone(), final_target));
            }
        }
        Ok(Table {
            name: name,
            path: Some(path),
            product_dir: prod_dir,
            exact: exact,
            inexact: inexact,
            env_var: env_var,
        })
    }

    /// Extracts the part of the table file that is related to the dependencies of the
    /// table file
    fn extract_setup(input: &str, re: &Regex) -> Option<Deps> {
        let temp_string = input;
        let mut required_map = FnvHashMap::default();
        let mut optional_map = FnvHashMap::default();
        for dep_cap in re.captures_iter(temp_string) {
            let option_type = &dep_cap["type"];
            let prod = &dep_cap["product"];
            let vers = match dep_cap.name("version") {
                Some(ver) => ver.as_str(),
                None => "",
            };
            // These are not missing the s character, the regex matches against not #
            // which catches the first s
            if option_type == "etupRequired" {
                required_map.insert(String::from(prod), String::from(vers));
            }
            if option_type == "etupOptional" {
                optional_map.insert(String::from(prod), String::from(vers));
            }
        }
        Some(Deps {
            required: required_map,
            optional: optional_map,
        })
    }
}
