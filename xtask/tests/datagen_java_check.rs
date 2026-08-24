use xtask::datagen::java_check::parse_java_major;

#[test]
fn parses_modern_single_number_version() {
    assert_eq!(
        parse_java_major("openjdk version \"25\" 2026-04-21\nOpenJDK Runtime Environment...\n"),
        Some(25)
    );
}

#[test]
fn parses_dotted_version() {
    assert_eq!(
        parse_java_major("openjdk version \"25.0.1\" 2026-05-01\n..."),
        Some(25)
    );
}

#[test]
fn parses_legacy_one_dot_scheme() {
    assert_eq!(parse_java_major("java version \"1.8.0_301\"\n..."), Some(8));
}

#[test]
fn returns_none_on_unparseable_input() {
    assert_eq!(parse_java_major("not a java version string"), None);
}
