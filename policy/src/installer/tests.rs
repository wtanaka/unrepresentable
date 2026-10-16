use super::{generate, parse_sums, Installer, TARGETS};
use crate::io;
use crate::process::{self, Args, Captured, EnvPair, Invocation, Status};
use crate::testkit::{Fail, Scratch};
use crate::types::{Answer, Count, Name, RelPath, Text};

macro_rules! installer {
    ($version:literal) => {
        Installer {
            template: Text::of(TEMPLATE),
            sums: sums(),
            version: Text::of($version),
        }
    };
}

const TEMPLATE: &str = include_str!("../../../ci/unrepresentable.sh");

/// A `SHA256SUMS` text with digest `i` repeated for the i-th target.
fn sums() -> Text {
    let lines: Vec<String> = TARGETS
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let suffix = if target.contains("windows") {
                ".exe"
            } else {
                ""
            };
            format!(
                "{}  unrepresentable-{target}{suffix}\n",
                index.to_string().repeat(64)
            )
        })
        .collect();
    Text::from_string(lines.concat())
}

fn sh(scratch: &Scratch, args: &Args) -> Result<Captured, Fail> {
    let program = Name::of("sh");
    Ok(process::run(
        &Invocation::new(&program, args).in_dir(scratch.path()),
    )?)
}

#[test]
fn template_is_posix_sh_syntax() {
    assert_eq!(
        check_template_is_posix_sh_syntax(),
        Ok(()),
        "template_is_posix_sh_syntax"
    );
}

fn check_template_is_posix_sh_syntax() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("t.sh"), &Text::of(TEMPLATE))?;
    ensure_eq!(
        sh(&scratch, &Args::of(&["-n", "t.sh"]))?.succeeded(),
        Answer::Yes,
        "the template parses"
    );
    Ok(())
}

#[test]
fn generated_script_parses_and_has_no_placeholders() {
    assert_eq!(
        check_generated_script_parses_and_has_no_placeholders(),
        Ok(()),
        "generated_script_parses_and_has_no_placeholders"
    );
}

fn check_generated_script_parses_and_has_no_placeholders() -> Result<(), Fail> {
    let text = generate(&installer!("1.2.3"))?;
    ensure!(!text.as_str().contains("@VERSION@"), "version filled");
    ensure!(!text.as_str().contains("@SHA256"), "digests filled");
    ensure!(text.as_str().contains("version='1.2.3'"), "version is set");
    ensure!(
        text.as_str()
            .contains(&format!("sha_linux_x86_64='{}'", "0".repeat(64))),
        "the first target's digest is set"
    );
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("g.sh"), &text)?;
    ensure_eq!(
        sh(&scratch, &Args::of(&["-n", "g.sh"]))?.succeeded(),
        Answer::Yes,
        "the generated script parses"
    );
    Ok(())
}

#[test]
fn missing_hash_or_bad_version_refused() {
    let first_line = sums()
        .as_str()
        .lines()
        .take(1)
        .collect::<Vec<&str>>()
        .join("\n");
    let one_target = Installer {
        sums: Text::from_string(format!("{first_line}\n")),
        ..installer!("1.2.3")
    };
    assert!(
        generate(&one_target).is_err(),
        "a missing target hash is refused"
    );
    assert!(
        generate(&installer!("1.2.3; rm -rf /")).is_err(),
        "a bad version is refused"
    );
    for bad in [
        "1.2.3-",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.3-rc.01",
        "1.2.3-alpha..1",
        "1.2.3-.alpha",
        "1.2.3-alpha.",
        "1.2.3+build",
        "1.2.3-alpha.1+build",
        "1.2.3-al_pha",
        "v1.2.3-alpha.1",
        "1.2-alpha.1",
        "1.2.3-alpha.1\n",
    ] {
        let installer = Installer {
            version: Text::of(bad),
            ..installer!("1.2.3")
        };
        assert!(generate(&installer).is_err(), "{bad:?} is refused");
    }
    assert!(
        parse_sums(&Text::of("zz  unrepresentable-x\n")).is_err(),
        "a bad sums line is refused"
    );
}

#[test]
fn pre_release_versions_are_accepted() {
    assert_eq!(
        check_pre_release_versions_are_accepted(),
        Ok(()),
        "pre_release_versions_are_accepted"
    );
}

fn check_pre_release_versions_are_accepted() -> Result<(), Fail> {
    for good in [
        "0.1.0-alpha.1",
        "1.2.3-alpha.1",
        "1.2.3-rc.0",
        "1.2.3-0",
        "0.0.0",
        "10.20.30",
        "1.2.3-x-y.7",
        "1.2.3-01a",
    ] {
        let text = generate(&Installer {
            version: Text::of(good),
            ..installer!("1.2.3")
        })?;
        ensure!(
            text.as_str().contains(&format!("version='{good}'")),
            "{good:?} is set in the script"
        );
    }
    Ok(())
}

#[test]
fn unfilled_template_refuses_to_run() {
    assert_eq!(
        check_unfilled_template_refuses_to_run(),
        Ok(()),
        "unfilled_template_refuses_to_run"
    );
}

fn check_unfilled_template_refuses_to_run() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("t.sh"), &Text::of(TEMPLATE))?;
    let ran = sh(&scratch, &Args::of(&["t.sh"]))?;
    ensure_eq!(ran.status, Status::Code(Count::new(2)), "exit code 2");
    let stderr = ran.stderr.lossy_text();
    ensure!(
        ["placeholder", "unsupported", "required"]
            .iter()
            .any(|word| stderr.as_str().contains(word)),
        "the refusal is explained: {}",
        stderr.as_str()
    );
    Ok(())
}

#[test]
fn script_uses_no_arithmetic_and_no_env_override() {
    let body: String = TEMPLATE
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<&str>>()
        .join("\n");
    assert!(!body.contains("$(("), "no shell arithmetic");
    assert!(!body.contains("expr "), "no expr");
    assert!(!body.contains("let "), "no let");
    let mut names: Vec<String> = Vec::new();
    for piece in body.split("${").skip(1) {
        let name: String = piece
            .chars()
            .take_while(|found| found.is_ascii_uppercase() || *found == '_')
            .collect();
        if !name.is_empty() && name != "GITHUB_BASE_REF" {
            names.push(name);
        }
    }
    assert_eq!(
        names,
        Vec::<String>::new(),
        "only GITHUB_BASE_REF is read from the environment"
    );
}

#[test]
fn hash_mismatch_is_refused_with_a_fake_download() {
    assert_eq!(
        check_hash_mismatch_is_refused_with_a_fake_download(),
        Ok(()),
        "hash_mismatch_is_refused_with_a_fake_download"
    );
}

fn check_hash_mismatch_is_refused_with_a_fake_download() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let curl = scratch.path().join("bin").join("curl");
    io::write_text(
        &curl,
        &Text::of("#!/bin/sh\nwhile [ $# -gt 0 ]; do if [ \"$1\" = \"--output\" ]; then out=\"$2\"; fi; shift; done\necho tampered > \"$out\"\n"),
    )?;
    process::make_executable(&curl)?;
    scratch.write(&RelPath::of("u.sh"), &generate(&installer!("1.2.3"))?)?;
    let path = format!(
        "{}:/usr/bin:/bin:/usr/local/bin",
        scratch.path().join("bin").to_string_lossy()
    );
    let program = Name::of("sh");
    let ran = process::run(
        &Invocation::new(&program, &Args::of(&["u.sh"]))
            .in_dir(scratch.path())
            .with_env(&EnvPair {
                name: Name::of("PATH"),
                value: Text::from_string(path),
            }),
    )?;
    ensure_eq!(ran.status, Status::Code(Count::new(2)), "exit code 2");
    ensure!(
        ran.combined().as_str().contains("mismatch"),
        "the mismatch is named: {}",
        ran.combined().as_str()
    );
    Ok(())
}
