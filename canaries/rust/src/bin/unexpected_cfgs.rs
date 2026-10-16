//! Canary.
use core::hint::black_box;

fn main() {
    let _: bool = black_box(cfg!(my_unknown_cfg)); //~ unexpected_cfgs
}
