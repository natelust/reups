/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use crate::argparse;
use crate::db;
use crate::db::DBBuilderTrait;
use fnv::{FnvHashMap, FnvHashSet};
use std::env;

/**
 * Lists info about products defined in the product database
 *
 * This function takes two arguments, one for command specific arguments,
 * and one for program general options. These arguments are parsed from the
 * command line, packaged and sent here. The arguments are defined in the
 * argparse module.
 */
pub fn list_command(
    sub_args: &argparse::ArgMatches,
    _main_args: &argparse::ArgMatches,
) -> Result<(), String> {
    let mut lister = ListImpl::new(sub_args, _main_args)?;
    lister.run();
    Ok(())
}

/// This enum controls if the list command shows tags, versions, or both
#[derive(Clone)]
enum OnlyPrint {
    Tags,
    Versions,
    All,
}

/**
 * The Listimpl structure is responsible for implementing the list subcomand functionality
 * It is created with argument matche from the command line in the new method. This method
 * prepopulates the database and some system variables. The sub command is executed with the run
 * command.
 */
struct ListImpl<'a> {
    sub_args: &'a argparse::ArgMatches<'a>,
    _main_args: &'a argparse::ArgMatches<'a>,
    output_string: String,
    current_products: FnvHashSet<(String, String)>,
    local_setups: FnvHashMap<String, String>,
    db: db::DB,
    tags: Option<Vec<String>>,
}

impl<'a> ListImpl<'a> {
    /** Creates a LIstImpl struct given argument matches from the command line
     */
    fn new(
        sub_args: &'a argparse::ArgMatches<'a>,
        _main_args: &'a argparse::ArgMatches<'a>,
    ) -> Result<ListImpl<'a>, String> {
        // Here we will process any of the global arguments in the future but for now there is
        // nothing so we do nothing but create the database. The global arguments might affect
        // construction in the future

        // cheat and look at the sub_args here to see if all products are listing all products or not. If
        // not, dont preload tag files in the database as this will slow things down
        let db_builder = db::DBBuilder::new();
        let db_builder = if !(sub_args.is_present("product")
            || sub_args.is_present("setup")
            || sub_args.is_present("tags")
            || sub_args.is_present("onlyTags")
            || sub_args.is_present("onlyVers")
            || sub_args.is_present("sources"))
        {
            // Mark the database to preload all the tag files off disk
            db_builder.set_load_control(db::DBLoadControl::Tags)
        } else {
            db_builder
        };
        let db = db_builder.build()?;
        // get any products that are currently setup
        let (current_products, local_setups) = find_setup_products();
        // String to hold the output
        let output_string = String::from("");
        // Hold tag information
        let tags = None;
        // create the object
        Ok(ListImpl {
            sub_args,
            _main_args,
            output_string,
            current_products,
            local_setups,
            db,
            tags,
        })
    }

    /// Runs the ListImpl over arguments given on the command line, and information
    /// gained from environment variables. Its result is the requested information is
    /// printed out to the user in the console.
    fn run(&mut self) {
        if self.sub_args.is_present("sources") {
            self.run_sources();
        } else {
            self.run_product();
        }
        println!("{}", self.output_string.trim_end_matches("\n"));
    }

    fn run_sources(&mut self) {
        let database_sources = self.db.get_db_sources();
        self.output_string.push_str("Source Identifier: Location\n");
        for (name, location) in database_sources.iter() {
            self.output_string.push_str(&format!(
                "{}: {}\n",
                name,
                location.to_str().expect("Issue unwrapping path")
            ));
        }
    }

    fn run_product(&mut self) {
        // If the user specified a specific product only generate output for that product
        let mut product_vec = if self.sub_args.is_present("product") {
            vec![self.sub_args.value_of("product").unwrap().to_string()]
        }
        // If the user specifed they want only setup products, get the list of those to display
        else if self.sub_args.is_present("setup") {
            self.current_products
                .iter()
                .map(|tup| tup.0.clone())
                .collect()
        }
        // If the user wants only products that have been locally setup, get the list of those
        // products
        else if self.sub_args.is_present("local") {
            self.local_setups.keys().map(|k| k.clone()).collect()
        }
        // Baring any input from the user, list all products found in the user and system databases
        else {
            self.db
                .get_all_products()
                .iter()
                .map(|a| a.to_string())
                .collect()
        };

        // check if we should restrict printing
        let select_printing = if self.sub_args.is_present("onlyTags") {
            OnlyPrint::Tags
        } else if self.sub_args.is_present("onlyVers") {
            OnlyPrint::Versions
        } else {
            OnlyPrint::All
        };

        // Read any tags the user supplied, to restrict printing to only products
        // with those tags
        let mut tags_vec = vec![];
        if self.sub_args.is_present("tags") {
            for t in self.sub_args.values_of("tags").unwrap() {
                tags_vec.push(t.to_string());
            }
            self.tags = Some(tags_vec);
        }

        // Sort the products to be listed so that the results come out deterministically and in
        // lexographic order
        product_vec.sort();
        // Loop over all products and print the information about that product.
        for product in product_vec.iter() {
            self.print_product(product, select_printing.clone());
        }
    }

    /**
     * Given a product to print, and printing options, this function retrieves
     * all the information required about the product from the database, formats
     * it, and appends it to the output string.
     */
    fn print_product(&mut self, product: &str, select_printing: OnlyPrint) {
        // If the user supplied tags, use those when determining the tags and
        // versions to print, else grab all tags associated with the given product
        let tags = if self.tags.is_some() {
            self.tags
                .as_ref()
                .unwrap()
                .iter()
                .map(|a| a.as_str())
                .collect()
        } else {
            self.db.product_tags(product)
        };

        // Switch on which printing is to be done, only tags, only versons, or all
        match select_printing {
            OnlyPrint::All => {
                // This builds an association between versions of a product and
                // what tags point to that version. Unfortunately this must
                // open and read a lot of files to do this.
                let mut version_to_tags = FnvHashMap::default();
                // dont accumulate versions if only locals are to be listed
                if !self.sub_args.is_present("local") {
                    for tag in tags.iter() {
                        if let OnlyPrint::All = select_printing {
                            let versions = self.db.get_versions_from_tag(product, &vec![tag]);
                            for v in versions {
                                version_to_tags
                                    .entry(v)
                                    .or_insert(Vec::<&str>::default())
                                    .push(tag);
                            }
                        }
                    }
                }
                // look for any local version that might be setup
                if let Some(local) = self.local_setups.get(product) {
                    version_to_tags
                        .entry(local)
                        .or_insert(Vec::<&str>::default());
                }

                // Turn the hashmap into a vector
                let mut version_to_tags_vec: Vec<(&str, Vec<&str>)> =
                    version_to_tags.into_iter().collect();
                // Sort the versions vector by version
                version_to_tags_vec.sort_by(|tup1, tup2| tup1.0.cmp(&tup2.0));
                // Iterate over and print results
                for (ver, tags) in version_to_tags_vec {
                    self.output_string.push_str(
                        format!(
                            "{:25}{:>25}{:10}{}]",
                            product,
                            ver,
                            "",
                            tags.iter()
                                .fold(String::from("["), |acc, &x| {
                                    // if the tag is current, color the string
                                    let name = if x == "current" {
                                        "\x1b[96mcurrent\x1b[0m"
                                    } else {
                                        x
                                    };
                                    acc + &name + ", "
                                })
                                .trim_right_matches(", ")
                        )
                        .as_str()
                        .trim(),
                    );
                    // Check if this product and version match any that are setup,
                    // and if so add a colored setup string
                    if self
                        .current_products
                        .contains(&(product.to_string(), ver.to_string()))
                    {
                        self.output_string.push_str("    \x1b[92mSetup\x1b[0m");
                    }
                    self.output_string.push_str("\n\n");
                }
            }
            OnlyPrint::Tags => {
                self.output_string.push_str(
                    format!(
                        "{:25}{:10}{}]",
                        product,
                        "",
                        tags.iter()
                            .fold(String::from("["), |acc, x| {
                                let name = if x == &"current" {
                                    "\x1b[96mcurrent\x1b[0m"
                                } else {
                                    &x
                                };
                                acc + name + ", "
                            })
                            .trim_right_matches(", ")
                    )
                    .as_str()
                    .trim(),
                );
                self.output_string.push_str("\n\n");
            }
            OnlyPrint::Versions => {
                let mut versions = if !self.sub_args.is_present("local") {
                    self.db.product_versions(product)
                } else {
                    vec![]
                };
                if let Some(local) = self.local_setups.get(product) {
                    versions.push(local);
                }
                self.output_string
                    .push_str(format!("{:25}{:10}", product, "").as_str());
                self.output_string.push_str("[");
                for version in versions {
                    if self
                        .current_products
                        .contains(&(product.to_string(), version.to_string()))
                    {
                        self.output_string
                            .push_str(format!("\x1b[92m{}\x1b[0m", version).as_str());
                    } else {
                        self.output_string.push_str(version);
                    }
                    self.output_string.push_str(", ");
                }
                // This removes the last space and comma added
                self.output_string.pop();
                self.output_string.pop();
                self.output_string.push_str("]");
                self.output_string.push_str("\n\n");
            }
        }
    }
}

/**
 * Read the environment variable and find all products that have previously been setup.
 *
 * Returns a tuple where the first element is a hash set of (product, version) tuples. The second
 * element is a hashmap of locally setup product names as keys, and their local setup path.
 */
fn find_setup_products() -> (FnvHashSet<(String, String)>, FnvHashMap<String, String>) {
    let mut product_set = FnvHashSet::default();
    let mut local_products = FnvHashMap::default();
    for (var, value) in env::vars() {
        if var.starts_with("SETUP_") {
            let value_vec: Vec<&str> = value.split(" ").collect();
            if value_vec.len() < 2 {
                // The value corresponding to a setup product should at least have
                // a Name and a version, if not there was an issue with that variable
                eprintln!("Warning, problem parsing {} skipping", var);
                continue;
            }
            // the first element is the product that is setup, the second is version
            product_set.insert((value_vec[0].to_string(), value_vec[1].to_string()));
            // Check if the product is a local setup. Track these differently, as
            // these versions will be the setup version, but not have a corresponding
            // version string in any database. This hashmap lets us display or append
            // local results onto results from databases
            if value_vec[1].starts_with("LOCAL") {
                local_products.insert(value_vec[0].to_string(), value_vec[1].to_string());
            }
        }
    }
    (product_set, local_products)
}
