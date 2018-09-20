/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

use argparse;
use std::env;
use db;
use fnv::{FnvHashMap, FnvHashSet};

pub fn list_command(sub_args: & argparse::ArgMatches,
                    _main_args: & argparse::ArgMatches) {
    let mut lister = ListImpl::new(sub_args, _main_args);
    lister.run();

}

#[derive(Clone)]
enum OnlyPrint {
    Tags,
    Versions,
    All
}

struct ListImpl<'a> {
    sub_args: & 'a argparse::ArgMatches<'a>,
    main_args: & 'a argparse::ArgMatches<'a>,
    output_string: String,
    current_products: FnvHashSet<(String, String)>,
    local_setups: FnvHashMap<String, String>,
    db : db::DB,
    tags : Option<Vec<String>>,
}

impl<'a> ListImpl<'a> {
    fn new(sub_args : & 'a argparse::ArgMatches<'a>, main_args : & 'a argparse::ArgMatches<'a>) -> ListImpl<'a> {
        // Here we will process any of the global arguments in the future but for now there is
        // nothing so we do nothing but create the database. The global arguments might affect
        // construction in the future

        // cheat and look at the sub_args here to see if all products are listing all products or not. If
        // not, dont preload tag files in the database as this will slow things down
        let preload = if sub_args.is_present("product") ||
            sub_args.is_present("setup") ||
            sub_args.is_present("tags") ||
            sub_args.is_present("onlyTags") ||
            sub_args.is_present("onlyVers"){
            None
        }
        else {
            Some(db::DBLoadControl::Tags)
        };
        let db = db::DB::new(None, None, None, preload);
        // get any products that are currently setup
        let (current_products, local_setups) = find_setup_products();
        // String to hold the output
        let output_string = String::from("");
        // Hold tag information
        let tags = None;
        // create the object
        ListImpl{
            sub_args,
            main_args,
            output_string,
            current_products,
            local_setups,
            db,
            tags,
        }
    }

    fn run(& mut self) {
        let mut product_vec = if self.sub_args.is_present("product") {
           vec![self.sub_args.value_of("product").unwrap().to_string()] 
        }
        else if self.sub_args.is_present("setup") {
            self.current_products.iter().map(|tup| tup.0.clone()).collect()
        }
        else if self.sub_args.is_present("local") {
            self.local_setups.keys().map(|k| k.clone()).collect()
        }
        else {
            self.db.get_all_products()
        };

        // check if we should restrict printing
        let select_printing = if self.sub_args.is_present ("onlyTags") {
            OnlyPrint::Tags
        }
        else if self.sub_args.is_present("onlyVers") {
            OnlyPrint::Versions
        }
        else {
            OnlyPrint::All
        };

        let mut tags_vec = vec![];
        if self.sub_args.is_present("tags") {
            for t in self.sub_args.values_of("tags").unwrap() {
                tags_vec.push(t.to_string());
            }
            self.tags = Some(tags_vec);
        }

        product_vec.sort();
        for product in product_vec.iter() {
            self.print_product(product, select_printing.clone());
        }
        println!("{}", self.output_string.trim_right_matches("\n\n"));
    }

    fn print_product(& mut self, product: &String, select_printing : OnlyPrint){
        let tags = if self.tags.is_some() {
            self.tags.as_ref().unwrap().clone()
        }
        else {
            self.db.product_tags(product)
        };
        match select_printing {
            OnlyPrint::All => {
                let mut version_to_tags = FnvHashMap::default();
                for tag in tags.iter() {
                    if let OnlyPrint::All = select_printing {
                        let versions = self.db.get_versions_from_tag(product, vec![tag]); 
                        for v in versions {
                            version_to_tags.entry(v).or_insert(vec![]).push(tag);
                        }
                    }
                }
                // look for any local version that might be setup
                if let Some(local) = self.local_setups.get(product) {
                    version_to_tags.entry(local.clone()).or_insert(vec![]);
                }

                let mut version_to_tags_vec : Vec<(String, Vec<&String>)> = version_to_tags.into_iter().collect();
                version_to_tags_vec.sort_by(|tup1, tup2| tup1.0.cmp(&tup2.0));
                for (ver, tags) in version_to_tags_vec {
                    self.output_string.push_str(format!("{:25}{:>25}{:10}{}]", product, ver, "", tags.iter().fold(String::from("["), |acc, &x| {
                        let name = if *x == "current" {
                            "\x1b[96mcurrent\x1b[0m".to_owned()
                        }
                        else {
                            (*x).clone()
                        };
                        acc + &name + ", "
                    }).trim_right_matches(", ")).as_str().trim());
                    if self.current_products.contains(&(product.clone(), ver)) {
                        self.output_string.push_str("    \x1b[92mSetup\x1b[0m");
                    }
                    self.output_string.push_str("\n\n");
                }
            },
            OnlyPrint::Tags => {
                self.output_string.push_str(format!("{:25}{:10}{}]", product, "", tags.iter().
                                                    fold(String::from("["), |acc, x| {
                                                        let name = if x == "current" {
                                                            "\x1b[96mcurrent\x1b[0m"
                                                        }
                                                        else {
                                                            &x
                                                        };
                                                        acc + name + ", "
                                                    }).trim_right_matches(", ")).as_str().trim());
                self.output_string.push_str("\n\n");
            },
            OnlyPrint::Versions => {
                let mut versions = self.db.product_versions(product);
                if let Some(local) = self.local_setups.get(product) {
                    versions.push(local.clone());
                }
                self.output_string.push_str(format!("{:25}{:10}", product, "").as_str());
                self.output_string.push_str("[");
                for version in versions {
                    if self.current_products.contains(&(product.clone(), version.clone())) {
                        self.output_string.push_str(format!("\x1b[92m{}\x1b[0m", version).as_str());
                    }
                    else {
                        self.output_string.push_str(version.as_str());
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

fn find_setup_products() -> (FnvHashSet<(String, String)>, FnvHashMap<String, String>) {
    let mut product_set = FnvHashSet::default();
    let mut local_products = FnvHashMap::default();
    for (var, value) in env::vars() {
        if var.starts_with("SETUP_") {
            let value_vec: Vec<&str> = value.split(" ").collect();
            if value_vec.len() < 2 {
                eprintln!("Warning, problem parsing {} skipping", var);
            }
            // the first element is the product that is setup, the second is version
            product_set.insert((value_vec[0].to_string(), value_vec[1].to_string()));
            if value_vec[1].starts_with("LOCAL") {
                local_products.insert(value_vec[0].to_string(), value_vec[1].to_string());
            }
        }
    }
    (product_set, local_products)
}
