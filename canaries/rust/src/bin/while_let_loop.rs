//! Canary.
fn main() {
    let mut src: Vec<bool> = Vec::new();
    loop { //~ clippy::while_let_loop
        let Some(item) = src.pop() else {
            break;
        };
        let _v = item.to_string();
    }
}
