/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2019*/

use super::DBFile;
use super::DBLoadControl;
use super::FnvHashMap;
use super::PathBuf;
use super::Table;
use crate::regex;
use std::fs;
use std::path;
use std::sync::mpsc;
use std::thread;

pub struct PosixDBImpl {
    directory: super::PathBuf,
    tag_to_product_info: FnvHashMap<String, FnvHashMap<String, DBFile>>,
    product_to_version_info: FnvHashMap<String, FnvHashMap<String, DBFile>>,
    product_to_tags: FnvHashMap<String, Vec<String>>,
    product_to_ident: Option<FnvHashMap<String, Vec<String>>>,
    product_ident_version: Option<FnvHashMap<String, FnvHashMap<String, String>>>,
}

impl PosixDBImpl {
    pub fn new(
        path: PathBuf,
        preload: Option<&DBLoadControl>,
        ident_regex: Option<regex::Regex>,
    ) -> PosixDBImpl {
        let (directory, product_to_info, tags_to_info, product_to_tags) = build_db(path, preload);
        let (product_to_ident, product_ident_version) = if ident_regex.is_some() {
            let mut product_to_ident = FnvHashMap::<String, Vec<String>>::default();
            let mut product_ident_version =
                FnvHashMap::<String, FnvHashMap<String, String>>::default();
            product_to_info.iter().for_each(|(product, version_vec)| {
                let mut idents = vec![];
                let mut ident_versions = FnvHashMap::<String, String>::default();
                version_vec.keys().for_each(|version| {
                    let found = ident_regex.as_ref().unwrap().find(version);
                    if found.is_some() {
                        let ident =
                            version[found.unwrap().start()..(found.unwrap().end() + 1)].to_string();
                        idents.push(ident.clone());
                        ident_versions.insert(ident, version.clone());
                    }
                });
                product_to_ident.insert(product.clone(), idents);
                product_ident_version.insert(product.clone(), ident_versions);
            });
            (Some(product_to_ident), Some(product_ident_version))
        } else {
            (None, None)
        };
        PosixDBImpl {
            directory,
            tag_to_product_info: tags_to_info,
            product_to_version_info: product_to_info,
            product_to_tags,
            product_to_ident,
            product_ident_version,
        }
    }
}

impl super::DBImpl<Table> for PosixDBImpl {
    fn get_location(&self) -> &super::PathBuf {
        &self.directory
    }

    fn get_products(&self) -> Vec<&str> {
        self.product_to_tags.keys().map(|a| a.as_str()).collect()
    }

    fn get_table(&self, product: &str, version: &str) -> Option<Table> {
        let db_file = self.product_to_version_info.get(product)?.get(version)?;
        let prod_dir = db_file.entry(&"PROD_DIR")?;
        let mut ups_dir = db_file.entry(&"UPS_DIR")?;
        let prod_dir_path = super::PathBuf::from(prod_dir);
        let mut complete = if prod_dir_path.is_absolute() {
            prod_dir_path
        } else {
            let base = self.directory.parent().unwrap().clone();
            base.join(prod_dir_path)
        };

        let mut product_table_name = product.to_string();
        product_table_name.push_str(".table");

        let complete_only_path = complete.clone();
        if ups_dir == "none" {
            ups_dir = &"ups";
        }

        complete.push(ups_dir);
        complete.push(product_table_name);
        crate::debug!("Making table for product {}, on path {}, with name {}", product, complete_only_path.to_str().unwrap(), complete.to_str().unwrap());
        let table = Table::new(product.to_owned(), complete, complete_only_path).ok();
        table
    }

    fn lookup_flavor_version(&self, product: &str, version: &str) -> Option<&str> {
        self.product_to_version_info
            .get(product)?
            .get(version)?
            .entry(&"FLAVOR")
    }

    fn get_tags(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_tags
                .get(product)?
                .iter()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn get_versions(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_version_info
                .get(product)?
                .keys()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn get_identities(&self, product: &str) -> Option<Vec<&str>> {
        Some(
            self.product_to_ident
                .as_ref()?
                .get(product)?
                .iter()
                .map(|a| a.as_str())
                .collect(),
        )
    }

    fn lookup_version_tag(&self, product: &str, tag: &str) -> Option<&str> {
        Some(
            self.tag_to_product_info
                .get(tag)?
                .get(product)?
                .entry("VERSION")?,
        )
    }

    fn lookup_version_ident(&self, product: &str, ident: &str) -> Option<&str> {
        Some(
            self.product_ident_version
                .as_ref()?
                .get(product)?
                .get(ident)?
                .as_str(),
        )
    }

    fn lookup_location_version(&self, product: &str, version: &str) -> Option<&PathBuf> {
        if self
            .product_to_version_info
            .get(product)?
            .get(version)
            .is_some()
        {
            Some(&self.directory)
        } else {
            None
        }
    }

    fn has_identity(&self, product: &str, ident: &str) -> bool {
        self.product_ident_version.is_some()
            && self
                .product_ident_version
                .as_ref()
                .unwrap()
                .get(product)
                .is_some()
            && self
                .product_ident_version
                .as_ref()
                .unwrap()
                .get(product)
                .unwrap()
                .contains_key(ident)
    }

    fn has_product(&self, product: &str) -> bool {
        self.product_to_version_info.contains_key(product)
    }

    fn identities_populated(&self) -> bool {
        self.product_ident_version.is_some()
    }
}

/// This function builds all the components which go into the creation of a database.
/// The functionality was sufficiently complex that it was factored out of new for the
/// sake of readability. The function makes heavy use of system threads to create worker
/// pools to speed up the process of reading all the database information off disk, as
/// io is inherently an asynchronous process.
fn build_db(
    eups_path: PathBuf,
    load_options: Option<&DBLoadControl>,
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
