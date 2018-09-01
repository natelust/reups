# Reups

A reimagined, from scratch, reimplementation of the [eups](https://github.com/RobertLuptonTheGood/eups) environment modules system in rust.

This software is designed to be a drop in replacement for much of the functionality of eups environment
management system. As such its job is to manage loading and unloading defined packages and their dependencies
into a shell environment. In this way a coherent software stack is available on a per-shell basis. The system
enables easily switching out different versions of a software package for changing to a new release, or
testing a new version.

While much of the functionally that eups has will be available in reups, it is not the stated goal of this
project to have a 100% identical interface. Instead, these tools will share some (maybe a lot) of
functionally, and be able to operate in the same environment on the same software products and databases.
Differing apis and command line options will allow exploring new ideas and design paradigms.

One reason that some functionality of reups will differ is that a primary focus of this tool is speed! The aim
is to be able to complete all common operations in orders of magnitude less time. As such, the runtime
performance of these common operations is placed at a higher precedence an architecture which includes all
the niche functionality. In the future more overlapping functionality may be introduced if it can be
performant and is in agreement with how the developers wish to spend their time. If your favorite feature is
missing, feel free to implement it and send a pull request!

## Getting Started

### Installing

* From Source
This project is built in the rust programming language, and is available as a crate on crates.io. Crate is the
package management/build tool for the rust programming language and can be used to fetch a copy of this code.

The source repository (and main home) for this project can be found on Github
[here](https://github.com/natelust/reups). If cloning from the github source, cargo is still required for
compilation.

* Binaries
Binaries of release versions can be found on Github [here](https://github.com/natelust/reups/releases). These
are available for both linux (distro agnostic), and MacOs (10.7 and higher).

###Usage
In it's current state reups bootstraps itself off an eups installation. As such eups must be setup prior to
using reups. After eups is setup, reups should be set up by evaulating the results of the builtin prep command, as can be seen in the following example.

__Note__ As of version 0.1.0, reups only implements the setup functionality of eups, with a command called `rsetup`,
chosen to not interfere with the default `setup` command.

####Example
```bash
# Setup the standard eups installation
source <path_to_eups>/bin/setups.sh

# Setup the reups installation
eval $(reups prep)

# Use reups to setup prduct foobar
rsetup foobar
```
__Note__: For those not comfortable with running eval in a shell, the `reups prep` command can be run by
itself, and the output inspected prior to evaluating it.
