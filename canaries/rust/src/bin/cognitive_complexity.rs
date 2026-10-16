//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn complex(num: usize) -> Vec<u8> { //~ clippy::cognitive_complexity clippy::too_many_lines
    let mut vec: Vec<u8> = Vec::new();
    if num == 1 {
        vec.push(1);
    }
    if num == 2 {
        vec.push(2);
    }
    if num == 3 {
        vec.push(3);
    }
    if num == 4 {
        vec.push(4);
    }
    if num == 5 {
        vec.push(5);
    }
    if num == 6 {
        vec.push(6);
    }
    if num == 7 {
        vec.push(7);
    }
    if num == 8 {
        vec.push(8);
    }
    if num == 9 {
        vec.push(9);
    }
    if num == 10 {
        vec.push(10);
    }
    if num == 11 {
        vec.push(11);
    }
    if num == 12 {
        vec.push(12);
    }
    if num == 13 {
        vec.push(13);
    }
    if num == 14 {
        vec.push(14);
    }
    if num == 15 {
        vec.push(15);
    }
    if num == 16 {
        vec.push(16);
    }
    if num == 17 {
        vec.push(17);
    }
    if num == 18 {
        vec.push(18);
    }
    if num == 19 {
        vec.push(19);
    }
    if num == 20 {
        vec.push(20);
    }
    if num == 21 {
        vec.push(21);
    }
    if num == 22 {
        vec.push(22);
    }
    if num == 23 {
        vec.push(23);
    }
    if num == 24 {
        vec.push(24);
    }
    if num == 25 {
        vec.push(25);
    }
    if num == 26 {
        vec.push(26);
    }
    if num == 27 {
        vec.push(27);
    }
    if num == 28 {
        vec.push(28);
    }
    if num == 29 {
        vec.push(29);
    }
    if num == 30 {
        vec.push(30);
    }
    vec
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(complex(args().count()).is_empty()));
}
