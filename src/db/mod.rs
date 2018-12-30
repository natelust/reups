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
mod dbfile;
pub mod graph;
pub mod table;

use self::dbfile::DBFile;
use crate::cogs;

use std::cell::RefCell;
use std::fmt;
use std::fs;
use std::path;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc;
use std::thread;

/// Implementation of an database object. The outside visible database
/// object, is comprised of some number of instances of these
/// implementations
struct DBImpl {
    directory: path::PathBuf,
    tag_to_product_info: FnvHashMap<String, FnvHashMap<String, DBFile>>,
    product_to_version_info: FnvHashMap<String, FnvHashMap<String, DBFile>>,
    product_to_tags: FnvHashMap<String, Vec<String>>,
}

/// Data structure to hold state related to iterating over a db object.
/// This iteration is used to loop over all the instance of DBImpls
/// contained in the database, which at this point includes the main
/// and user dbs
struct DBIter<'a> {
    inner: &'a DB,
    pos: usize,
}

/// Implementing the iterator type trait for DBIter so that the stuct
/// can be used in places where iteration happens
impl<'a> Iterator for DBIter<'a> {
    type Item = &'a DBImpl;

    fn next(&mut self) -> Option<Self::Item> {
        // Match the position state variable to know where in the
        // iteration the iterable object is
        match self.pos {
            // This is the main system db
            0 => {
                self.pos += 1;
                Some(&self.inner.system_db)
            }
            // This corresponds to the user db
            // This object is already an option, as there may not
            // be a user db, so this match will either return some
            // with the user db inside, or None, which will terminate
            // the iterator
            1 => {
                self.pos += 1;
                // user_db is already an option
                self.inner.user_db.as_ref()
            }
            // Terminate the iterator if this branch is reached
            _ => None,
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
/// filesystem besed database.
pub struct DB {
    system_db: DBImpl,
    user_db: Option<DBImpl>,
    stack_root: path::PathBuf,
    cache: RefCell<FnvHashMap<(String, String), table::Table>>,
}

/// Describes how the db will be shown when written in a formatted
impl fmt::Debug for DB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Database at {:?}\n", self.system_db.directory)?;
        if let Some(ref user_db) = self.user_db {
            write!(f, "User database at {:?}\n", user_db.directory)?;
        }
        write!(f, "Stack at {:?}\n", self.stack_root)
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
        def_stack_root: Option<&str>,
        preload: Option<DBLoadControl>,
    ) -> DB {
        // Check to see if a path was passed into the db builder, else get the
        // eups system variable
        let eups_path = match db_path {
            Some(path) => PathBuf::from(path),
            None => cogs::get_eups_path_from_env(),
        };

        let (directory, product_to_info, tags_to_info, product_to_tags) =
            build_db(eups_path.clone(), preload.clone());
        let system_db = DBImpl {
            directory: directory,
            tag_to_product_info: tags_to_info,
            product_to_version_info: product_to_info,
            product_to_tags: product_to_tags,
        };

        // Check if a user directory was supplied, if so implement a db, if not try to get a defaul, else record None
        let user_db_path = match user_tag_root {
            Some(user_path) => Some(PathBuf::from(user_path)),
            None => cogs::get_user_path_from_home(),
        };

        let user_db = match user_db_path {
            Some(user_path) => {
                let (directory, product_to_info, tags_to_info, product_to_tags) =
                    build_db(user_path, preload);
                Some(DBImpl {
                    directory: directory,
                    tag_to_product_info: tags_to_info,
                    product_to_version_info: product_to_info,
                    product_to_tags: product_to_tags,
                })
            }
            None => None,
        };

        // Check if a stack root was provided, else construct one relative to the parent of db_path
        let stack_root = match def_stack_root {
            Some(path) => path::PathBuf::from(path),
            None => path::PathBuf::from(eups_path)
                .parent()
                .unwrap_or_else(|| {
                    println!("problem creating stack root");
                    process::exit(1);
                })
                .to_path_buf(),
        };
        let cache = RefCell::new(FnvHashMap::default());

        // Consruct and return the database struct
        DB {
            system_db,
            user_db,
            stack_root,
            cache,
        }
    }

    /// Returns a vector containing the names of all the products that are known to the database.
    pub fn get_all_products(&self) -> Vec<String> {
        // iterate over all dbs, getting a vector of keys of products, and append them to one
        // overall vector
        let return_vec: Vec<String> = vec![];
        self.iter()
            .fold(return_vec, |mut acc: Vec<String>, db: &DBImpl| {
                acc.extend(db.product_to_tags.keys().map(|x| x.clone()));
                acc
            })
    }

    /// Returns the paths to the system and (optionally if one exists) user databases
    pub fn get_db_directories(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for db in self.iter() {
            paths.push(db.directory.clone());
        }
        paths
    }

    /// Produces a vector of all the versions of the specified product
    pub fn product_versions(&self, product: &String) -> Vec<String> {
        let mut product_versions = vec![];
        for db in self.iter() {
            if db.product_to_version_info.contains_key(product) {
                product_versions.extend(
                    db.product_to_version_info[product]
                        .keys()
                        .map(|x| x.clone()),
                );
            }
        }
        product_versions
    }

    /// Outputs a vector of all tags corresponding to the specified product
    pub fn product_tags(&self, product: &String) -> Vec<String> {
        let mut tags_vec = vec![];
        for db in self.iter() {
            if db.product_to_tags.contains_key(product) {
                tags_vec.extend(db.product_to_tags[product].clone());
            }
        }
        tags_vec
    }

    /// Looks up the table corresponding to the product, version combination specified.
    pub fn get_table_from_version(
        &self,
        product: &String,
        version: &String,
    ) -> Option<table::Table> {
        // try getting from the db cache
        // block this so that the reference to cache goes out of scope once we are done
        {
            let cache_borrow = self.cache.borrow();
            let table_option = cache_borrow.get(&(product.clone(), version.clone()));
            if let Some(table_from_cache) = table_option {
                return Some(table_from_cache.clone());
            }
        }

        let mut tables_vec: Vec<Option<(path::PathBuf, path::PathBuf)>> = vec![];

        for db in self.iter() {
            if db.product_to_version_info.contains_key(product)
                && db.product_to_version_info[product].contains_key(version)
            {
                let prod_dir =
                    db.product_to_version_info[product][version].entry(&"PROD_DIR".to_string());
                let mut ups_dir =
                    db.product_to_version_info[product][version].entry(&"UPS_DIR".to_string());
                if prod_dir.is_none() || ups_dir.is_none() {
                    tables_vec.push(None);
                    continue;
                }
                let mut total = self.stack_root.clone();
                let mut product_clone = product.clone();
                product_clone.push_str(".table");
                total.push(prod_dir.unwrap());
                let total_only_prod = total.clone();
                // hacky code to support bad eups product declatations
                if ups_dir.as_ref().unwrap() == "none" {
                    ups_dir = Some(String::from("ups"));
                }
                total.push(ups_dir.unwrap());
                total.push(product_clone);
                tables_vec.push(Some((total_only_prod, total)));
            }
        }

        match tables_vec.len() {
            x if x > 0 => {
                let (prod_dir, total) = tables_vec.remove(x - 1).unwrap();
                let resolved_table = table::Table::new(product.clone(), total, prod_dir).ok();
                self.cache.borrow_mut().insert(
                    (product.clone(), version.clone()),
                    resolved_table.as_ref().unwrap().clone(),
                );
                resolved_table
            }
            _ => None,
        }
    }

    /// Lists the flavors of a product corresponding to a specified product and version
    pub fn get_flavors_from_version(&self, product: &String, version: &String) -> Vec<String> {
        let mut flavors = Vec::new();
        for db in self.iter() {
            if db.product_to_version_info.contains_key(product)
                && db.product_to_version_info[product].contains_key(version)
            {
                let flavor_option =
                    db.product_to_version_info[product][version].entry(&String::from("FLAVOR"));
                if let Some(flav) = flavor_option {
                    flavors.push(flav);
                }
            }
        }
        flavors
    }

    /// Looks up all the versions which correspond to specified prodcut and tag
    pub fn get_versions_from_tag(&self, product: &String, tag: Vec<&String>) -> Vec<String> {
        // store versions found in the main db and the user db
        let mut versions_vec: Vec<String> = vec![];
        // look up the products
        for db in self.iter() {
            for t in &tag {
                if db.tag_to_product_info.contains_key(t.clone()) {
                    let ref tag_map = db.tag_to_product_info[t.clone()];
                    if let Some(product_file) = tag_map.get(product) {
                        let version = product_file.entry(&"VERSION".to_string());
                        versions_vec.push(version.unwrap());
                        break;
                    }
                }
            }
        }
        versions_vec
    }

    /// Looks up a table file given a product and tag
    pub fn get_table_from_tag(&self, product: &String, tag: Vec<&String>) -> Option<table::Table> {
        let versions_vec = self.get_versions_from_tag(product, tag);
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
        DBIter {
            inner: self,
            pos: 0,
        }
    }

    /// Look up if a given product exists in the database
    pub fn has_product(&self, product: &String) -> bool {
        // iterate over the global and user db
        for db in self.iter() {
            if db.product_to_version_info.contains_key(product) {
                return true;
            } else if db.product_to_tags.contains_key(product) {
                return true;
            }
        }
        return false;
    }
}

/// This function builds all the components which go into the creation of a database.
/// The functionality was sufficiently complex that it was factored out of new for the
/// sake of readability. The function makes heavy use of system threads to create worker
/// pools to speed up the process of reading all the database information off disk, as
/// io is inherently an asynchronous process.
fn build_db(
    eups_path: PathBuf,
    load_options: Option<DBLoadControl>,
) -> (
    path::PathBuf,
    FnvHashMap<String, FnvHashMap<String, DBFile>>,
    FnvHashMap<String, FnvHashMap<String, DBFile>>,
    FnvHashMap<String, Vec<String>>,
) {
    // Create channels that each of the threads will communicate over
    let (name_tx, name_rx) = mpsc::channel::<(String, path::PathBuf)>();
    let (tag_tx, tag_rx) = mpsc::channel::<(String, path::PathBuf)>();
    let (worker1_tx, worker1_rx) = mpsc::channel::<path::PathBuf>();
    let (worker2_tx, worker2_rx) = mpsc::channel::<path::PathBuf>();

    // bundle the woker communication end points so that they can be looped over
    let worker_tx_vec = vec![worker1_tx, worker2_tx];
    let worker_rx_vec = vec![worker1_rx, worker2_rx];

    let (mut load_version, mut load_tag) = (false, false);
    match load_options {
        Some(DBLoadControl::Versions) => {
            load_version = true;
        }
        Some(DBLoadControl::Tags) => {
            load_tag = true;
        }
        Some(DBLoadControl::All) => {
            load_version = true;
            load_tag = true;
        }
        None => (),
    }

    let names_thread = thread::spawn(move || {
        // #product -> #version -> struct(path, info)
        let mut product_hash: FnvHashMap<String, FnvHashMap<String, DBFile>> =
            FnvHashMap::default();

        // create a pool of workers to make dbfiles
        let mut tx_vec = vec![];
        let mut threads_vec = vec![];
        for _ in 0..2 {
            let (tx, rx) = mpsc::channel::<(String, String, path::PathBuf, bool)>();
            tx_vec.push(tx);
            threads_vec.push(thread::spawn(move || {
                let mut dbfiles = vec![];
                for (version, product, path, preload) in rx {
                    dbfiles.push((version, product, DBFile::new(path, preload)));
                }
                dbfiles
            }));
        }
        // block to ensure chained iterator goes out of scope
        {
            let mut tx_vec_cycle = tx_vec.iter().cycle();
            for (product, file) in name_rx {
                let version;
                // The code below is scoped so that the borrow of file goes out scope and
                // the file can be moved into the DBFile constructor
                {
                    let version_file_name = file.file_name().unwrap().to_str().unwrap();
                    let version_str: Vec<&str> = version_file_name.split(".version").collect();
                    version = String::from(version_str[0]);
                }
                tx_vec_cycle
                    .next()
                    .unwrap()
                    .send((version, product, file, load_version))
                    .unwrap();
            }
        }
        // work is done collect from threads
        drop(tx_vec);
        for thread in threads_vec {
            let result = thread.join().unwrap();
            for (version, product, dbfile) in result {
                let version_hash = product_hash.entry(product).or_insert(FnvHashMap::default());
                version_hash.insert(version, dbfile);
            }
        }
        product_hash
    });

    let tags_thread = thread::spawn(move || {
        // #tag -> #product -> (path, info)
        let mut tags_hash: FnvHashMap<String, FnvHashMap<String, DBFile>> = FnvHashMap::default();
        let mut product_to_tags: FnvHashMap<String, Vec<String>> = FnvHashMap::default();
        //
        // create a pool of workers to make dbfiles
        let mut tx_vec = vec![];
        let mut threads_vec = vec![];
        for _ in 0..2 {
            let (tx, rx) = mpsc::channel::<(String, String, path::PathBuf, bool)>();
            tx_vec.push(tx);
            threads_vec.push(thread::spawn(move || {
                let mut dbfiles = vec![];
                for (product, tag, path, preload) in rx {
                    dbfiles.push((product, tag, DBFile::new(path, preload)));
                }
                dbfiles
            }));
        }
        {
            let mut tx_vec_cycle = tx_vec.iter().cycle();

            for (product, file) in tag_rx {
                let tag;
                // The code below is scoped so that the borrow of file goes out scope and
                // the file can be moved into the DBFile constructor
                {
                    let tag_file_name = file.file_name().unwrap().to_str().unwrap();
                    let tag_str: Vec<&str> = tag_file_name.split(".chain").collect();
                    tag = String::from(tag_str[0]);
                }
                let tags_vec = product_to_tags.entry(product.clone()).or_insert(Vec::new());
                tags_vec.push(tag.clone());
                tx_vec_cycle
                    .next()
                    .unwrap()
                    .send((product, tag, file, load_tag))
                    .unwrap();
            }
        }
        // work is done, collect from threads
        drop(tx_vec);
        for thread in threads_vec {
            let result = thread.join().unwrap();
            for (product, tag, dbfile) in result {
                let product_hash = tags_hash.entry(tag).or_insert(FnvHashMap::default());
                product_hash.insert(product, dbfile);
            }
        }
        (tags_hash, product_to_tags)
    });

    // Create a worker "pool" to list and sort directories passed to them, passing off files
    // by type to other threads which accumulate
    let mut worker_threads = vec![];
    for reciver in worker_rx_vec {
        let name_tx_clone = mpsc::Sender::clone(&name_tx);
        let tag_tx_clone = mpsc::Sender::clone(&tag_tx);

        worker_threads.push(thread::spawn(move || {
            for entry in reciver {
                if !entry.is_dir() {
                    continue;
                }
                let entry_name = String::from(entry.file_name().unwrap().to_str().unwrap());
                let contents = fs::read_dir(entry).expect("problem in worker thread read_dir");
                for file in contents {
                    let obj = file.unwrap();
                    let obj_name = obj.file_name().to_str().unwrap().to_string();
                    let message = (entry_name.clone(), obj.path().clone());
                    if obj_name.ends_with(".version") {
                        name_tx_clone.send(message).unwrap();
                    } else if obj_name.ends_with(".chain") {
                        tag_tx_clone.send(message).unwrap();
                    }
                }
            }
        }));
    }

    // run this in a scope block so the iterator gets cleaned up afterwards
    {
        // create an iterator that cycles between the worker transmitter such
        // that the work will be distributed to each worker in sequence
        let mut worker_iter = worker_tx_vec.iter().cycle();
        for entry in fs::read_dir(eups_path.clone()).expect("issue in main list") {
            worker_iter
                .next()
                .unwrap()
                .send(entry.unwrap().path())
                .unwrap();
        }
    }

    // drop the worker transmitters so that the worker threads get closed
    drop(worker_tx_vec);

    // Join the worker threads to make sure they cleanly end
    for thread in worker_threads {
        thread.join().unwrap();
    }

    // Drop the version and tag db accumulators so the threads close
    drop(name_tx);
    drop(tag_tx);

    // collect the results of the accumulators
    let product_to_info = names_thread.join().unwrap();
    let (tags_to_info, product_to_tags) = tags_thread.join().unwrap();

    (eups_path, product_to_info, tags_to_info, product_to_tags)
}
