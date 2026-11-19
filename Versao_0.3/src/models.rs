//! Estruturas de dados fortemente tipadas para comunicação JSON entre Rust e Python.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// 1. MODELOS DE AÇÕES E VISÃO GERAL DE MERCADO (DAX1, DAX2, DAX3, DAX4)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TopBarTicker {
    pub code: String,
    pub subtitle: String,
    pub value: String,
    pub change_pct: f64,
    pub last_date: String,
    pub sparkline: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MoverItem {
    pub code: String,
    pub change_pct: f64,
    pub price: f64,
    pub last_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AssetCardData {
    pub ticker: String,
    pub short_code: String,
    pub name: String,
    pub sector: String,
    pub country: String,
    pub currency: String,
    pub price: f64,
    pub change_day_pct: f64,
    pub change_12m_pct: f64,
    pub market_cap_bi: f64,
    pub volume: u64,
    pub pe: f64,
    pub pb: f64,
    pub dy: f64,
    pub roe: f64,
    pub eps: f64,
    pub last_date: String,
    pub sparkline: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MarketOverviewResponse {
    pub status: String,
    pub source_status: String,
    pub updated_at: String,
    pub top_bar: Vec<TopBarTicker>,
    pub gainers: Vec<MoverItem>,
    pub losers: Vec<MoverItem>,
    pub assets: Vec<AssetCardData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StockMetrics {
    pub current_price: f64,
    pub change_day_pct: f64,
    pub min_52w: f64,
    pub max_52w: f64,
    pub min_month: f64,
    pub max_month: f64,
    pub dividend_yield_pct: f64,
    pub dividends_12m: f64,
    pub change_12m_pct: f64,
    pub change_month_pct: f64,
    pub market_cap_bi: f64,
    pub volume_last: u64,
    pub pe_ratio: f64,
    pub pb_ratio: f64,
    pub roe_pct: f64,
    pub eps: f64,
    pub volatility_annual_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CandlePoint {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub sma20: f64,
    pub sma50: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StockDetailResponse {
    pub status: String,
    pub source_status: String,
    pub ticker: String,
    pub short_code: String,
    pub company_name: String,
    pub sector: String,
    pub country: String,
    pub native_currency: String,
    pub display_currency: String,
    pub fx_factor: f64,
    pub period: String,
    pub updated_at: String,
    pub metrics: StockMetrics,
    pub candles: Vec<CandlePoint>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. MODELOS DE MOEDAS E ÍNDICE DÓLAR (DXY)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ForexHistoryPoint {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub base100: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ForexPairData {
    pub ticker: String,
    pub display_code: String,
    pub name: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub country: String,
    pub current_rate: f64,
    pub change_day_pct: f64,
    pub change_period_pct: f64,
    pub high_period: f64,
    pub low_period: f64,
    pub volatility_annual_pct: f64,
    pub history: Vec<ForexHistoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ForexResponse {
    pub status: String,
    pub source_status: String,
    pub period: String,
    pub updated_at: String,
    pub usd_rates: HashMap<String, f64>,
    pub pairs: Vec<ForexPairData>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. MODELOS DE TAXAS DE JUROS GLOBAIS (FRED / BCB / BCE / BOE / BOJ)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InterestHistoryPoint {
    pub date: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CountryInterestData {
    pub country_code: String,
    pub country_name: String,
    pub central_bank: String,
    pub rate_name: String,
    pub series_code: String,
    pub fred_id: String,
    pub currency: String,
    pub current_rate_pct: f64,
    pub previous_rate_pct: f64,
    pub change_bps: f64,
    pub inflation_cpi_pct: f64,
    pub real_interest_pct: f64,
    pub history: Vec<InterestHistoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InterestResponse {
    pub status: String,
    pub source_status: String,
    pub cache_month: String,
    pub updated_at: String,
    pub fred_key_configured: bool,
    pub countries: Vec<CountryInterestData>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. MODELOS SOCIOECONÔMICOS DO BANCO MUNDIAL (WORLD BANK API)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct IndicatorMeta {
    pub key: String,
    pub wb_code: String,
    pub name: String,
    pub unit: String,
    pub category: String,
    pub update_frequency: String,
    pub description: String,
    pub market_impact: String,
    pub source_name: String,
    pub reference_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct YearValuePoint {
    pub year: i32,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CountryMacroProfile {
    pub iso3: String,
    pub country_name: String,
    pub latest: HashMap<String, f64>,
    pub series: HashMap<String, Vec<YearValuePoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MacroResponse {
    pub status: String,
    pub source_status: String,
    pub cache_month: String,
    pub start_year: i32,
    pub end_year: i32,
    pub updated_at: String,
    pub indicators_catalog: Vec<IndicatorMeta>,
    pub countries: Vec<CountryMacroProfile>,
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. RESPOSTA DE EXPORTAÇÃO DE GRÁFICOS (PLOTLY / MATPLOTLIB)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ChartExportResponse {
    pub status: String,
    pub format: String,
    pub file_path: String,
    pub message: String,
}

