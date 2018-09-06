/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use std::env;
use std::process;
use std::path::PathBuf;

pub fn get_eups_path_from_env() -> PathBuf {
    let env_var = env::var("EUPS_PATH").unwrap_or_else(|e|{
        eprintln!("Problem loading eups path {}", e);
        process::exit(1);
    });
    let eups_path_vec : Vec<&str> = env_var.split(":").collect();
    // only return the first member of the vec, which should be the most
    // recently added eups path
    let eups_path_option = eups_path_vec.first();
    let mut eups_path = match eups_path_option {
        Some(eups_path) => PathBuf::from(eups_path),
        None => {
            eprintln!("Problem loading eups path from env var");
            process::exit(1);
        }
    };
    eups_path.push("ups_db");
    if eups_path.is_dir(){
        eups_path
    }
    else {
       eprintln!("Eups path defined in env var does not appear to be a directory");
       process::exit(1);
    }
}

pub fn get_user_path_from_home() -> Option<PathBuf> {
    let user_home= env::home_dir();
    let mut user_path = user_home?;
    user_path.push(".eups/ups_db");
    if user_path.is_dir() {
        Some(user_path)
    }
    else {
        None
    }
}
