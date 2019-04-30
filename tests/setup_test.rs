extern crate reups_lib;
use reups_lib as reups;
use std::env;
use std::io::Cursor;
use std::path::PathBuf;

fn common(args: Vec<&str>, expected: &str, json: bool) {
    let current_exe = std::env::current_exe().unwrap();
    let exe_string = current_exe.to_str().unwrap().to_string();
    env::set_var("PATH", "");
    env::set_var("REUPS_HISTORY", "");
    let mut args: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crate_path = root.clone();
    if !json {
        root.push("resources/posix_db");
    } else {
        root.push("resources/json_db/json_db.json");
    }
    let root_string = root.to_str().unwrap().to_string();
    args.push(root_string);

    let mut cursor = Cursor::new(vec![]);

    let app = reups::build_cli();
    let matches = app.get_matches_from(args);
    let (_, m) = matches.subcommand();
    let result = reups::setup_command(m.unwrap(), &matches, &mut cursor);
    assert!(result.is_ok());

    let mut output = String::from_utf8(cursor.into_inner()).unwrap();
    output = output.replace(" --nocapture", "");
    let expected = expected.replace("$CRATE_PATH", crate_path.to_str().unwrap());
    let expected = expected.replace("$CURRENT_EXE", exe_string.as_str());
    assert_eq!(output, expected);
}

#[test]
fn test_setup_exact() {
    let args = vec!["reups", "setup", "-U", "-S", "fooA", "-Z"];
    let expected = "export FOOB_DIR=$CRATE_PATH/resources/test_packages/fooB SETUP_FOOA=fooA\\ v3\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ REUPS_HISTORY=\"|$CURRENT_EXE\" PATH=$CRATE_PATH/resources/test_packages/fooA/bin: SETUP_FOOB=fooB\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ FOOC_DIR=$CRATE_PATH/resources/test_packages/fooC SETUP_FOOC=fooC\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ FOOA_DIR=$CRATE_PATH/resources/test_packages/fooA \n";
    common(args, expected, false);
}

#[test]
fn test_setup_inexact() {
    let args = vec!["reups", "setup", "-U", "-S", "-E", "fooA", "-Z"];
    let expected = "export FOOB_DIR=$CRATE_PATH/resources/test_packages/fooB SETUP_FOOA=fooA\\ v3\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ REUPS_HISTORY=\"|$CURRENT_EXE\" PATH=$CRATE_PATH/resources/test_packages/fooA/bin: SETUP_FOOB=fooB\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ FOOC_DIR=$CRATE_PATH/resources/test_packages/fooC SETUP_FOOC=fooC\\ v2\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/posix_db/ FOOA_DIR=$CRATE_PATH/resources/test_packages/fooA \n";
    common(args, expected, false);
}

#[test]
fn test_setup_exact_json() {
    let args = vec!["reups", "setup", "-U", "-S", "fooA", "-Z"];
    let expected = "export FOOB_DIR=$CRATE_PATH/resources/test_packages/fooB SETUP_FOOA=fooA\\ v3\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json REUPS_HISTORY=\"|$CURRENT_EXE\" PATH=$CRATE_PATH/resources/test_packages/fooA/bin: SETUP_FOOB=fooB\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json FOOC_DIR=$CRATE_PATH/resources/test_packages/fooC SETUP_FOOC=fooC\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json FOOA_DIR=$CRATE_PATH/resources/test_packages/fooA \n";
    common(args, expected, true);
}

#[test]
fn test_setup_inexact_json() {
    let args = vec!["reups", "setup", "-U", "-S", "-E", "fooA", "-Z"];
    let expected = "export FOOB_DIR=$CRATE_PATH/resources/test_packages/fooB SETUP_FOOA=fooA\\ v3\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json REUPS_HISTORY=\"|$CURRENT_EXE\" PATH=$CRATE_PATH/resources/test_packages/fooA/bin: SETUP_FOOB=fooB\\ v1\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json FOOC_DIR=$CRATE_PATH/resources/test_packages/fooC SETUP_FOOC=fooC\\ v2\\ -f\\ Linux64\\ -Z\\ $CRATE_PATH/resources/json_db/json_db.json FOOA_DIR=$CRATE_PATH/resources/test_packages/fooA \n";
    common(args, expected, true);
}
