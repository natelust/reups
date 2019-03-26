/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2019*/

pub mod posix_db_impl;
pub use self::posix_db_impl::*;
use super::table::Table;
use super::DBFile;
use super::DBLoadControl;
use super::FnvHashMap;
use super::PathBuf;
#[doc(hidden)]
use time;
#[doc(hidden)]
use users;

/// Implementation of an database object. The outside visible database
/// object, is comprised of some number of instances of structs which
/// implement this trait
pub trait DBImpl<T> {
    fn get_location(&self) -> &super::PathBuf;
    fn get_table(&self, product: &str, version: &str) -> Option<T>;
    fn get_tags(&self, product: &str) -> Option<Vec<&str>>;
    fn get_versions(&self, product: &str) -> Option<Vec<&str>>;
    fn get_products(&self) -> Vec<&str>;
    fn get_identities(&self, product: &str) -> Option<Vec<&str>>;
    fn lookup_flavor_version(&self, product: &str, version: &str) -> Option<&str>;
    fn lookup_version_tag(&self, product: &str, tag: &str) -> Option<&str>;
    fn lookup_version_ident(&self, product: &str, ident: &str) -> Option<&str>;
    fn lookup_location_version(&self, product: &str, version: &str) -> Option<&PathBuf>;
    fn has_identity(&self, product: &str, ident: &str) -> bool;
    fn has_product(&self, product: &str) -> bool;
    fn identities_populated(&self) -> bool;

    fn declare_in_memory_impl(&mut self, inputs: &Vec<DeclareInputs>) -> Result<(), String>;
    fn sync(&self, product: &str) -> std::io::Result<()>;
}

pub trait DBImplDeclare: Sized {
    fn declare(self, inputs: &Vec<DeclareInputs>) -> Result<Self, (Self, String)>;
    fn declare_in_memory(self, inputs: &Vec<DeclareInputs>) -> Result<Self, (Self, String)>;
}

impl DBImplDeclare for Box<dyn DBImpl<Table>> {
    fn declare_in_memory(mut self, inputs: &Vec<DeclareInputs>) -> Result<Self, (Self, String)> {
        let result = self.declare_in_memory_impl(inputs);
        match result {
            Err(msg) => Err((self, msg)),
            Ok(_) => Ok(self),
        }
    }
    fn declare(mut self, inputs: &Vec<DeclareInputs>) -> Result<Self, (Self, String)> {
        let result = self.declare_in_memory_impl(inputs);
        if let Err(msg) = result {
            return Err((self, msg));
        }
        for input in inputs.iter() {
            crate::debug!("Syncing input product {}", input.product);
            let result = self.sync(input.product);
            if !result.is_ok() {
                exit_with_message!(format!(
                    "Problem syncing {} to disk, version or tag may not have been written",
                    input.product
                ));
            }
        }
        Ok(self)
    }
}

pub fn get_declare_info() -> (String, String) {
    // look up the user name
    let user_option = users::get_user_by_uid(users::get_current_uid());
    let user: String;
    if let Some(x) = user_option {
        user = String::from(x.name().to_string_lossy());
    } else {
        exit_with_message!("Problem looking up current user");
    };
    // look up the current datetime
    let now = time::now().ctime().to_string();
    (user, now)
}

pub struct DeclareInputs<'a> {
    pub product: &'a str,
    pub prod_dir: &'a PathBuf,
    pub version: &'a str,
    pub tag: Option<&'a str>,
    pub ident: Option<&'a str>,
    pub flavor: Option<&'a str>,
    pub table: Option<Table>, // table is not used in posix database declare
}
