//! Canary.
fn main() {
    cfg_select! {
        _ => {}
        unix => {} //~ unreachable_cfg_select_predicates
    }
}
