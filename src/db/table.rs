use db::fnv::FnvHashMap;
use regex::Regex;
use std::path;
use std::io;
use std::fs::File;
use std::io::prelude::*;

lazy_static! {
    static ref EXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](?P<product>.+?)\s+[-]j\s(?P<version>.+?)[)]").unwrap();
    static ref EXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](?P<product>.+?)\s+[-]j\s(?P<version>.+?)[)]").unwrap();
    static ref INEXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](?P<product>.+?)\s(?P<version>.+?)\s\[").unwrap();
    static ref INEXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](?P<product>.+?)\s(?P<version>.+?)\s\[").unwrap();
    static ref ENV_PREPEND: Regex = Regex::new(r"^(envPrepend|pathPrepend)[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
    static ref ENV_APPEND: Regex = Regex::new(r"^(envAppend|pathAppend)[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
}

pub enum VersionType {
    Exact,
    Inexact
}

#[derive(Debug, Clone)]
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
                                  &*EXACT_REQUIRED,
                                  &*EXACT_OPTIONAL);
        // Get the inexact mapping
        let inexact = Table::extract_setup(contents.as_str(),
                                    &*INEXACT_REQUIRED,
                                    &*INEXACT_OPTIONAL);
        let mut env_var = FnvHashMap::default();
        let env_re_vec : Vec<& Regex> = vec![&*ENV_PREPEND, &*ENV_APPEND];
        for (re, action) in env_re_vec.iter().zip([EnvActionType::Prepend, EnvActionType::Append].iter()){
            for cap in re.captures_iter(contents.as_str()){
                let var = String::from(&cap["var"]);
                let target = String::from(&cap["target"]);
                let final_target = target.replace("${PRODUCT_DIR}", prod_dir.to_str().unwrap());
                env_var.insert(var, (action.clone(), final_target));
            }
        }
        Ok(Table {name: name, path: path, product_dir: prod_dir, exact: exact, inexact: inexact, env_var: env_var})
    }

    fn extract_setup(input: &str, required_regex: & Regex,
                     optional_regex: & Regex) -> Option<Deps> {
        let temp_string = input;
        let mut required_map = FnvHashMap::default();
        let mut optional_map = FnvHashMap::default();
        let re_vec = [required_regex, optional_regex];
        for (re, map) in re_vec.iter().zip(
                         [& mut required_map, & mut optional_map].iter_mut()) {
            for dep_cap in re.captures_iter(temp_string) {
                let prod = &dep_cap["product"];
                let vers = &dep_cap["version"];
                map.insert(String::from(prod), String::from(vers));
            }
        }
        Some(Deps { required: required_map, optional: optional_map })
    }
}
