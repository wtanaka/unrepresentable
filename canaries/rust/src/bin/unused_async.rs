//! Canary.
/// Canary item.
async fn work() { //~ clippy::unused_async
}
fn main() {
    let _v = work();
}
