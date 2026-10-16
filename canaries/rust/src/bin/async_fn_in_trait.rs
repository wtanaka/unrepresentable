//! Canary.
#[rustfmt::skip]
/// Canary item.
pub trait Tr { //~ dead_code_pub_in_binary
    /// Canary item.
    async fn run(); //~ async_fn_in_trait
}

fn main() {}
