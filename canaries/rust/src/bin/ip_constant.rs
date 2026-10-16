//! Canary.
use core::net::Ipv4Addr;

fn main() {
    let addr = Ipv4Addr::new(127, 0, 0, 1); //~ clippy::ip_constant
    assert!(addr.is_loopback(), "loopback");
}
