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

/**
 * This is the entry-point for the declare subcommand. Declare is used to add products to a
 * database for future use. This subcommand must be supplied a product name, version, and path to a
 * product. It may optionally be supplied with a tag. If there is only one writable database source
 * the command will write to that source. If more than one sources are found, the source
 * argument must be supplied. The source argument specifies what database backend the declared
 * product should be written to. Depending on the backend, the ident argument may or mar not be
 * optional. Currently a posix backend does not require it, but a JSON backend does. An additional
 * flag specifies if the path to the product to be declared is relative or not. If this flag is not
 * set declare will turn whatever path given into an absolute path. If the path is relative, it
 * should be relative to the directory containing the database source. For a posix backend, this
 * would be the directory containing ups_db, if is is JSON it would be the directory containing the
 * JSON file.
 *
 * * sub_args - Arguments matched from the command line to the given sub command
 * * _main_args - Arguments matched from the command line to the main reups executable,
 **/
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
        logger::build_logger(sub_args, std::io::stdout());
        DeclareCommandImpl {
            sub_args,
            _main_args,
        }
    }

    fn run(&mut self) -> Result<(), String> {
        let mut db = db::DBBuilder::from_args(self.sub_args).build()?;
        // see if the user wants to specify product path relative to db location
        let relative = self.sub_args.is_present("relative");
        let prod_path_string = self.sub_args.value_of("path").unwrap();
        let prod_path = if relative {
            let mut paths = vec![];
            for (_, path) in db.get_db_sources().iter() {
                let mut tmp_path = PathBuf::from(path)
                    .parent()
                    .expect("problem getting parent from db source path")
                    .to_path_buf();
                tmp_path.push(prod_path_string);
                if tmp_path.exists() {
                    paths.push(tmp_path)
                }
            }
            if paths.len() > 1 {
                return Err(
                    "There was more than one database source matching relative path".to_string(),
                );
            }
            if paths.len() == 0 {
                return Err("No paths were found relative to any db source".to_string());
            }
            paths.remove(0)
        } else {
            PathBuf::from(prod_path_string)
        };
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
        if !table_path.exists() {
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

        let prod_dir = if relative {
            PathBuf::from(prod_path_string)
        } else {
            prod_path
        };

        let input = db::DeclareInputs {
            product,
            prod_dir: &prod_dir,
            version,
            tag,
            ident,
            flavor,
            table,
            relative: self.sub_args.is_present("relative"),
        };

        let result = db.declare(vec![input], source);
        use db::DeclareResults::*;
        match result {
            NoSource => {
                exit_with_message!("No source found with supplied name");
            }
            NoneWritable => {
                exit_with_message!("No writable source found");
            }
            MultipleWriteable => {
                exit_with_message!("More than one writable db found, specify source with --source");
            }
            Error(name, msg) => {
                exit_with_message!(format!("Problem declaring to {}, check that version, and optionally tag and ident are not already declared. Error message: {}", name, msg));
            }
            Success(name) => {
                crate::info!("Wrote declared product {} to source {}", product, name);
            }
        }
        Ok(())
    }
}
