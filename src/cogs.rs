/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/*!
The `cogs` module contains miscellaneous components used to interact with
reups.

These functions are defined here, to make them available in one central
location within the `reups_lib` library. The main library re-exports this
module, so end users of `reups_lib` should see all functions exposed there.
*/

use app_dirs;
use dirs;
use std::env;
use std::path::PathBuf;

const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
    name: "reups",
    author: "Reups Community",
};

// Determine the system on which this comand is run. In eups past there used to be
// more flavors (i.e. just linux) but these systems are almost never used and are
// dropped from consideration in reups.
#[cfg(target_os = "macos")]
pub static SYSTEM_OS: &str = "Darwin64";
#[cfg(target_os = "linux")]
pub static SYSTEM_OS: &str = "Linux64";

/** Macro used to print an error message to the console and terminate execution
 *
 * This may be replaced in the future with the use of a logging system.
 */
#[macro_export]
macro_rules! exit_with_message {
    ($message:expr) => {
        use std::process::exit;
        eprintln!("{}", $message);
        exit(1);
    };
}

/// Splits apart a string with paths separated by colons into a vector of paths
pub fn path_string_to_vec(path_string: &str) -> Result<Vec<PathBuf>, String> {
    let eups_path_vec: Vec<&str> = path_string.split(":").collect();
    if eups_path_vec.is_empty() {
        return Err("Path is empty".to_string());
    }
    let eups_pathbuf_vec: Vec<PathBuf> = eups_path_vec
        .iter()
        .filter_map(|path| {
            let mut converted_path = PathBuf::from(path);
            let extension = converted_path.extension();
            // Check if the supplied path is a json file, if so just return it
            if extension.is_some() && extension.unwrap() == "json" {
                Some(converted_path)
            } else {
                converted_path.push("ups_db");
                if !converted_path.is_dir() {
                    return None;
                }
                Some(converted_path)
            }
        })
        .collect();
    if eups_path_vec.len() != eups_pathbuf_vec.len() {
        return Err(format!(
            "One of the paths specified in {} is not a valid db\n",
            path_string
        ));
    }
    Ok(eups_pathbuf_vec)
}

/// Returns the eups system path as determined from the EUPS_PATH environment variable.
///
/// If EUPS_PATH contains more than one database path, they should be seperated by a pipe
/// character.
pub fn get_eups_path_from_env() -> Result<Vec<PathBuf>, String> {
    let env_var = match env::var("EUPS_PATH") {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Problem loading eups path: {}", e));
        }
    };
    crate::debug!("Found {} in environment variable", env_var);
    let system_paths_option = path_string_to_vec(env_var.as_str());
    match system_paths_option {
        Ok(system_paths) => Ok(system_paths),
        Err(_) => Err("Problem loading eups paths from env".to_string()),
    }
}

/// Returns the path to a user database, defined in users home directory, if one is present.
pub fn get_eups_user_db() -> Option<PathBuf> {
    let user_home = dirs::home_dir();
    let mut user_path = user_home?;
    user_path.push(".eups/ups_db");
    if user_path.is_dir() {
        Some(user_path)
    } else {
        None
    }
}

/// Returns the reups paths as determined from REUPS_PATH environment variable.
/// This should point to json db source files.
///
/// If REUPS_PATH contains more than one database path, they should be seperated by a pipe
/// character.
pub fn get_reups_path_from_env() -> Result<Vec<PathBuf>, String> {
    let env_var = match env::var("REUPS_PATH") {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Problem loading eups path: {}", e));
        }
    };
    crate::debug!("Found {} in environment variable", env_var);
    let system_paths_option = path_string_to_vec(env_var.as_str());
    match system_paths_option {
        Ok(system_paths) => Ok(system_paths),
        Err(_) => Err("Problem loading eups paths from env".to_string()),
    }
}

/// Returns the path to a user database, defined in users home directory, if one is present.
pub fn get_reups_user_db() -> Option<PathBuf> {
    let user_data = app_dirs::app_root(app_dirs::AppDataType::UserData, &APP_INFO);
    let mut user_path = match user_data {
        Ok(x) => x,
        Err(_) => {
            crate::error!(
                "There was a problem determining the user app directory on this platform"
            );
            return None;
        }
    };
    user_path.push("reups_user_db.json");
    if (user_path.exists()) {
        Some(user_path)
    } else {
        None
    }
}
