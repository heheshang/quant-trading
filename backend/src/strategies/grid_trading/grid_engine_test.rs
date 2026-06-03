//! P1-1 integration tests for the live-trading `GridEngine::run` loop.
//!
//! Covers the three contract requirements from the task body:
//!  1. 价格穿越网格下单 — price crossing a grid level emits a buy/sell order
//!  2. 价格区间内不重复下单 — same-level ticks do not duplicate orders
//!  3. 价格破上下界暂停 — out-of-bounds price exits the run loop with
//!     `GridRunExit::OutOfBounds`
//!
//! The tests are deliberately written against the public async `run()` API
//! (not against the internal `on_price_update` method) so they double as a
//! smoke test for the channel-based order emission contract.

use super::grid_engine::{GridRunExit, PriceTick};
use super::grid_engine::GridEngine;
use super::types::{GridConfig, OrderSide};
use tokio::sync::mpsc;

fn cfg() -> GridConfig {
    GridConfig {
        symbol: "BTCUSDT".into(),
        lower_price: 50_000.0,
        upper_price: 60_000.0,
        grid_count: 10, // spacing = 1000
        quantity_per_grid: 0.01,
        martingale: None,
        dynamic: None,
    }
}

/// Helper: build a tick at `price` with a stable timestamp pair.
fn tick(price: f64) -> PriceTick {
    PriceTick::new(price, 1_700_000_000_000, 1_700_000_000_000)
}

/// Drive `engine.run(...)` with `prices` pushed sequentially, then drop the
/// tick sender to close the channel. Collects every order pushed downstream.
async fn run_with_prices(
    mut engine: GridEngine,
    prices: Vec<f64>,
) -> (Result<GridRunExit, String>, Vec<super::types::GridOrder>) {
    let (tick_tx, tick_rx) = mpsc::channel::<PriceTick>(32);
    let (order_tx, mut order_rx) = mpsc::channel::<super::types::GridOrder>(64);
    for p in prices {
        tick_tx.send(tick(p)).await.expect("send tick");
    }
    drop(tick_tx);
    let run_handle = tokio::spawn(async move { engine.run(tick_rx, order_tx).await });
    let mut orders = Vec::new();
    while let Some(o) = order_rx.recv().await {
        orders.push(o);
    }
    let exit = run_handle.await.expect("engine task joined");
    (exit, orders)
}

#[tokio::test]
async fn price_crossing_grid_emits_order() {
    // Spacing is 1000; 51000 = level 1, 52000 = level 2. Going from 50500 → 52000
    // crosses level 1 (price 51000) and lands on level 2.
    let (exit, orders) = run_with_prices(GridEngine::new(cfg()), vec![50_500.0, 52_000.0]).await;
    assert_eq!(exit, Ok(GridRunExit::InputClosed));
    assert!(
        !orders.is_empty(),
        "expected at least one order from a price crossing the grid"
    );
    // The crossed level is 1 → grid_price = 51000.0, side = BUY (price >= grid_price).
    let crossed = orders
        .iter()
        .find(|o| (o.price - 51_000.0).abs() < 1e-9)
        .expect("crossed-level order at 51000");
    assert_eq!(crossed.side, OrderSide::BUY);
    assert!((crossed.quantity - 0.01).abs() < 1e-9);
}

#[tokio::test]
async fn price_in_range_does_not_duplicate_order() {
    // 50500 sits in level 0; 51000 sits in level 1. The second tick crosses
    // a single grid level → exactly one order at 51000. The two trailing
    // ticks re-enter the same level → no new orders (no duplicates).
    let prices = vec![50_500.0, 51_000.0, 51_000.0, 51_000.0];
    let (exit, orders) = run_with_prices(GridEngine::new(cfg()), prices).await;
    assert_eq!(exit, Ok(GridRunExit::InputClosed));
    let at_51000: Vec<_> = orders
        .iter()
        .filter(|o| (o.price - 51_000.0).abs() < 1e-9)
        .collect();
    assert_eq!(
        at_51000.len(),
        1,
        "expected exactly one order at level 1 (one cross, no duplicates), got {}",
        at_51000.len()
    );
}

#[tokio::test]
async fn price_below_lower_bound_exits_with_out_of_bounds() {
    // First tick is in range (seeds last_level); second tick is below lower.
    let prices = vec![55_000.0, 49_000.0];
    let (exit, orders) = run_with_prices(GridEngine::new(cfg()), prices).await;
    assert_eq!(exit, Ok(GridRunExit::OutOfBounds));
    // No orders should have been emitted for the out-of-bounds tick.
    assert!(
        orders.iter().all(|o| o.price >= 50_000.0 && o.price <= 60_000.0),
        "orders must be within configured bounds, got {:?}",
        orders
    );
}

#[tokio::test]
async fn price_above_upper_bound_exits_with_out_of_bounds() {
    // Symmetric to the lower-bound test: first tick seeds state, second tick
    // is above upper_price.
    let prices = vec![55_000.0, 61_000.0];
    let (exit, _orders) = run_with_prices(GridEngine::new(cfg()), prices).await;
    assert_eq!(exit, Ok(GridRunExit::OutOfBounds));
}

#[tokio::test]
async fn run_returns_input_closed_when_sender_drops() {
    // No ticks at all — closing the sender must cause a clean `InputClosed` exit.
    let (tick_tx, tick_rx) = mpsc::channel::<PriceTick>(4);
    let (order_tx, _order_rx) = mpsc::channel::<super::types::GridOrder>(4);
    drop(tick_tx);
    let mut engine = GridEngine::new(cfg());
    let exit = engine.run(tick_rx, order_tx).await.expect("run ok");
    assert_eq!(exit, GridRunExit::InputClosed);
}
