//! Canary.
/// Canary item.
trait Job {
    /// Canary item.
    async fn run(&self) -> bool;
}
/// Canary item.
struct Holder;
impl Job for Holder {
    async fn run(&self) -> bool { //~ clippy::unused_async_trait_impl
        true
    }
}
fn main() {
    let holder = Holder;
    let _v = holder.run();
}
