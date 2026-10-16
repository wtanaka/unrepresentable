//! Canary.
use core::mem::ManuallyDrop;

fn main() {
    drop(ManuallyDrop::new(String::new())); //~ undropped_manually_drops
}
