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
    contents: RefCell<FnvHashMap<String, String>>,
}

impl DBFile {
    /// Creates a new DBFile object. Input is the path to the file on disk this
    /// object represents, and the preload boolean controls if that file should
    /// be loaded at creation time, or left until first access of this object.
    pub fn new(path: path::PathBuf, preload: bool) -> DBFile {
        let db_file = DBFile {
            path: path.clone(),
            contents: RefCell::new(FnvHashMap::default()),
        };

        if preload {
            db_file.load_file().unwrap_or_else(|_e| {
                exit_with_message!(format!(
                    "Problem accessing {}, could not create database",
                    path.to_str().unwrap()
                ));
            });
        }
        db_file
    }

    pub fn new_with_contents(path: path::PathBuf, file_contents: String) -> DBFile {
        let db_file = DBFile {
            path: path,
            contents: RefCell::new(FnvHashMap::default()),
        };
        db_file.parse_string(file_contents);
        db_file
    }

    /// Retrieves the value of the DBFile corresponding to the supplied key
    pub fn entry(&self, key: &str) -> Option<&str> {
        let db_is_empty: bool;
        {
            db_is_empty = self.contents.borrow().is_empty();
        }
        if db_is_empty {
            self.load_file().unwrap_or_else(|_e| {
                exit_with_message!(format!(
                    "Problem accessing {}, could not create database",
                    self.path.to_str().unwrap()
                ));
            });
        }
        // This unsafe block exists because rust will not allow a reference to be
        // taken from inside a refcell. A refcell gets a reference to the underlying
        // type each time you borrow(), and taking the reference on the borrowed
        // reference fails when the borrowed reference goes out of scope. The reason
        // you cannot get a direct reference to the underlying scope, is that Cells
        // (which have their own method of borrowing through swapping in and out)
        // and RefCells can have their data switched out, and the compiler cannot
        // be sure that the reference you are taking will exist later.
        //
        // In the case of this type, we the programmer know that once the contents are
        // created, there is no way to mutate them, as it is a private internal variable
        // thus we implement this unsafe block with a raw pointer to get a reference to
        // the underlying string, as we are sure that this can never actually change.
        //
        // I THINK rust is smart enough to know that I have declared I am returning a
        // reference and thus the reference should not outlive the object that provides
        // it. However it is possible that if one takes a reference from this function
        // and then the underlying DBFile is dropped, then there could be an issue
        // accessing the wrong part of memory. All my tests done so far indicate that
        // this is true, rust will not let the reference be used after the object
        // is dropped
        let r = unsafe {
            let ptr = self.contents.as_ptr();
            (*ptr).get(key)?
        };
        crate::debug!("Found key, value {}, {}  in DBFile", key, r);
        Some(r)
    }

    /// Loads the file associated with this DBFile object off disk, and then
    /// parses the file line by line. Any line that has an equals in it is
    /// split with the left side of the equals being the key, and the right
    /// becomes the value
    fn load_file(&self) -> Result<(), io::Error> {
        crate::debug!(
            "Populating DBFile with {} from disk",
            self.path.to_str().unwrap()
        );
        let contents = fs::read_to_string(&self.path)?;
        self.parse_string(contents);

        Ok(())
    }

    fn parse_string(&self, contents: String) {
        for line in contents.lines() {
            for (i, char) in line.char_indices() {
                if char == '=' {
                    let key = line[0..i].trim();
                    let value = line[i + 1..].trim();
                    self.contents
                        .borrow_mut()
                        .insert(key.to_string(), value.to_string());
                    break;
                }
            }
        }
    }
}
