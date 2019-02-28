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

static TABLE_STR: &str = "FILE = version
PRODUCT = {product} 
CHAIN = {tag}
#***************************************

#Group:
   FLAVOR = {flavor}
   VERSION = {version}
   QUALIFIERS = \"\"
   DECLARER = {user}
   DECLARED = {date}
#End:
";

static VERSION_STR: &str = "FILE = version
PRODUCT = {product}
VERSION = {version}
#***************************************

Group:
   FLAVOR = {flavor}
   QUALIFIERS = \"\"
   DECLARER = {user}
   DECLARED = {date}
   PROD_DIR = {prod_dir}
   UPS_DIR = {ups_dir}
   TABLE_FILE = {table_file}
End:
";

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
    ) -> Result<PosixDBImpl, String> {
        let (directory, product_to_info, tags_to_info, product_to_tags) = build_db(path, preload)?;
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
        Ok(PosixDBImpl {
            directory,
            tag_to_product_info: tags_to_info,
            product_to_version_info: product_to_info,
            product_to_tags,
            product_to_ident,
            product_ident_version,
        })
    }

    fn format_template_file(
        &self,
        input: &str,
        fields: Vec<&str>,
        map: &FnvHashMap<&str, &str>,
    ) -> String {
        let mut formatted_string = String::from(input);
        for field in fields.iter() {
            let value = match map.get(field) {
                Some(value) => value,
                None => "",
            };
            let pattern = format!("{{{}}}", field);
            let start_range = formatted_string
                .find(&pattern)
                .expect("Problem matching field in template formatting");
            let end_range = start_range + pattern.len();
            formatted_string.replace_range(start_range..end_range, value);
        }
        formatted_string
    }

    fn format_version_file(&self, map: &FnvHashMap<&str, &str>) -> String {
        let fields: Vec<&str> = vec![
            "product",
            "version",
            "flavor",
            "user",
            "date",
            "prod_dir",
            "ups_dir",
            "table_file",
        ];
        self.format_template_file(VERSION_STR, fields, map)
    }

    fn format_version_dbfile(&self, dbfile: &DBFile) -> String {
        crate::info!("Formatting dbfile into version string");
        let mut translate = FnvHashMap::default();
        translate.insert("product", "PRODUCT");
        translate.insert("version", "VERSION");
        translate.insert("flavor", "FLAVOR");
        translate.insert("user", "DECLARER");
        translate.insert("date", "DECLARED");
        translate.insert("prod_dir", "PROD_DIR");
        translate.insert("ups_dir", "UPS_DIR");
        translate.insert("table_file", "TABLE_FILE");
        let mut new_map = FnvHashMap::<&str, &str>::default();
        for (k, v) in translate.iter() {
            crate::debug!("inserting key value: {}, {}", k, v);
            new_map.insert(k, dbfile.entry(v).unwrap());
        }
        self.format_version_file(&new_map)
    }

    fn format_tag_file(&self, map: &FnvHashMap<&str, &str>) -> String {
        let fields: Vec<&str> = vec!["product", "tag", "flavor", "version", "user", "date"];
        self.format_template_file(TABLE_STR, fields, map)
    }

    fn format_tag_dbfile(&self, dbfile: &DBFile) -> String {
        crate::info!("Formatting dbfile into tag string");
        let mut translate = FnvHashMap::default();
        translate.insert("product", "PRODUCT");
        translate.insert("tag", "CHAIN");
        translate.insert("flavor", "FLAVOR");
        translate.insert("version", "VERSION");
        translate.insert("user", "DECLARER");
        translate.insert("date", "DECLARED");
        let mut new_map = FnvHashMap::<&str, &str>::default();
        for (k, v) in translate.iter() {
            new_map.insert(k, dbfile.entry(v).unwrap());
        }
        self.format_tag_file(&new_map)
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
        crate::debug!(
            "Making table for product {}, on path {}, with name {}",
            product,
            complete_only_path.to_str().unwrap(),
            complete.to_str().unwrap()
        );
        let table = Table::from_file(product.to_owned(), complete, complete_only_path).ok();
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

    fn declare_in_memory_impl(&mut self, inputs: &Vec<super::DeclareInputs>) -> Result<(), String> {
        let mut base_dir = self.directory.clone();
        let check_version_name = |input: &super::DeclareInputs| {
            let version = if let Some(id) = input.ident {
                format!("{}-{}", input.version, id)
            } else {
                input.version.to_string()
            };
            version
        };

        // verify that all inputs to be declared are not in the db already
        for input in inputs.iter() {
            let version = check_version_name(input);
            // check that none of the supplied info is in the database, this must be done
            // not quite elegantly at the same time as insertion because we don't want to do
            // any insertions unless the database does not contain any of the info

            // check that the version is not already in the database
            if self.product_to_version_info.contains_key(input.product)
                && self.product_to_version_info[input.product].contains_key(&version)
            {
                return Err(format!(
                    "Database already contains product {} with version {}",
                    input.product, version
                ));
            }

            // check if tag is in place
            // This check assumes that tag keys were added to all data members
            // appropriately
            if let Some(tg) = input.tag {
                if self.tag_to_product_info.contains_key(tg)
                    && self.tag_to_product_info[tg].contains_key(input.product)
                {
                    return Err(format!(
                        "Database already contains tag {} for product {} version {}",
                        tg, input.product, &version
                    ));
                }
            }

            // This check assumes that ident keys were added to all data members
            // appropriately
            if let Some(id) = input.ident {
                if let Some(prod_map) = self.product_ident_version.as_ref() {
                    if prod_map.contains_key(input.product)
                        && prod_map[input.product].contains_key(id)
                    {
                        return Err(format!(
                            "Database already contains id {} for product {} version {}",
                            id, input.product, &version
                        ));
                    }
                }
            }
        }
        // If the function has gotten this far, no products exist and all should be added
        for input in inputs.iter() {
            base_dir.push(input.product);

            let (user, date) = super::get_declare_info();
            let flav = if let Some(flav) = input.flavor {
                flav
            } else {
                ""
            };
            let ups_dir = "ups";
            let mut table_file = input.prod_dir.clone();
            table_file.push(ups_dir);
            table_file.push(format!("{}{}", input.product, ".table"));
            let mut version_map = FnvHashMap::default();
            let version = if let Some(id) = input.ident {
                format!("{}-{}", input.version, id)
            } else {
                input.version.to_string()
            };
            version_map.insert("product", input.product);
            version_map.insert("version", version.as_str());
            version_map.insert("flavor", flav);
            version_map.insert("user", user.as_str());
            version_map.insert("date", date.as_str());
            let abs_prod_dir = input
                .prod_dir
                .canonicalize()
                .expect("problem building absolute path for declared product");
            version_map.insert(
                "prod_dir",
                abs_prod_dir
                    .to_str()
                    .expect("Problem declaring with prod_dir"),
            );
            version_map.insert("ups_dir", ups_dir);
            version_map.insert(
                "table_file",
                table_file
                    .to_str()
                    .expect("Problem declaring with table file path"),
            );
            // Construct the version file string
            let version_contents = self.format_version_file(&version_map);
            let mut version_dir = base_dir.clone();
            version_dir.push(format!("{}.version", version));

            self.product_to_version_info
                .entry(input.product.to_string())
                .or_insert(FnvHashMap::default())
                .entry(version.clone())
                .or_insert(DBFile::new_with_contents(version_dir, version_contents));

            if let Some(tg) = input.tag {
                version_map.insert("table", tg);
                let tag_contents = self.format_tag_file(&version_map);
                let mut tag_dir = base_dir.clone();
                tag_dir.push(format!("{}.chain", tg));

                // insert the info about the product tags into the database
                self.tag_to_product_info
                    .entry(tg.to_string())
                    .or_insert(FnvHashMap::default())
                    .entry(input.product.to_string())
                    .or_insert(DBFile::new_with_contents(tag_dir, tag_contents));

                if self
                    .product_to_tags
                    .entry(input.product.to_string())
                    .or_insert(vec![])
                    .iter()
                    .position(|x| x == tg)
                    .is_none()
                {
                    self.product_to_tags
                        .get_mut(input.product)
                        .unwrap()
                        .push(tg.to_string());
                }
            }

            if let Some(id) = input.ident {
                if let Some(prod_map) = self.product_to_ident.as_mut() {
                    if prod_map
                        .entry(input.product.to_string())
                        .or_insert(vec![])
                        .iter()
                        .position(|x| x == id)
                        .is_none()
                    {
                        prod_map
                            .get_mut(input.product)
                            .unwrap()
                            .push(id.to_string());
                    }
                }
                if let Some(prod_map) = self.product_ident_version.as_mut() {
                    prod_map
                        .entry(input.product.to_string())
                        .or_insert(FnvHashMap::default())
                        .entry(id.to_string())
                        .or_insert(version.to_string());
                }
            }
        }
        Ok(())
    }

    fn sync(&self, product: &str) -> std::io::Result<()> {
        crate::info!("Running sync in posix_db_impl for product {}", product);
        // Get a string representation of the file contents
        // Make sure product directory exists
        let mut product_dir = self.directory.clone();
        product_dir.push(product);
        if !product_dir.exists() {
            let _ = fs::create_dir(&product_dir)?;
        }
        // loop over all tags
        crate::debug!("loop over all input products in sync");
        if self.product_to_tags.contains_key(product) {
            crate::debug!("Syncing product {} in impl", product);
            for tag in self.product_to_tags[product].iter() {
                let tag_prod_file = self.tag_to_product_info.get(tag);
                if let Some(tag_dbfile_map) = tag_prod_file {
                    if let Some(tag_file) = tag_dbfile_map.get(product) {
                        let mut table_dir = product_dir.clone();
                        table_dir.push(format!("{}.chain", tag));
                        if table_dir.exists() {
                            continue;
                        } else {
                            let tag_contents = self.format_tag_dbfile(tag_file);
                            crate::info!("Syncing tag {} file for {} to disk", tag, product);
                            fs::write(table_dir, tag_contents)?;
                        }
                    } else {
                        exit_with_message!(format!(
                            "Problem getting tag dbfile for tag {} product {}",
                            tag, product
                        ));
                    }
                } else {
                    exit_with_message!(format!(
                        "Problem getting product tag file map for tag {}",
                        tag
                    ));
                }
            }
        }

        crate::debug!("Sync versions of product {}", product);
        // loop over all versions
        if let Some(vers_map) = self.product_to_version_info.get(product) {
            for (k, v) in vers_map {
                let mut version_dir = product_dir.clone();
                version_dir.push(format!("{}.version", k));
                if version_dir.exists() {
                    crate::debug!(
                        "Product {} with version {} already exists, skipping",
                        product,
                        k
                    );
                    continue;
                } else {
                    let version_contents = self.format_version_dbfile(v);
                    crate::debug!("Syncing version {} file for {} to disk", k, product);
                    fs::write(version_dir, version_contents)?;
                }
            }
        } else {
            exit_with_message!(format!("Problem looking up product {} to sync", product));
        }
        Ok(())
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
) -> Result<
    (
        path::PathBuf,
        FnvHashMap<String, FnvHashMap<String, DBFile>>,
        FnvHashMap<String, FnvHashMap<String, DBFile>>,
        FnvHashMap<String, Vec<String>>,
    ),
    String,
> {
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
        let directory_iterator = fs::read_dir(eups_path.clone());
        if !directory_iterator.is_ok() {
            return Err(format!(
                "Problem reading database at {}",
                eups_path.to_str().unwrap()
            )
            .to_string());
        }
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

    Ok((eups_path, product_to_info, tags_to_info, product_to_tags))
}
