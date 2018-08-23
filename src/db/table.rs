use db::fnv::FnvHashMap;
use regex::Regex;
use std::path;
use std::io;
use std::fs::File;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use std::thread;
use std::io::prelude::*;

lazy_static! {
    static ref EXACT_REGEX: Regex = Regex::new(r"(?ms)if\s[(]type\s==\sexact[)]\s[{](.+?)[}]").unwrap();
    static ref INEXACT_REGEX: Regex = Regex::new(r"(?ms)else\s+[{](.+?)[}]").unwrap();

    static ref EXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](?P<product>.+?)\s+[-]j\s(?P<version>.+?)[)]").unwrap();
    static ref EXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](?P<product>.+?)\s+[-]j\s(?P<version>.+?)[)]").unwrap();
    static ref INEXACT_OPTIONAL: Regex = Regex::new(r"setupOptional[(](?P<product>.+?)\s(?P<version>.+?)\s\[").unwrap();
    static ref INEXACT_REQUIRED: Regex = Regex::new(r"setupRequired[(](?P<product>.+?)\s(?P<version>.+?)\s\[").unwrap();
    static ref ENV_PREPEND: Regex = Regex::new(r"^envPrepend[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
    static ref ENV_APPEND: Regex = Regex::new(r"^envAppend[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
    static ref PATH_PREPEND: Regex = Regex::new(r"^pathPrepend[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
    static ref PATH_APPEND: Regex = Regex::new(r"^pathAppend[(](?P<var>.+)[,]\s(?P<target>.+)[)]").unwrap();
}

static mut CHANNELS : Option<Mutex<(Vec<Sender<(String, & Regex, & EnvActionType, String)>>,
                              Receiver<Option<(String, (EnvActionType, String))>>)>> = None;

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

        // If the static variable CHANNELS is none, then the worker threads
        // have not been started, and this is the first table to be created
        // start some workers that will live the lifetime of the program
        // execution that will handle some of the regex tasks. This must
        // be marked unsafe for rust, but has been tested and should be ok
        unsafe {
            if CHANNELS.is_none() {
                spawn_threads();
            }
        }

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
        let prod_dir_string = String::from(prod_dir.to_str().unwrap());
        unsafe {
            let guard = CHANNELS.as_ref().unwrap().lock().unwrap();
            let mut sender_iter = guard.0.iter().cycle(); 
            let env_re_vec : Vec<& Regex> = vec![&*ENV_PREPEND, &*ENV_APPEND, &*PATH_PREPEND, &*PATH_APPEND];
            for (re, action) in env_re_vec.iter().zip([EnvActionType::Prepend, EnvActionType::Append, EnvActionType::Prepend, EnvActionType::Append].iter()){
                let sender = sender_iter.next().unwrap();
                sender.send((contents.clone(), re, action, prod_dir_string.clone())).unwrap();
            }
            for _ in 0..env_re_vec.len(){
                if let Some(result) = guard.1.recv().unwrap() {
                    env_var.insert(result.0, result.1);
                }
            }
        }
        Ok(Table {name: name, path: path, product_dir: prod_dir, exact: exact, inexact: inexact, env_var: env_var})
    }

    fn extract_setup(input: &str, outer_regex: & Regex,
                     required_regex: & Regex,
                     optional_regex: & Regex) -> Option<Deps> {
        let exact_re = outer_regex.captures(input);
        match exact_re {
            Some(caps) => {
                let temp_string  = caps.get(1).unwrap().as_str();
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
            },
            None => None
        }
    }
}

fn spawn_threads() {
    let mut sender_vec : Vec<Sender<(String, & Regex, &EnvActionType, String)>> = vec![];
    let (send_to_main, recv_from_thread) = channel();
    for _ in 0..4 {
        let (send_to_thread, recv_from_main) = channel();
        sender_vec.push(send_to_thread);
        let send_to_main_clone = send_to_main.clone();
        thread::spawn(move || {
            for entry in recv_from_main.iter(){
                let (text, re, action, prod) = entry;
                if !re.is_match(text.as_str()) {
                    let _ = send_to_main_clone.send(None);
                }
                for cap in re.captures_iter(text.as_str()){
                    let var = String::from(&cap["var"]);
                    let target = String::from(&cap["target"]);
                    let final_target = target.replace("${PRODUCT_DIR}", prod.as_str());
                    let _ = send_to_main_clone.send(Some((var, (action.clone(), final_target))));
                }
            }
        });
    }
    unsafe {
        CHANNELS = Some(Mutex::new((sender_vec, recv_from_thread)));
    }
}
