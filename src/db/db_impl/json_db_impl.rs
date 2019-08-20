/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2019*/

/**
 * json_db_impl is a backend database source for the main DB class. It
 * stores all of the information about products in a single file.
 **/
use super::DBImpl;
use super::FnvHashMap;
use super::PathBuf;
use super::Table;
use fs2::FileExt;
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::prelude::*;
use std::io::{Error, ErrorKind, Write};

/// Struct representing the serialized form a JsonDBImpl will take on disk
#[derive(Serialize, Deserialize)]
struct NewSerde {
    #[serde(rename = "Versions")]
    versions: Vec<FnvHashMap<String, String>>,
    #[serde(rename = "Tables")]
    tables: Vec<TableInfoJson>,
    #[serde(rename = "Tags")]
    tags: Vec<FnvHashMap<String, String>>,
}

/// Structure to contain the dependency structure of a table
#[derive(Serialize, Deserialize, Debug)]
struct TableDepJson {
    required: FnvHashMap<String, String>,
    optional: FnvHashMap<String, String>,
}

impl TableDepJson {
    fn new() -> TableDepJson {
        TableDepJson {
            required: FnvHashMap::default(),
            optional: FnvHashMap::default(),
        }
    }
}

/// Structure to represent a table on disk
#[derive(Serialize, Deserialize, Debug)]
struct TableInfoJson {
    exact: TableDepJson,
    inexact: TableDepJson,
    env: FnvHashMap<String, (crate::db::table::EnvActionType, String)>,
}

impl TableInfoJson {
    fn new() -> TableInfoJson {
        TableInfoJson {
            exact: TableDepJson::new(),
            inexact: TableDepJson::new(),
            env: FnvHashMap::default(),
        }
    }
}

// Database backend source that stores data in a single json file
make_db_source_struct!(JsonDBImpl,
                      FnvHashMap<String, String>,
                      product_to_version_table: FnvHashMap<String, FnvHashMap<String, Table>>);

impl JsonDBImpl {
    /// Creates a new empty JsonDBImpl instance, which will be stored at the location provided
    /// if written to disk.
    pub fn new(loc: &PathBuf) -> Result<JsonDBImpl, String> {
        Ok(JsonDBImpl {
            location: loc.clone(),
            tag_to_product_info: FnvHashMap::default(),
            product_to_version_info: FnvHashMap::default(),
            product_to_tags: FnvHashMap::default(),
            product_to_ident: Some(FnvHashMap::default()),
            product_ident_version: Some(FnvHashMap::default()),
            product_to_version_table: FnvHashMap::default(),
        })
    }

    /// Creates a new JsonDBImpl from a previously serialized struct stored in the JSON file
    /// located at the path provided.
    pub fn from_file(loc: &PathBuf) -> std::io::Result<JsonDBImpl> {
        let mut json_file_raw = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(loc)?;
        json_file_raw.try_lock_shared()?;
        let mut json_file = String::new();
        let _ = json_file_raw.read_to_string(&mut json_file);

        let mut json_db: JsonDBImpl = match serde_json::from_str(&json_file) {
            Ok(x) => x,
            Err(_) => {
                let _ = json_file_raw.unlock();
                return Err(Error::new(
                    ErrorKind::Other,
                    "Problem reading json file from disk\n",
                ));
            }
        };
        json_db.location = loc.clone();

        let _ = json_file_raw.unlock();
        Ok(json_db)
    }

    pub fn update_paths(&mut self) {}
}

// Deserialize trait, used to load an object from disk
impl<'de> Deserialize<'de> for JsonDBImpl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // helper is the struct deserialized from disk by serde, it must be converted to the in
        // memory representation of the db source
        let mut helper = NewSerde::deserialize(deserializer)?;
        // create a new in memory db source, initialized to an empty location, consumers of the
        // deserialized source should set this location.
        let mut new_dbimpl = JsonDBImpl::new(&PathBuf::new()).unwrap();
        // do Versions first
        for (mut version_info, table_info) in helper.versions.drain(..).zip(helper.tables.drain(..))
        {
            // pop off the product version, and ident from the hashmap, eliminates creating copies
            let product = version_info.remove("PRODUCT").unwrap();
            let version = version_info.remove("VERSION").unwrap();
            let ident = version_info.remove("IDENT").unwrap();
            let product_dir = PathBuf::from(version_info.get("PROD_DIR").as_ref().unwrap());
            // Create a new table object to populate
            /*
            for entry in &mut table_info.env {
                let tup = entry.1;
                tup.1 = tup
                    .1
                    .replace("${PRODUCT_DIR}", product_dir.to_str().unwrap());
            }*/
            let new_table = super::Table {
                name: product.clone(),
                path: None,
                product_dir,
                exact: Some(super::table::Deps {
                    required: table_info.exact.required,
                    optional: table_info.exact.optional,
                }),
                inexact: Some(super::table::Deps {
                    required: table_info.inexact.required,
                    optional: table_info.inexact.optional,
                }),
                env_var: table_info.env,
            };
            // populate the various fields of the impl struct
            new_dbimpl
                .product_to_ident
                .as_mut()
                .unwrap()
                .entry(product.clone())
                .or_insert(vec![])
                .push(ident.clone());
            new_dbimpl
                .product_ident_version
                .as_mut()
                .unwrap()
                .entry(product.clone())
                .or_insert(FnvHashMap::default())
                .insert(ident, version.clone());
            let map = new_dbimpl
                .product_to_version_info
                .entry(product.clone())
                .or_insert(FnvHashMap::default());
            map.insert(version.clone(), version_info);
            new_dbimpl
                .product_to_version_table
                .entry(product)
                .or_insert(FnvHashMap::default())
                .insert(version, new_table);
        }
        // now take care of tags
        for mut tag_info in helper.tags.drain(..) {
            // pop the product and tag fields from the dict to save on allocations
            let product = tag_info.remove("PRODUCT").unwrap();
            let tag = tag_info.remove("TAG").unwrap();
            // populate the fields on the impl
            new_dbimpl
                .tag_to_product_info
                .entry(tag.clone())
                .or_insert(FnvHashMap::default())
                .insert(product.clone(), tag_info);
            new_dbimpl
                .product_to_tags
                .entry(product)
                .or_insert(vec![])
                .push(tag);
        }
        Ok(new_dbimpl)
    }
}

// Serialized trait, used to store an object to disk
impl Serialize for JsonDBImpl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Create the data structures which will be serialized
        let mut versions: Vec<FnvHashMap<String, String>> = vec![];
        let mut tables: Vec<TableInfoJson> = vec![];
        let mut tags: Vec<FnvHashMap<String, String>> = vec![];

        // populate the structures
        for (product, version_map) in self.product_to_version_info.iter() {
            // lets process tags first
            if self.product_to_tags.contains_key(product) {
                for tag in self.product_to_tags[product].iter() {
                    // a given tag may not be associated with the product under
                    // consideration, so verify the mapping contains this product
                    if self
                        .tag_to_product_info
                        .get(tag)
                        .unwrap()
                        .contains_key(product)
                    {
                        // Fetch the associated tag mapping
                        let mut tag_info = self
                            .tag_to_product_info
                            .get(tag)
                            .unwrap()
                            .get(product)
                            .unwrap()
                            .clone();
                        // insert product and tag info into the mapping so the info
                        // will be available to use in deserializing
                        tag_info.insert("PRODUCT".to_string(), product.clone());
                        tag_info.insert("TAG".to_string(), tag.clone());
                        tags.push(tag_info);
                    }
                }
            }
            // now for versions
            for (version, version_info) in version_map {
                // look up the identity associated with this version
                let ident_vec: Vec<(&String, &String)> =
                    self.product_ident_version.as_ref().unwrap()[product]
                        .iter()
                        .filter(|(_, y)| y.as_str() == version.as_str())
                        .collect();
                let (ident, _) = ident_vec[0];

                // Fetch the table corresponding to this product, version from the
                // in memory table and convert it a struct for serialization
                let in_memory_table = self.get_table(product, version).unwrap();
                let mut new_table = TableInfoJson::new();
                match in_memory_table.exact {
                    Some(deps) => {
                        new_table.exact = TableDepJson {
                            required: deps.required.clone(),
                            optional: deps.optional.clone(),
                        };
                    }
                    None => {
                        new_table.exact = TableDepJson::new();
                    }
                }
                match in_memory_table.inexact {
                    Some(deps) => {
                        new_table.inexact = TableDepJson {
                            required: deps.required.clone(),
                            optional: deps.optional.clone(),
                        };
                    }
                    None => {
                        new_table.inexact = TableDepJson::new();
                    }
                }
                let mut env_var_new = FnvHashMap::default();
                for (k, (t, p)) in in_memory_table.env_var {
                    let new_p = p.replace(
                        in_memory_table.product_dir.to_str().unwrap(),
                        "${PRODUCT_DIR}",
                    );
                    env_var_new.insert(k.clone(), (t.clone(), new_p));
                }
                new_table.env = env_var_new;
                tables.push(new_table);

                // Use the version info mapping and add product, version, identity
                // as entries so they can be used in the deserialization process
                let mut new_version_map = version_info.clone();
                new_version_map.insert("PRODUCT".to_string(), product.clone());
                new_version_map.insert("VERSION".to_string(), version.clone());
                new_version_map.insert("IDENT".to_string(), ident.clone());

                versions.push(new_version_map);
            }
        }
        // create the serialization struct, and serialize it
        let tmp = NewSerde {
            versions,
            tables,
            tags,
        };
        NewSerde::serialize(&tmp, serializer)
    }
}

// Implement the trait to make JsonDBImpl a database source
impl super::DBImpl for JsonDBImpl {
    // Add in pre-defined methods from the base instance
    make_db_source_default_methods!();

    fn get_table(&self, product: &str, version: &str) -> Option<Table> {
        let mut table = self
            .product_to_version_table
            .get(product)?
            .get(version)?
            .clone();
        if table.product_dir.is_relative() {
            table.product_dir = self
                .location
                .parent()
                .expect("Problem finding json db location parent")
                .join(table.product_dir)
                .canonicalize()
                .expect("Problem expanding json table location to abs path");
        }
        for (_, entry) in &mut table.env_var {
            entry.1 = entry.1.replace(
                "${PRODUCT_DIR}",
                table
                    .product_dir
                    .to_str()
                    .expect("convert table product_dir to stri"),
            );
        }
        Some(table)
    }

    fn is_writable(&self) -> bool {
        // If the location does not exist, report writable for now
        // and let other code down the line attemp to create the file
        if !self.get_location().exists() {
            return true;
        } else {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .read(true)
                .open(&self.get_location());
            match file {
                Ok(_) => true,
                Err(_) => false,
            }
        }
    }

    fn declare_in_memory_impl(&mut self, inputs: &Vec<super::DeclareInputs>) -> Result<(), String> {
        // This function takes the list of inputs to declare, insures the inputs are to already in
        // the database source and if not, adds the input information to the relevant fields of the
        // db source

        // verify that all inputs to be declared are not in the db already
        for input in inputs.iter() {
            if input.ident.is_none() {
                return Err(format!(
                    "Json database sources must be declared with an identity, none for input {}",
                    input.product
                ));
            }
            let version = input.version;
            // check that none of the supplied info is in the database, this must be done
            // not quite elegantly at the same time as insertion because we don't want to do
            // any insertions unless the database does not contain any of the info

            // check that the version is not already in the database
            if self.product_to_version_info.contains_key(input.product)
                && self.product_to_version_info[input.product].contains_key(version)
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
            let (user, date) = super::get_declare_info();
            let flav = if let Some(flav) = input.flavor {
                flav
            } else {
                ""
            };
            let ups_dir = "ups";
            let mut version_map = FnvHashMap::<String, String>::default();
            let version = input.version.to_string();
            let product = input.product.to_string();
            version_map.insert("PRODUCT".to_string(), product.clone());
            version_map.insert("VERSION".to_string(), version.clone());
            version_map.insert("FLAVOR".to_string(), flav.to_string());
            version_map.insert("DECLARER".to_string(), user.clone());
            version_map.insert("DECLARED".to_string(), date.clone());
            version_map.insert("QUALIFIERS".to_string(), "".to_string());
            let abs_prod_dir = if input.relative {
                crate::warn!("Declaring product with relative path, assumed to be relative to db source path");
                input.prod_dir.clone()
            } else {
                input
                    .prod_dir
                    .canonicalize()
                    .expect("problem building absolute path for declared product")
            };
            version_map.insert(
                "PROD_DIR".to_string(),
                abs_prod_dir
                    .to_str()
                    .expect("Problem declaring with prod_dir")
                    .to_string(),
            );
            version_map.insert("UPS_DIR".to_string(), ups_dir.to_string());

            self.product_to_version_info
                .entry(input.product.to_string())
                .or_insert(FnvHashMap::default())
                .insert(version.clone(), version_map);

            let ups_dir = "ups";
            let mut table_file = abs_prod_dir.clone();
            table_file.push(ups_dir);
            table_file.push(format!("{}{}", input.product, ".table"));

            let table_result =
                Table::from_file(input.product.to_string(), table_file, abs_prod_dir.clone());
            let table = match table_result {
                Ok(table) => table,
                Err(e) => {
                    return Err(e.to_string());
                }
            };

            self.product_to_version_table
                .entry(product)
                .or_insert(FnvHashMap::default())
                .insert(version.clone(), table);

            if let Some(tg) = input.tag {
                let mut tag_map = FnvHashMap::<String, String>::default();
                tag_map.insert("VERSION".to_string(), version.clone());
                tag_map.insert("DECLARER".to_string(), user);
                tag_map.insert("DECLARED".to_string(), date);

                // insert the info about the product tags into the database
                self.tag_to_product_info
                    .entry(tg.to_string())
                    .or_insert(FnvHashMap::default())
                    .insert(input.product.to_string(), tag_map);

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
                        .or_insert(version);
                }
            }
        }
        Ok(())
    }

    fn sync(&self, product: &str) -> std::io::Result<()> {
        // This function syncs a product to disk. It first reads in the existing on disk
        // representation of the database, in case it has changed since the in memory version was
        // created. If no on disk representation is found one is created to sync to. It then
        // compares the specified product from the in memory representation to the one loaded
        // from disk, and then adds any missing fields.

        crate::info!("Running sync in json_db_impl for product {}", product);
        // check if the source already exists
        let json_exists = self.location.exists();
        // get the File object for the on disk json file, creating it if it does not exist
        // As this is a write operation, lock the file to prevent issues
        // convert the json to in memory representation if there is a file on disk
        let (mut json_db, mut json_file) = if json_exists {
            let mut json_file = fs::OpenOptions::new().read(true).open(&self.location)?;
            let mut f = String::new();
            let _ = json_file.read_to_string(&mut f);
            let mut ydb: JsonDBImpl = match serde_json::from_str(&f) {
                Ok(x) => x,
                Err(_) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Problem reading json file from disk\n",
                    ));
                }
            };
            ydb.location = self.location.clone();
            drop(json_file);
            let json_file = fs::OpenOptions::new()
                .truncate(true)
                .write(true)
                .open(&self.location)?;
            (ydb, json_file)
        } else {
            // create a new empty json object
            let json_file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&self.location)?;
            (JsonDBImpl::new(&self.location).unwrap(), json_file)
        };
        json_file.try_lock_exclusive()?;

        // As the in memory and on disk representations might differ, only add
        // to the read in object so others work can not be forgotten

        if self.product_to_tags.contains_key(product) {
            crate::debug!("Syncing tags for product {}", product);
            for tag in &self.product_to_tags[product] {
                let product_map = &mut json_db
                    .tag_to_product_info
                    .entry(tag.clone())
                    .or_insert(FnvHashMap::default());
                if product_map.contains_key(product) {
                    // extra verification can be done here
                    continue;
                }
                product_map.insert(
                    product.to_string(),
                    self.tag_to_product_info
                        .get(tag)
                        .unwrap()
                        .get(product)
                        .unwrap()
                        .clone(),
                );
                json_db
                    .product_to_tags
                    .entry(product.to_string())
                    .or_insert(vec![])
                    .push(tag.clone());
            }
        }
        if self.product_to_version_info.contains_key(product) {
            crate::debug!("Syncing versions for product {}", product);
            let new_product_map = json_db
                .product_to_version_info
                .entry(product.to_string())
                .or_insert(FnvHashMap::default());
            let old_product_map = self.product_to_version_info.get(product).unwrap();
            let new_table_map = json_db
                .product_to_version_table
                .entry(product.to_string())
                .or_insert(FnvHashMap::default());
            let old_table_map = self.product_to_version_table.get(product).unwrap();
            for version in old_product_map.keys() {
                if new_product_map.contains_key(version) {
                    // extra verification can be done here
                    continue;
                }
                new_product_map.insert(
                    version.clone(),
                    old_product_map.get(version).unwrap().clone(),
                );
                new_table_map.insert(version.clone(), old_table_map.get(version).unwrap().clone());
            }

            crate::debug!("Syncing identities for product {}", product);
            let new_ident_map = json_db
                .product_ident_version
                .as_mut()
                .unwrap()
                .entry(product.to_string())
                .or_insert(FnvHashMap::default());
            let old_ident_map = self
                .product_ident_version
                .as_ref()
                .unwrap()
                .get(product)
                .unwrap();
            let new_ident_vec = json_db
                .product_to_ident
                .as_mut()
                .unwrap()
                .entry(product.to_string())
                .or_insert(vec![]);
            for ident in old_ident_map.keys() {
                if new_ident_map.contains_key(ident) {
                    // extra verification can be done here
                    continue;
                }
                new_ident_map.insert(ident.clone(), old_ident_map.get(ident).unwrap().clone());
                new_ident_vec.push(ident.clone());
            }
        }

        crate::debug!("Serializing out the json db");
        // serialized the json_db out to a string before writing
        let serialized_json_db = match serde_json::to_string_pretty(&json_db) {
            Ok(x) => x,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Issue serializing back to json representation\n"),
                ));
            }
        };
        let _ = json_file.write(serialized_json_db.as_bytes())?;
        json_file.unlock()?;
        crate::debug!("Done syncing out the database");
        Ok(())
    }
}
