use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::models::{
    Asset, PaperFillLine, PaperFillSide, PortfolioSnapshot, Position, WalletBalance,
};

/// Default paper wallet cash before any recorded fills (typical backtest starting capital).
pub fn paper_initial_quote_balance() -> Decimal {
    Decimal::from(1_000u32)
}

pub fn paper_snapshot(quote: &Asset) -> PortfolioSnapshot {
    paper_snapshot_with_fills(quote, &[])
}

pub fn paper_snapshot_with_fills(quote: &Asset, fills: &[PaperFillLine]) -> PortfolioSnapshot {
    let mut quote_net = Decimal::ZERO;
    for fill in fills {
        if !symbol_uses_quote(&fill.symbol, quote) {
            continue;
        }
        let amount = finite_quote_amount(fill.quote_amount);
        match fill.side {
            PaperFillSide::Buy => quote_net -= amount,
            PaperFillSide::Sell => quote_net += amount,
        }
    }
    let available = paper_initial_quote_balance() + quote_net;
    let positions = paper_positions_from_fills(quote, fills);
    let as_of_ms = if fills.is_empty() {
        0
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    };
    PortfolioSnapshot {
        balances: vec![WalletBalance {
            asset: quote.clone(),
            available,
            locked: Decimal::ZERO,
        }],
        positions,
        realized_pnl_by_quote: BTreeMap::new(),
        as_of_ms,
    }
}

fn symbol_uses_quote(symbol: &str, quote: &Asset) -> bool {
    let normalized = symbol.trim().to_ascii_uppercase();
    let quote_code = quote.as_str();
    normalized
        .split_once('/')
        .map(|(_, q)| q == quote_code)
        .unwrap_or(false)
}

fn finite_quote_amount(value: f64) -> Decimal {
    if !value.is_finite() || value <= 0.0 {
        return Decimal::ZERO;
    }
    Decimal::try_from(value).unwrap_or(Decimal::ZERO)
}

fn paper_positions_from_fills(quote: &Asset, fills: &[PaperFillLine]) -> Vec<Position> {
    let mut by_symbol: BTreeMap<String, (Asset, Asset, Decimal, Decimal)> = BTreeMap::new();
    for fill in fills {
        if !symbol_uses_quote(&fill.symbol, quote) {
            continue;
        }
        let unit_price = match fill.fill_unit_price {
            Some(p) if p.is_finite() && p > 0.0 => p,
            _ => continue,
        };
        let (base_code, quote_code) = match split_spot_symbol(&fill.symbol) {
            Some(parts) => parts,
            None => continue,
        };
        if quote_code != quote.as_str() {
            continue;
        }
        let base_asset = match Asset::new(base_code) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let quote_asset = match Asset::new(quote_code) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let base_delta = fill.quote_amount / unit_price;
        if !base_delta.is_finite() || base_delta <= 0.0 {
            continue;
        }
        let base_delta = Decimal::try_from(base_delta).unwrap_or(Decimal::ZERO);
        if base_delta <= Decimal::ZERO {
            continue;
        }
        let entry = by_symbol
            .entry(fill.symbol.clone())
            .or_insert_with(|| (base_asset, quote_asset, Decimal::ZERO, Decimal::ZERO));
        match fill.side {
            PaperFillSide::Buy => {
                let total_cost = entry.2 * entry.3
                    + base_delta * Decimal::try_from(unit_price).unwrap_or(Decimal::ZERO);
                entry.2 += base_delta;
                entry.3 = if entry.2 > Decimal::ZERO {
                    total_cost / entry.2
                } else {
                    Decimal::ZERO
                };
            }
            PaperFillSide::Sell => {
                entry.2 -= base_delta;
                if entry.2 <= Decimal::ZERO {
                    entry.2 = Decimal::ZERO;
                    entry.3 = Decimal::ZERO;
                }
            }
        }
    }
    by_symbol
        .into_iter()
        .filter(|(_, (_, _, qty, _))| *qty > Decimal::ZERO)
        .map(
            |(symbol, (base_asset, quote_asset, quantity, average_entry_price))| Position {
                symbol,
                base_asset,
                quote_asset,
                quantity,
                average_entry_price,
            },
        )
        .collect()
}

fn split_spot_symbol(symbol: &str) -> Option<(String, String)> {
    let normalized = symbol.trim().to_ascii_uppercase();
    normalized
        .split_once('/')
        .map(|(base, quote)| (base.to_string(), quote.to_string()))
}
