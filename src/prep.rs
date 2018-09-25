/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

///Prepping the environment to use reups involves adding functions to
///the users shell. The string returned from this function adds various
///components (at this point only rsetup) to the users environment. The
///resulting string must be eval-ed by the user, most commonly done with
///eval $(reups prep)
pub fn build_prep_string() -> &'static str {
    "rsetup() {
    args=\"$*\";
    if [[ $args = *\"-h\"* ]] || [[ $args = *\"--help\"* ]];
    then
        reups setup \"$@\";
    else
        eval $(reups setup $args);
    fi;
}"
}
