//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let res: Result<u8, u8> = Ok(1);
    sink(res.ok().expect("msg")); //~ clippy::ok_expect clippy::expect_used
}
