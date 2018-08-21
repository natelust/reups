use db::fnv::FnvHashMap;
use regex::Regex;
use std::path;
use std::io;
use std::fs::File;
use std::io::prelude::*;

lazy_static! {
    static ref EXACT_REGEX: Regex = Regex::new(r"(?ms)if\s[(]type\s==\sexact[)]\s[{](.+)[}]").unwrap();
    static ref INEXACT_REGEX: Regex = Regex::new(r"(?ms)else\s+[{](.+)[}]").unwrap();
    static ref EXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](.+)\s+[-]j\s(.+)[)]").unwrap();
    static ref EXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](.+)\s+[-]j\s(.+)[)]").unwrap();
    static ref INEXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](.+)\s(.+)\s\[").unwrap();
    static ref INEXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](.+)\s(.+)\s\[").unwrap();
    static ref ENV_PREPEND: Regex = Regex::new(r"^envPrepend[(](.+)[,]\s(.+)[)]").unwrap();
    static ref ENV_APPEND: Regex = Regex::new(r"^envAppend[(](.+)[,]\s(.+)[)]").unwrap();
    static ref PATH_PREPEND: Regex = Regex::new(r"^pathPrepend[(](.+)[,]\s(.+)[)]").unwrap();
    static ref PATH_APPEND: Regex = Regex::new(r"^pathAppend[(](.+)[,]\s(.+)[)]").unwrap();

}

pub enum VersionType {
    Exact,
    Inexact
}

#[derive(Debug)]
pub enum EnvActionType {
    Prepend,
    Append
}

#[derive(Debug)]
pub struct Deps {
    pub required: FnvHashMap<String, String>,
    pub optional: FnvHashMap<String, String>
}

#[derive(Debug)]
pub struct Table {
    pub name: String,
    pub path: path::PathBuf,
    pub product_dir: path::PathBuf,
    pub exact: Option<Deps>,
    pub inexact: Option<Deps>,
    pub env_var: FnvHashMap<String, (EnvActionType, String)>
}

impl Table {
    pub fn new(name: String, path: path::PathBuf, prod_dir: path::PathBuf)
        -> Result<Table, io::Error>{
        let mut f = File::open(path.clone())?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        // Get the exact mapping
        // Dereferencing and taking a reference is nesseary to cause the
        // lazy static object defined at the top to be evaluated and turned into
        // a proper static, this only happens at first dereference. These are
        // defined as statics because they will remain between different tables
        // being created
        let exact = Table::extract_setup(contents.as_str(),
                                  &*EXACT_REGEX,
                                  &*EXACT_REQUIRED,
                                  &*EXACT_OPTIONAL);
        // Get the inexact mapping
        let inexact = Table::extract_setup(contents.as_str(),
                                    &*INEXACT_REGEX,
                                    &*INEXACT_REQUIRED,
                                    &*INEXACT_OPTIONAL);
        let mut env_var = FnvHashMap::default();
        for line in contents.as_str().lines() {
            let mut tmp_match = ENV_PREPEND.captures(line);
            let mut action_type = EnvActionType::Prepend;
            if let None = tmp_match {
                tmp_match = ENV_APPEND.captures(line);
                action_type = EnvActionType::Append
            }
            if let None = tmp_match {
                tmp_match = PATH_PREPEND.captures(line);
                action_type = EnvActionType::Prepend;
            }
            if let None = tmp_match {
                tmp_match = PATH_APPEND.captures(line);
                action_type = EnvActionType::Append;
            }
            if let Some(cap) = tmp_match {
                let var = String::from(cap.get(1).unwrap().as_str());
                let target= String::from(cap.get(2).unwrap().as_str());
                let final_target = target.replace("${PRODUCT_DIR}",
                                                  prod_dir.to_str().unwrap()); 
                env_var.insert(var, (action_type, final_target));
            }
        }
        Ok(Table {name: name, path: path, product_dir: prod_dir,  exact: exact, inexact: inexact, env_var: env_var})
    }

    fn extract_setup(input: &str, outer_regex: & Regex,
                     required_regex: & Regex,
                     optional_regex: & Regex) -> Option<Deps> {
        let exact_re = outer_regex.captures(input);
        match exact_re {
            Some(caps) => {
                let temp_string = caps.get(1).unwrap().as_str();
                let mut required_map = FnvHashMap::default();
                let mut optional_map = FnvHashMap::default();
                for line in temp_string.lines() {
                    // Check if the line is an option or required
                    let required_re = required_regex.captures(line);
                    let optional_re = optional_regex.captures(line);
                    // Add to corresponding vector if the capture is not none
                    if let Some(req_cap) = required_re {
                        let prod = req_cap.get(1).unwrap().as_str().trim();
                        let vers = req_cap.get(2).unwrap().as_str().trim();
                        required_map.insert(String::from(prod), String::from(vers));
                    }
                    if let Some(opt_cap) = optional_re {
                        let prod = opt_cap.get(1).unwrap().as_str().trim();
                        let vers = opt_cap.get(2).unwrap().as_str().trim();
                        optional_map.insert(String::from(prod), String::from(vers));

                    }
                }
                Some(Deps { required: required_map, optional: optional_map })
            },
            None => None
        }
    }
}
