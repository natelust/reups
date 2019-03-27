/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

/*!
A reimagined, from scratch, reimplementation of the
[eups](https://github.com/RobertLuptonTheGood/eups) environment modules system in rust.

This software is designed to be a drop in replacement for much of the functionality of eups
environment management system. As such its job is to manage loading and unloading defined packages
and their dependencies into a shell environment. In this way a coherent software stack is available
on a per-shell basis. The system enables easily switching out different versions of a software
package for changing to a new release, or testing a new version.

While much of the functionally that eups has will be available in reups, it is not the stated goal
of this project to have a 100% identical interface. Instead, these tools will share some (maybe a
lot) of functionally, and be able to operate in the same environment on the same software products
and databases. Differing apis and command line options will allow exploring new ideas and design
paradigms.

One reason that some functionality of reups will differ is that a primary focus of this tool is
speed! The aim is to be able to complete all common operations in orders of magnitude less time. As
such, the runtime performance of these common operations is placed at a higher precedence an
architecture which includes all the niche functionality. In the future more overlapping
functionality may be introduced if it can be performant and is in agreement with how the developers
wish to spend their time. If your favorite feature is missing, feel free to implement it and send a
pull request!

# Installing

**From Source**

This project is built in the rust programming language, and is available as a crate on crates.io. Crate is the
package management/build tool for the rust programming language and can be used to fetch a copy of this code.

The source repository (and main home) for this project can be found on Github
[here](https://github.com/natelust/reups). If cloning from the github source, cargo is still required for
compilation.

**Binaries**

Binaries of release versions can be found on Github [here](https://github.com/natelust/reups/releases). These
are available for both linux (distro agnostic), and MacOs (10.7 and higher).

# Usage
In it's current state reups bootstraps itself off an eups installation. As such eups must be setup prior to
using reups. To use the functionality of reups place the binary in your path, and execute `eval $(reups prep)`
in each shell where it is to be used. The eval step is necessary to setup all the machinery required to
export environment variables into the currently running environment.

The functionality of reups is split among different commands, each having their own options and configuration.
Details of these commands are as follows:

**Prep**

This command is used to setup reups, and is responsible for assembling all the shell functionality such as
providing the `rsetup`, `rrestore`, and `rsave` tools. This command is most commonly used as `eval $(reups prep).

**Completions**

This command generates a shell completion script for the specified shell (one of bash, zsh, elvish,
and fish) and dumps it to standard out. The user may then take this output and put it in whatever
location is appropriate for their shell.

For example a bash user may pipe the output from standard out and put it into a file located in
`/etc/bash_completions.d/` for system wide completions. If the user does not have write access to
the system directory, bash completions can be put into `~/.bash_completions`. This file however is
shared between all extra auto completions in the users directory, a work around for this can be
found
[here](https://serverfault.com/questions/506612/standard-place-for-user-defined-bash-completion-d-scripts).

**Setup**

Setup is the entry point for setting up a (r)eups product. This command will return a string containing all
relivant environment variables modified or created to setup the requested product. As this function returns
a string, most commonly this function will not be executed as `reups setup` but with the command `rsetup`,
$which is a shell function created with `eval $(eups prep)`, that evaluates the string returned by
`reups setup` into the currently running shell process.

The setup command (and thus rsetup) support the following options:
* -j --just: Only setup this product, no dependencies
* -r --relative: Setup directory or table file specified by a relative path
* -k --keep: Keep any products already setup, dont replace them when reruning a new command
* -t --tag: Use this tag when setting up products, multiple are allowed and are evaluated left to right
* -E --inexact: Use only tags in deciding what to setup, ignore any versions declared in table files
* \<product\>: Positional argument which is the name of the product to setup, conflicts with relative option

**List**

Lists various properties about managed packages, and the current environment

* -s --setup: Only list setup products
* -t --tags: Only list specified tag(s). Multiple instances are ok
* --onlyTags: Only list products and tags on output. This is faster than listing products, tags, and versions, conflicts with only Versions
* --onlyVers: Only list product and versions on output. This is faster than listing products, tags, and versions conflicts with onlyTags
* -l --local Only list products that have been setup with the -r option. Conflicts with setup or a product as an argument.
* \<product>: Name of product to list

**Env**

Env is a subcommand for saving and restoring a users environment as it has been setup. The idea behind this
module is that a user may issue several different calls to rsetup (reups setup) to configure the exact
packages active. The user can then save this setup to restore into a different shell. This is not meant as a
complete replacement for tagging product directories, but as a sort of save your work buffer for setups that
are not intended to live for a long time. As a caveat this also replays the commands issued exactly, so if
anything changed (such as the current tag changing) commands will not necessarily recreate the exact
environment at the time of saving, but will reconstruct an environment as if the current (r)eups environment
was the environment when the commands were first issued.

Users will most often interact with this subcommand with the supplied `rsave` and `rrestore` shell functions supplied by `reups prep`. `rrestore`
must be used to restore an environment, as shell environment variables are being set or manipulated.
`rsave` is exactly identical to typing out `reups env save` and is supplied as a convienence to the user.

* -v --verbose Sets the level of verbosity, multiple occurances increases verbosity
* \<action\>: Required, one of save, restore, delete, list
* \<name\>: Optional, a name to use when saving or restoring


**/

extern crate reups_lib;

use reups_lib as reups;
use std::io::Write;

fn handle_result(res: Result<(), String>) {
    match res {
        Ok(_) => (),
        Err(msg) => {
            std::io::stderr()
                .write(msg.as_bytes())
                .expect("Error writing error message");
        }
    };
}

fn main() {
    let args = reups::parse_args();

    match args.subcommand() {
        ("setup", Some(m)) => handle_result(reups::setup_command(m, &args, std::io::stdout())),
        ("prep", Some(_)) => {
            println!("{}", reups::build_prep_string());
        }
        ("list", Some(m)) => handle_result(reups::list_command(m, &args, std::io::stdout())),
        ("completions", Some(m)) => {
            reups::write_completions_stdout(m.value_of("shell").unwrap());
        }
        ("env", Some(m)) => {
            reups::env_command(m, &args, std::io::stdout());
        }
        ("declare", Some(m)) => handle_result(reups::declare_command(m, &args)),
        _ => println!("{}", args.usage()),
    }
}
