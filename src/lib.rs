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
use log::{debug, error, info, warn};
#[doc(hidden)]
use regex;
#[macro_use]
mod cogs;
mod argparse;
mod completions;
#[macro_use]
mod db;
mod declare;
mod env;
mod list;
mod logger;
mod prep;
mod setup;
pub use crate::argparse::*;
pub use crate::cogs::*;
pub use crate::completions::*;
pub use crate::db::*;
pub use crate::declare::*;
pub use crate::env::*;
pub use crate::list::*;
pub use crate::logger::*;
pub use crate::prep::*;
pub use crate::setup::*;
