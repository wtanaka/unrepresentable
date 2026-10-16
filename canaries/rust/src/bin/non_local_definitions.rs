//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
struct Foo;

#[rustfmt::skip]
fn main() {
    impl Foo { //~ non_local_definitions
        const fn function() {}
    }
    let _: Foo = black_box(Foo);
    Foo::function();
}
