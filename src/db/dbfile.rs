/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/*!
  A DBFile is an in memory representation of a (r)eups database file.
  These files are either version files, or tag files, and describe mappings
  of tags to version files, and version files to table file locations.

  By in normal circumstances a DBFile does not read the contents of the file it
  represents off disk until the first time it is accessed. This dramatically
  increases program run time as io is time consuming. However, in some cases it
  makes more sense to get the io out of the way ans so there is a preload
  boolean in the new function that determines if the file should be read at the
  creation time of the object.
 */

use fnv::FnvHashMap;
use std::io;
use std::path;

use std::cell::RefCell;
use std::fs;

#[derive(Debug)]
pub struct DBFile {
    path: path::PathBuf,
    // Contents are a ReffCell so that there can be a mutable hashmap in an immutable
    // DBFile
    contents: RefCell<FnvHashMap<String, String>>
}

impl DBFile {
    /// Creates a new DBFile object. Input is the path to the file on disk this
    /// object represents, and the preload boolean controls if that file should
    /// be loaded at creation time, or left until first access of this object.
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

    /// Retrives the value of the DBFile corresponding to the supplied key
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

    /// Loads the file associated with this DBFile object off disk, and then
    /// parses the file line by line. Any line that has an equals in it is
    /// split with the left side of the equals being the key, and the right
    /// becomes the value
    fn load_file(& self) -> Result<(), io::Error> {
        let contents = fs::read_to_string(&self.path)?;

        for line in contents.lines() {
            for (i, char) in line.char_indices() {
                if char == '=' {
                    let key = line[0..i].trim();
                    let value = line[i+1..].trim();
                    self.contents.borrow_mut().insert(key.to_owned(),
                                                      value.to_owned());
                    break;
                }
            }
        }

        Ok(())
    }
}
