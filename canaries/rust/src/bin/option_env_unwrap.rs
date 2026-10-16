//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink(option_env!("SOME_ENV_VAR_X").unwrap()); //~ clippy::option_env_unwrap clippy::unwrap_used
}
