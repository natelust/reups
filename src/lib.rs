/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/*!
  `reups_lib` is the main library for the reups package management system. It contains all the functionality
  used by the reups application. Any application wishing to make use of reups functionality should link to this
  library.
 */
#[doc(hidden)]
extern crate regex;
#[doc(hidden)]
extern crate fnv;
#[doc(hidden)]
#[macro_use] extern crate clap;
#[doc(hidden)]
#[macro_use] extern crate log;
#[doc(hidden)]
#[macro_use] extern crate lazy_static;
#[macro_use] mod cogs;
mod argparse;
mod db;
mod setup;
mod list;
mod logger;
mod prep;
mod completions;
pub use argparse::*;
pub use db::*;
pub use setup::*;
pub use list::*;
pub use cogs::*;
pub use logger::*;
pub use prep::*;
pub use completions::*;
