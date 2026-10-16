//! Canary.
#[track_caller] //~ ungated_async_fn_track_caller
/// Canary item.
async fn task() {} //~ clippy::unused_async

fn main() {
    let _future = task();
}
