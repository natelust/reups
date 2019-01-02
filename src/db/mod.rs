/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/**!
   The db module is the heart of the reups program. The custom in memory
   database it provides encodes the tag, version, table file path relations
   between all the products reups is aware of.
*/
use fnv::FnvHashMap;
mod db_impl;
mod dbfile;
pub mod graph;
pub mod table;

use self::dbfile::DBFile;
use crate::cogs;

use std::cell::RefCell;
use std::fmt;
use std::path::PathBuf;

/// Data structure to hold state related to iterating over a db object.
/// This iteration is used to loop over all the instance of DBImpls
/// contained in the database, which at this point includes the main
/// and user dbs
struct DBIter<'a> {
    inner: &'a DB,
    pos: usize,
    len: usize,
}

impl<'a> DBIter<'a> {
    fn new(inner: &'a DB) -> DBIter {
        DBIter {
            inner,
            pos: 0,
            len: inner.database_names.len(),
        }
    }
}

/// Implementing the iterator type trait for DBIter so that the stuct
/// can be used in places where iteration happens
impl<'a> Iterator for DBIter<'a> {
    type Item = (&'a str, &'a Box<dyn db_impl::DBImpl<table::Table>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.len {
            None
        } else {
            let name = &self.inner.database_names[self.pos];
            let db = &self.inner.database_map[name];
            self.pos += 1;
            Some((name, db))
        }
    }
}

/// Enum to describe what types of `DBFile`s should be loaded at DB creation time.
#[derive(Clone)]
pub enum DBLoadControl {
    Versions,
    Tags,
    All,
}

/// Database object that library consumers interact though. This DB encodes all the
/// relations between products, versions, tags, and tables that are encoded in the
/// filesystem based database.
pub struct DB {
    database_map: FnvHashMap<String, Box<dyn db_impl::DBImpl<table::Table>>>,
    database_names: Vec<String>,
    cache: RefCell<FnvHashMap<(String, String), table::Table>>,
}

/// Describes how the db will be shown when written in a formatted
impl fmt::Debug for DB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (name, db) in self.iter() {
            write!(f, "Database named {} at {:?}\n", name, db.get_location())?;
        }
        writeln!(f)
    }
}

impl DB {
    /// Creates a new DB object. Optionally takes the path to a system database, a user database,
    /// and where the products themselves are located. Another optional argument is a
    /// DBLoadControl, which specifies which products are to be preloaded from disk at database
    /// creation time. Set the option to None if no products are to be loaded
    /// and
    pub fn new(
        db_path: Option<&str>,
        user_tag_root: Option<&str>,
        preload: Option<DBLoadControl>,
    ) -> DB {
        // Create a db hashmap
        let mut database_map =
            FnvHashMap::<String, Box<dyn db_impl::DBImpl<table::Table>>>::default();
        // Create a list of db names to maintain insertion order
        let mut database_names = vec![];

        // Check to see if a path was passed into the db builder, else get the
        // eups system variable
        let eups_paths = match db_path {
            Some(path) => {
                let paths = cogs::path_string_to_vec(path);
                match paths {
                    Some(split_paths) => split_paths,
                    None => {
                        exit_with_message!(format!("Cannot parse {} into system paths", path));
                    }
                }
            }
            None => cogs::get_eups_path_from_env(),
        };
        for path in eups_paths.iter() {
            crate::debug!(
                "Adding {} to databases",
                path.to_str().expect("Malformed database string")
            );
            let temp_db = db_impl::PosixDBImpl::new(path.clone(), preload.as_ref(), None);
            // expect should be safe here, as we pushed a directory on previously
            // Format the database map name in a deterministic way with the last bit of the path
            let db_name = format!(
                "posix_system_{}",
                path.parent()
                    .expect("Problem with database path after stripping off upd_db")
                    .file_name()
                    .expect("There was a problem getting the final directory in database path")
                    .to_str()
                    .expect("Problem turning directory osString to str")
            );
            database_map.insert(db_name.clone(), Box::new(temp_db));
            database_names.push(db_name);
        }

        // Check if a user directory was supplied, if so implement a db, if not try to get a default, else record None
        let user_db_path = match user_tag_root {
            Some(user_path) => {
                let user_pathbuf = PathBuf::from(user_path);
                if !user_pathbuf.is_dir() {
                    exit_with_message!(format!(
                        "The supplied user database {} is not a vailid path",
                        user_pathbuf
                            .to_str()
                            .expect("The user_path string is invalid")
                    ));
                }
                Some(user_pathbuf)
            }
            None => cogs::get_user_path_from_home(),
        };

        if user_db_path.is_some() {
            crate::debug!(
                "Adding {} to databases",
                user_db_path
                    .clone()
                    .unwrap()
                    .to_str()
                    .expect("Malformed database string")
            );
            let user_db = db_impl::PosixDBImpl::new(user_db_path.unwrap(), preload.as_ref(), None);
            let database_name = String::from("posix_user");
            database_map.insert(database_name.clone(), Box::new(user_db));
            database_names.push(database_name);
        }

        let cache = RefCell::new(FnvHashMap::default());

        // Construct and return the database struct
        DB {
            database_map,
            database_names,
            cache,
        }
    }

    /// Returns a vector containing the names of all the products that are known to the database.
    pub fn get_all_products(&self) -> Vec<&str> {
        // iterate over all dbs, getting a vector of keys of products, and append them to one
        // overall vector
        let return_vec: Vec<&str> = vec![];
        self.iter().fold(
            return_vec,
            |mut acc: Vec<&str>, (_, db): (&str, &Box<dyn db_impl::DBImpl<table::Table>>)| {
                acc.extend(db.get_products());
                acc
            },
        )
    }

    /// Returns the paths to the system and (optionally if one exists) user databases
    pub fn get_db_sources(&self) -> Vec<(String, PathBuf)> {
        let mut paths = Vec::new();
        for (name, db) in self.iter() {
            paths.push((String::from(name), db.get_location().clone()));
        }
        paths
    }

    /// Produces a vector of all the versions of the specified product
    pub fn product_versions(&self, product: &str) -> Vec<&str> {
        let mut product_versions = vec![];
        for (_, db) in self.iter() {
            let ver_vec = db.get_versions(product);
            if ver_vec.is_some() {
                product_versions.extend(ver_vec.unwrap());
            }
        }
        product_versions
    }

    /// Outputs a vector of all tags corresponding to the specified product
    pub fn product_tags(&self, product: &str) -> Vec<&str> {
        let mut product_tags = vec![];
        for (_, db) in self.iter() {
            let tags_vec = db.get_tags(product);
            if tags_vec.is_some() {
                product_tags.extend(tags_vec.unwrap());
            }
        }
        product_tags
    }

    /// Looks up the table corresponding to the product, version combination specified.
    pub fn get_table_from_version(&self, product: &str, version: &str) -> Option<table::Table> {
        crate::debug!("Getting table from version {}", version);
        // try getting from the db cache
        // block this so that the reference to cache goes out of scope once we are done
        {
            let cache_borrow = self.cache.borrow();
            let table_option = cache_borrow.get(&(product.to_string(), version.to_string()));
            if let Some(table_from_cache) = table_option {
                crate::debug!("Found table in cache returning");
                return Some(table_from_cache.clone());
            }
        }

        let mut tables_vec: Vec<(Option<table::Table>, &str)> = vec![];

        for (name, db) in self.iter() {
            if let Some(product_table) = db.get_table(product, version) {
                tables_vec.push((Some(product_table), name));
            }
        }
        crate::debug!("Found {} tables", tables_vec.len());

        match tables_vec.len() {
            x if x == 0 => None,
            x if x == 1 => tables_vec.into_iter().last().unwrap().0,
            _ => {
                let mut acc_map = FnvHashMap::<String, usize>::default();
                self.database_names
                    .iter()
                    .enumerate()
                    .for_each(|(i, name)| {
                        acc_map.insert(name.clone(), i);
                    });
                tables_vec.sort_by_key(|a| acc_map[a.1]);
                let (table, db_name) = tables_vec
                    .into_iter()
                    .next()
                    .expect("Failure with extracting table file in tables_vec");
                crate::warn!(
                    "Multiple table files for version {} found, using the product from {}",
                    version,
                    db_name
                );
                self.cache.borrow_mut().insert(
                    (product.to_string(), version.to_string()),
                    table.as_ref().unwrap().clone(),
                );
                table
            }
        }
    }

    /// Lists the flavors of a product corresponding to a specified product and version
    pub fn get_flavors_from_version(&self, product: &str, version: &str) -> Vec<&str> {
        let mut flavors = Vec::new();
        for (_, db) in self.iter() {
            if let Some(flavor) = db.lookup_flavor_version(product, version) {
                flavors.push(flavor);
            }
        }
        flavors
    }

    /// Looks up all the versions which correspond to specified prodcut and tag
    pub fn get_versions_from_tag(&self, product: &str, tags: &Vec<&str>) -> Vec<&str> {
        crate::debug!("Looking up all versions for tagged product");
        // store versions found in the main db and the user db
        let mut versions_vec: Vec<&str> = vec![];
        // look up the products
        for t in tags {
            for (name, db) in self.iter() {
                crate::debug!(
                    "Looking up versions for product: {}, tag: {} in db {}",
                    product,
                    t,
                    name
                );
                if let Some(version) = db.lookup_version_tag(product, t) {
                    crate::debug!("Found version {}", version);
                    versions_vec.push(version);
                }
            }
        }
        versions_vec
    }

    pub fn get_database_path_from_version(&self, product: &str, version: &str) -> PathBuf {
        let mut db_path_vec = vec![];
        for (name, db) in self.iter() {
            if let Some(dir) = db.lookup_location_version(product, version) {
                db_path_vec.push((dir, name));
            }
        }
        if db_path_vec.len() == 0 {
            return PathBuf::from("");
        }

        let entry = if db_path_vec.len() > 1 {
            let mut name_map = FnvHashMap::<String, usize>::default();
            self.database_names
                .iter()
                .enumerate()
                .for_each(|(i, name)| {
                    name_map.insert(name.clone(), i);
                });
            db_path_vec.sort_by_key(|a| name_map[a.1]);
            let entry = db_path_vec.first();
            crate::warn!(
                "Multiple entries for the same version {} found, using the product from {}",
                version,
                entry
                    .expect("Failure with extracting db_path in db_path_vec")
                    .1
            );
            entry
        } else {
            db_path_vec.first()
        };
        entry
            .expect("Failure with extracting db_path in db_path_vec")
            .0
            .clone()
    }

    /// Looks up a table file given a product and tag
    pub fn get_table_from_tag(&self, product: &str, tag: &Vec<&str>) -> Option<table::Table> {
        crate::debug!("Looking up table from tag");
        let versions_vec = self.get_versions_from_tag(product, tag);
        crate::debug!("Found versions {:?}", versions_vec);
        // use the last element, as this will select the user tag if one is present else
        // it will return the result from the main tag
        // it is safe to unwrap here, as there must be at least one db to construct the
        // object. The real Option to worry about is the one that is contained in the vec

        match versions_vec.len() {
            x if x > 0 => {
                let mut res: Option<table::Table> = None;
                for ver in versions_vec.iter().rev() {
                    res = self.get_table_from_version(product, ver);
                    // if we found the product in a given database, then bail out, no need
                    // to search further
                    if res.is_some() {
                        break;
                    }
                }
                res
            }
            _ => None,
        }
    }

    /// Creates an iterator over the database object. This will loop over the system
    /// and user databases
    fn iter<'a>(&'a self) -> DBIter<'a> {
        DBIter::new(&self)
    }

    /// Look up if a given product exists in the database
    pub fn has_product(&self, product: &String) -> bool {
        // iterate over the global and user db
        for (_, db) in self.iter() {
            if db.has_product(product) {
                return true;
            }
        }
        return false;
    }
}
