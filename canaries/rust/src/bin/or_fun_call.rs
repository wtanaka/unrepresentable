//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<String> = None;
    sink(opt.unwrap_or(String::from("a"))); //~ clippy::or_fun_call clippy::unnecessary_literal_unwrap
}
