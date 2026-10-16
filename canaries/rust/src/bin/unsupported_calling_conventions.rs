//! Canary.
/// Canary item.
const extern "cdecl" fn function() {} //~ unsupported_calling_conventions

fn main() {
    function();
}
