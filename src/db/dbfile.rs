use db::fnv::FnvHashMap;
use std::io;
use std::path;
use std::process;

use std::cell::RefCell;
use std::fs::File;
use std::io::prelude::*;

use regex::Regex;

lazy_static! {
    static ref KEY_REGEX: Regex = Regex::new(r"(.*) = (.*)").unwrap();
}

#[derive(Debug)]
pub struct DBFile {
    path: path::PathBuf,
    contents: RefCell<FnvHashMap<String, String>>
}

impl DBFile {
    pub fn new(path: path::PathBuf) -> DBFile {
        DBFile {
            path: path,
            contents: RefCell::new(FnvHashMap::default())
        }
    }

    pub fn entry(& self, key: & String) -> Option<String> {
        let db_is_empty: bool;
        {
            db_is_empty = self.contents.borrow().is_empty();
        }
        if db_is_empty {
            self.load_file().unwrap_or_else(|_e|{
                println!("Problem accessing {}, could not create database",
                         self.path.to_str().unwrap());
                process::exit(1);
            });
        }
        match self.contents.borrow().get(key) {
           Some(value) => Some(value.clone()),
           None => None
        }
    }


    fn load_file(& self) -> Result<(), io::Error> {
        let mut f = File::open(&self.path)?;

        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        for line in contents.as_str().lines() {
            let cap = KEY_REGEX.captures(line);
            match cap {
                Some(c) => {
                    self.contents.borrow_mut().insert(String::from(c.get(1).unwrap().as_str().trim()),
                                                      String::from(c.get(2).unwrap().as_str().trim()));
                },
                None => {
                    continue;
                }
            }
        }
        Ok(())
    }
}

