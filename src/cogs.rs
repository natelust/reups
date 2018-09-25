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

use std::env;
use std::path::PathBuf;

/** Macro used to print an error message to the console and terminate execution
 *
 * This may be replaced in the future with the use of a logging system.
 */
macro_rules! exit_with_message {
    ($message:expr) => {
        use std::process::exit;
        eprintln!("{}", $message);
        exit(1);
    };
}

/// Returns the eups system path as determined from the EUPS_PATH environment variable.
///
/// If EUPS_PATH contains more than one database path, they should be seperated by a pipe
/// character. This function will return the first database path, as it should be the most
/// recently added to the environment.
pub fn get_eups_path_from_env() -> PathBuf {
    let env_var = env::var("EUPS_PATH").unwrap_or_else(|e| {
        exit_with_message!(format!("Problem loading eups path: {}", e));
    });
    let eups_path_vec: Vec<&str> = env_var.split(":").collect();
    // only return the first member of the vec, which should be the most
    // recently added eups path
    let eups_path_option = eups_path_vec.first();
    let mut eups_path = match eups_path_option {
        Some(eups_path) => PathBuf::from(eups_path),
        None => {
            exit_with_message!("Problem loading eups path from env var");
        }
    };
    eups_path.push("ups_db");
    if eups_path.is_dir() {
        eups_path
    } else {
        exit_with_message!("Eups path defined in env var does not appear to be a directory");
    }
}

/// Returns the path to a user database, defined in users home directory, if one is present.
pub fn get_user_path_from_home() -> Option<PathBuf> {
    let user_home = env::home_dir();
    let mut user_path = user_home?;
    user_path.push(".eups/ups_db");
    if user_path.is_dir() {
        Some(user_path)
    } else {
        None
    }
}
