use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::PathBuf;

use saltwater_parser::{preprocess, Definition, Opt, Token};

fn tokens(source: &str, mut opt: Opt) -> Vec<String> {
    opt.filename = PathBuf::from("tests/fixtures/profile_main.c");
    let program = preprocess(source, opt);
    let result = program.result.expect("preprocessor fixture must succeed");
    result
        .into_iter()
        .filter(|token| !matches!(token.data, Token::Whitespace(_)))
        .map(|token| token.data.to_string())
        .collect()
}

#[test]
fn object_and_function_macros_are_deterministic() {
    let source = r#"
#define BASE 40
#define ADD_ONE(value) ((value) + 1)
int answer = ADD_ONE(BASE);
"#;
    let first = tokens(source, Opt::default());
    let second = tokens(source, Opt::default());
    assert_eq!(first, second);
    let rendered = first.join("");
    assert!(rendered.contains("40"));
    assert!(rendered.contains("1"));
    assert!(!rendered.contains("BASE"));
    assert!(!rendered.contains("ADD_ONE"));
}

#[test]
fn if_integer_expression_and_command_line_define_undef_are_selected() {
    let mut definitions = HashMap::new();
    definitions.insert(
        "COSMIC_PROFILE".into(),
        Definition::try_from("1").expect("literal command-line definition"),
    );
    let source = r#"
#if COSMIC_PROFILE && (2 + 2 == 4)
#define SELECTED 7
#else
#define SELECTED 99
#endif
#undef COSMIC_PROFILE
#ifdef COSMIC_PROFILE
#define AFTER_UNDEF 99
#else
#define AFTER_UNDEF 1
#endif
int answer = SELECTED + AFTER_UNDEF;
"#;
    let rendered = tokens(
        source,
        Opt {
            definitions,
            ..Opt::default()
        },
    )
    .join("");
    assert!(rendered.contains("7"));
    assert!(rendered.contains("1"));
    assert!(!rendered.contains("99"));
    assert!(!rendered.contains("COSMIC_PROFILE"));
}

#[test]
fn quoted_include_uses_the_source_file_directory() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let source = "#include \"profile_header.h\"\nint answer = PROFILE_VALUE;\n";
    let mut opt = Opt::default();
    opt.search_path.push(fixture_dir);
    let rendered = tokens(source, opt).join("");
    assert!(rendered.contains("int"));
    assert!(rendered.contains("7"));
    assert!(!rendered.contains("PROFILE_VALUE"));
}

#[test]
fn preprocessing_errors_keep_a_stable_source_location() {
    let program = preprocess(
        "#if\nint answer = 0;\n#endif\n",
        Opt {
            filename: PathBuf::from("tests/fixtures/profile_main.c"),
            ..Opt::default()
        },
    );
    let error = program.result.expect_err("invalid expression must fail");
    let first = error.front().expect("one syntax diagnostic");
    assert_eq!(
        program.files.name(first.location.file),
        OsStr::new("tests/fixtures/profile_main.c")
    );
    assert_eq!(first.location.span.start, 3);
}

#[test]
fn explicit_include_paths_preserve_precedence() {
    let root =
        std::env::temp_dir().join(format!("cosmicc-include-precedence-{}", std::process::id()));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).expect("create first include directory");
    std::fs::create_dir_all(&second).expect("create second include directory");
    std::fs::write(first.join("precedence.h"), "#define INCLUDE_VALUE 11\n")
        .expect("write first header");
    std::fs::write(second.join("precedence.h"), "#define INCLUDE_VALUE 22\n")
        .expect("write second header");

    let mut opt = Opt::default();
    opt.search_path.push(first);
    opt.search_path.push(second);
    let rendered = tokens(
        "#include <precedence.h>\nint answer = INCLUDE_VALUE;\n",
        opt,
    )
    .join("");

    assert!(rendered.contains("11"));
    assert!(!rendered.contains("22"));
    let _ = std::fs::remove_dir_all(root);
}


#[test]
fn nested_builtin_header_keeps_its_full_path() {
    let rendered = tokens(
        "#include <sys/types.h>\nssize_t answer = 0;\n",
        Opt::default(),
    )
    .join("");
    assert!(rendered.contains("answer"));
    assert!(!rendered.contains("ssize_t"));
}
