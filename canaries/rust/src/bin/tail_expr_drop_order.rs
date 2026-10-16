//! Canary.
use core::hint::black_box;

/// Canary item.
struct Droppy(i32);

impl Droppy {
    /// Canary item.
    const fn get(&self) -> i32 {
        self.0
    }
}

impl Drop for Droppy {
    fn drop(&mut self) {
        let _: i32 = black_box(self.0);
    }
}

/// Canary item.
fn edition_2021() -> i32 {
    let _another = Droppy(0);
    Droppy(1).get() //~ tail_expr_drop_order
}

fn main() {
    let _: i32 = black_box(edition_2021());
}
