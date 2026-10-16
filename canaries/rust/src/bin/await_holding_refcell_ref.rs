//! Canary.
use core::cell::RefCell;
use core::future::ready;

/// Canary item.
async fn other() {
    ready(()).await;
}

fn main() {
    let cell: RefCell<u8> = RefCell::new(1);
    let fut = async {
        let guard = cell.borrow(); //~ clippy::await_holding_refcell_ref
        other().await;
        drop(guard);
    };
    drop(fut);
}
