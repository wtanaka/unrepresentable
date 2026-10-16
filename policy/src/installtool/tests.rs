use super::parse_lock;
use crate::types::{Name, Text};

#[test]
fn real_tools_lock_parses() {
    let lock = Text::of(include_str!("../../../policy/tools.lock"));
    let tools = parse_lock(&lock);
    assert!(
        tools
            .as_ref()
            .is_ok_and(|found| found.contains_key(&Name::of("zizmor"))
                && found.contains_key(&Name::of("cargo-deny"))
                && found.contains_key(&Name::of("cargo-mutants"))),
        "the three pinned tools are listed"
    );
}

#[test]
fn tool_lines_are_strict() {
    let digest = "a".repeat(64);
    let good = format!("zizmor 1.0.0 https://github.com/o/r/releases/download/v1.0.0/zizmor-x86_64-unknown-linux-gnu.tar.gz {digest}\n");
    assert!(
        parse_lock(&Text::from_string(good)).is_ok(),
        "a good line parses"
    );
    for bad in [
        format!("zizmor 1.0.0 http://github.com/o/r/releases/download/v1.0.0/zizmor-x86_64-unknown-linux-gnu.tar.gz {digest}"),
        format!("zizmor 1.0.0 https://evil.example/o/r/releases/download/v1.0.0/zizmor-x86_64-unknown-linux-gnu.tar.gz {digest}"),
        "zizmor 1.0.0 https://github.com/o/r/releases/download/v1.0.0/zizmor-x86_64-unknown-linux-gnu.tar.gz abc".to_owned(),
        format!("Zizmor 1.0.0 https://github.com/o/r/releases/download/v1.0.0/zizmor-x86_64-unknown-linux-gnu.tar.gz {digest}"),
    ] {
        assert!(
            parse_lock(&Text::from_string(bad.clone())).is_err(),
            "{bad} must be refused"
        );
    }
    assert!(
        parse_lock(&Text::of("# comment\n\n")).is_ok_and(|found| found.is_empty()),
        "comments and blanks are skipped"
    );
}

#[test]
fn the_url_must_be_the_release_asset_of_the_pinned_version() {
    let digest = "a".repeat(64);
    let asset = "zizmor-x86_64-unknown-linux-gnu.tar.gz";
    let line = |url: &str| format!("zizmor 1.30.1 {url} {digest}");
    for good in [
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/releases/download/1.30.1/{asset}"),
        "https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/zizmor-1.30.1-x86_64-unknown-linux-musl.tar.gz".to_owned(),
    ] {
        assert!(
            parse_lock(&Text::from_string(line(&good))).is_ok(),
            "{good} is the asset"
        );
    }
    for bad in [
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.0/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.10/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/releases/latest/download/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/archive/v1.30.1/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/other-{asset}"),
        "https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/zizmor-x86_64-unknown-linux-gnu.zip".to_owned(),
        "https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/cargo-deny-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/{asset}?x=1"),
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/{asset}/more"),
        format!("https://github.com/../zizmor/releases/download/v1.30.1/{asset}"),
        format!("https://github.com/zizmorcore/releases/download/v1.30.1/{asset}"),
        format!("https://github.com/zizmorcore/zizmor/releases/download/v1.30.1/x/{asset}"),
    ] {
        assert!(
            parse_lock(&Text::from_string(line(&bad))).is_err(),
            "{bad} is refused"
        );
    }
}
