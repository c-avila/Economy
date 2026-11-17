//! Gerenciamento de persistência das configurações do usuário em JSON (`config/user_settings.json`) — DAX v0.2.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Catálogo estático que vincula cada Par Cambial da Aba (b) ao País de Juros (Aba c) e ao País World Bank (Aba d).
pub const CATALOGO_MOEDAS_PAISES: &[(&str, &str, &str, &str)] = &[
    // (Yahoo Ticker, Friendly Label, Central Bank Rate Code, World Bank ISO-3 Code)
    ("USDBRL=X", "BRL — Brazilian Real (Brazil)", "BR", "BRA"),
    ("EURUSD=X", "EUR — Euro (Eurozone / Germany)", "EU", "DEU"),
    ("GBPUSD=X", "GBP — British Pound (United Kingdom)", "GB", "GBR"),
    ("USDJPY=X", "JPY — Japanese Yen (Japan)", "JP", "JPN"),
    ("USDCNY=X", "CNY — Chinese Yuan (China)", "CN", "CHN"),
    ("USDCHF=X", "CHF — Swiss Franc (Switzerland)", "CH", "CHE"),
    ("USDCAD=X", "CAD — Canadian Dollar (Canada)", "CA", "CAN"),
    ("AUDUSD=X", "AUD — Australian Dollar (Australia)", "AU", "AUS"),
    ("USDMXN=X", "MXN — Mexican Peso (Mexico)", "MX", "MEX"),
    ("USDARS=X", "ARS — Argentine Peso (Argentina)", "AR", "ARG"),
    ("USDINR=X", "INR — Indian Rupee (India)", "IN", "IND"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    pub favorite_tickers: Vec<String>,
    pub selected_ticker: String,
    pub selected_period: String,
    pub display_currency: String,
    pub selected_forex_pairs: Vec<String>,
    pub forex_spotlight_three: [String; 3],
    pub disabled_forex_chart_pairs: Vec<String>,
    pub selected_interest_countries: Vec<String>,
    pub selected_macro_countries: Vec<String>,
    pub macro_start_year: i32,
    pub macro_end_year: i32,
    pub use_cache: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        let mut cfg = Self {
            favorite_tickers: vec![
                "PETR4".into(),
                "VALE3".into(),
                "ITUB4".into(),
                "BBDC4".into(),
                "BBAS3".into(),
                "AAPL".into(),
                "NVDA".into(),
                "MSFT".into(),
            ],
            selected_ticker: "PETR4".into(),
            selected_period: "1y".into(),
            display_currency: "ORIGINAL".into(),
            selected_forex_pairs: vec![
                "DX-Y.NYB".into(),
                "USDBRL=X".into(),
                "EURUSD=X".into(),
                "GBPUSD=X".into(),
                "USDJPY=X".into(),
                "USDCNY=X".into(),
            ],
            forex_spotlight_three: [
                "USDBRL=X".into(),
                "EURUSD=X".into(),
                "USDJPY=X".into(),
            ],
            disabled_forex_chart_pairs: Vec::new(),
            selected_interest_countries: Vec::new(),
            selected_macro_countries: Vec::new(),
            macro_start_year: 2018,
            macro_end_year: 2024,
            use_cache: true,
        };
        cfg.sincronizar_paises_com_moedas();
        cfg
    }
}

impl UserSettings {
    /// Returns the path to the local configuration file.
    /// On macOS when launched from a `.app` bundle, stores settings in
    /// `~/Library/Application Support/DAX_Economy/config/user_settings.json`.
    pub fn caminho_arquivo() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            if let Ok(exe) = std::env::current_exe() {
                let exe_str = exe.to_string_lossy();
                if exe_str.contains(".app/Contents/MacOS") {
                    if let Ok(home) = std::env::var("HOME") {
                        return PathBuf::from(home)
                            .join("Library")
                            .join("Application Support")
                            .join("DAX_Economy")
                            .join("config")
                            .join("user_settings.json");
                    }
                }
            }
        }
        PathBuf::from("config").join("user_settings.json")
    }

    /// Sincroniza automaticamente os países das Abas (c) Taxas de Juros e (d) Socioeconomia (WB)
    /// para refletirem exatamente os países das moedas selecionadas na Aba (b) Moedas & Dólar (DXY).
    pub fn sincronizar_paises_com_moedas(&mut self) {
        let mut juros_paises = vec!["US".to_string()];
        let mut macro_paises = vec!["USA".to_string()];

        for par in &self.selected_forex_pairs {
            for &(ticker_cat, _nome, cod_juros, cod_wb) in CATALOGO_MOEDAS_PAISES {
                if par.eq_ignore_ascii_case(ticker_cat) {
                    if !juros_paises.iter().any(|c| c == cod_juros) {
                        juros_paises.push(cod_juros.to_string());
                    }
                    if !macro_paises.iter().any(|c| c == cod_wb) {
                        macro_paises.push(cod_wb.to_string());
                    }
                }
            }
        }

        self.selected_interest_countries = juros_paises;
        self.selected_macro_countries = macro_paises;

        // Garante que as 3 moedas em destaque pertençam às moedas selecionadas (ou "" para Nenhum)
        let disponiveis: Vec<String> = self
            .selected_forex_pairs
            .iter()
            .filter(|p| !p.eq_ignore_ascii_case("DX-Y.NYB"))
            .cloned()
            .collect();

        for i in 0..3 {
            let atual = &self.forex_spotlight_three[i];
            if !atual.is_empty() && !disponiveis.iter().any(|d| d.eq_ignore_ascii_case(atual)) {
                self.forex_spotlight_three[i] = disponiveis.get(i).cloned().unwrap_or_default();
            }
        }
    }

    /// Conta quantas moedas (excluindo o índice DXY) estão cadastradas na carteira cambial (limite: 10).
    pub fn total_moedas_sem_dxy(&self) -> usize {
        self.selected_forex_pairs
            .iter()
            .filter(|p| !p.eq_ignore_ascii_case("DX-Y.NYB"))
            .count()
    }

    /// Carrega as configurações do disco ou retorna o padrão caso o arquivo não exista.
    pub fn carregar() -> Self {
        let path = Self::caminho_arquivo();
        if let Ok(conteudo) = fs::read_to_string(&path) {
            if let Ok(mut cfg) = serde_json::from_str::<UserSettings>(&conteudo) {
                if !cfg
                    .selected_forex_pairs
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case("DX-Y.NYB"))
                {
                    cfg.selected_forex_pairs.insert(0, "DX-Y.NYB".to_string());
                }
                cfg.sincronizar_paises_com_moedas();
                return cfg;
            }
        }
        let padrao = Self::default();
        let _ = padrao.salvar();
        padrao
    }

    /// Salva as configurações atuais em `config/user_settings.json`.
    pub fn salvar(&self) -> Result<(), String> {
        let path = Self::caminho_arquivo();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {e}"))?;
        fs::write(&path, json_str).map_err(|e| format!("Failed to write settings: {e}"))
    }
}
