use super::{map_approval, parse_approvals, run, SelfLint};
use crate::exit::ExitStatus;
use crate::io;
use crate::process;
use crate::reason::{Code, Commits};
use crate::testkit::{Fail, Scratch};
use crate::types::{Answer, Message, RelPath, Text};
use std::path::Path;

macro_rules! code_of {
    ($line:literal) => {
        map_approval(&Text::of($line)).code
    };
}

/// A stand-in for the base binary. It exits 2 unless it was run behind the
/// prefix with a build directory of its own, on a read-only repository tree
/// with the base config (first run) or against the base (second run).
const CHECKING_BINARY: &str = "#!/bin/sh\n\
    test -n \"$CARGO_TARGET_DIR\" || exit 2\n\
    touch \"$CARGO_TARGET_DIR/probe\" || exit 2\n\
    test -z \"$GH_TOKEN\" || exit 2\n\
    test \"$1\" = check || exit 2\n\
    case \"$2\" in\n\
      --range)\n\
        test \"$3\" = HEAD || exit 2\n\
        test \"$(git -C \"$4\" rev-list --count HEAD)\" = 1 || exit 2\n\
        grep -q base-config \"$4/.unrepresentable/config\" || exit 2\n\
        ! touch \"$4/probe\" 2>/dev/null || exit 2\n\
        ;;\n\
      --against)\n\
        test \"$3\" = bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb || exit 2\n\
        ! touch \"$4/probe\" 2>/dev/null || exit 2\n\
        ;;\n\
      *) exit 2 ;;\n\
    esac\n\
    exit 0\n";

#[test]
fn approval_lines_map_to_reason_codes() {
    assert_eq!(
        code_of!("baseline grew for x"),
        Code::BaselineGrew,
        "baseline"
    );
    assert_eq!(
        code_of!("a ledger row was added"),
        Code::LedgerRowAdded,
        "ledger"
    );
    assert_eq!(code_of!("config loosened"), Code::ConfigLoosened, "config");
    assert_eq!(code_of!("something else"), Code::ToolApproval, "default");
}

#[test]
fn only_approval_lines_are_read() {
    let output =
        Text::of("noise\napproval: LEDGER_ROW_ADDED x\nsrc/a.rs:1:1: r: m\napproval: other\n");
    assert_eq!(parse_approvals(&output).len(), 2, "two approval lines");
}

#[test]
fn the_base_binary_runs_behind_the_prefix_on_read_only_trees() {
    assert_eq!(
        check_the_base_binary_runs_behind_the_prefix_on_read_only_trees(),
        Ok(()),
        "the_base_binary_runs_behind_the_prefix_on_read_only_trees"
    );
}

fn check_the_base_binary_runs_behind_the_prefix_on_read_only_trees() -> Result<(), Fail> {
    let head = Scratch::repository()?;
    head.write(&RelPath::of("src/lib.rs"), &Text::of("pub fn a() {}\n"))?;
    head.write(
        &RelPath::of(".unrepresentable/config"),
        &Text::of("head-config\n"),
    )?;
    head.commit_all(&Message::of("head"))?;
    let base = Scratch::repository()?;
    base.write(
        &RelPath::of(".unrepresentable/config"),
        &Text::of("base-config\n"),
    )?;
    let binary = base.path().join("fake-binary");
    io::write_text(&binary, &Text::of(CHECKING_BINARY))?;
    process::make_executable(&binary)?;
    let out = base.path().join("out.txt");
    let report = run(&SelfLint {
        base_dir: base.path().to_path_buf(),
        binary,
        commits: Commits {
            base: sha!('b'),
            head: head.head()?,
        },
        head_dir: head.path().to_path_buf(),
        out,
        prefix: vec![
            Text::of("env"),
            Text::of("-i"),
            Text::of("PATH=/usr/bin:/bin"),
        ],
    })?;
    process::make_writable(head.path())?;
    ensure_eq!(
        (report.status, report.lines),
        (ExitStatus::Clean, Vec::<Text>::new()),
        "both runs saw a build directory, no token and read-only trees"
    );
    ensure_eq!(
        io::exists_no_follow(&head.path().join("probe")),
        Answer::No,
        "the head checkout was not written to"
    );
    Ok(())
}

/// A head whose tool directory or tool file is a link, and the file it leads to.
fn linked_head(link: &RelPath, victim: &Path) -> Result<Scratch, Fail> {
    let head = Scratch::repository()?;
    head.write(&RelPath::of("src/lib.rs"), &Text::of("pub fn a() {}\n"))?;
    head.symlink(
        &Text::from_string(victim.to_string_lossy().into_owned()),
        link,
    )?;
    head.commit_all(&Message::of("head"))?;
    Ok(head)
}

#[test]
fn a_link_where_a_tool_file_goes_is_refused_and_nothing_is_written_through_it() {
    assert_eq!(
        check_a_link_where_a_tool_file_goes_is_refused_and_nothing_is_written_through_it(),
        Ok(()),
        "a_link_where_a_tool_file_goes_is_refused_and_nothing_is_written_through_it"
    );
}

fn check_a_link_where_a_tool_file_goes_is_refused_and_nothing_is_written_through_it(
) -> Result<(), Fail> {
    let base = Scratch::repository()?;
    for name in [
        ".unrepresentable/config",
        ".unrepresentable/ledger.md",
        ".unrepresentable/canary-exemptions.md",
    ] {
        base.write(&RelPath::of(name), &Text::of("base-config\n"))?;
    }
    let victims = Scratch::repository()?;
    victims.write(&RelPath::of("gitconfig"), &Text::of("runner file\n"))?;
    victims.write(&RelPath::of("dir/keep"), &Text::of("x\n"))?;
    for (link, victim) in [
        (".unrepresentable", victims.path().join("dir")),
        (".unrepresentable/config", victims.path().join("gitconfig")),
    ] {
        let head = if link == ".unrepresentable" {
            linked_head(&RelPath::of(link), &victim)?
        } else {
            let made = Scratch::repository()?;
            made.write(&RelPath::of(".unrepresentable/ledger.md"), &Text::of("h\n"))?;
            made.symlink(
                &Text::from_string(victim.to_string_lossy().into_owned()),
                &RelPath::of(link),
            )?;
            made.commit_all(&Message::of("head"))?;
            made
        };
        let result = run(&SelfLint {
            base_dir: base.path().to_path_buf(),
            binary: base.path().join("never-run"),
            commits: Commits {
                base: sha!('b'),
                head: head.head()?,
            },
            head_dir: head.path().to_path_buf(),
            out: base.path().join("out.txt"),
            prefix: Vec::new(),
        });
        let refusal = match result {
            Err(failure) => failure.text(),
            Ok(_) => Message::of("it ran"),
        };
        ensure!(
            refusal.as_str().contains("has a link on the way to"),
            "a link at {link} makes the evaluation refuse, not write through it: {}",
            refusal.as_str()
        );
        ensure_eq!(
            io::read_text(&victims.path().join("gitconfig"))?,
            Text::of("runner file\n"),
            "the file the link leads to is untouched ({link})"
        );
        ensure_eq!(
            io::exists_no_follow(&victims.path().join("dir/ledger.md")),
            Answer::No,
            "nothing was written into the directory the link leads to ({link})"
        );
    }
    Ok(())
}
