extern crate reups_lib;
use reups_lib as reups;
use std::io::Cursor;
use std::path::PathBuf;

fn common(args: Vec<&str>, expected: &str, json: bool) {
    let mut args: Vec<String> = args.iter().map(|&s| s.to_string()).collect();
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if json {
        root.push("resources/json_db/json_db.json")
    } else {
        root.push("resources/posix_db");
    }
    let root_string = root.to_str().unwrap().to_string();
    args.push(root_string);

    let mut cursor = Cursor::new(vec![]);

    let app = reups::build_cli();
    let matches = app.get_matches_from(args);
    let (_, m) = matches.subcommand();
    let result = reups::list_command(m.unwrap(), &matches, &mut cursor);
    assert!(result.is_ok());

    let output = String::from_utf8(cursor.into_inner()).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_list_full() {
    let args = vec!["reups", "list", "-U", "-S", "-Z"];
    let expected = "fooA                                            v1          []

fooA                                            v2          []

fooA                                            v3          [\u{1b}[96mcurrent\u{1b}[0m]

fooB                                            v1          [\u{1b}[96mcurrent\u{1b}[0m]

fooC                                            v1          []

fooC                                            v2          [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, false)
}

#[test]
fn test_list_full_json() {
    let args = vec!["reups", "list", "-U", "-S", "-Z"];
    let expected = "fooA                                            v1          []

fooA                                            v2          []

fooA                                            v3          [\u{1b}[96mcurrent\u{1b}[0m]

fooB                                            v1          [\u{1b}[96mcurrent\u{1b}[0m]

fooC                                            v1          []

fooC                                            v2          [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, true)
}

#[test]
fn test_list_only_vers() {
    let args = vec!["reups", "list", "-U", "-S", "--onlyVers", "-Z"];
    let expected = "fooA                               [v1, v2, v3]

fooB                               [v1]

fooC                               [v1, v2]
";
    common(args, expected, false)
}

#[test]
fn test_list_only_vers_json() {
    let args = vec!["reups", "list", "-U", "-S", "--onlyVers", "-Z"];
    let expected = "fooA                               [v1, v2, v3]

fooB                               [v1]

fooC                               [v1, v2]
";
    common(args, expected, true)
}

#[test]
fn test_list_only_tags() {
    let args = vec!["reups", "list", "-U", "-S", "--onlyTags", "-Z"];
    let expected = "fooA                               [\u{1b}[96mcurrent\u{1b}[0m]

fooB                               [\u{1b}[96mcurrent\u{1b}[0m]

fooC                               [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, false)
}

#[test]
fn test_list_only_tags_json() {
    let args = vec!["reups", "list", "-U", "-S", "--onlyTags", "-Z"];
    let expected = "fooA                               [\u{1b}[96mcurrent\u{1b}[0m]

fooB                               [\u{1b}[96mcurrent\u{1b}[0m]

fooC                               [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, true)
}

#[test]
fn test_list_one_product() {
    let args = vec!["reups", "list", "-U", "-S", "fooA", "-Z"];
    let expected = "fooA                                            v1          []

fooA                                            v2          []

fooA                                            v3          [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, false)
}

#[test]
fn test_list_one_product_json() {
    let args = vec!["reups", "list", "-U", "-S", "fooA", "-Z"];
    let expected = "fooA                                            v1          []

fooA                                            v2          []

fooA                                            v3          [\u{1b}[96mcurrent\u{1b}[0m]
";
    common(args, expected, true)
}
