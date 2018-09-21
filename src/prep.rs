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
