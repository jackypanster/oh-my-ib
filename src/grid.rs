//! `grid` — the pure mean-reversion grid planner + config (ADR 0033).
//!
//! Offline and gateway-free: `plan_grid_tick` is a pure reconcile-to-desired-state function
//! (no client, no I/O) — the frozen surface. The gateway DRIVER lives in `src/ib/grid.rs` and
//! composes the existing `trade.rs` choke points; it contains NO raw `place_order`/`cancel_order`
//! (ADR 0017 containment holds).
//!
//! Model: per configured symbol with a cost anchor (`qty > 0`), desire a Sell rung at
//! `round2(avg_cost * (1 + rise%))` and, IFF cash is above the net-liq floor AND
//! `qty + lot <= max_shares`, a Buy rung at `round2(avg_cost * (1 - drop%))`. Existing orders
//! that match (same side, same qty ±1e-6, |Δlimit| <= 0.005) are KEPT; the rest are cancelled
//! and the desired ones (re-)placed. A flat symbol (`qty == 0`) cancels its lingering orders
//! and places nothing. Symbols NOT in the config are never iterated ⇒ invisible (blast-radius
//! guard). Output orders ALL `Cancel`s before ALL `Place`s.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::error::AppError;

/// The side of an order leg. Mirrors `ibapi::orders::Action` for the STK verbs only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// A planned action: cancel an existing order, or place a new LMT STK order.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Cancel { order_id: i32 },
    Place { symbol: String, side: Side, qty: f64, limit: f64 },
}

/// The two account metrics the planner keys off: available cash and total equity.
#[derive(Debug, Clone, Copy)]
pub struct AccountSnap {
    pub total_cash: f64,
    pub net_liquidation: f64,
}

/// A held position's size and cost basis — the grid's anchor.
#[derive(Debug, Clone, Copy)]
pub struct PositionLite {
    pub qty: f64,
    pub avg_cost: f64,
}

/// A resting order the planner may keep or cancel (the trimmed view of an open order).
#[derive(Debug, Clone)]
pub struct OpenOrderLite {
    pub order_id: i32,
    pub symbol: String,
    pub side: Side,
    pub limit: f64,
    pub qty: f64,
}

fn d_lot() -> u32 {
    100
}
fn d_floor() -> f64 {
    50.0
}
fn d_pct() -> f64 {
    2.0
}
fn d_max() -> u32 {
    300
}

/// The grid config (TOML). `[[symbol]]` tables → `symbols`. Absent fields take documented
/// defaults via serde; `validate()` enforces the cross-field constraints AFTER parse.
#[derive(Debug, Clone, Deserialize)]
pub struct GridConfig {
    #[serde(default = "d_lot")]
    pub lot: u32,
    #[serde(default = "d_floor")]
    pub cash_floor_pct: f64,
    #[serde(rename = "symbol", default)]
    pub symbols: Vec<SymbolCfg>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SymbolCfg {
    pub name: String,
    #[serde(default = "d_pct")]
    pub drop_pct: f64,
    #[serde(default = "d_pct")]
    pub rise_pct: f64,
    #[serde(default = "d_max")]
    pub max_shares: u32,
}

impl GridConfig {
    /// Parse from a TOML string, apply serde defaults, then validate. FROZEN: the test calls
    /// this directly (no filesystem). Both parse errors and validation failures map to
    /// `AppError::config` (exit 5).
    pub fn from_toml_str(s: &str) -> Result<GridConfig, AppError> {
        let cfg: GridConfig = toml::from_str(s)
            .map_err(|e| AppError::config(format!("invalid grid config: {e}"), "grid-config"))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Read a TOML config file then `from_toml_str`. Driver-only (not frozen).
    pub fn load(path: &std::path::Path) -> Result<GridConfig, AppError> {
        let s = std::fs::read_to_string(path).map_err(|e| {
            AppError::config(
                format!("cannot read grid config {}: {e}", path.display()),
                "grid-config",
            )
        })?;
        Self::from_toml_str(&s)
    }

    /// Cross-field validation (config bucket, exit 5). `max_shares < lot` is legal = sell-only.
    fn validate(&self) -> Result<(), AppError> {
        if self.symbols.is_empty() {
            return Err(AppError::config(
                "grid config must define at least one [[symbol]]",
                "grid-config",
            ));
        }
        if self.lot < 1 {
            return Err(AppError::config(
                format!("grid lot must be >= 1, got {}", self.lot),
                "grid-config",
            ));
        }
        if !(0.0..=100.0).contains(&self.cash_floor_pct) {
            return Err(AppError::config(
                format!(
                    "cash_floor_pct must be in [0,100], got {}",
                    self.cash_floor_pct
                ),
                "grid-config",
            ));
        }
        for s in &self.symbols {
            if s.name.trim().is_empty() {
                return Err(AppError::config(
                    "each [[symbol]] must have a non-empty name",
                    "grid-config",
                ));
            }
            if !(0.0..100.0).contains(&s.drop_pct) {
                return Err(AppError::config(
                    format!("{} drop_pct must be in (0,100), got {}", s.name, s.drop_pct),
                    "grid-config",
                ));
            }
            if !(0.0..100.0).contains(&s.rise_pct) {
                return Err(AppError::config(
                    format!("{} rise_pct must be in (0,100), got {}", s.name, s.rise_pct),
                    "grid-config",
                ));
            }
        }
        Ok(())
    }
}

/// Round to 2 decimal places (cents) — the IB price tick for the grid rungs.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// The PURE reconcile-to-desired-state planner (ADR 0033 D-PLANNER). No client, no I/O —
/// deterministic from its inputs, which is why the frozen spec can pin it offline.
///
/// Returns the planned actions with ALL `Cancel`s ordered before ALL `Place`s (global, not
/// per-symbol), so a driver draining cancels-first never races a replacement into the same slot.
pub fn plan_grid_tick(
    cfg: &GridConfig,
    acct: &AccountSnap,
    positions: &HashMap<String, PositionLite>,
    open: &[OpenOrderLite],
) -> Vec<Action> {
    let mut cancels: Vec<Action> = Vec::new();
    let mut places: Vec<Action> = Vec::new();

    for s in &cfg.symbols {
        let existing: Vec<&OpenOrderLite> = open.iter().filter(|o| o.symbol == s.name).collect();
        let mut desired: Vec<(Side, f64, f64)> = Vec::new();

        // A held position (qty > 0) anchors the rungs; a flat/absent symbol desires nothing.
        if let Some(pos) = positions.get(&s.name).filter(|p| p.qty > 0.0) {
            let avg = pos.avg_cost;
            desired.push((
                Side::Sell,
                pos.qty.min(cfg.lot as f64),
                round2(avg * (1.0 + s.rise_pct / 100.0)),
            ));
            // Buy only above the cash floor AND under the strict max_shares ceiling.
            let floor = (cfg.cash_floor_pct / 100.0) * acct.net_liquidation;
            if acct.total_cash >= floor && pos.qty + cfg.lot as f64 <= s.max_shares as f64 {
                desired.push((Side::Buy, cfg.lot as f64, round2(avg * (1.0 - s.drop_pct / 100.0))));
            }
        }

        // Reconcile: keep an existing order iff same side & qty (±1e-6) & |Δlimit| <= 0.005;
        // otherwise the desired order is (re-)placed. Each existing order can absorb at most
        // one desired (the `kept` set prevents double-matching).
        let mut kept: HashSet<i32> = HashSet::new();
        for (side, qty, limit) in &desired {
            let matched = existing.iter().find(|o| {
                o.side == *side
                    && (o.qty - qty).abs() < 1e-6
                    && (o.limit - limit).abs() <= 0.005
                    && !kept.contains(&o.order_id)
            });
            match matched {
                Some(o) => {
                    kept.insert(o.order_id);
                }
                None => places.push(Action::Place {
                    symbol: s.name.clone(),
                    side: *side,
                    qty: *qty,
                    limit: *limit,
                }),
            }
        }
        for o in &existing {
            if !kept.contains(&o.order_id) {
                cancels.push(Action::Cancel { order_id: o.order_id });
            }
        }
    }

    cancels.into_iter().chain(places).collect()
}
