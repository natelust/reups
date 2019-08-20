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
#[macro_use]
mod db_impl;
mod dbfile;
pub mod graph;
pub mod table;

use self::dbfile::DBFile;
use crate::argparse;
use crate::cogs;

use self::db_impl::DBImplDeclare;
pub use self::db_impl::DeclareInputs;
pub use self::db_impl::*;
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
    type Item = (&'a str, &'a Box<dyn db_impl::DBImpl>);

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

/// Creates a new DB object. Optionally takes the path to a system database, a user database,
/// and where the products themselves are located. Another optional argument is a
/// DBLoadControl, which specifies which products are to be preloaded from disk at database
/// creation time. Set the option to None if no products are to be loaded
pub struct DBBuilder {
    eups_env: bool,
    eups_user: bool,
    reups_env: bool,
    reups_user: bool,
    db_sources: FnvHashMap<String, PathBuf>,
    extra_id: u32,
    load_control: Option<DBLoadControl>,
    allow_empty: bool,
}

type BuildBundle = Result<DBBuilder, String>;

impl DBBuilder {
    pub fn new() -> BuildBundle {
        Ok(DBBuilder {
            eups_env: true,
            eups_user: true,
            reups_env: true,
            reups_user: true,
            db_sources: FnvHashMap::default(),
            extra_id: 0,
            load_control: Some(DBLoadControl::All),
            allow_empty: true,
        })
    }

    pub fn from_args(args: &argparse::ArgMatches) -> BuildBundle {
        let mut db = DBBuilder::new();
        if args.is_present("nouser") {
            db = db.add_eups_user(false);
            db = db.add_reups_user(false);
        }
        if args.is_present("nosys") {
            db = db.add_eups_env(false);
            db = db.add_reups_env(false);
        }
        if args.is_present("database") {
            db = db.add_path_str(args.value_of("database").unwrap());
        }
        db
    }
}

pub trait DBBuilderTrait {
    fn add_eups_env(self, x: bool) -> BuildBundle;
    fn add_eups_user(self, x: bool) -> BuildBundle;
    fn add_reups_env(self, x: bool) -> BuildBundle;
    fn add_reups_user(self, x: bool) -> BuildBundle;
    fn add_path_str(self, path_str: &str) -> BuildBundle;
    fn add_path_vec(self, path_vec: Vec<PathBuf>) -> BuildBundle;
    fn add_path(self, pth: PathBuf) -> BuildBundle;
    fn set_load_control(self, mode: DBLoadControl) -> BuildBundle;
    fn allow_empty(self, x: bool) -> BuildBundle;
    fn build(self) -> Result<DB, String>;
}

impl DBBuilderTrait for BuildBundle {
    fn add_eups_env(self, x: bool) -> BuildBundle {
        let mut me = self?;
        me.eups_env = x;
        Ok(me)
    }

    fn add_eups_user(self, x: bool) -> BuildBundle {
        let mut me = self?;
        me.eups_user = x;
        Ok(me)
    }

    fn add_reups_env(self, x: bool) -> BuildBundle {
        let mut me = self?;
        me.reups_env = x;
        Ok(me)
    }

    fn add_reups_user(self, x: bool) -> BuildBundle {
        let mut me = self?;
        me.reups_user = x;
        Ok(me)
    }

    fn add_path_str(self, path_str: &str) -> BuildBundle {
        match cogs::path_string_to_vec(path_str) {
            Ok(path_vec) => self.add_path_vec(path_vec),
            Err(msg) => Err(msg),
        }
    }

    fn add_path_vec(self, path_vec: Vec<PathBuf>) -> BuildBundle {
        let mut me = self?;
        for pth in path_vec {
            me = Ok(me).add_path(pth)?;
        }
        Ok(me)
    }

    fn add_path(self, pth: PathBuf) -> BuildBundle {
        let mut me = self?;
        me.db_sources.insert(format!("Extra_{}", me.extra_id), pth);
        me.extra_id += 1;
        Ok(me)
    }

    fn set_load_control(self, mode: DBLoadControl) -> BuildBundle {
        let mut me = self?;
        me.load_control = Some(mode);
        Ok(me)
    }

    fn allow_empty(self, x: bool) -> BuildBundle {
        let mut me = self?;
        me.allow_empty = x;
        Ok(me)
    }

    fn build(self) -> Result<DB, String> {
        let mut db_dict = FnvHashMap::<String, Box<db_impl::DBImpl>>::default();
        let me = self?;
        if me.eups_env {
            let eups_env_path_result = cogs::get_eups_path_from_env();
            let eups_env_path = match eups_env_path_result {
                Err(e) => {
                    if me.allow_empty {
                        vec![]
                    } else {
                        return Err(e);
                    }
                }
                Ok(x) => x,
            };
            for pth in eups_env_path.iter() {
                crate::debug!(
                    "Adding {} to databases",
                    pth.to_str().expect("Malformed database string")
                );
                let temp_db =
                    match db_impl::PosixDBImpl::new(pth.clone(), me.load_control.as_ref(), None) {
                        Ok(x) => x,
                        Err(msg) => return Err(msg),
                    };
                // expect should be safe here, as we pushed a directory on previously
                // Format the database map name in a deterministic way with the last bit of the path
                let db_name = format!(
                    "posix_system_{}",
                    pth.parent()
                        .expect("Problem with database path after stripping off upd_db")
                        .file_name()
                        .expect("There was a problem getting the final directory in database path")
                        .to_str()
                        .expect("Problem turning directory osString to str")
                );
                db_dict.insert(db_name.clone(), Box::new(temp_db));
            }
        };
        // Handle the user paths
        if me.eups_user {
            let eups_user_path = cogs::get_eups_user_db();
            if eups_user_path.is_some() {
                let pth = eups_user_path.unwrap();
                crate::debug!(
                    "Adding {} to databases",
                    pth.clone().to_str().expect("Malformed database string")
                );
                let user_db = match db_impl::PosixDBImpl::new(pth, me.load_control.as_ref(), None) {
                    Ok(x) => x,
                    Err(msg) => return Err(msg),
                };
                let database_name = String::from("posix_user");
                db_dict.insert(database_name.clone(), Box::new(user_db));
            }
        };
        if me.reups_env {
            let reups_env_path_result = cogs::get_reups_path_from_env();
            let reups_env_path = match reups_env_path_result {
                Err(e) => {
                    if me.allow_empty {
                        vec![]
                    } else {
                        return Err(e);
                    }
                }
                Ok(x) => x,
            };
            for pth in reups_env_path.iter() {
                crate::debug!(
                    "Adding {} to databases",
                    pth.to_str().expect("Malformed database string")
                );
                let temp_db = match db_impl::JsonDBImpl::new(&pth) {
                    Ok(x) => x,
                    Err(msg) => return Err(msg),
                };
                // expect should be safe here, as we pushed a directory on previously
                // Format the database map name in a deterministic way with the last bit of the path
                let db_name = format!(
                    "json_system_{}",
                    pth.parent()
                        .expect("Problem with database path after stripping file")
                        .file_name()
                        .expect("There was a problem getting the final directory in database path")
                        .to_str()
                        .expect("Problem turning directory osString to str")
                );
                db_dict.insert(db_name.clone(), Box::new(temp_db));
            }
        }
        if me.reups_user {
            let reups_user_path = cogs::get_reups_user_db();
            if reups_user_path.is_some() {
                let pth = reups_user_path.unwrap();
                crate::debug!(
                    "Adding {} to databases",
                    pth.clone().to_str().expect("Malformed database string")
                );
                let user_db = match db_impl::JsonDBImpl::from_file(&pth) {
                    Ok(x) => x,
                    Err(msg) => return Err(format!("{}", msg)),
                };
                let database_name = String::from("json_user");
                db_dict.insert(database_name, Box::new(user_db));
            }
        }
        // Handle any other paths that were added
        for (name, pth) in me.db_sources.iter() {
            let extension = pth.extension();
            let extra_db: Box<db_impl::DBImpl> =
                if extension.is_some() && extension.unwrap() == "json" {
                    if !pth.exists() {
                        println!("in not exists");
                        crate::warn!(
                        "The backend {} does not exist on disk, creating empty source in memory",
                        pth.to_str().unwrap()
                    );
                        match db_impl::JsonDBImpl::new(pth) {
                            Ok(x) => Box::new(x),
                            Err(_) => return Err("Problem creating new json source\n".to_string()),
                        }
                    } else {
                        match db_impl::JsonDBImpl::from_file(pth) {
                            Ok(x) => Box::new(x),
                            Err(e) => return Err(format!("{}\n", e.to_string())),
                        }
                    }
                } else {
                    match db_impl::PosixDBImpl::new(pth.clone(), me.load_control.as_ref(), None) {
                        Ok(x) => Box::new(x),
                        Err(msg) => return Err(msg),
                    }
                };
            db_dict.insert(name.clone(), extra_db);
        }
        let db_names: Vec<String> = db_dict.keys().map(|x| x.clone()).collect();
        Ok(DB {
            database_map: db_dict,
            database_names: db_names,
            cache: RefCell::new(FnvHashMap::default()),
        })
    }
}

/// Database object that library consumers interact though. This DB encodes all the
/// relations between products, versions, tags, and tables that are encoded in the
/// filesystem based database.
pub struct DB {
    database_map: FnvHashMap<String, Box<dyn db_impl::DBImpl>>,
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
    /// Returns a vector containing the names of all the products that are known to the database.
    pub fn get_all_products(&self) -> Vec<&str> {
        // iterate over all dbs, getting a vector of keys of products, and append them to one
        // overall vector
        let return_vec: Vec<&str> = vec![];
        self.iter().fold(
            return_vec,
            |mut acc: Vec<&str>, (_, db): (&str, &Box<dyn db_impl::DBImpl>)| {
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

    pub fn get_table_from_identity(&self, product: &str, id: &str) -> Option<table::Table> {
        for (_, db) in self.iter() {
            if db.has_identity(product, id) {
                let version = db.lookup_version_ident(product, id)?;
                return self.get_table_from_version(product, version);
            }
        }
        None
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
    pub fn has_product(&self, product: &str) -> bool {
        // iterate over the global and user db
        for (_, db) in self.iter() {
            if db.has_product(product) {
                return true;
            }
        }
        return false;
    }

    /// Look up if a given product exists in the database
    pub fn has_identity(&self, product: &str, id: &str) -> bool {
        // iterate over the global and user db
        for (_, db) in self.iter() {
            if db.has_identity(product, id) {
                return true;
            }
        }
        return false;
    }

    /// Declares a new product to the database
    pub fn declare(
        &mut self,
        inputs: Vec<db_impl::DeclareInputs>,
        source: Option<&str>,
    ) -> DeclareResults {
        let source_name = if let Some(src) = source {
            if !self.database_map.contains_key(src) {
                return DeclareResults::NoSource;
            }
            if !self
                .database_map
                .get(src)
                .unwrap()
                .get_location()
                .metadata()
                .expect(&format!("Problem with metadata on source {} path", src))
                .permissions()
                .readonly()
            {
                src.to_string()
            } else {
                return DeclareResults::NoneWritable;
            }
        } else {
            let mut write_set: Vec<String> = vec![];
            for (name, db) in self.iter() {
                if db.is_writable() {
                    write_set.push(name.to_string());
                }
            }
            crate::debug!("found {} writable db sources", write_set.len());
            match write_set.len() {
                0 => return DeclareResults::NoneWritable,
                1 => write_set.remove(0),
                _ => return DeclareResults::MultipleWriteable,
            }
        };

        let active_db = self.database_map.remove(&source_name).unwrap();
        crate::debug!("Adding input into database source {}", source_name);
        let new_result = active_db.declare(&inputs);
        match new_result {
            Err((new, msg)) => {
                self.database_map.insert(source_name.clone(), new);
                return DeclareResults::Error(source_name, msg);
            }
            Ok(new) => {
                self.database_map.insert(source_name.clone(), new);
                return DeclareResults::Success(source_name);
            }
        }
    }
}

#[derive(Debug)]
pub enum DeclareResults {
    MultipleWriteable,
    NoneWritable,
    Success(String),
    Error(String, String),
    NoSource,
}
