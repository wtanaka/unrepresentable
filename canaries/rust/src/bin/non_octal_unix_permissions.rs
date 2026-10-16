//! Canary.
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt; //~ clippy::unused_trait_names
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink(Permissions::from_mode(644)); //~ clippy::non_octal_unix_permissions
}
