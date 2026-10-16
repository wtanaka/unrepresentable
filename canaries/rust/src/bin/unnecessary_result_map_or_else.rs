//! Canary.
fn main() {
    let src: Result<bool, bool> = Ok(true);
    let _v: bool = src.map_or_else(|err| err, |item| item); //~ clippy::unnecessary_result_map_or_else
}
