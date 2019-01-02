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
}
