//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Point { //~ clippy::struct_field_names
    /// Canary item.
    point_x: u8, //~ clippy::struct_field_names
    /// Canary item.
    point_y: u8, //~ clippy::struct_field_names
    /// Canary item.
    point_z: u8, //~ clippy::struct_field_names
}

#[rustfmt::skip]
fn main() {
    let point = Point {
        point_x: 1,
        point_y: 2,
        point_z: 3,
    };
    assert!(
        point.point_x < point.point_y && point.point_y < point.point_z,
        "ordered"
    );
}
