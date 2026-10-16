//! `self-test`: the binary proves its own rules against built-in canaries.

use crate::support::{Exit, Fail, Message, Name, Sandbox};

#[test]
fn self_test_exits_zero() {
    assert_eq!(check_self_test_exits_zero(), Ok(()), "self_test_exits_zero");
}

fn check_self_test_exits_zero() -> Result<(), Fail> {
    let sandbox = Sandbox::create(&Name::of("selftest"))?;
    let outcome = sandbox.self_test()?;
    ensure_eq!(
        outcome.findings(),
        Vec::<Message>::new(),
        "every built-in canary fires where it is marked; stderr: {:?}",
        outcome.stderr()
    );
    ensure_eq!(outcome.exit, Exit::Clean, "self-test exits 0");
    Ok(())
}
