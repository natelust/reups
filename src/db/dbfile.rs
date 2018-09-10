/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use fnv::FnvHashMap;
use std::io;
use std::path;
use std::str;

use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub struct DBFile {
    path: path::PathBuf,
    contents: RefCell<FnvHashMap<String, String>>
}

impl DBFile {
    pub fn new(path: path::PathBuf, preload: bool) -> DBFile {
        let db_file = DBFile {
            path: path.clone(),
            contents: RefCell::new(FnvHashMap::default())
        };
        if preload {
            db_file.load_file().unwrap_or_else(|_e| {
                exit_with_message!(
                    format!("Problem accessing {}, could not create database",
                            path.to_str().unwrap()));
            });
        }
        db_file
    }

    pub fn entry(& self, key: & String) -> Option<String> {
        let db_is_empty: bool;
        {
            db_is_empty = self.contents.borrow().is_empty();
        }
        if db_is_empty {
            self.load_file().unwrap_or_else(|_e|{
                exit_with_message!(
                    format!("Problem accessing {}, could not create database",
                            self.path.to_str().unwrap()));
            });
        }
        match self.contents.borrow().get(key) {
           Some(value) => Some(value.clone()),
           None => None
        }
    }

    fn load_file(& self) -> Result<(), io::Error> {
        let mut f = File::open(&self.path)?;
        let file_len = f.metadata().unwrap().len();

        let mut contents = String::with_capacity(file_len as usize +1);
        f.read_to_string(&mut contents)?;

        for line in contents.as_str().lines() {
            let split : Vec<&str> = line.split("=").collect();
            if split.len() == 2 {
                let key = String::from(split[0].trim());
                let value = String::from(split[1].trim());
                self.contents.borrow_mut().insert(key, value);
            }
        }

        Ok(())
    }
}

