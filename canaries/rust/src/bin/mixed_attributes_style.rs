//! Canary.
#[cfg(unix)] //~ clippy::mixed_attributes_style
/// Canary item.
const fn func() {
    #![cfg(unix)]
}

fn main() {
    #[cfg(unix)]
    func();
}
