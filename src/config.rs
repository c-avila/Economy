//! Gerenciamento de persistência das configurações do usuário em JSON (`config/user_settings.json`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    pub favorite_tickers: Vec<String>,
    pub selected_ticker: String,
    pub selected_period: String,
    pub display_currency: String,
    pub selected_forex_pairs: Vec<String>,
    pub selected_interest_countries: Vec<String>,
    pub selected_macro_countries: Vec<String>,
    pub macro_start_year: i32,
    pub macro_end_year: i32,
    pub use_cache: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
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
            selected_interest_countries: vec![
                "BR".into(),
                "US".into(),
                "EU".into(),
                "GB".into(),
                "JP".into(),
            ],
            selected_macro_countries: vec![
                "BRA".into(),
                "USA".into(),
                "CHN".into(),
                "DEU".into(),
                "GBR".into(),
                "JPN".into(),
            ],
            macro_start_year: 2018,
            macro_end_year: 2024,
            use_cache: true,
        }
    }
}

impl UserSettings {
    /// Retorna o caminho do arquivo de configuração local.
    pub fn caminho_arquivo() -> PathBuf {
        PathBuf::from("config").join("user_settings.json")
    }

    /// Carrega as configurações do disco ou retorna o padrão caso o arquivo não exista.
    pub fn carregar() -> Self {
        let path = Self::caminho_arquivo();
        if let Ok(conteudo) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<UserSettings>(&conteudo) {
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
            .map_err(|e| format!("Erro ao serializar configurações: {e}"))?;
        fs::write(&path, json_str).map_err(|e| format!("Erro ao gravar configurações: {e}"))
    }
}

