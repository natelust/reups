/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2019*/

/**!
 * Declare is a module representing the subcommand of reups that manages adding new entries into a
 * database.
 *
 **/
use crate::argparse;
use crate::cogs;
use crate::db;
use crate::db::DBBuilderTrait;
use crate::logger;
use std::path::PathBuf;

pub fn declare_command(
    sub_args: &argparse::ArgMatches,
    _main_args: &argparse::ArgMatches,
) -> Result<(), String> {
    let mut declare_command = DeclareCommandImpl::new(sub_args, _main_args);
    declare_command.run()
}

struct DeclareCommandImpl<'a> {
    sub_args: &'a argparse::ArgMatches<'a>,
    _main_args: &'a argparse::ArgMatches<'a>,
}

impl<'a> DeclareCommandImpl<'a> {
    fn new(
        sub_args: &'a argparse::ArgMatches<'a>,
        _main_args: &'a argparse::ArgMatches<'a>,
    ) -> DeclareCommandImpl<'a> {
        logger::build_logger(sub_args, false);
        DeclareCommandImpl {
            sub_args,
            _main_args,
        }
    }

    fn run(&mut self) -> Result<(), String> {
        let prod_path_string = self.sub_args.value_of("path").unwrap();
        let prod_path = PathBuf::from(prod_path_string);
        if !prod_path.exists() {
            exit_with_message!("The supplied path to product does not exists");
        }

        // safe to unwrap, because they are required in the argument parsing
        let product = self.sub_args.value_of("product").unwrap();
        let version = self.sub_args.value_of("version").unwrap();

        let tag = self.sub_args.value_of("tag");
        let source = self.sub_args.value_of("source");

        let ident = self.sub_args.value_of("ident");
        let flavor = Some(cogs::SYSTEM_OS);
        // add the path to the table file
        let mut table_path = prod_path.clone();
        table_path.push("ups");
        if !prod_path.exists() {
            exit_with_message!(format!(
                "No ups directory found at {}",
                table_path.to_str().expect("Unwrapping table path")
            ));
        }
        table_path.push(format!("{}.table", product));
        if !prod_path.exists() {
            exit_with_message!(format!(
                "Cannot find table file {}",
                table_path.to_str().expect("Unwrapping full table bath")
            ));
        }
        let table =
            db::table::Table::from_file(product.to_string(), table_path, prod_path.clone()).ok();

        let input = db::DeclareInputs {
            product,
            prod_dir: &prod_path,
            version,
            tag,
            ident,
            flavor,
            table,
        };

        let db = db::DBBuilder::new().build()?;
        let result = db.declare(vec![input], source);
        use db::DeclareResults::*;
        match result {
            NoSource(_) => {
                exit_with_message!("No source found with supplied name");
            }
            NoneWritable(_) => {
                exit_with_message!("No writable source found");
            }
            MultipleWriteable(_) => {
                exit_with_message!(
                    "More than one writable db found, specify source with --source"
                );
            }
            Error(_, name, msg) => {
                exit_with_message!(format!("Problem declaring to {}, check that version, and optionally tag and ident are not already declared. Error message: {}", name, msg));
            }
            Success(_, name) => {
                crate::info!("Wrote declared product {} to source {}", product, name);
            }
        }
        Ok(())
    }
}
