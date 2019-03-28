/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/**!
 * This module manages saving and restoring a users environment as it is setup. The idea behind
 * this module is that a user may issue several different calls to rsetup (reups setup) to
 * configure the exact packages active. The user can then save this setup to restore into a
 * different shell. This is not meant as a complete replacement for tagging product directories,
 * but as a sort of save your work buffer for setups that are not intended to live for a long
 * time. As a caveat this also replays the commands issued exactly, so if anything changed (such as
 * the current tag changing) commands will not necessarily recreate the exact environment at the
 * time of saving, but will reconstruct an environment as if the current (r)eups environment was
 * the environment when the commands were first issued.
 *
 **/
use crate::argparse;
use crate::logger;
use preferences;
use preferences::Preferences;
use std::env as stdEnv;
use std::io::{stdin, Write};

// This is the information used to differentiate this application to the preferences crate and is
// used to determine what path the settings will be saved to.
const APP_INFO: preferences::AppInfo = preferences::AppInfo {
    name: "reups",
    author: "Reups Community",
};

// This determines the exact location within the app's configuration space the environments will be
// saved in
const PREF_KEY: &str = "saved/environments";

/**
 * This is the main entry point for the env sub command. This command is used to save and restore
 * the (r)eups managed environment that is setup in the current shell. This function has different
 * effects based on the sub command argument supplied. The save argument will write out the current
 * environment either named default, or with the optinally supplied name. A convienence shell
 * function called rsave is supplied by reups prep to do the same task with less typing. The list subcommand will
 * list all the environments previously saved. If the delete argument is supplied, the given named
 * environment will be discarded, note the default environment cannot be deleted. The restore
 * argument is used by this program to reconstruct the chosen environment. Because of the
 * limitations of working with shells, the user should interact with this though the shell function
 * rrestore that is supplied with the reups prep command.
 *
 * * sub_args - Arguments matched from the command line to the given sub command
 * * _main_args - Arguments matched from the command line to the main reups executable,
 *                global arguments
 **/
pub fn env_command<W: Write>(
    sub_args: &argparse::ArgMatches,
    _main_args: &argparse::ArgMatches,
    writer: &mut W,
) {
    let mut env_command = EnvCommandImpl::new(sub_args, _main_args, writer);
    env_command.run();
}

/**
 * This is the internal implementation of the env sub command
 */
struct EnvCommandImpl<'a, W: Write> {
    sub_args: &'a argparse::ArgMatches<'a>,
    _main_args: &'a argparse::ArgMatches<'a>,
    current_commands: Vec<String>,
    name: String,
    saved_envs: preferences::PreferencesMap<Vec<String>>,
    writer: &'a mut W,
}

impl<'a, W: Write> EnvCommandImpl<'a, W> {
    /**
     * Creates a new EnvCommandImpl. The function uses the supplied argument matches, looks up the
     * commands that were executed in the current environment, parses the name to use in
     * processing, and then looks up if there was an existing environment store, creating one if
     * there was none present.
     **/
    fn new(
        sub_args: &'a argparse::ArgMatches<'a>,
        _main_args: &'a argparse::ArgMatches<'a>,
        writer: &'a mut W,
    ) -> EnvCommandImpl<'a, W> {
        // make a logger object
        logger::build_logger(sub_args, std::io::stdout());
        // Get the environment variable
        let current_commands = match stdEnv::var("REUPS_HISTORY") {
            Ok(existing) => existing.split("|").map(|x| String::from(x)).collect(),
            _ => vec![],
        };

        // Get a name to consider if one is supplied, otherwise use default
        let name = {
            if sub_args.is_present("name") {
                String::from(sub_args.value_of("name").unwrap())
            } else {
                String::from("default")
            }
        };

        // Load in an existing save environment
        let saved_envs = preferences::PreferencesMap::<Vec<String>>::load(&APP_INFO, PREF_KEY);
        // Check that there was an existing environment, otherwise create one.
        let saved_envs = {
            if saved_envs.is_ok() {
                crate::debug!("saved_envs loaded existing env");
                saved_envs.unwrap()
            } else {
                // there is no existing preferences
                crate::debug!("Existing env was not loaded, create and use new env store");
                crate::warn!("No existing env store could be found create a new one? (y/N)");
                let mut s = String::new();
                stdin()
                    .read_line(&mut s)
                    .expect("Did not enter a correct option");
                if let Some('\n') = s.chars().next_back() {
                    s.pop();
                }
                if let Some('\r') = s.chars().next_back() {
                    s.pop();
                }
                if s == "y" || s == "Y" {
                    crate::warn!("Creating new env store");
                    preferences::PreferencesMap::<Vec<String>>::new()
                } else {
                    exit_with_message!("No env store found or created, exiting");
                }
            }
        };

        // initialize and return a new struct
        EnvCommandImpl {
            sub_args,
            _main_args,
            current_commands,
            name,
            saved_envs: saved_envs,
            writer,
        }
    }

    /** The man entry point for running this command. The function looks at what action argument
     * was provided, and dispatches to the corresponding functionality
     **/
    fn run(&mut self) {
        // look at what the current command is
        match self.sub_args.value_of("command").unwrap() {
            "save" => self.run_save(),
            "restore" => self.run_restore(),
            "delete" => self.run_delete(),
            "list" => self.run_list(),
            _ => (),
        }
    }

    /** Saves any commands that were executed in the current environment
     **/
    fn run_save(&mut self) {
        self.saved_envs
            .insert(self.name.clone(), self.current_commands.clone());
        let save_result = self.saved_envs.save(&APP_INFO, PREF_KEY);
        save_result.expect("There was a problem saving the current env");
    }

    /** Restores a given environment. This action is most likely to be activated by the rrestore
     * shell function provided by reups prep. Direct invocation is most likely only for debug
     * reasons. This is a limitation of modifying shell environments.
     **/
    fn run_restore(&mut self) {
        // get env to restore from the supplied name
        let env_list_option = &self.saved_envs.get(&self.name);
        // Verify the environment could be found, otherwise exit
        let env_list = match env_list_option {
            Some(list) => list,
            None => {
                exit_with_message!(format!(
                    "Cannot find environment {}. Use reups env list to see saved environments",
                    &self.name
                ));
            }
        };
        // build an application object to parse the saved commands. This is done to verify that
        // a command was indeed a setup command. This could be done in other ways by string
        // parsing, but the overhead is so little reuse of existing code is preferable
        let app = argparse::build_cli();
        for command in *env_list {
            // split a command string into a vector  and use the app to match
            let args = app.clone().get_matches_from(command.split(" "));
            match args.subcommand() {
                ("setup", Some(_)) => {
                    let _ = self
                        .writer
                        .write(format!("eval $({});\n", command).as_bytes());
                }
                _ => {
                    exit_with_message!(format!("Problem restoring environment {}", &self.name));
                }
            };
        }
    }

    /** This function is responsible for managing the delete action. Using the name supplied the
     * specified saved environment is discarded. It will not discard the default environment.
     * Simply save over default if a change is desired.
     **/
    fn run_delete(&mut self) {
        // Don't delete the default environment
        if self.name == "default" {
            exit_with_message!("Cannot delete default save");
        }
        self.saved_envs.remove(&self.name);
        let save_result = self.saved_envs.save(&APP_INFO, PREF_KEY);
        if !save_result.is_ok() {
            exit_with_message!("There was a problem deleting the environment");
        }
    }

    /** This function will list all named environments that have been saved in the past
     */
    fn run_list(&mut self) {
        let _ = self.writer.write(b"Environments Found:\n");
        for (k, v) in &self.saved_envs {
            let _ = self.writer.write(format!("{}\n", k).as_bytes());
            crate::info!("{:?}", v);
        }
    }
}
