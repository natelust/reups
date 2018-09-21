/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 * Copyright Nate Lust 2018*/

pub fn build_prep_string() -> & 'static str {
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
