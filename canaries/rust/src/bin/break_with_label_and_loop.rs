//! Canary.
use std::env;

#[rustfmt::skip]
fn main() {
    let flag: bool = env::args().count() > 1;
    'outer: loop {
        if flag {
            break 'outer loop { //~ break_with_label_and_loop
                if flag {
                    break;
                }
            };
        }
    }
}
