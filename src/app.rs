//! Interface Gráfica Principal — DAX - Data Analysis Exchange (Versão 0.2).
//! Implementa as 5 abas obrigatórias (a, b, c, d, e) com todas as melhorias da v0.2:
//! - Layout 100% contido na janela com barras de rolagem sempre visíveis;
//! - Remoção fácil de Ativos Favoritos;
//! - Aba (b): Até 3 moedas em destaque frente ao USD, ativar/desativar linhas no gráfico,
//!   inserir/retirar até 10 moedas e eixo X formatado com Data (Ano);
//! - Aba (c): Cards da taxa de juros atual dos EUA e de todos os demais países selecionados + eixo X com Data (Ano);
//! - Aba (d): Eixo dos anos com Data de Início e Final explícita;
//! - Sincronização automática dos países entre (b) Moedas, (c) Juros e (d) Socioeconomia (WB).

use crate::bridge::{BridgeMessage, PythonBridge};
use crate::config::{UserSettings, CATALOGO_MOEDAS_PAISES, CATALOGO_PAISES_MACRO};
use crate::models::{
    ForexResponse, InterestResponse, MacroResponse, MarketOverviewResponse, StockDetailResponse,
};
use crate::theme::{
    aplicar_tema_dax, badge_variacao, card_frame_mode, cor_variacao, desenhar_sparkline,
    desenhar_tooltip_serie_borda_segura, distancia_ponto_segmento,
    encontrar_ponto_serie_sob_mouse, formatar_moeda, hero_blue_frame, palette,
    renderizar_legenda_abaixo, ThemePalette, ACCENT_ORANGE, GREEN_UP, RED_DOWN,
};
use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_plot::{
    uniform_grid_spacer, Bar, BarChart, BoxElem, BoxPlot, BoxSpread, Line, Plot, PlotPoints,
};
use std::time::{Duration, Instant};

/// Abas principais da aplicação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    /// (a) Analisar ações de empresas (preços históricos, OHLCV, candles, estatísticas - Dax3)
    Stocks,
    /// (b) Cotações de moedas e Dólar Americano Index (DXY)
    Forex,
    /// (c) Taxas de juros de diferentes países (Fed, Selic, BoE, BCE, BoJ, etc.)
    InterestRates,
    /// (d) Comparar variáveis socioeconômicas entre países (World Bank API)
    MacroSocioeconomic,
    /// (e) Dashboards, Altas/Baixas, Cards e Stocks Scanner (Dax1, Dax2, Dax4)
    DashboardScanner,
}

/// Modo de visualização dentro da Aba (e) Dashboard (Cards estilo Dax2 vs Scanner estilo Dax4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardViewMode {
    CardsDax2,
    ScannerDax4,
}

/// Tipo de gráfico na Aba (a) Ações.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockChartMode {
    Candlestick,
    AreaLine,
}

pub struct DaxEconomiaApp {
    pub settings: UserSettings,
    pub bridge: PythonBridge,
    pub active_tab: ActiveTab,
    pub initialized: bool,

    // Dados carregados do motor Python
    pub overview_data: Option<MarketOverviewResponse>,
    pub stock_data: Option<StockDetailResponse>,
    pub forex_data: Option<ForexResponse>,
    pub interest_data: Option<InterestResponse>,
    pub macro_data: Option<MacroResponse>,

    // Controles de Favoritos na barra lateral
    pub new_fav_input: String,

    // Estado de controles da Aba (a) Ações
    pub ticker_input: String,
    pub stock_chart_mode: StockChartMode,
    pub show_sma20: bool,
    pub show_sma50: bool,

    // Estado de controles da Aba (b) Moedas e Conversor
    pub forex_base100_mode: bool,
    pub conv_amount: f64,
    pub conv_from: String,
    pub conv_to: String,
    pub currency_to_add: String,

    // Estado de controles da Aba (d) Socioeconomia (World Bank)
    pub selected_wb_indicator: String,
    pub macro_country_to_add: String,

    // Estado de controles da Aba (e) Dashboard & Scanner (Dax2 e Dax4)
    pub dashboard_mode: DashboardViewMode,
    pub scanner_search: String,
    pub scanner_country: String,
    pub filter_min_mcap: f64,
    pub filter_max_pe: f64,
    pub filter_min_dy: f64,
    pub filter_min_roe: f64,

    // Mensagens de status e temporizador de 10 minutos
    pub status_message: String,
    pub error_banner: Option<String>,
    pub last_indices_refresh: Instant,
}

impl DaxEconomiaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut settings = UserSettings::carregar();
        settings.sincronizar_paises_com_moedas();
        aplicar_tema_dax(&cc.egui_ctx, settings.dark_mode);
        let ticker_inicial = settings.selected_ticker.clone();
        let mut bridge = PythonBridge::new();
        bridge.offline_mode = settings.offline_mode;

        Self {
            settings,
            bridge,
            active_tab: ActiveTab::DashboardScanner,
            initialized: false,
            overview_data: None,
            stock_data: None,
            forex_data: None,
            interest_data: None,
            macro_data: None,
            new_fav_input: String::new(),
            ticker_input: ticker_inicial,
            stock_chart_mode: StockChartMode::Candlestick,
            show_sma20: true,
            show_sma50: true,
            forex_base100_mode: true,
            conv_amount: 1000.0,
            conv_from: "USD".to_string(),
            conv_to: "BRL".to_string(),
            currency_to_add: "USDCHF=X".to_string(),
            selected_wb_indicator: "gdp_total_bi".to_string(),
            macro_country_to_add: "FRA".to_string(),
            dashboard_mode: DashboardViewMode::CardsDax2,
            scanner_search: String::new(),
            scanner_country: "All countries".to_string(),
            filter_min_mcap: 0.0,
            filter_max_pe: 100.0,
            filter_min_dy: 0.0,
            filter_min_roe: -50.0,
            status_message: "Starting DAX - Data Analysis Exchange (v0.2)...".into(),
            error_banner: None,
            last_indices_refresh: Instant::now(),
        }
    }

    fn pal(&self) -> ThemePalette {
        palette(self.settings.dark_mode)
    }

    fn card(&self) -> egui::Frame {
        card_frame_mode(self.settings.dark_mode)
    }

    fn parsed_start_year(&self) -> i32 {
        let end_y = self.parsed_end_year();
        let raw_start = self
            .settings
            .analysis_start_date
            .get(..4)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(self.settings.macro_start_year)
            .clamp(1990, 2025);
        // Ensure annual macroeconomic & interest series always span at least 5 years (e.g. 2019..2025)
        if end_y - raw_start < 4 {
            (end_y - 6).clamp(2018, 2021)
        } else {
            raw_start
        }
    }

    fn parsed_end_year(&self) -> i32 {
        self.settings
            .analysis_end_date
            .get(..4)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(self.settings.macro_end_year)
            .clamp(2020, 2025)
    }

    fn apply_date_preset(&mut self, days_back: i64, period_code: &str, ctx: &egui::Context) {
        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(days_back);
        self.settings.analysis_start_date = start.format("%Y-%m-%d").to_string();
        self.settings.analysis_end_date = end.format("%Y-%m-%d").to_string();
        self.settings.selected_period = period_code.to_string();
        self.settings.macro_start_year = self.parsed_start_year();
        self.settings.macro_end_year = self.parsed_end_year();
        let _ = self.settings.salvar();
        self.reload_all_with_dates(false, ctx);
    }

    fn reload_all_with_dates(&mut self, force_refresh: bool, ctx: &egui::Context) {
        self.settings.sincronizar_paises_com_moedas();
        self.settings.macro_start_year = self.parsed_start_year();
        self.settings.macro_end_year = self.parsed_end_year();
        let _ = self.settings.salvar();

        let ticker = self.settings.selected_ticker.clone();
        let period = self.settings.selected_period.clone();
        let start_d = self.settings.analysis_start_date.clone();
        let end_d = self.settings.analysis_end_date.clone();
        let curr = self.settings.display_currency.clone();
        let pairs = self.settings.selected_forex_pairs.clone();
        let int_countries = self.settings.selected_interest_countries.clone();
        let macro_countries = self.settings.selected_macro_countries.clone();
        let s_yr = self.settings.macro_start_year;
        let e_yr = self.settings.macro_end_year;

        self.bridge
            .request_stock(&ticker, &period, &start_d, &end_d, &curr, force_refresh, ctx.clone());
        self.bridge
            .request_forex(&pairs, &period, &start_d, &end_d, force_refresh, ctx.clone());
        // Interest rates & Macroeconomic indicators update on a Monthly cycle, not daily Refresh Now
        self.bridge
            .request_interest(&int_countries, s_yr, e_yr, false, ctx.clone());
        if !force_refresh || self.macro_data.is_none() {
            if !self.carregar_macro_permanente_se_compativel() {
                self.bridge
                    .request_macro(&macro_countries, s_yr, e_yr, false, ctx.clone());
            }
        }
    }

    fn carregar_macro_permanente_se_compativel(&mut self) -> bool {
        let path = UserSettings::macro_permanent_path();
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(mut cached) = serde_json::from_str::<MacroResponse>(&raw) {
                let current_month = chrono::Local::now().format("%Y-%m").to_string();
                let month_ok = self.settings.offline_mode || cached.cache_month == current_month;
                let expected: Vec<String> = self
                    .settings
                    .selected_macro_countries
                    .iter()
                    .map(|c| c.to_uppercase())
                    .collect();
                let loaded: Vec<String> = cached
                    .countries
                    .iter()
                    .map(|c| c.iso3.to_uppercase())
                    .collect();
                let same_countries = expected.len() == loaded.len()
                    && expected.iter().all(|c| loaded.contains(c));
                let same_years = cached.start_year <= self.settings.macro_start_year
                    && cached.end_year >= self.settings.macro_end_year;
                if month_ok && same_countries && same_years && cached.indicators_catalog.len() >= 14 {
                    cached.source_status = if self.settings.offline_mode {
                        "Offline Mode (Permanent Local Snapshot)".to_string()
                    } else {
                        format!("Monthly Local Snapshot ({current_month})")
                    };
                    self.macro_data = Some(cached);
                    return true;
                }
            }
        }
        false
    }

    fn salvar_macro_permanente(data: &MacroResponse) {
        let path = UserSettings::macro_permanent_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json_str) = serde_json::to_string_pretty(data) {
            let _ = std::fs::write(path, json_str);
        }
    }

    fn export_csv_and_open(&mut self, filename: &str, headers: &[&str], rows: Vec<Vec<String>>) {
        let dir = UserSettings::exports_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(filename);
        let mut out = String::new();
        out.push_str(&headers.join(","));
        out.push('\n');
        for r in rows {
            let escaped: Vec<String> = r
                .into_iter()
                .map(|cell| {
                    if cell.contains(',') || cell.contains('"') {
                        format!("\"{}\"", cell.replace('"', "\"\""))
                    } else {
                        cell
                    }
                })
                .collect();
            out.push_str(&escaped.join(","));
            out.push('\n');
        }
        match std::fs::write(&path, out) {
            Ok(_) => {
                self.status_message = format!("CSV exported: {}", path.display());
                let _ = open::that(&path);
            }
            Err(e) => {
                self.error_banner = Some(format!("Failed to export CSV: {e}"));
            }
        }
    }

    /// Dispara o carregamento inicial de todas as seções em background.
    fn disparar_carga_inicial(&mut self, ctx: &egui::Context) {
        self.settings.sincronizar_paises_com_moedas();
        let favs = self.settings.favorite_tickers.clone();
        self.bridge.request_overview(&favs, false, ctx.clone());
        self.reload_all_with_dates(false, ctx);
    }

    /// Constant renewal of daily market data (World Indices, Stocks, Currencies & DXY)
    /// without touching permanent long-term Macroeconomic & Monthly Interest Rate databases.
    fn renovar_dados_mercado_diario(&mut self, ctx: &egui::Context) {
        self.last_indices_refresh = Instant::now();
        let today = chrono::Local::now().date_naive();
        if let Ok(saved_end) =
            chrono::NaiveDate::parse_from_str(self.settings.analysis_end_date.trim(), "%Y-%m-%d")
        {
            let diff = (today - saved_end).num_days();
            if (1..=30).contains(&diff) {
                self.settings.analysis_end_date = today.format("%Y-%m-%d").to_string();
                let _ = self.settings.salvar();
            }
        }

        let favs = self.settings.favorite_tickers.clone();
        let ticker = self.settings.selected_ticker.clone();
        let period = self.settings.selected_period.clone();
        let start_d = self.settings.analysis_start_date.clone();
        let end_d = self.settings.analysis_end_date.clone();
        let curr = self.settings.display_currency.clone();
        let pairs = self.settings.selected_forex_pairs.clone();

        self.bridge.request_overview(&favs, true, ctx.clone());
        self.bridge
            .request_stock(&ticker, &period, &start_d, &end_d, &curr, true, ctx.clone());
        self.bridge
            .request_forex(&pairs, &period, &start_d, &end_d, true, ctx.clone());
    }

    /// Synchronizes Tab (c) Interest Rates with Tab (b) Currencies & DXY.
    /// Tab (d) Macroeconomic countries remain independent unless the user clicks Sync in Tab (d).
    fn atualizar_moedas_e_sincronizar_paises(&mut self, ctx: &egui::Context) {
        self.settings.sincronizar_paises_com_moedas();
        let _ = self.settings.salvar();

        let pairs = self.settings.selected_forex_pairs.clone();
        let period = self.settings.selected_period.clone();
        let start_d = self.settings.analysis_start_date.clone();
        let end_d = self.settings.analysis_end_date.clone();
        let int_countries = self.settings.selected_interest_countries.clone();
        let s_yr = self.parsed_start_year();
        let e_yr = self.parsed_end_year();

        self.bridge
            .request_forex(&pairs, &period, &start_d, &end_d, false, ctx.clone());
        self.bridge
            .request_interest(&int_countries, s_yr, e_yr, false, ctx.clone());
    }

    /// Processa mensagens assíncronas recebidas da ponte Python.
    fn processar_mensagens_bridge(&mut self) {
        for msg in self.bridge.poll_messages() {
            match msg {
                BridgeMessage::OverviewLoaded(data) => {
                    self.status_message = format!(
                        "Market overview updated ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.overview_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::StockLoaded(data) => {
                    self.status_message = format!(
                        "Stock {} loaded in {} ({}) — {}",
                        data.short_code, data.display_currency, data.source_status, data.updated_at
                    );
                    self.stock_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::ForexLoaded(data) => {
                    self.status_message = format!(
                        "Currencies & DXY updated ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.forex_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::InterestLoaded(data) => {
                    self.status_message = format!(
                        "Interest rates synchronized ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.interest_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::MacroLoaded(data) => {
                    Self::salvar_macro_permanente(&data);
                    self.status_message = format!(
                        "Macroeconomic data saved permanently ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.macro_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::ChartExported(resp) => {
                    self.status_message = resp.message.clone();
                    if !resp.file_path.is_empty() {
                        let _ = open::that(&resp.file_path);
                    }
                }
                BridgeMessage::CacheCleared(msg) => {
                    self.status_message = msg;
                    self.error_banner = None;
                }
                BridgeMessage::Error(err) => {
                    self.error_banner = Some(err.clone());
                    self.status_message = "Attention: check the notice displayed at the top.".into();
                }
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // BARRA SUPERIOR DE ÍNDICES DAS PRINCIPAIS BOLSAS MUNDIAIS (DAX1)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_barra_superior_dax1(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let dec = self.last_indices_refresh.elapsed().as_secs();
        let restante = 600_u64.saturating_sub(dec);
        let min_rest = restante / 60;
        let seg_rest = restante % 60;

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🌐 MAJOR GLOBAL STOCK MARKET INDICES")
                    .size(12.0)
                    .strong()
                    .color(self.pal().accent_cyan),
            );
            ui.label(
                RichText::new(
                    "(Brazil • Wall Street • Nasdaq • Japan • China • Germany • UK • France • Spain • Argentina)",
                )
                .size(11.0)
                .color(self.pal().text_muted),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        RichText::new("🔄 Refresh Now")
                            .size(12.0)
                            .strong()
                            .color(GREEN_UP),
                    )
                    .on_hover_text("Immediately refresh global stock market indices, equities, and FX quotes")
                    .clicked()
                {
                    self.renovar_dados_mercado_diario(ctx);
                }

                ui.separator();

                let ultima_hora = self
                    .overview_data
                    .as_ref()
                    .map(|o| o.updated_at.as_str())
                    .unwrap_or("--:--:--");
                ui.label(
                    RichText::new(format!(
                        "⏱ Auto (10 min): {:02}:{:02}  |  Last: {}",
                        min_rest, seg_rest, ultima_hora
                    ))
                    .size(11.0)
                    .color(self.pal().text_muted),
                );
            });
        });

        ui.add_space(4.0);

        let mut indice_clicado: Option<String> = None;

        egui::ScrollArea::horizontal()
            .id_salt("top_bar_horiz_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(7.0, 6.0);

                    if let Some(ov) = &self.overview_data {
                        for item in &ov.top_bar {
                            egui::Frame::none()
                                .fill(self.pal().bg_card)
                                .stroke(Stroke::new(1.0_f32, self.pal().border_subtle))
                                .rounding(Rounding::same(6.0))
                                .inner_margin(egui::Margin::symmetric(9.0, 5.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(138.0);
                                    ui.set_max_width(152.0);
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(&item.code)
                                                            .strong()
                                                            .size(11.5)
                                                            .color(self.pal().text_primary),
                                                    )
                                                    .frame(false),
                                                )
                                                .on_hover_text(format!(
                                                    "Click to open detailed chart for {} ({})",
                                                    item.code, item.subtitle
                                                ))
                                                .clicked()
                                            {
                                                indice_clicado = Some(item.code.clone());
                                            }

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let sinal =
                                                        if item.change_pct > 0.0 { "+" } else { "" };
                                                    ui.label(
                                                        RichText::new(
                                                            format!("{sinal}{:.2}%", item.change_pct),
                                                        )
                                                        .size(11.5)
                                                        .strong()
                                                        .color(cor_variacao(item.change_pct)),
                                                    );
                                                },
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(&item.value)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(self.pal().accent_cyan),
                                                );
                                                let data_idx = if item.last_date.is_empty() {
                                                    self.settings.analysis_end_date.as_str()
                                                } else {
                                                    item.last_date.as_str()
                                                };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} • {}",
                                                        item.subtitle, data_idx
                                                    ))
                                                    .size(9.5)
                                                    .color(self.pal().text_muted),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    desenhar_sparkline(
                                                        ui,
                                                        &item.sparkline,
                                                        Vec2::new(54.0, 22.0),
                                                        item.change_pct >= 0.0,
                                                    );
                                                },
                                            );
                                        });
                                    });
                                });
                        }
                    } else {
                        ui.label(
                            RichText::new(
                                "Loading global stock markets (IBOV, Wall St, Nasdaq, Nikkei, Shanghai, DAX, FTSE, CAC, IBEX, Merval)...",
                            )
                            .color(self.pal().text_muted),
                        );
                    }
                });
            });

        if let Some(cod) = indice_clicado {
            self.selecionar_acao(&cod, ctx);
            self.active_tab = ActiveTab::Stocks;
        }

        ui.add_space(5.0);
        let pal = self.pal();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("📅 Global Date Range:")
                    .size(11.5)
                    .strong()
                    .color(pal.accent_cyan),
            );
            ui.label(RichText::new("Start:").size(11.5).color(pal.text_muted));
            ui.add(
                egui::TextEdit::singleline(&mut self.settings.analysis_start_date)
                    .desired_width(86.0)
                    .hint_text("YYYY-MM-DD"),
            );
            ui.label(RichText::new("End:").size(11.5).color(pal.text_muted));
            ui.add(
                egui::TextEdit::singleline(&mut self.settings.analysis_end_date)
                    .desired_width(86.0)
                    .hint_text("YYYY-MM-DD"),
            );
            if ui
                .button(RichText::new("Apply Dates").size(11.5).strong())
                .on_hover_text("Apply Start and End dates across Equities, FX, Rates, and Macro")
                .clicked()
            {
                self.reload_all_with_dates(false, ctx);
            }
            ui.separator();
            for (lbl, days, p_cod) in [
                ("1M", 30_i64, "1mo"),
                ("6M", 180_i64, "6mo"),
                ("1Y", 365_i64, "1y"),
                ("5Y", 1825_i64, "5y"),
                ("MAX", 5475_i64, "5y"),
            ] {
                if ui.small_button(lbl).clicked() {
                    self.apply_date_preset(days, p_cod, ctx);
                }
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MENU LATERAL DE NAVEGAÇÃO + REMOÇÃO DE FAVORITOS (DAX v0.2)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_menu_lateral(&mut self, ui: &mut Ui) {
        // Trava estritamente a largura interna da barra lateral para nunca expandir nem criar espaço vazio
        ui.set_min_width(220.0);
        ui.set_max_width(220.0);

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("📊 DAX")
                    .size(22.0)
                    .strong()
                    .color(GREEN_UP),
            );
            ui.label(
                RichText::new("v0.3")
                    .size(12.5)
                    .strong()
                    .color(self.pal().accent_cyan),
            );
        });
        ui.label(
            RichText::new("Data Analysis Exchange")
                .size(13.5)
                .strong()
                .color(self.pal().text_primary),
        );
        ui.label(
            RichText::new("Equities • FX DXY • Rates • Macro")
                .size(11.0)
                .color(self.pal().text_muted),
        );
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let abas = [
            (
                ActiveTab::DashboardScanner,
                "⚡ (e) Dashboard & Scanner",
                "Gainers/Losers (Dax1), Asset Cards (Dax2) & Stocks Scanner (Dax4)",
            ),
            (
                ActiveTab::Stocks,
                "📈 (a) Equities & Candles",
                "OHLCV Analysis, Candlesticks, Moving Averages & Multiples (Dax3)",
            ),
            (
                ActiveTab::Forex,
                "💵 (b) Currencies & DXY",
                "Up to 10 currencies, 3 spotlights vs USD, curve filter & DXY=",
            ),
            (
                ActiveTab::InterestRates,
                "🏦 (c) Interest Rates (CBs)",
                "Current policy rates for all countries selected in (b) + Real Rate",
            ),
            (
                ActiveTab::MacroSocioeconomic,
                "🌍 (d) Macroeconomics (WB)",
                "GDP, GDP per capita, Inflation, Unemployment, Exchange Rate & Population",
            ),
        ];

        for (tab, titulo, dica) in abas {
            let selecionado = self.active_tab == tab;
            let fill = if selecionado {
                Color32::from_rgb(0, 120, 72)
            } else {
                Color32::TRANSPARENT
            };
            let stroke = if selecionado {
                Stroke::new(1.2_f32, GREEN_UP)
            } else {
                Stroke::NONE
            };

            egui::Frame::none()
                .fill(fill)
                .stroke(stroke)
                .rounding(Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                .show(ui, |ui| {
                    ui.set_max_width(204.0);
                    let btn = ui.add_sized(
                        [200.0, 24.0],
                        egui::Button::new(
                            RichText::new(titulo)
                                .size(13.0)
                                .strong()
                                .color(if selecionado { Color32::WHITE } else { self.pal().text_primary }),
                        )
                        .frame(false),
                    );
                    if btn.on_hover_text(dica).clicked() {
                        self.active_tab = tab;
                    }
                });
            ui.add_space(3.0);
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        ui.label(
            RichText::new("⭐ Favorite Assets (Click ✖ to remove)")
                .size(11.5)
                .strong()
                .color(self.pal().accent_cyan),
        );
        ui.add_space(4.0);

        // Quick input to add a new favorite directly in the sidebar
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.new_fav_input)
                    .desired_width(155.0)
                    .hint_text("New (e.g. WEGE3)"),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (enter || ui.button("➕").on_hover_text("Add to favorites").clicked())
                && !self.new_fav_input.trim().is_empty()
            {
                let novo = self.new_fav_input.trim().to_uppercase();
                if !self
                    .settings
                    .favorite_tickers
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(&novo))
                {
                    self.settings.favorite_tickers.push(novo);
                    let _ = self.settings.salvar();
                }
                self.new_fav_input.clear();
            }
        });

        ui.add_space(6.0);

        let favs_clone = self.settings.favorite_tickers.clone();
        let mut ticker_clicado: Option<String> = None;
        let mut ticker_para_remover: Option<String> = None;

        // Grade fixa de 2 colunas para os favoritos: nunca ultrapassa 220px de largura!
        egui::ScrollArea::vertical()
            .id_salt("fav_list_scroll_v02")
            .max_height(210.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_max_width(214.0);
                egui::Grid::new("fav_tickers_grid_2cols")
                    .num_columns(2)
                    .spacing([6.0, 6.0])
                    .show(ui, |ui| {
                        for par_favs in favs_clone.chunks(2) {
                            for fav in par_favs {
                                let ativo_atual =
                                    self.settings.selected_ticker.eq_ignore_ascii_case(fav);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    let btn_open = ui.add_sized(
                                        [72.0, 22.0],
                                        egui::Button::new(
                                            RichText::new(fav)
                                                .size(11.0)
                                                .strong()
                                                .color(if ativo_atual {
                                                    Color32::BLACK
                                                } else {
                                                    Color32::WHITE
                                                }),
                                        )
                                        .fill(if ativo_atual {
                                            GREEN_UP
                                        } else {
                                            self.pal().bg_card_alt
                                        }),
                                    );
                                    if btn_open
                                        .on_hover_text(format!("Open analysis for {fav}"))
                                        .clicked()
                                    {
                                        ticker_clicado = Some(fav.clone());
                                    }

                                    let btn_del = ui.add_sized(
                                        [20.0, 22.0],
                                        egui::Button::new(
                                            RichText::new("✖")
                                                .size(10.0)
                                                .strong()
                                                .color(RED_DOWN),
                                        )
                                        .fill(self.pal().bg_card),
                                    );
                                    if btn_del
                                        .on_hover_text(format!(
                                            "Remove {fav} from Favorite Assets"
                                        ))
                                        .clicked()
                                    {
                                        ticker_para_remover = Some(fav.clone());
                                    }
                                });
                            }
                            ui.end_row();
                        }
                    });
            });

        if let Some(rem) = ticker_para_remover {
            self.settings
                .favorite_tickers
                .retain(|f| !f.eq_ignore_ascii_case(&rem));
            let _ = self.settings.salvar();
            self.status_message = format!("Asset '{rem}' removed from favorites.");
        }

        if let Some(tk) = ticker_clicado {
            self.selecionar_acao(&tk, ui.ctx());
            self.active_tab = ActiveTab::Stocks;
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            let pal = self.pal();
            ui.label(
                RichText::new("🔒 Privacy: 100% local data (SQLite + JSON)")
                    .size(10.0)
                    .color(pal.text_muted),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let theme_label = if self.settings.dark_mode {
                    "☀ Light Mode"
                } else {
                    "🌙 Dark Mode"
                };
                if ui
                    .button(RichText::new(theme_label).size(11.0))
                    .on_hover_text("Switch between Dark Mode and Light Mode")
                    .clicked()
                {
                    self.settings.dark_mode = !self.settings.dark_mode;
                    aplicar_tema_dax(ui.ctx(), self.settings.dark_mode);
                    let _ = self.settings.salvar();
                }

                if ui
                    .button(RichText::new("🗑 Cache").size(11.0))
                    .on_hover_text("Clear local SQLite cache")
                    .clicked()
                {
                    self.bridge.request_clear_cache(ui.ctx().clone());
                }
            });
            let offline_label = if self.settings.offline_mode {
                "📡 Offline Mode: ON"
            } else {
                "🌐 Online Mode: ON"
            };
            if ui
                .button(RichText::new(offline_label).size(11.0))
                .on_hover_text("Toggle Offline Mode (uses local cache/fallback without network delays)")
                .clicked()
            {
                self.settings.offline_mode = !self.settings.offline_mode;
                self.bridge.offline_mode = self.settings.offline_mode;
                let _ = self.settings.salvar();
                if self.settings.offline_mode {
                    self.status_message = "Offline Mode enabled (using local cache & permanent DB).".into();
                } else {
                    self.status_message = "Online Mode enabled — refreshing live market data...".into();
                    let ctx_clone = ui.ctx().clone();
                    self.renovar_dados_mercado_diario(&ctx_clone);
                }
            }
            if self.bridge.active_tasks > 0 {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!(
                            "Processing {} task(s)...",
                            self.bridge.active_tasks
                        ))
                        .size(11.0)
                        .color(ACCENT_ORANGE),
                    );
                });
            }
        });
    }

    /// Updates the selected stock, persists to `user_settings.json`, and fetches data from Python.
    fn selecionar_acao(&mut self, ticker: &str, ctx: &egui::Context) {
        let limpo = ticker.trim().to_uppercase();
        if limpo.is_empty() {
            return;
        }
        self.ticker_input = limpo.clone();
        self.settings.selected_ticker = limpo.clone();
        let _ = self.settings.salvar();
        self.bridge.request_stock(
            &limpo,
            &self.settings.selected_period,
            &self.settings.analysis_start_date,
            &self.settings.analysis_end_date,
            &self.settings.display_currency,
            false,
            ctx.clone(),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (a): ANÁLISE DE AÇÕES, CANDLESTICKS E ESTATÍSTICAS (ESTILO DAX3)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_acoes(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("🔍 Asset / Ticker:").strong());
                let resp_input = ui.add(
                    egui::TextEdit::singleline(&mut self.ticker_input)
                        .desired_width(110.0)
                        .hint_text("e.g. PETR4, AAPL"),
                );
                if (resp_input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    || ui.button(RichText::new("Lookup").strong()).clicked()
                {
                    let tk = self.ticker_input.clone();
                    self.selecionar_acao(&tk, ctx);
                }

                let tk_upper = self.settings.selected_ticker.to_uppercase();
                let ja_favorito = self
                    .settings
                    .favorite_tickers
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(&tk_upper));
                let label_fav = if ja_favorito {
                    "★ Remove Favorite"
                } else {
                    "☆ Add Favorite"
                };
                if ui.button(label_fav).clicked() {
                    if ja_favorito {
                        self.settings
                            .favorite_tickers
                            .retain(|f| !f.eq_ignore_ascii_case(&tk_upper));
                    } else {
                        self.settings.favorite_tickers.push(tk_upper);
                    }
                    let _ = self.settings.salvar();
                }

                ui.separator();
                ui.label(RichText::new("Period:").strong());
                let periodos = [
                    ("5d", "5 DAYS", 5_i64),
                    ("1mo", "30 DAYS", 30_i64),
                    ("6mo", "6 MONTHS", 180_i64),
                    ("1y", "1 YEAR", 365_i64),
                    ("5y", "5 YEARS", 1825_i64),
                ];
                for (cod, rotulo, days) in periodos {
                    let sel = self.settings.selected_period == cod;
                    if ui.selectable_label(sel, rotulo).clicked() {
                        self.apply_date_preset(days, cod, ctx);
                    }
                }

                ui.separator();
                ui.label(RichText::new("Currency:").strong());
                let moedas = [
                    ("ORIGINAL", "Native Currency"),
                    ("BRL", "R$ (BRL)"),
                    ("USD", "US$ (USD)"),
                    ("EUR", "€ (EUR)"),
                ];
                for (cod_m, rot_m) in moedas {
                    let sel = self.settings.display_currency == cod_m;
                    if ui.selectable_label(sel, rot_m).clicked() {
                        self.settings.display_currency = cod_m.to_string();
                        let _ = self.settings.salvar();
                        self.bridge.request_stock(
                            &self.settings.selected_ticker,
                            &self.settings.selected_period,
                            &self.settings.analysis_start_date,
                            &self.settings.analysis_end_date,
                            &self.settings.display_currency,
                            false,
                            ctx.clone(),
                        );
                    }
                }
            });
        });

        ui.add_space(8.0);

        let Some(dados) = self.stock_data.clone() else {
            ui.label(
                RichText::new("Loading OHLCV historical series and fundamental multiples...")
                    .size(15.0)
                    .color(self.pal().text_muted),
            );
            return;
        };

        let m = &dados.metrics;
        let moeda = &dados.display_currency;
        let data_ult_candle: String = dados
            .candles
            .last()
            .map(|c| c.date.clone())
            .unwrap_or_else(|| self.settings.analysis_end_date.clone());
        let data_pri_candle: String = dados
            .candles
            .first()
            .map(|c| c.date.clone())
            .unwrap_or_else(|| self.settings.analysis_start_date.clone());

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{} — {}", dados.short_code, dados.company_name))
                    .size(20.0)
                    .strong()
                    .color(self.pal().text_primary),
            );
            badge_variacao(ui, m.change_day_pct);
            ui.label(
                RichText::new(format!(
                    " | 📅 Quote Date: {} | Sector: {} | Country: {} | Currency: {} (FX: {:.4})",
                    data_ult_candle, dados.sector, dados.country, dados.display_currency, dados.fx_factor
                ))
                .size(12.0)
                .color(self.pal().text_muted),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("📊 Interactive Report (Plotly HTML)")
                    .clicked()
                {
                    self.bridge.request_export_chart(
                        &dados.ticker,
                        &dados.period,
                        &self.settings.analysis_start_date,
                        &self.settings.analysis_end_date,
                        &dados.display_currency,
                        "plotly",
                        ctx.clone(),
                    );
                }
                if ui.button("🖼 Export PNG (Matplotlib)").clicked() {
                    self.bridge.request_export_chart(
                        &dados.ticker,
                        &dados.period,
                        &self.settings.analysis_start_date,
                        &self.settings.analysis_end_date,
                        &dados.display_currency,
                        "matplotlib",
                        ctx.clone(),
                    );
                }
            });
        });

        ui.add_space(6.0);

        // 5 Cards de Indicadores Superiores (Dax3)
        ui.columns(5, |cols| {
            hero_blue_frame().show(&mut cols[0], |ui| {
                ui.set_min_height(74.0);
                ui.label(
                    RichText::new(format!("CURRENT PRICE ({data_ult_candle})"))
                        .size(11.0)
                        .color(self.pal().text_primary),
                );
                ui.label(
                    RichText::new(formatar_moeda(m.current_price, moeda))
                        .size(20.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                let seta = if m.change_day_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta} {:.2}% ({data_ult_candle})", m.change_day_pct),
                    )
                    .size(11.5)
                    .strong()
                    .color(cor_variacao(m.change_day_pct)),
                );
            });

            self.card().show(&mut cols[1], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("52-WEEK LOW").size(11.0).color(self.pal().text_muted));
                ui.label(
                    RichText::new(formatar_moeda(m.min_52w, moeda))
                        .size(18.0)
                        .strong()
                        .color(self.pal().text_primary),
                );
                ui.label(
                    RichText::new(format!("MONTH LOW: {}", formatar_moeda(m.min_month, moeda)))
                        .size(11.0)
                        .color(self.pal().text_muted),
                );
            });

            self.card().show(&mut cols[2], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("52-WEEK HIGH").size(11.0).color(self.pal().text_muted));
                ui.label(
                    RichText::new(formatar_moeda(m.max_52w, moeda))
                        .size(18.0)
                        .strong()
                        .color(self.pal().text_primary),
                );
                ui.label(
                    RichText::new(format!("MONTH HIGH: {}", formatar_moeda(m.max_month, moeda)))
                        .size(11.0)
                        .color(self.pal().text_muted),
                );
            });

            self.card().show(&mut cols[3], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("DIVIDEND YIELD").size(11.0).color(self.pal().text_muted));
                ui.label(
                    RichText::new(format!("{:.2}%", m.dividend_yield_pct))
                        .size(18.0)
                        .strong()
                        .color(self.pal().accent_cyan),
                );
                ui.label(
                    RichText::new(format!(
                        "LAST 12M: {}",
                        formatar_moeda(m.dividends_12m, moeda)
                    ))
                    .size(11.0)
                    .color(self.pal().text_muted),
                );
            });

            self.card().show(&mut cols[4], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("RETURN (12M)").size(11.0).color(self.pal().text_muted));
                let seta12 = if m.change_12m_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta12} {:.2}%", m.change_12m_pct),
                    )
                    .size(18.0)
                    .strong()
                    .color(cor_variacao(m.change_12m_pct)),
                );
                ui.label(
                    RichText::new(
                        format!("CURRENT MONTH: {:.2}%", m.change_month_pct),
                    )
                    .size(11.0)
                    .color(cor_variacao(m.change_month_pct)),
                );
            });
        });

        ui.add_space(8.0);

        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} QUOTE ({}) — 📅 {} to {}",
                        dados.short_code, dados.display_currency, data_pri_candle, data_ult_candle
                    ))
                    .size(14.0)
                    .strong()
                    .color(self.pal().accent_cyan),
                );
                ui.separator();
                ui.selectable_value(
                    &mut self.stock_chart_mode,
                    StockChartMode::Candlestick,
                    "🕯 Candlesticks (OHLC)",
                );
                ui.selectable_value(
                    &mut self.stock_chart_mode,
                    StockChartMode::AreaLine,
                    "📈 Continuous Line (Dax3)",
                );
                ui.separator();
                ui.checkbox(&mut self.show_sma20, "SMA 20");
                ui.checkbox(&mut self.show_sma50, "SMA 50");
            });

            ui.add_space(4.0);
            let candles = &dados.candles;
            let datas_plot: Vec<String> = candles.iter().map(|c| c.date.clone()).collect();
            let cor_sma20 = self.pal().accent_cyan;
            let cor_sma50 = Color32::from_rgb(250, 204, 21);

            Plot::new("stock_main_plot")
                .width((largura_max - 36.0).max(400.0))
                .height(265.0)
                .allow_scroll(false)
                .show_x(false)
                .show_y(false)
                .label_formatter(|_, _| String::new())
                .x_axis_label("Date / Year")
                .x_axis_formatter(move |mark, _range| {
                    let idx = mark.value.round() as isize;
                    if idx >= 0 && (idx as usize) < datas_plot.len() {
                        datas_plot[idx as usize].clone()
                    } else {
                        String::new()
                    }
                })
                .show(ui, |plot_ui| {
                    // (dist_px, anchor_screen, cor, titulo_serie, linha_valor, linha_data)
                    let mut melhor_hit: Option<(f32, egui::Pos2, Color32, String, String, String)> =
                        None;

                    match self.stock_chart_mode {
                        StockChartMode::Candlestick => {
                            let mut elementos = Vec::with_capacity(candles.len());
                            for (i, c) in candles.iter().enumerate() {
                                let alta = c.close >= c.open;
                                let cor = if alta { GREEN_UP } else { RED_DOWN };
                                let box_min = c.open.min(c.close);
                                let box_max = c.open.max(c.close);
                                let mediana = (c.open + c.close) * 0.5;
                                elementos.push(
                                    BoxElem::new(
                                        i as f64,
                                        BoxSpread::new(c.low, box_min, mediana, box_max, c.high),
                                    )
                                    .name(&c.date)
                                    .stroke(Stroke::new(1.3_f32, cor))
                                    .fill(cor.gamma_multiply(0.65))
                                    .box_width(0.62),
                                );
                            }
                            plot_ui.box_plot(BoxPlot::new(elementos).name("Candles OHLC"));

                            if let Some(mouse_pos) = plot_ui.ctx().pointer_hover_pos() {
                                if plot_ui.response().rect.contains(mouse_pos) {
                                    for (i, c) in candles.iter().enumerate() {
                                        let p_high = plot_ui
                                            .screen_from_plot(egui_plot::PlotPoint::new(i as f64, c.high));
                                        let p_low = plot_ui
                                            .screen_from_plot(egui_plot::PlotPoint::new(i as f64, c.low));
                                        let d = distancia_ponto_segmento(mouse_pos, p_high, p_low);
                                        if d <= 11.0
                                            && melhor_hit.as_ref().map_or(true, |m| d < m.0)
                                        {
                                            let p_close = plot_ui.screen_from_plot(
                                                egui_plot::PlotPoint::new(i as f64, c.close),
                                            );
                                            let cor = if c.close >= c.open { GREEN_UP } else { RED_DOWN };
                                            melhor_hit = Some((
                                                d,
                                                p_close,
                                                cor,
                                                format!("{} — Candle OHLC", dados.short_code),
                                                format!(
                                                    "Close: {} {:.2} (Open: {:.2} | High: {:.2} | Low: {:.2})",
                                                    dados.display_currency, c.close, c.open, c.high, c.low
                                                ),
                                                format!("Date: {}", c.date),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        StockChartMode::AreaLine => {
                            let vec_close: Vec<[f64; 2]> = candles
                                .iter()
                                .enumerate()
                                .map(|(i, c)| [i as f64, c.close])
                                .collect();
                            let pts_close: PlotPoints = vec_close.iter().copied().collect();
                            let piso = m.min_52w * 0.96;
                            plot_ui.line(
                                Line::new(pts_close)
                                    .name("Close")
                                    .color(ACCENT_ORANGE)
                                    .width(2.4_f32)
                                    .fill(piso as f32),
                            );
                            if let Some((idx, dist, pos_screen)) =
                                encontrar_ponto_serie_sob_mouse(plot_ui, &vec_close, 13.0)
                            {
                                let c = &candles[idx];
                                melhor_hit = Some((
                                    dist,
                                    pos_screen,
                                    ACCENT_ORANGE,
                                    format!("{} — Closing Price", dados.short_code),
                                    format!("Value: {} {:.2}", dados.display_currency, c.close),
                                    format!("Date: {}", c.date),
                                ));
                            }
                        }
                    }

                    if self.show_sma20 {
                        let vec_sma20: Vec<[f64; 2]> = candles
                            .iter()
                            .enumerate()
                            .map(|(i, c)| [i as f64, c.sma20])
                            .collect();
                        let pts_sma20: PlotPoints = vec_sma20.iter().copied().collect();
                        plot_ui.line(
                            Line::new(pts_sma20)
                                .name("SMA 20")
                                .color(cor_sma20)
                                .width(1.6_f32),
                        );
                        if let Some((idx, dist, pos_screen)) =
                            encontrar_ponto_serie_sob_mouse(plot_ui, &vec_sma20, 13.0)
                        {
                            if melhor_hit.as_ref().map_or(true, |m| dist <= m.0) {
                                let c = &candles[idx];
                                melhor_hit = Some((
                                    dist,
                                    pos_screen,
                                    cor_sma20,
                                    format!("{} — SMA 20", dados.short_code),
                                    format!("Value: {} {:.2}", dados.display_currency, c.sma20),
                                    format!("Date: {}", c.date),
                                ));
                            }
                        }
                    }

                    if self.show_sma50 {
                        let vec_sma50: Vec<[f64; 2]> = candles
                            .iter()
                            .enumerate()
                            .map(|(i, c)| [i as f64, c.sma50])
                            .collect();
                        let pts_sma50: PlotPoints = vec_sma50.iter().copied().collect();
                        plot_ui.line(
                            Line::new(pts_sma50)
                                .name("SMA 50")
                                .color(cor_sma50)
                                .width(1.6_f32),
                        );
                        if let Some((idx, dist, pos_screen)) =
                            encontrar_ponto_serie_sob_mouse(plot_ui, &vec_sma50, 13.0)
                        {
                            if melhor_hit.as_ref().map_or(true, |m| dist <= m.0) {
                                let c = &candles[idx];
                                melhor_hit = Some((
                                    dist,
                                    pos_screen,
                                    cor_sma50,
                                    format!("{} — SMA 50", dados.short_code),
                                    format!("Value: {} {:.2}", dados.display_currency, c.sma50),
                                    format!("Date: {}", c.date),
                                ));
                            }
                        }
                    }

                    if let Some((_dist, pos_screen, cor, titulo, valor_str, data_str)) = melhor_hit
                    {
                        desenhar_tooltip_serie_borda_segura(
                            plot_ui,
                            pos_screen,
                            cor,
                            &titulo,
                            &valor_str,
                            &data_str,
                        );
                    }
                });

            // Legenda posicionada abaixo do gráfico de Ações (sem sobrepor as linhas/candles)
            let mut legenda_acoes: Vec<(Color32, String)> = Vec::new();
            match self.stock_chart_mode {
                StockChartMode::Candlestick => {
                    legenda_acoes.push((GREEN_UP, "Bullish Candle (Close ≥ Open)".to_string()));
                    legenda_acoes.push((RED_DOWN, "Bearish Candle (Close < Open)".to_string()));
                }
                StockChartMode::AreaLine => {
                    legenda_acoes.push((
                        ACCENT_ORANGE,
                        format!("Closing Price ({})", dados.short_code),
                    ));
                }
            }
            if self.show_sma20 {
                legenda_acoes.push((self.pal().accent_cyan, "SMA 20 (20-period Moving Average)".to_string()));
            }
            if self.show_sma50 {
                legenda_acoes.push((
                    Color32::from_rgb(250, 204, 21),
                    "SMA 50 (50-period Moving Average)".to_string(),
                ));
            }
            renderizar_legenda_abaixo(ui, &legenda_acoes);
        });

        ui.add_space(8.0);

        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📋 OHLCV Historical Series (Most recent trading days on top)")
                        .size(13.5)
                        .strong()
                        .color(self.pal().text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("📥 Export CSV").clicked() {
                        let rows: Vec<Vec<String>> = dados
                            .candles
                            .iter()
                            .rev()
                            .map(|c| {
                                vec![
                                    c.date.clone(),
                                    format!("{:.4}", c.open),
                                    format!("{:.4}", c.high),
                                    format!("{:.4}", c.low),
                                    format!("{:.4}", c.close),
                                    format!("{:.4}", c.sma20),
                                    format!("{}", c.volume),
                                ]
                            })
                            .collect();
                        self.export_csv_and_open(
                            &format!("stock_{}.csv", dados.short_code),
                            &["Date", "Open", "High", "Low", "Close", "SMA20", "Volume"],
                            rows,
                        );
                    }
                });
            });
            ui.add_space(4.0);

            let candles_rev: Vec<_> = dados.candles.iter().rev().collect();
            egui::ScrollArea::horizontal()
                .id_salt("stock_table_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::initial(95.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(100.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(130.0))
                        .max_scroll_height(185.0)
                        .header(22.0, |mut header| {
                            for col in [
                                "Date",
                                "Open",
                                "High",
                                "Low",
                                "Close",
                                "SMA 20",
                                "Trading Volume",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(col).strong().color(self.pal().accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(20.0, candles_rev.len(), |mut row| {
                                let c = candles_rev[row.index()];
                                let cor_candle =
                                    if c.close >= c.open { GREEN_UP } else { RED_DOWN };
                                row.col(|ui| {
                                    ui.label(&c.date);
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.open));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.high));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.low));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(
                                            format!("{:.2}", c.close),
                                        )
                                        .strong()
                                        .color(cor_candle),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.sma20));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{}", c.volume));
                                });
                            });
                        });
                });
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (b): COTAÇÕES DE MOEDAS E ÍNDICE DÓLAR (DXY=) — MELHORIAS v0.2
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_moedas(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        let Some(fx) = self.forex_data.clone() else {
            ui.label(RichText::new("Loading currency quotes and DXY index...").size(15.0));
            return;
        };

        // 1. CURRENCY CONVERTER PANEL + MANAGER FOR UP TO 10 CURRENCIES (ADD / REMOVE)
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("💱 Currency Converter:")
                        .size(13.0)
                        .strong()
                        .color(self.pal().accent_cyan),
                );
                ui.add(
                    egui::DragValue::new(&mut self.conv_amount)
                        .speed(10.0)
                        .range(0.0..=1_000_000_000.0)
                        .prefix("$ "),
                );

                let moedas_conv = [
                    "USD", "BRL", "EUR", "GBP", "JPY", "CNY", "CHF", "CAD", "AUD", "MXN", "ARS",
                    "INR",
                ];
                egui::ComboBox::from_id_salt("cb_conv_from")
                    .selected_text(&self.conv_from)
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for m in moedas_conv {
                            ui.selectable_value(&mut self.conv_from, m.to_string(), m);
                        }
                    });

                ui.label(RichText::new("➡").strong());

                egui::ComboBox::from_id_salt("cb_conv_to")
                    .selected_text(&self.conv_to)
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for m in moedas_conv {
                            ui.selectable_value(&mut self.conv_to, m.to_string(), m);
                        }
                    });

                let taxa_orig = fx.usd_rates.get(&self.conv_from).copied().unwrap_or(1.0);
                let taxa_dest = fx.usd_rates.get(&self.conv_to).copied().unwrap_or(1.0);
                let convertido = if taxa_orig > 0.0 {
                    (self.conv_amount / taxa_orig) * taxa_dest
                } else {
                    self.conv_amount
                };

                ui.label(
                    RichText::new(format!("= {}", formatar_moeda(convertido, &self.conv_to)))
                        .size(15.0)
                        .strong()
                        .color(GREEN_UP),
                );

                ui.separator();
                ui.label(RichText::new("Period:").strong());
                for (cod, rot, days) in [
                    ("1mo", "1M", 30_i64),
                    ("6mo", "6M", 180_i64),
                    ("1y", "1Y", 365_i64),
                    ("5y", "5Y", 1825_i64),
                ] {
                    if ui
                        .selectable_label(self.settings.selected_period == cod, rot)
                        .clicked()
                    {
                        self.apply_date_preset(days, cod, ctx);
                    }
                }
            });

            ui.separator();

            // Gerenciador de Inserção / Retirada de Moedas (Limite de 10 moedas)
            let qtd_moedas = self.settings.total_moedas_sem_dxy();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "🌐 Selected Currencies ({qtd_moedas} / 10 max) — Syncs countries with (c) Rates & (d) Macro:"
                    ))
                    .size(12.5)
                    .strong()
                    .color(self.pal().accent_cyan),
                );

                egui::ComboBox::from_id_salt("cb_add_currency")
                    .selected_text({
                        CATALOGO_MOEDAS_PAISES
                            .iter()
                            .find(|&&(tk, _, _, _)| tk.eq_ignore_ascii_case(&self.currency_to_add))
                            .map(|&(_, nome, _, _)| nome)
                            .unwrap_or(&self.currency_to_add)
                    })
                    .width(235.0)
                    .show_ui(ui, |ui| {
                        for &(tk, nome, _, _) in CATALOGO_MOEDAS_PAISES {
                            ui.selectable_value(&mut self.currency_to_add, tk.to_string(), nome);
                        }
                    });

                let ja_existe = self
                    .settings
                    .selected_forex_pairs
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&self.currency_to_add));
                let pode_inserir = qtd_moedas < 10 && !ja_existe;

                if ui
                    .add_enabled(
                        pode_inserir,
                        egui::Button::new(RichText::new("➕ Add Currency").strong()),
                    )
                    .on_hover_text("Add the selected currency (maximum limit of 10 currencies)")
                    .clicked()
                {
                    self.settings
                        .selected_forex_pairs
                        .push(self.currency_to_add.clone());
                    self.atualizar_moedas_e_sincronizar_paises(ctx);
                }

                if qtd_moedas >= 10 {
                    ui.label(
                        RichText::new("(10 currency limit reached)")
                            .size(11.0)
                            .color(ACCENT_ORANGE),
                    );
                }
            });

            ui.add_space(4.0);
            let mut moeda_para_retirar: Option<String> = None;
            let pode_remover = self.settings.total_moedas_sem_dxy() > 1;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                for par_cod in &self.settings.selected_forex_pairs {
                    if par_cod.eq_ignore_ascii_case("DX-Y.NYB") {
                        continue;
                    }
                    let nome_curto = CATALOGO_MOEDAS_PAISES
                        .iter()
                        .find(|&&(tk, _, _, _)| tk.eq_ignore_ascii_case(par_cod))
                        .map(|&(_, nome, _, _)| nome)
                        .unwrap_or(par_cod.as_str());

                    let rotulo_botao = if pode_remover {
                        format!("{nome_curto}  ✖")
                    } else {
                        nome_curto.to_string()
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(rotulo_botao).size(11.0).color(self.pal().text_primary),
                            )
                            .fill(self.pal().bg_card_alt),
                        )
                        .on_hover_text("Click to remove this currency from the list")
                        .clicked()
                        && pode_remover
                    {
                        moeda_para_retirar = Some(par_cod.clone());
                    }
                }
            });

            if let Some(rem) = moeda_para_retirar {
                self.settings
                    .selected_forex_pairs
                    .retain(|p| !p.eq_ignore_ascii_case(&rem));
                self.atualizar_moedas_e_sincronizar_paises(ctx);
            }
        });

        ui.add_space(8.0);

        // 2. MECANISMO PARA ESCOLHER ATÉ 3 MOEDAS EM DESTAQUE EM RELAÇÃO AO DÓLAR (USD) + CARD DXY
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "📌 Spotlights vs US Dollar (USD) — Choose up to 3 Currencies:",
                    )
                    .size(13.0)
                    .strong()
                    .color(self.pal().accent_cyan),
                );

                let opcoes_moedas: Vec<String> = self
                    .settings
                    .selected_forex_pairs
                    .iter()
                    .filter(|p| !p.eq_ignore_ascii_case("DX-Y.NYB"))
                    .cloned()
                    .collect();

                let mut mudou_destaque = false;
                for slot in 0..3 {
                    ui.label(RichText::new(format!("Currency {}:", slot + 1)).size(11.5));
                    let atual_txt = if self.settings.forex_spotlight_three[slot].is_empty() {
                        "(None)".to_string()
                    } else {
                        self.settings.forex_spotlight_three[slot].clone()
                    };
                    egui::ComboBox::from_id_salt(format!("cb_spotlight_fx_{slot}"))
                        .selected_text(atual_txt)
                        .width(105.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(
                                    &mut self.settings.forex_spotlight_three[slot],
                                    String::new(),
                                    "(None)",
                                )
                                .changed()
                            {
                                mudou_destaque = true;
                            }
                            for m_cod in &opcoes_moedas {
                                if ui
                                    .selectable_value(
                                        &mut self.settings.forex_spotlight_three[slot],
                                        m_cod.clone(),
                                        m_cod,
                                    )
                                    .changed()
                                {
                                    mudou_destaque = true;
                                }
                            }
                        });
                }
                if mudou_destaque {
                    let _ = self.settings.salvar();
                }
            });

            ui.add_space(6.0);

            // Monta os cards a exibir: DXY= + até 3 moedas escolhidas (sempre contidos na largura da janela)
            let mut cards_destaque = Vec::new();
            if let Some(dxy_par) = fx
                .pairs
                .iter()
                .find(|p| p.ticker.eq_ignore_ascii_case("DX-Y.NYB") || p.display_code == "DXY=")
            {
                cards_destaque.push(dxy_par);
            }
            for slot in 0..3 {
                let alvo = &self.settings.forex_spotlight_three[slot];
                if !alvo.is_empty() {
                    if let Some(par) = fx.pairs.iter().find(|p| {
                        p.ticker.eq_ignore_ascii_case(alvo)
                            || p.display_code.eq_ignore_ascii_case(alvo)
                    }) {
                        cards_destaque.push(par);
                    }
                }
            }

            let n_cols = cards_destaque.len().max(1);
            ui.columns(n_cols, |cols| {
                for (i, par) in cards_destaque.iter().enumerate() {
                    egui::Frame::none()
                        .fill(self.pal().bg_card_alt)
                        .stroke(Stroke::new(1.0_f32, self.pal().border_subtle))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(&mut cols[i], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&par.display_code)
                                        .size(13.5)
                                        .strong()
                                        .color(self.pal().text_primary),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        badge_variacao(ui, par.change_day_pct);
                                    },
                                );
                            });
                            ui.label(
                                RichText::new(format!("{} ({})", par.name, par.country))
                                    .size(10.5)
                                    .color(self.pal().text_muted),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(
                                        format!("{:.4}", par.current_rate),
                                    )
                                    .size(16.5)
                                    .strong()
                                    .color(self.pal().accent_cyan),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let spark: Vec<f64> = par
                                            .history
                                            .iter()
                                            .rev()
                                            .take(25)
                                            .rev()
                                            .map(|h| h.close)
                                            .collect();
                                        desenhar_sparkline(
                                            ui,
                                            &spark,
                                            Vec2::new(72.0, 24.0),
                                            par.change_period_pct >= 0.0,
                                        );
                                    },
                                );
                            });
                            let data_fx_ult = par
                                .history
                                .last()
                                .map(|h| h.date.as_str())
                                .unwrap_or(&self.settings.analysis_end_date);
                            ui.label(
                                RichText::new(format!(
                                    "Period vs USD: {:+.2}%  |  📅 {}",
                                    par.change_period_pct, data_fx_ult
                                ))
                                .size(10.5)
                                .color(cor_variacao(par.change_period_pct)),
                            );
                        });
                }
            });
        });

        ui.add_space(8.0);

        // 3. GRÁFICO DE EVOLUÇÃO COMPARATIVA CAMBIAL (ATIVAR/DESATIVAR MOEDAS + EIXO X COM DATA/ANO)
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                let titulo_grafico = if self.forex_base100_mode {
                    "📈 Comparative FX & DXY Evolution (Base 100 = Start of Period)"
                } else {
                    "📈 Comparative FX & DXY Evolution (Nominal Quote)"
                };
                ui.label(
                    RichText::new(titulo_grafico)
                        .size(13.5)
                        .strong()
                        .color(self.pal().text_primary),
                );
                ui.separator();
                ui.checkbox(&mut self.forex_base100_mode, "Base 100 Scale");
            });

            // Checkboxes to enable or disable each currency on the chart
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Show/Hide on chart:")
                        .size(11.5)
                        .strong()
                        .color(self.pal().accent_cyan),
                );

                if ui.small_button("Select All").clicked() {
                    self.settings.disabled_forex_chart_pairs.clear();
                    let _ = self.settings.salvar();
                }
                if ui.small_button("Deselect All").clicked() {
                    self.settings.disabled_forex_chart_pairs =
                        fx.pairs.iter().map(|p| p.ticker.clone()).collect();
                    let _ = self.settings.salvar();
                }

                for par in &fx.pairs {
                    let mut ativo = !self
                        .settings
                        .disabled_forex_chart_pairs
                        .iter()
                        .any(|d| d.eq_ignore_ascii_case(&par.ticker));
                    if ui
                        .checkbox(
                            &mut ativo,
                            format!("{} ({})", par.display_code, par.country),
                        )
                        .changed()
                    {
                        if ativo {
                            self.settings
                                .disabled_forex_chart_pairs
                                .retain(|d| !d.eq_ignore_ascii_case(&par.ticker));
                        } else {
                            self.settings
                                .disabled_forex_chart_pairs
                                .push(par.ticker.clone());
                        }
                        let _ = self.settings.salvar();
                    }
                }
            });

            ui.add_space(4.0);

            // Extrai vetor de datas reais (YYYY-MM-DD) para exibir Data/Ano claramente no eixo X
            let datas_eixo_x: Vec<String> = fx
                .pairs
                .first()
                .map(|p| p.history.iter().map(|h| h.date.clone()).collect())
                .unwrap_or_default();

            let cores = [
                GREEN_UP,
                self.pal().accent_cyan,
                ACCENT_ORANGE,
                Color32::from_rgb(244, 114, 182),
                Color32::from_rgb(167, 139, 250),
                Color32::from_rgb(250, 204, 21),
                Color32::from_rgb(52, 211, 153),
                Color32::from_rgb(251, 113, 133),
                Color32::from_rgb(96, 165, 250),
                Color32::from_rgb(192, 132, 252),
                Color32::from_rgb(253, 224, 71),
            ];

            let mut legenda_moedas: Vec<(Color32, String)> = Vec::new();
            for (idx_p, par) in fx.pairs.iter().enumerate() {
                let desativado = self
                    .settings
                    .disabled_forex_chart_pairs
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(&par.ticker));
                if !desativado {
                    legenda_moedas.push((
                        cores[idx_p % cores.len()],
                        format!("{} ({})", par.display_code, par.country),
                    ));
                }
            }

            Plot::new("forex_comparison_plot_v02")
                .width((largura_max - 36.0).max(400.0))
                .height(250.0)
                .allow_scroll(false)
                .show_x(false)
                .show_y(false)
                .label_formatter(|_, _| String::new())
                .x_axis_label("Date / Year")
                .x_axis_formatter(move |mark, _range| {
                    let idx = mark.value.round() as isize;
                    if idx >= 0 && (idx as usize) < datas_eixo_x.len() {
                        let d = &datas_eixo_x[idx as usize];
                        // Formata YYYY-MM-DD como MM/AAAA (com o Ano em destaque)
                        if d.len() >= 7 {
                            format!("{}/{}", &d[5..7], &d[0..4])
                        } else {
                            d.clone()
                        }
                    } else {
                        String::new()
                    }
                })
                .show(ui, |plot_ui| {
                    let mut melhor_hit: Option<(f32, egui::Pos2, Color32, String, String, String)> =
                        None;

                    for (idx_p, par) in fx.pairs.iter().enumerate() {
                        let desativado = self
                            .settings
                            .disabled_forex_chart_pairs
                            .iter()
                            .any(|d| d.eq_ignore_ascii_case(&par.ticker));
                        if desativado {
                            continue;
                        }
                        let cor_linha = cores[idx_p % cores.len()];
                        let vec_pts: Vec<[f64; 2]> = par
                            .history
                            .iter()
                            .enumerate()
                            .map(|(i, h)| {
                                let y = if self.forex_base100_mode {
                                    h.base100
                                } else {
                                    h.close
                                };
                                [i as f64, y]
                            })
                            .collect();
                        let pts: PlotPoints = vec_pts.iter().copied().collect();
                        let rotulo = format!("{} ({})", par.display_code, par.country);
                        plot_ui.line(
                            Line::new(pts)
                                .name(&rotulo)
                                .color(cor_linha)
                                .width(2.0_f32),
                        );

                        if let Some((idx, dist, pos_screen)) =
                            encontrar_ponto_serie_sob_mouse(plot_ui, &vec_pts, 13.0)
                        {
                            if melhor_hit.as_ref().map_or(true, |m| dist < m.0) {
                                if let Some(h) = par.history.get(idx) {
                                    let val_str = if self.forex_base100_mode {
                                        format!("Value (Base 100): {:.2} (Quote: {:.4})", h.base100, h.close)
                                    } else {
                                        format!("Value: {:.4}", h.close)
                                    };
                                    melhor_hit = Some((
                                        dist,
                                        pos_screen,
                                        cor_linha,
                                        rotulo,
                                        val_str,
                                        format!("Date: {}", h.date),
                                    ));
                                }
                            }
                        }
                    }

                    if let Some((_dist, pos_screen, cor, titulo, valor_str, data_str)) = melhor_hit
                    {
                        desenhar_tooltip_serie_borda_segura(
                            plot_ui,
                            pos_screen,
                            cor,
                            &titulo,
                            &valor_str,
                            &data_str,
                        );
                    }
                });

            // Legenda abaixo do gráfico de Moedas & DXY
            renderizar_legenda_abaixo(ui, &legenda_moedas);
        });

        ui.add_space(8.0);

        // 4. TABELA DETALHADA DE MOEDAS CONTIDA NA JANELA COM SCROLL
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📋 Comparative Table: Currencies & US Dollar Index (DXY)")
                        .size(13.5)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("📥 Export CSV").clicked() {
                        let rows: Vec<Vec<String>> = fx
                            .pairs
                            .iter()
                            .map(|p| {
                                vec![
                                    p.display_code.clone(),
                                    p.name.clone(),
                                    p.country.clone(),
                                    format!("{:.4}", p.current_rate),
                                    format!("{:.2}", p.change_day_pct),
                                    format!("{:.2}", p.change_period_pct),
                                    format!("{:.4}", p.low_period),
                                    format!("{:.4}", p.high_period),
                                    format!("{:.2}", p.volatility_annual_pct),
                                ]
                            })
                            .collect();
                        self.export_csv_and_open(
                            "forex_dxy_comparison.csv",
                            &[
                                "Ticker",
                                "Pair",
                                "Country",
                                "Rate",
                                "DayChgPct",
                                "PeriodChgPct",
                                "Low",
                                "High",
                                "VolatilityPct",
                            ],
                            rows,
                        );
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::horizontal()
                .id_salt("forex_table_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(85.0))
                        .column(Column::initial(205.0))
                        .column(Column::initial(135.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(90.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(115.0))
                        .column(Column::initial(110.0))
                        .max_scroll_height(190.0)
                        .header(22.0, |mut header| {
                            for h in [
                                "Ticker",
                                "Pair Description",
                                "Region / Country",
                                "Current Rate",
                                "Day Chg (%)",
                                "Period Chg",
                                "Low / High",
                                "Volatility (p.a.)",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(self.pal().accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, fx.pairs.len(), |mut row| {
                                let p = &fx.pairs[row.index()];
                                row.col(|ui| {
                                    ui.label(RichText::new(&p.display_code).strong());
                                });
                                row.col(|ui| {
                                    ui.label(&p.name);
                                });
                                row.col(|ui| {
                                    ui.label(&p.country);
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(
                                            format!("{:.4}", p.current_rate),
                                        )
                                        .strong(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", p.change_day_pct))
                                            .color(cor_variacao(p.change_day_pct)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", p.change_period_pct))
                                            .color(cor_variacao(p.change_period_pct)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!(
                                        "{:.2} / {:.2}",
                                        p.low_period, p.high_period
                                    ));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", p.volatility_annual_pct));
                                });
                            });
                        });
                });
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (c): TAXAS DE JUROS GLOBAIS — SINCRONIZADA COM (b) + EIXO DE ANOS
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_juros(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        let Some(juros) = self.interest_data.clone() else {
            ui.label(
                RichText::new("Loading Central Bank interest rates...").size(15.0),
            );
            return;
        };

        let pal = self.pal();
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "🏦 Central Bank Policy Rates — Synced with (b) Currencies  |  📅 Update Cycle: Monthly",
                    )
                    .size(13.0)
                    .strong()
                    .color(pal.accent_cyan),
                );
                ui.label(
                    RichText::new(format!(
                        " | Global Date Range: {} → {} | {}",
                        self.settings.analysis_start_date,
                        self.settings.analysis_end_date,
                        juros.source_status
                    ))
                    .size(11.5)
                    .color(pal.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("🔄 Update Rates (Monthly)")
                        .on_hover_text("Manually fetch fresh Central Bank policy rates from FRED / BCB")
                        .clicked()
                    {
                        let countries = self.settings.selected_interest_countries.clone();
                        let s_yr = self.parsed_start_year();
                        let e_yr = self.parsed_end_year();
                        self.bridge.request_interest(&countries, s_yr, e_yr, true, ctx.clone());
                    }
                });
            });
        });

        ui.add_space(8.0);

        // 1. GRADE DE CARDS COM A TAXA DE JUROS ATUAL DOS EUA E DE TODOS OS OUTROS PAÍSES SELECIONADOS
        // Dividido em linhas de 3 colunas para caber 100% dentro da janela aberta sem extrapolar!
        let n_cols = 3;
        for grupo in juros.countries.chunks(n_cols) {
            ui.columns(n_cols, |cols| {
                for (i, item) in grupo.iter().enumerate() {
                    let eh_eua = item.country_code == "US";
                    let frame_card = if eh_eua {
                        hero_blue_frame()
                    } else {
                        self.card()
                    };
                    frame_card.show(&mut cols[i], |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} ({})",
                                    item.country_name, item.currency
                                ))
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let ult_data_j = item
                                        .history
                                        .last()
                                        .map(|h| h.date.as_str())
                                        .unwrap_or("--");
                                    ui.label(
                                        RichText::new(format!(
                                            "Series: {} | 📅 {}",
                                            item.series_code, ult_data_j
                                        ))
                                        .size(10.5)
                                        .color(self.pal().accent_cyan),
                                    );
                                },
                            );
                        });
                        ui.label(
                            RichText::new(&item.central_bank)
                                .size(11.0)
                                .color(self.pal().text_muted),
                        );
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Current Rate:")
                                    .size(11.5)
                                    .color(self.pal().text_primary),
                            );
                            ui.label(
                                RichText::new(
                                    format!("{:.2}% p.a.", item.current_rate_pct),
                                )
                                .size(18.5)
                                .strong()
                                .color(GREEN_UP),
                            );
                            ui.label(
                                RichText::new(format!("({:+.0} bps)", item.change_bps))
                                    .size(11.0)
                                    .color(self.pal().text_muted),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "CPI Inflation: {:.2}%  |  Real Rate: {:+.2}%",
                                item.inflation_cpi_pct, item.real_interest_pct
                            ))
                            .size(11.0)
                            .color(if item.real_interest_pct >= 0.0 {
                                self.pal().accent_cyan
                            } else {
                                RED_DOWN
                            }),
                        );
                    });
                }
            });
            ui.add_space(6.0);
        }

        ui.add_space(4.0);

        // 2. GRÁFICO DE TRAJETÓRIA DAS TAXAS DE JUROS COM DATA (ANO) NO EIXO X
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("📈 Historical Policy Interest Rate Trajectory (% p.a.)")
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(4.0);

            // Filter history by multi-year window (at least 12 months) so all countries share the exact same timeline
            let start_ym_str = format!("{:04}-01", self.parsed_start_year());
            let start_ym = start_ym_str.as_str();
            let end_ym = self.settings.analysis_end_date.get(..7).unwrap_or("2026-12");

            let filtrar_hist = |hist: &[crate::models::InterestHistoryPoint]| -> Vec<crate::models::InterestHistoryPoint> {
                let f: Vec<_> = hist
                    .iter()
                    .filter(|h| h.date.as_str() >= start_ym && h.date.as_str() <= end_ym)
                    .cloned()
                    .collect();
                if f.len() >= 12 { f } else { hist.to_vec() }
            };

            // Extrai as datas (YYYY-MM) da série mais longa para rotular o eixo dos anos
            let datas_juros: Vec<String> = juros
                .countries
                .iter()
                .map(|c| filtrar_hist(&c.history))
                .max_by_key(|h| h.len())
                .map(|h| h.into_iter().map(|pt| pt.date).collect())
                .unwrap_or_default();

            let cores = [
                GREEN_UP,
                self.pal().accent_cyan,
                ACCENT_ORANGE,
                Color32::from_rgb(244, 114, 182),
                Color32::from_rgb(167, 139, 250),
                Color32::from_rgb(250, 204, 21),
                Color32::from_rgb(52, 211, 153),
                Color32::from_rgb(251, 113, 133),
                Color32::from_rgb(96, 165, 250),
                Color32::from_rgb(192, 132, 252),
            ];

            let legenda_juros: Vec<(Color32, String)> = juros
                .countries
                .iter()
                .enumerate()
                .map(|(idx, c)| {
                    (
                        cores[idx % cores.len()],
                        format!(
                            "{} — {:.2}% ({})",
                            c.country_name, c.current_rate_pct, c.series_code
                        ),
                    )
                })
                .collect();

            Plot::new("interest_rates_plot_v02")
                .width((largura_max - 36.0).max(400.0))
                .height(250.0)
                .allow_scroll(false)
                .show_x(false)
                .show_y(false)
                .label_formatter(|_, _| String::new())
                .x_axis_label("Date / Year")
                .x_axis_formatter(move |mark, _range| {
                    let idx = mark.value.round() as isize;
                    if idx >= 0 && (idx as usize) < datas_juros.len() {
                        let d = &datas_juros[idx as usize];
                        // Converte YYYY-MM para MM/AAAA (ex.: 12/2023)
                        if d.len() >= 7 {
                            format!("{}/{}", &d[5..7], &d[0..4])
                        } else {
                            d.clone()
                        }
                    } else {
                        String::new()
                    }
                })
                .show(ui, |plot_ui| {
                    let mut melhor_hit: Option<(f32, egui::Pos2, Color32, String, String, String)> =
                        None;

                    for (idx, c) in juros.countries.iter().enumerate() {
                        let cor_linha = cores[idx % cores.len()];
                        let hist_f = filtrar_hist(&c.history);
                        let vec_pts: Vec<[f64; 2]> = hist_f
                            .iter()
                            .enumerate()
                            .map(|(i, h)| [i as f64, h.rate])
                            .collect();
                        let pts: PlotPoints = vec_pts.iter().copied().collect();
                        let rotulo = format!(
                            "{} — {:.2}% ({})",
                            c.country_name, c.current_rate_pct, c.series_code
                        );
                        plot_ui.line(
                            Line::new(pts)
                                .name(&rotulo)
                                .color(cor_linha)
                                .width(2.3_f32),
                        );

                        if let Some((i_pt, dist, pos_screen)) =
                            encontrar_ponto_serie_sob_mouse(plot_ui, &vec_pts, 13.0)
                        {
                            if melhor_hit.as_ref().map_or(true, |m| dist < m.0) {
                                if let Some(h) = hist_f.get(i_pt) {
                                    melhor_hit = Some((
                                        dist,
                                        pos_screen,
                                        cor_linha,
                                        format!("{} ({})", c.country_name, c.series_code),
                                        format!("Value: {:.2}% p.a.", h.rate),
                                        format!("Date: {}", h.date),
                                    ));
                                }
                            }
                        }
                    }

                    if let Some((_dist, pos_screen, cor, titulo, valor_str, data_str)) = melhor_hit
                    {
                        desenhar_tooltip_serie_borda_segura(
                            plot_ui,
                            pos_screen,
                            cor,
                            &titulo,
                            &valor_str,
                            &data_str,
                        );
                    }
                });

            // Legenda abaixo do gráfico de Taxas de Juros
            renderizar_legenda_abaixo(ui, &legenda_juros);
        });

        ui.add_space(8.0);

        // 3. TABELA COMPARATIVA DE POLÍTICA MONETÁRIA CONTIDA NA JANELA COM SCROLL
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📋 Monetary Policy & Real Interest Rate Comparison")
                        .size(13.5)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("📥 Export CSV").clicked() {
                        let rows: Vec<Vec<String>> = juros
                            .countries
                            .iter()
                            .map(|c| {
                                vec![
                                    c.country_name.clone(),
                                    c.central_bank.clone(),
                                    c.series_code.clone(),
                                    format!("{:.2}", c.current_rate_pct),
                                    format!("{:.2}", c.previous_rate_pct),
                                    format!("{:.2}", c.inflation_cpi_pct),
                                    format!("{:.2}", c.real_interest_pct),
                                ]
                            })
                            .collect();
                        self.export_csv_and_open(
                            "interest_rates_comparison.csv",
                            &[
                                "Country",
                                "CentralBank",
                                "Series",
                                "CurrentRatePct",
                                "PreviousRatePct",
                                "InflationCPI",
                                "RealRatePct",
                            ],
                            rows,
                        );
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::horizontal()
                .id_salt("interest_table_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(145.0))
                        .column(Column::initial(210.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(125.0))
                        .max_scroll_height(190.0)
                        .header(22.0, |mut header| {
                            for h in [
                                "Country / Bloc",
                                "Central Bank",
                                "Series / Code",
                                "Current Rate",
                                "Previous Rate",
                                "Inflation (CPI)",
                                "Ex-Post Real Rate",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(self.pal().accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, juros.countries.len(), |mut row| {
                                let c = &juros.countries[row.index()];
                                row.col(|ui| {
                                    ui.label(RichText::new(&c.country_name).strong());
                                });
                                row.col(|ui| {
                                    ui.label(&c.central_bank);
                                });
                                row.col(|ui| {
                                    ui.label(&c.series_code);
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:.2}%", c.current_rate_pct))
                                            .strong()
                                            .color(GREEN_UP),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", c.previous_rate_pct));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", c.inflation_cpi_pct));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", c.real_interest_pct))
                                            .strong()
                                            .color(cor_variacao(c.real_interest_pct)),
                                    );
                                });
                            });
                        });
                });
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (d): VARIÁVEIS SOCIOECONÔMICAS (WORLD BANK & GLOBAL CYCLES)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_macro(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        let Some(macro_resp) = self.macro_data.clone() else {
            self.card().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(
                            "Loading Macroeconomic & Market-Cycle Indicators (World Bank WDI • FRED • OECD • BLS • S&P Global PMI)...",
                        )
                        .size(14.5),
                    );
                    if ui.button("🔄 Reload Macroeconomic Data").clicked() {
                        let list = self.settings.selected_macro_countries.clone();
                        let s_yr = self.parsed_start_year();
                        let e_yr = self.parsed_end_year();
                        self.bridge.request_macro(&list, s_yr, e_yr, true, ctx.clone());
                    }
                });
            });
            return;
        };

        let pal = self.pal();

        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("🌍 Macroeconomic & Market-Cycle Indicator (14 Series):")
                        .size(13.0)
                        .strong()
                        .color(pal.accent_cyan),
                );

                for ind in &macro_resp.indicators_catalog {
                    let sel = self.selected_wb_indicator == ind.key;
                    let resp = ui.selectable_label(sel, &ind.name);
                    let dica = if !ind.source_name.is_empty() {
                        format!(
                            "{}\n• Role: {}\n• Impact: {}\n• Source: {}",
                            ind.description, ind.category, ind.market_impact, ind.source_name
                        )
                    } else {
                        ind.name.clone()
                    };
                    if resp.on_hover_text(dica).clicked() {
                        self.selected_wb_indicator = ind.key.clone();
                    }
                }
            });

            ui.separator();

            // Country Selector (same Add/Remove pattern as Tab (b) Currencies & DXY)
            let qtd_paises = self.settings.selected_macro_countries.len();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "🏳 Selected Countries ({qtd_paises} / {}):",
                        CATALOGO_PAISES_MACRO.len()
                    ))
                    .size(12.5)
                    .strong()
                    .color(pal.accent_cyan),
                );

                egui::ComboBox::from_id_salt("cb_add_macro_country")
                    .selected_text({
                        CATALOGO_PAISES_MACRO
                            .iter()
                            .find(|&&(cod, _)| cod.eq_ignore_ascii_case(&self.macro_country_to_add))
                            .map(|&(_, nome)| nome)
                            .unwrap_or(&self.macro_country_to_add)
                    })
                    .width(210.0)
                    .show_ui(ui, |ui| {
                        for &(cod, nome) in CATALOGO_PAISES_MACRO {
                            ui.selectable_value(
                                &mut self.macro_country_to_add,
                                cod.to_string(),
                                nome,
                            );
                        }
                    });

                let ja_existe_pais = self
                    .settings
                    .selected_macro_countries
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&self.macro_country_to_add));
                let pode_inserir_pais = qtd_paises < 13 && !ja_existe_pais;

                if ui
                    .add_enabled(
                        pode_inserir_pais,
                        egui::Button::new(RichText::new("➕ Add Country").strong()),
                    )
                    .on_hover_text("Add selected country to Macroeconomic analysis and save permanently")
                    .clicked()
                {
                    self.settings
                        .selected_macro_countries
                        .push(self.macro_country_to_add.clone());
                    let _ = self.settings.salvar();
                    let list = self.settings.selected_macro_countries.clone();
                    self.bridge.request_macro(
                        &list,
                        self.settings.macro_start_year,
                        self.settings.macro_end_year,
                        false,
                        ctx.clone(),
                    );
                }

                if ui
                    .button("🔗 Sync with (b) Currencies")
                    .on_hover_text("Copy the countries of the currencies currently selected in Tab (b)")
                    .clicked()
                {
                    self.settings.sincronizar_macro_com_moedas();
                    let _ = self.settings.salvar();
                    let list = self.settings.selected_macro_countries.clone();
                    self.bridge.request_macro(
                        &list,
                        self.settings.macro_start_year,
                        self.settings.macro_end_year,
                        false,
                        ctx.clone(),
                    );
                }
            });

            ui.add_space(4.0);
            let mut pais_para_retirar: Option<String> = None;
            let pode_remover_pais = self.settings.selected_macro_countries.len() > 1;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                for cod_pais in &self.settings.selected_macro_countries {
                    let nome_curto = CATALOGO_PAISES_MACRO
                        .iter()
                        .find(|&&(cod, _)| cod.eq_ignore_ascii_case(cod_pais))
                        .map(|&(_, nome)| nome)
                        .unwrap_or(cod_pais.as_str());

                    let rotulo_botao = if pode_remover_pais {
                        format!("{nome_curto}  ✖")
                    } else {
                        nome_curto.to_string()
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(rotulo_botao)
                                    .size(11.0)
                                    .color(pal.text_primary),
                            )
                            .fill(pal.bg_card_alt),
                        )
                        .on_hover_text("Click to remove this country from Macroeconomic analysis")
                        .clicked()
                        && pode_remover_pais
                    {
                        pais_para_retirar = Some(cod_pais.clone());
                    }
                }
            });

            if let Some(rem) = pais_para_retirar {
                self.settings
                    .selected_macro_countries
                    .retain(|c| !c.eq_ignore_ascii_case(&rem));
                let _ = self.settings.salvar();
                let list = self.settings.selected_macro_countries.clone();
                self.bridge.request_macro(
                    &list,
                    self.settings.macro_start_year,
                    self.settings.macro_end_year,
                    false,
                    ctx.clone(),
                );
            }

            ui.separator();

            // Garantir limites seguros para os DragValues de ano (evitando intervalos invertidos)
            if self.settings.macro_end_year < 2020 || self.settings.macro_end_year > 2025 {
                self.settings.macro_end_year = 2025;
            }
            if self.settings.macro_start_year >= self.settings.macro_end_year
                || self.settings.macro_start_year < 2000
            {
                self.settings.macro_start_year = (self.settings.macro_end_year - 6).max(2018);
            }

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("📅 Year Range:")
                        .size(12.0)
                        .strong()
                        .color(pal.text_primary),
                );

                ui.label("Start:");
                let max_start = (self.settings.macro_end_year - 1).max(2000);
                ui.add(
                    egui::DragValue::new(&mut self.settings.macro_start_year)
                        .range(2000..=max_start),
                );

                ui.label("End:");
                let min_end = (self.settings.macro_start_year + 1).min(2025);
                ui.add(
                    egui::DragValue::new(&mut self.settings.macro_end_year)
                        .range(min_end..=2025),
                );

                if ui
                    .button(RichText::new("Apply Period").strong())
                    .on_hover_text("Query macroeconomic series for the selected Start and End years")
                    .clicked()
                {
                    let _ = self.settings.salvar();
                    let list = self.settings.selected_macro_countries.clone();
                    self.bridge.request_macro(
                        &list,
                        self.settings.macro_start_year,
                        self.settings.macro_end_year,
                        false,
                        ctx.clone(),
                    );
                }

                if ui
                    .button("🔄 Update Macro Data (Online)")
                    .on_hover_text("Manually fetch fresh World Bank data and update the permanent local database")
                    .clicked()
                {
                    let _ = self.settings.salvar();
                    let list = self.settings.selected_macro_countries.clone();
                    self.bridge.request_macro(
                        &list,
                        self.settings.macro_start_year,
                        self.settings.macro_end_year,
                        true,
                        ctx.clone(),
                    );
                }

                ui.label(
                    RichText::new(format!(" | Source Status: {}", macro_resp.source_status))
                        .size(11.0)
                        .color(pal.text_muted),
                );
            });
        });

        ui.add_space(6.0);

        let ind_meta = macro_resp
            .indicators_catalog
            .iter()
            .find(|i| i.key == self.selected_wb_indicator);
        let nome_ind = ind_meta
            .map(|i| format!("{} ({})", i.name, i.unit))
            .unwrap_or_else(|| self.selected_wb_indicator.clone());

        // Card de Referência Analítica, Impacto Setorial e Fonte Oficial do Indicador Selecionado
        if let Some(meta) = ind_meta {
            self.card().show(ui, |ui| {
                ui.set_max_width(largura_max - 20.0);
                ui.horizontal_wrapped(|ui| {
                    let cor_cat = if meta.category.contains("Leading") {
                        GREEN_UP
                    } else if meta.category.contains("Coincident") {
                        pal.accent_cyan
                    } else {
                        ACCENT_ORANGE
                    };

                    ui.label(
                        RichText::new(format!("📌 {}", meta.name))
                            .size(14.0)
                            .strong()
                            .color(pal.text_primary),
                    );
                    ui.label(
                        RichText::new(format!("[{}]", if meta.category.is_empty() { "Macro Indicator" } else { &meta.category }))
                            .size(12.0)
                            .strong()
                            .color(cor_cat),
                    );
                    let freq_txt = if meta.update_frequency.is_empty() {
                        "Monthly"
                    } else {
                        &meta.update_frequency
                    };
                    let cor_freq = if freq_txt.contains("Monthly") {
                        GREEN_UP
                    } else {
                        ACCENT_ORANGE
                    };
                    ui.label(
                        RichText::new(format!("📅 Update Cycle: {freq_txt}"))
                            .size(11.5)
                            .strong()
                            .color(cor_freq),
                    );
                    ui.label(
                        RichText::new(format!(" | Series Code: {} | Unit: {}", meta.wb_code, meta.unit))
                            .size(11.5)
                            .color(pal.text_muted),
                    );

                    if !meta.reference_url.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(
                                    RichText::new("🔗 Open Official Data Source & Methodology")
                                        .strong()
                                        .color(pal.accent_cyan),
                                )
                                .on_hover_text(&meta.reference_url)
                                .clicked()
                            {
                                let _ = open::that(&meta.reference_url);
                            }
                        });
                    }
                });

                if !meta.description.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("• What it shows: {}", meta.description))
                            .size(12.0)
                            .color(pal.text_primary),
                    );
                }
                if !meta.market_impact.is_empty() || !meta.source_name.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        if !meta.market_impact.is_empty() {
                            ui.label(
                                RichText::new(format!("• Market & Sector Impact: {}", meta.market_impact))
                                    .size(11.5)
                                    .strong()
                                    .color(GREEN_UP),
                            );
                        }
                        if !meta.source_name.is_empty() {
                            ui.label(
                                RichText::new(format!("  |  📚 Official Source: {}", meta.source_name))
                                    .size(11.5)
                                    .color(pal.accent_cyan),
                            );
                        }
                    });
                }
            });
            ui.add_space(6.0);
        }

        // Filter series by active Year Range (synchronized with Global Date Range)
        let target_start = self.settings.macro_start_year;
        let target_end = self.settings.macro_end_year;
        let mut ano_min = target_end;
        let mut ano_max = target_start;
        let mut encontrou_ano = false;
        for c in &macro_resp.countries {
            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                for pt in serie.iter().filter(|pt| pt.year >= target_start && pt.year <= target_end) {
                    ano_min = ano_min.min(pt.year);
                    ano_max = ano_max.max(pt.year);
                    encontrou_ano = true;
                }
            }
        }
        if !encontrou_ano {
            ano_min = macro_resp.start_year;
            ano_max = macro_resp.end_year;
        }

        let cores_macro = [
            GREEN_UP,
            pal.accent_cyan,
            ACCENT_ORANGE,
            Color32::from_rgb(244, 114, 182),
            Color32::from_rgb(167, 139, 250),
            Color32::from_rgb(250, 204, 21),
            Color32::from_rgb(52, 211, 153),
            Color32::from_rgb(251, 113, 133),
            Color32::from_rgb(96, 165, 250),
            Color32::from_rgb(192, 132, 252),
        ];

        let legenda_macro: Vec<(Color32, String)> = macro_resp
            .countries
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                (
                    cores_macro[idx % cores_macro.len()],
                    format!("{} ({})", c.country_name, c.iso3),
                )
            })
            .collect();

        let unit_macro_tt = ind_meta
            .map(|i| i.unit.clone())
            .unwrap_or_default();

        ui.columns(2, |cols| {
            let unit_line_tt = unit_macro_tt.clone();

            self.card().show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(format!(
                        "📈 Historical Series ({ano_min} to {ano_max}) — {nome_ind}"
                    ))
                    .size(13.0)
                    .strong(),
                );
                ui.add_space(4.0);

                Plot::new("macro_series_plot_v02")
                    .height(245.0)
                    .allow_scroll(false)
                    .show_x(false)
                    .show_y(false)
                    .label_formatter(|_, _| String::new())
                    .x_grid_spacer(uniform_grid_spacer(|_| [1.0, 2.0, 5.0]))
                    .x_axis_label(format!("Year (Start: {ano_min} — End: {ano_max})"))
                    .x_axis_formatter(move |mark, _range| {
                        let ano = mark.value.round() as i32;
                        if (mark.value - ano as f64).abs() < 0.05 {
                            if ano == ano_min {
                                format!("{ano} (Start)")
                            } else if ano == ano_max {
                                format!("{ano} (End)")
                            } else {
                                format!("{ano}")
                            }
                        } else {
                            String::new()
                        }
                    })
                    .show(ui, |plot_ui| {
                        let mut melhor_hit: Option<(f32, egui::Pos2, Color32, String, String, String)> =
                            None;

                        for (idx, c) in macro_resp.countries.iter().enumerate() {
                            let cor_linha = cores_macro[idx % cores_macro.len()];
                            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                                let mut pts_vec: Vec<[f64; 2]> = serie
                                    .iter()
                                    .filter(|pt| pt.year >= ano_min && pt.year <= ano_max)
                                    .map(|pt| [pt.year as f64, pt.value])
                                    .collect();
                                if pts_vec.is_empty() {
                                    pts_vec = serie.iter().map(|pt| [pt.year as f64, pt.value]).collect();
                                }
                                let pts: PlotPoints = pts_vec.iter().copied().collect();
                                let rotulo = format!("{} ({})", c.country_name, c.iso3);
                                plot_ui.line(
                                    Line::new(pts)
                                        .name(&rotulo)
                                        .color(cor_linha)
                                        .width(2.4_f32),
                                );

                                if let Some((i_pt, dist, pos_screen)) =
                                    encontrar_ponto_serie_sob_mouse(plot_ui, &pts_vec, 13.0)
                                {
                                    if melhor_hit.as_ref().map_or(true, |m| dist < m.0) {
                                        let ano_pt = pts_vec[i_pt][0].round() as i32;
                                        let val_pt = pts_vec[i_pt][1];
                                        melhor_hit = Some((
                                            dist,
                                            pos_screen,
                                            cor_linha,
                                            rotulo,
                                            format!("Value: {:.2} {}", val_pt, unit_line_tt),
                                            format!("Date (Year): {}", ano_pt),
                                        ));
                                    }
                                }
                            }
                        }

                        if let Some((_dist, pos_screen, cor, titulo, valor_str, data_str)) =
                            melhor_hit
                        {
                            desenhar_tooltip_serie_borda_segura(
                                plot_ui,
                                pos_screen,
                                cor,
                                &titulo,
                                &valor_str,
                                &data_str,
                            );
                        }
                    });

                // Legenda posicionada abaixo do gráfico de Série Histórica
                renderizar_legenda_abaixo(ui, &legenda_macro);
            });

            self.card().show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(format!(
                        "📊 Latest Comparison ({ano_max}) — {nome_ind}"
                    ))
                    .size(13.0)
                    .strong(),
                );
                ui.add_space(4.0);

                let nomes_paises: Vec<String> = macro_resp
                    .countries
                    .iter()
                    .map(|c| c.iso3.clone())
                    .collect();
                let unit_bar_tt = unit_macro_tt.clone();

                Plot::new("macro_bar_plot_v02")
                    .height(245.0)
                    .allow_scroll(false)
                    .show_x(false)
                    .show_y(false)
                    .label_formatter(|_, _| String::new())
                    .x_grid_spacer(uniform_grid_spacer(|_| [1.0, 1.0, 1.0]))
                    .x_axis_label(format!("Compared Countries (Data: {ano_min}–{ano_max})"))
                    .x_axis_formatter(move |mark, _range| {
                        let i = mark.value.round() as isize;
                        if (mark.value - i as f64).abs() < 0.05
                            && i >= 0
                            && (i as usize) < nomes_paises.len()
                        {
                            nomes_paises[i as usize].clone()
                        } else {
                            String::new()
                        }
                    })
                    .show(ui, |plot_ui| {
                        let mut barras = Vec::new();
                        let mut dados_barras: Vec<(usize, Color32, String, f64)> = Vec::new();
                        for (idx, c) in macro_resp.countries.iter().enumerate() {
                            let cor_barra = cores_macro[idx % cores_macro.len()];
                            let val = c
                                .series
                                .get(&self.selected_wb_indicator)
                                .and_then(|s| {
                                    s.iter()
                                        .rev()
                                        .find(|pt| pt.year >= ano_min && pt.year <= ano_max)
                                        .map(|pt| pt.value)
                                })
                                .unwrap_or_else(|| {
                                    c.latest
                                        .get(&self.selected_wb_indicator)
                                        .copied()
                                        .unwrap_or(0.0)
                                });
                            barras.push(
                                Bar::new(idx as f64, val)
                                    .name(&c.country_name)
                                    .width(0.55)
                                    .fill(cor_barra),
                            );
                            dados_barras.push((
                                idx,
                                cor_barra,
                                format!("{} ({})", c.country_name, c.iso3),
                                val,
                            ));
                        }
                        plot_ui.bar_chart(BarChart::new(barras));

                        if let Some(mouse_pos) = plot_ui.ctx().pointer_hover_pos() {
                            if plot_ui.response().rect.contains(mouse_pos) {
                                if let Some(coord) = plot_ui.pointer_coordinate() {
                                    for (idx, cor_barra, rotulo, val) in dados_barras {
                                        let dx = (coord.x - idx as f64).abs();
                                        let y_min = val.min(0.0);
                                        let y_max = val.max(0.0);
                                        let top_screen = plot_ui.screen_from_plot(
                                            egui_plot::PlotPoint::new(idx as f64, val),
                                        );
                                        let dentro_y =
                                            (coord.y >= y_min && coord.y <= y_max)
                                                || (mouse_pos.y - top_screen.y).abs() <= 12.0;
                                        if dx <= 0.30 && dentro_y {
                                            desenhar_tooltip_serie_borda_segura(
                                                plot_ui,
                                                top_screen,
                                                cor_barra,
                                                &rotulo,
                                                &format!("Value: {:.2} {}", val, unit_bar_tt),
                                                &format!("Date (Year): {}", ano_max),
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    });

                // Legenda posicionada abaixo do gráfico de Barras Comparativo
                renderizar_legenda_abaixo(ui, &legenda_macro);
            });
        });

        ui.add_space(8.0);

        // 1. TABELA ORIGINAL PRESERVADA: Core World Bank WDI Socioeconomic Indicators
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "📋 Core Socioeconomic Panel — World Bank WDI (Period {ano_min} to {ano_max})"
                    ))
                    .size(13.5)
                    .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("📥 Export Core WDI CSV").clicked() {
                        let rows: Vec<Vec<String>> = macro_resp
                            .countries
                            .iter()
                            .map(|c| {
                                vec![
                                    c.country_name.clone(),
                                    c.iso3.clone(),
                                    format!("{:.2}", c.latest.get("gdp_total_bi").copied().unwrap_or(0.0)),
                                    format!("{:.0}", c.latest.get("gdp_per_capita").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("inflation_cpi").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("unemployment").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("exchange_rate").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("population_mi").copied().unwrap_or(0.0)),
                                ]
                            })
                            .collect();
                        self.export_csv_and_open(
                            &format!("worldbank_core_macro_{ano_min}_{ano_max}.csv"),
                            &[
                                "Country",
                                "ISO3",
                                "GDP_Billions_USD",
                                "GDP_Per_Capita_USD",
                                "CPI_Inflation_Pct",
                                "Unemployment_Pct",
                                "Official_FX",
                                "Population_Millions",
                            ],
                            rows,
                        );
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::horizontal()
                .id_salt("macro_table_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(165.0))
                        .column(Column::initial(120.0))
                        .column(Column::initial(125.0))
                        .column(Column::initial(115.0))
                        .column(Column::initial(115.0))
                        .column(Column::initial(125.0))
                        .column(Column::initial(130.0))
                        .max_scroll_height(195.0)
                        .header(24.0, |mut header| {
                            for h in [
                                "Country (ISO-3)",
                                "Total GDP (US$ B)",
                                "GDP per Capita (US$)",
                                "CPI Inflation (%)",
                                "Unemployment (%)",
                                "Official FX (/US$)",
                                "Population (Millions)",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(pal.accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, macro_resp.countries.len(), |mut row| {
                                let c = &macro_resp.countries[row.index()];
                                let gdp = c.latest.get("gdp_total_bi").copied().unwrap_or(0.0);
                                let gdppc = c.latest.get("gdp_per_capita").copied().unwrap_or(0.0);
                                let cpi = c.latest.get("inflation_cpi").copied().unwrap_or(0.0);
                                let unemp = c.latest.get("unemployment").copied().unwrap_or(0.0);
                                let fx = c.latest.get("exchange_rate").copied().unwrap_or(0.0);
                                let pop = c.latest.get("population_mi").copied().unwrap_or(0.0);

                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} ({})", c.country_name, c.iso3))
                                            .strong(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("US$ {:.1} B", gdp));
                                });
                                row.col(|ui| {
                                    ui.label(format!("US$ {:.0}", gdppc));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:.2}%", cpi))
                                            .color(if cpi > 5.0 { RED_DOWN } else { GREEN_UP }),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", unemp));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", fx));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.1} M", pop));
                                });
                            });
                        });
                });
        });

        ui.add_space(8.0);

        // 2. NOVA TABELA DE CICLO DE MERCADO E SETORES (Leading, Coincident & Lagging Indicators)
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "📊 Market-Cycle & Sector Activity Panel — Leading, Coincident & Lagging Indicators ({ano_max})"
                    ))
                    .size(13.5)
                    .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("📥 Export All 14 Indicators CSV").clicked() {
                        let rows: Vec<Vec<String>> = macro_resp
                            .countries
                            .iter()
                            .map(|c| {
                                vec![
                                    c.country_name.clone(),
                                    c.iso3.clone(),
                                    format!("{:.2}", c.latest.get("gdp_total_bi").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("gdp_growth_pct").copied().unwrap_or(0.0)),
                                    format!("{:.0}", c.latest.get("gdp_per_capita").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("inflation_cpi").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("interest_rate_pct").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("yield_curve_10y2y").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("employment_nfp_yoy").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("unemployment").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("retail_sales_yoy").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("industrial_production_yoy").copied().unwrap_or(0.0)),
                                    format!("{:.2}", c.latest.get("building_permits_yoy").copied().unwrap_or(0.0)),
                                    format!("{:.1}", c.latest.get("pmi_composite").copied().unwrap_or(0.0)),
                                ]
                            })
                            .collect();
                        self.export_csv_and_open(
                            &format!("macro_market_cycle_all14_{ano_min}_{ano_max}.csv"),
                            &[
                                "Country",
                                "ISO3",
                                "GDP_Billions_USD",
                                "Real_GDP_Growth_YoY_Pct",
                                "GDP_Per_Capita_USD",
                                "CPI_Inflation_Pct",
                                "Policy_Interest_Rate_Pct",
                                "Yield_Curve_10Y2Y_Pct",
                                "Employment_NFP_YoY_Pct",
                                "Unemployment_Pct",
                                "Retail_Sales_YoY_Pct",
                                "Industrial_Production_YoY_Pct",
                                "Building_Permits_YoY_Pct",
                                "PMI_Composite_Index",
                            ],
                            rows,
                        );
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::horizontal()
                .id_salt("macro_cycle_table_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(155.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(118.0))
                        .column(Column::initial(115.0))
                        .column(Column::initial(112.0))
                        .column(Column::initial(115.0))
                        .column(Column::initial(120.0))
                        .column(Column::initial(110.0))
                        .max_scroll_height(205.0)
                        .header(24.0, |mut header| {
                            for h in [
                                "Country (ISO-3)",
                                "Real GDP (% YoY)",
                                "Policy Rate (%)",
                                "Yield Curve (10Y-2Y)",
                                "NFP / Payrolls (%)",
                                "Retail Sales (%)",
                                "Ind. Prod. (% YoY)",
                                "Building Permits (%)",
                                "PMI (50+ = Exp.)",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(pal.accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(22.0, macro_resp.countries.len(), |mut row| {
                                let c = &macro_resp.countries[row.index()];
                                let gdp_g = c.latest.get("gdp_growth_pct").copied().unwrap_or(0.0);
                                let rate = c.latest.get("interest_rate_pct").copied().unwrap_or(0.0);
                                let yc = c.latest.get("yield_curve_10y2y").copied().unwrap_or(0.0);
                                let nfp = c.latest.get("employment_nfp_yoy").copied().unwrap_or(0.0);
                                let retail = c.latest.get("retail_sales_yoy").copied().unwrap_or(0.0);
                                let ind_p = c.latest.get("industrial_production_yoy").copied().unwrap_or(0.0);
                                let permits = c.latest.get("building_permits_yoy").copied().unwrap_or(0.0);
                                let pmi = c.latest.get("pmi_composite").copied().unwrap_or(50.0);

                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} ({})", c.country_name, c.iso3))
                                            .strong(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", gdp_g))
                                            .strong()
                                            .color(cor_variacao(gdp_g)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", rate));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", yc))
                                            .strong()
                                            .color(if yc >= 0.0 { GREEN_UP } else { RED_DOWN }),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", nfp))
                                            .color(cor_variacao(nfp)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", retail))
                                            .color(cor_variacao(retail)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", ind_p))
                                            .color(cor_variacao(ind_p)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:+.2}%", permits))
                                            .color(cor_variacao(permits)),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:.1}", pmi))
                                            .strong()
                                            .color(if pmi >= 50.0 { GREEN_UP } else { RED_DOWN }),
                                    );
                                });
                            });
                        });
                });
        });

        ui.add_space(8.0);

        // 3. Geographic Ranking by % Change between Start Year and End Year
        let mut ranking: Vec<(String, String, f64, f64, f64)> = Vec::new();
        for c in &macro_resp.countries {
            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                let filtrada: Vec<_> = serie
                    .iter()
                    .filter(|pt| pt.year >= ano_min && pt.year <= ano_max)
                    .collect();
                let (first_opt, last_opt) = if filtrada.len() >= 2 {
                    (filtrada.first().copied(), filtrada.last().copied())
                } else {
                    (serie.first(), serie.last())
                };
                if let (Some(first), Some(last)) = (first_opt, last_opt) {
                    let pct = if first.value.abs() > 1e-9 {
                        ((last.value - first.value) / first.value.abs()) * 100.0
                    } else {
                        0.0
                    };
                    ranking.push((
                        c.country_name.clone(),
                        c.iso3.clone(),
                        first.value,
                        last.value,
                        pct,
                    ));
                }
            }
        }
        ranking.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

        if !ranking.is_empty() {
            self.card().show(ui, |ui| {
                ui.set_max_width(largura_max - 20.0);
                ui.label(
                    RichText::new(format!(
                        "🗺 Geographic Ranking ({ano_min} → {ano_max}) — {nome_ind}"
                    ))
                    .size(13.5)
                    .strong()
                    .color(pal.accent_cyan),
                );
                ui.add_space(4.0);
                for (pos, (name, iso3, v_ini, v_fim, pct)) in ranking.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("#{}  {} ({})", pos + 1, name, iso3))
                                .strong()
                                .color(pal.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("{:.2} → {:.2}", v_ini, v_fim))
                                .size(11.5)
                                .color(pal.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            badge_variacao(ui, *pct);
                        });
                    });
                }
            });
        }

        ui.add_space(8.0);

        // 4. GUIA DE ANÁLISE DE MERCADO (Leading vs Coincident vs Lagging) & FONTES OFICIAIS DOS DADOS
        self.card().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("🧭 How to Use Macroeconomic Indicators for Market Analysis & Official Data Sources")
                    .size(14.0)
                    .strong()
                    .color(pal.accent_cyan),
            );
            ui.add_space(6.0);

            ui.columns(3, |cols| {
                egui::Frame::none()
                    .fill(pal.bg_card_alt)
                    .stroke(Stroke::new(1.0_f32, GREEN_UP))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(&mut cols[0], |ui| {
                        ui.label(
                            RichText::new("1. Leading Indicators (Forecast Future Moves)")
                                .size(12.5)
                                .strong()
                                .color(GREEN_UP),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "• PMI (Purchasing Managers' Index): > 50 signals expansion in manufacturing & services; < 50 signals contraction.\n\
                                 • Yield Curve (10Y - 2Y): Inversion (< 0%) warns of recession risk; steepening signals recovery.\n\
                                 • Building Permits & Home Sales: Anticipate real estate cycles, credit demand, and consumer confidence.\n\
                                 • Policy Interest Rates: Forward driver of equity valuation multiples (DCF) and FX flows."
                            )
                            .size(11.0)
                            .color(pal.text_primary),
                        );
                    });

                egui::Frame::none()
                    .fill(pal.bg_card_alt)
                    .stroke(Stroke::new(1.0_f32, pal.accent_cyan))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(&mut cols[1], |ui| {
                        ui.label(
                            RichText::new("2. Coincident Indicators (Current Economy State)")
                                .size(12.5)
                                .strong()
                                .color(pal.accent_cyan),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "• Industrial Production & Inventories: Reflect real-time manufacturing output, energy usage, and supply-chain restocking.\n\
                                 • Retail Sales & Consumption: Measure household spending strength (the primary 60%–70% driver of GDP).\n\
                                 • Employment (NFP & Jobless Claims): Non-Farm Payrolls show active hiring; rising jobless claims warn of labor market deterioration."
                            )
                            .size(11.0)
                            .color(pal.text_primary),
                        );
                    });

                egui::Frame::none()
                    .fill(pal.bg_card_alt)
                    .stroke(Stroke::new(1.0_f32, ACCENT_ORANGE))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(&mut cols[2], |ui| {
                        ui.label(
                            RichText::new("3. Lagging Indicators (Confirm Active Trends)")
                                .size(12.5)
                                .strong()
                                .color(ACCENT_ORANGE),
                        );
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(
                                "• Gross Domestic Product (Nominal GDP, Real GDP Growth & GDP per Capita): Confirm broad economic expansion or contraction.\n\
                                 • Consumer Price Index (CPI Inflation): Confirms price trends, purchasing power erosion, and central bank rate paths.\n\
                                 • Unemployment Rate: Confirms structural labor slack or tightness after economic turning points."
                            )
                            .size(11.0)
                            .color(pal.text_primary),
                        );
                    });
            });

            ui.add_space(8.0);
            ui.label(
                RichText::new("📚 Reference Directory — Indicators, Sector Impacts & Official Data Sources (Click '🔗 Source' to open documentation):")
                    .size(12.5)
                    .strong()
                    .color(pal.text_primary),
            );
            ui.add_space(4.0);

            egui::ScrollArea::horizontal()
                .id_salt("macro_sources_reference_hscroll")
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(200.0))
                        .column(Column::initial(125.0))
                        .column(Column::initial(110.0))
                        .column(Column::initial(235.0))
                        .column(Column::initial(220.0))
                        .column(Column::initial(90.0))
                        .max_scroll_height(230.0)
                        .header(22.0, |mut header| {
                            for h in [
                                "Indicator",
                                "Cycle Role",
                                "Update Cycle",
                                "Market & Sector Impact",
                                "Official Data Source & Series ID",
                                "Reference Link",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(pal.accent_cyan));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(24.0, macro_resp.indicators_catalog.len(), |mut row| {
                                let ind = &macro_resp.indicators_catalog[row.index()];
                                let cor_cat = if ind.category.contains("Leading") {
                                    GREEN_UP
                                } else if ind.category.contains("Coincident") {
                                    pal.accent_cyan
                                } else {
                                    ACCENT_ORANGE
                                };
                                let freq_txt = if ind.update_frequency.is_empty() {
                                    "Monthly"
                                } else {
                                    &ind.update_frequency
                                };
                                let cor_freq = if freq_txt.contains("Monthly") {
                                    GREEN_UP
                                } else {
                                    ACCENT_ORANGE
                                };
                                row.col(|ui| {
                                    if ui
                                        .selectable_label(self.selected_wb_indicator == ind.key, &ind.name)
                                        .on_hover_text(&ind.description)
                                        .clicked()
                                    {
                                        self.selected_wb_indicator = ind.key.clone();
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(&ind.category).size(11.0).strong().color(cor_cat));
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(freq_txt).size(11.0).strong().color(cor_freq));
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(&ind.market_impact).size(11.0));
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(&ind.source_name).size(11.0).color(pal.text_muted));
                                });
                                row.col(|ui| {
                                    if !ind.reference_url.is_empty()
                                        && ui
                                            .small_button("🔗 Source")
                                            .on_hover_text(&ind.reference_url)
                                            .clicked()
                                    {
                                        let _ = open::that(&ind.reference_url);
                                    }
                                });
                            });
                        });
                });
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (e): DASHBOARD & SCANNER INTERATIVO (ESTILO DAX1, DAX2 E DAX4)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_dashboard_scanner(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        let Some(ov) = self.overview_data.clone() else {
            ui.label(
                RichText::new("Loading Market Dashboard, Gainers/Losers, and Scanner...")
                    .size(15.0),
            );
            return;
        };

        let mut acao_para_abrir: Option<String> = None;

        // 1. GAINERS & LOSERS PANEL (DAX1.PNG)
        let data_pregao_ov: String = ov
            .assets
            .first()
            .map(|a| a.last_date.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.settings.analysis_end_date.clone());

        ui.columns(2, |cols| {
            self.card().show(&mut cols[0], |ui| {
                ui.set_min_height(112.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⬆ Top Gainers")
                            .size(15.0)
                            .strong()
                            .color(GREEN_UP),
                    );
                    ui.label(
                        RichText::new(format!(" (📅 {data_pregao_ov})"))
                            .size(11.5)
                            .color(self.pal().text_muted),
                    );
                });
                ui.add_space(6.0);
                egui::Grid::new("gainers_grid_4cols")
                    .num_columns(4)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        for linha in ov.gainers.chunks(4) {
                            for item in linha {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&item.code)
                                            .size(11.5)
                                            .strong()
                                            .color(self.pal().text_primary),
                                    );
                                    let btn = ui.add_sized(
                                        [76.0, 24.0],
                                        egui::Button::new(
                                            RichText::new(
                                                format!("+{:.2}%", item.change_pct.abs()),
                                            )
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::from_rgb(10, 15, 22)),
                                        )
                                        .fill(GREEN_UP)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                    let dt_item = if item.last_date.is_empty() {
                                        data_pregao_ov.as_str()
                                    } else {
                                        item.last_date.as_str()
                                    };
                                    if btn
                                        .on_hover_text(format!(
                                            "{} | Price: {:.2} | Date: {}",
                                            item.code, item.price, dt_item
                                        ))
                                        .clicked()
                                    {
                                        acao_para_abrir = Some(item.code.clone());
                                    }
                                });
                            }
                            ui.end_row();
                        }
                    });
            });

            self.card().show(&mut cols[1], |ui| {
                ui.set_min_height(112.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⬇ Top Losers")
                            .size(15.0)
                            .strong()
                            .color(RED_DOWN),
                    );
                    ui.label(
                        RichText::new(format!(" (📅 {data_pregao_ov})"))
                            .size(11.5)
                            .color(self.pal().text_muted),
                    );
                });
                ui.add_space(6.0);
                egui::Grid::new("losers_grid_4cols")
                    .num_columns(4)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        for linha in ov.losers.chunks(4) {
                            for item in linha {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&item.code)
                                            .size(11.5)
                                            .strong()
                                            .color(self.pal().text_primary),
                                    );
                                    let btn = ui.add_sized(
                                        [76.0, 24.0],
                                        egui::Button::new(
                                            RichText::new(
                                                format!("-{:.2}%", item.change_pct.abs()),
                                            )
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                        )
                                        .fill(RED_DOWN)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                    let dt_item = if item.last_date.is_empty() {
                                        data_pregao_ov.as_str()
                                    } else {
                                        item.last_date.as_str()
                                    };
                                    if btn
                                        .on_hover_text(format!(
                                            "{} | Price: {:.2} | Date: {}",
                                            item.code, item.price, dt_item
                                        ))
                                        .clicked()
                                    {
                                        acao_para_abrir = Some(item.code.clone());
                                    }
                                });
                            }
                            ui.end_row();
                        }
                    });
            });
        });

        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("View Mode:")
                    .size(13.5)
                    .strong()
                    .color(self.pal().accent_cyan),
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::CardsDax2,
                "📊 Cards with Sparklines (Dax2 / TradeMap Style)",
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::ScannerDax4,
                "🔍 Stocks Scanner & Filters (Dax4 / xStation Style)",
            );
        });

        ui.add_space(6.0);

        match self.dashboard_mode {
            DashboardViewMode::CardsDax2 => {
                let n_cols = 3;
                for chunk in ov.assets.chunks(n_cols) {
                    ui.columns(n_cols, |cols| {
                        for (i, ativo) in chunk.iter().enumerate() {
                            self.card().show(&mut cols[i], |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&ativo.short_code)
                                                .size(16.5)
                                                .strong()
                                                .color(self.pal().text_primary),
                                        );
                                        let dt_card = if ativo.last_date.is_empty() {
                                            data_pregao_ov.as_str()
                                        } else {
                                            ativo.last_date.as_str()
                                        };
                                        ui.label(
                                            RichText::new(format!("{} • 📅 {}", ativo.name, dt_card))
                                                .size(10.5)
                                                .color(self.pal().text_muted),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            badge_variacao(ui, ativo.change_day_pct);
                                            ui.label(
                                                RichText::new(formatar_moeda(
                                                    ativo.price,
                                                    &ativo.currency,
                                                ))
                                                .size(14.0)
                                                .strong()
                                                .color(self.pal().text_primary),
                                            );
                                        },
                                    );
                                });

                                ui.add_space(5.0);
                                let w = ui.available_width().max(150.0);
                                desenhar_sparkline(
                                    ui,
                                    &ativo.sparkline,
                                    Vec2::new(w, 60.0),
                                    ativo.change_day_pct >= 0.0,
                                );
                                ui.add_space(5.0);

                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
                                    ui.label(
                                        RichText::new(format!("Mcap: {:.1}B", ativo.market_cap_bi))
                                            .size(10.5)
                                            .color(self.pal().text_muted),
                                    );
                                    ui.label(
                                        RichText::new(format!("DY: {:.2}%", ativo.dy))
                                            .size(10.5)
                                            .color(self.pal().text_muted),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "12M: {:+.1}%",
                                            ativo.change_12m_pct
                                        ))
                                        .size(10.5)
                                        .strong()
                                        .color(cor_variacao(ativo.change_12m_pct)),
                                    );

                                    if ui.small_button("🔍 Details").clicked() {
                                        acao_para_abrir = Some(ativo.short_code.clone());
                                    }
                                });
                            });
                        }
                    });
                    ui.add_space(8.0);
                }
            }

            DashboardViewMode::ScannerDax4 => {
                self.card().show(ui, |ui| {
                    ui.set_max_width(largura_max - 20.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Country:").strong());
                        egui::ComboBox::from_id_salt("scanner_country_cb")
                            .selected_text(&self.scanner_country)
                            .show_ui(ui, |ui| {
                                for p in [
                                    "All countries",
                                    "Brazil",
                                    "United States",
                                    "Germany",
                                    "Netherlands",
                                ] {
                                    ui.selectable_value(
                                        &mut self.scanner_country,
                                        p.to_string(),
                                        p,
                                    );
                                }
                            });

                        ui.separator();
                        ui.label(RichText::new("Search instrument:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.scanner_search)
                                .desired_width(150.0)
                                .hint_text("Name or ticker..."),
                        );

                        if ui.button("Clear Filters").clicked() {
                            self.scanner_country = "All countries".to_string();
                            self.scanner_search.clear();
                            self.filter_min_mcap = 0.0;
                            self.filter_max_pe = 100.0;
                            self.filter_min_dy = 0.0;
                            self.filter_min_roe = -50.0;
                        }
                    });

                    ui.add_space(6.0);
                    ui.columns(4, |cols| {
                        cols[0].vertical(|ui| {
                            ui.label(
                                RichText::new("Min Market Cap (B)")
                                    .size(11.0)
                                    .color(self.pal().text_muted),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.filter_min_mcap, 0.0..=1000.0)
                                    .suffix(" B"),
                            );
                        });
                        cols[1].vertical(|ui| {
                            ui.label(
                                RichText::new("Max P/E Ratio")
                                    .size(11.0)
                                    .color(self.pal().text_muted),
                            );
                            ui.add(egui::Slider::new(&mut self.filter_max_pe, 2.0..=100.0));
                        });
                        cols[2].vertical(|ui| {
                            ui.label(
                                RichText::new("Min Dividend Yield (%)")
                                    .size(11.0)
                                    .color(self.pal().text_muted),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.filter_min_dy, 0.0..=20.0).suffix("%"),
                            );
                        });
                        cols[3].vertical(|ui| {
                            ui.label(
                                RichText::new("Min ROE (%)")
                                    .size(11.0)
                                    .color(self.pal().text_muted),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.filter_min_roe, -20.0..=80.0)
                                    .suffix("%"),
                            );
                        });
                    });
                });

                ui.add_space(8.0);

                let termo = self.scanner_search.to_lowercase();
                let filtrados: Vec<_> = ov
                    .assets
                    .iter()
                    .filter(|a| {
                        let ok_pais = self.scanner_country == "All countries"
                            || a.country.eq_ignore_ascii_case(&self.scanner_country);
                        let ok_busca = termo.is_empty()
                            || a.name.to_lowercase().contains(&termo)
                            || a.short_code.to_lowercase().contains(&termo)
                            || a.sector.to_lowercase().contains(&termo);
                        let ok_mcap = a.market_cap_bi >= self.filter_min_mcap;
                        let ok_pe = a.pe <= self.filter_max_pe;
                        let ok_dy = a.dy >= self.filter_min_dy;
                        let ok_roe = a.roe >= self.filter_min_roe;
                        ok_pais && ok_busca && ok_mcap && ok_pe && ok_dy && ok_roe
                    })
                    .collect();

                self.card().show(ui, |ui| {
                    ui.set_max_width(largura_max - 20.0);
                    egui::ScrollArea::horizontal()
                        .id_salt("scanner_table_hscroll")
                        .show(ui, |ui| {
                            TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .column(Column::initial(155.0))
                                .column(Column::initial(155.0))
                                .column(Column::initial(100.0))
                                .column(Column::initial(105.0))
                                .column(Column::initial(70.0))
                                .column(Column::initial(70.0))
                                .column(Column::initial(95.0))
                                .column(Column::initial(70.0))
                                .column(Column::initial(75.0))
                                .column(Column::initial(85.0))
                                .max_scroll_height(330.0)
                                .header(24.0, |mut header| {
                                    for h in [
                                        "FULL NAME",
                                        "INDUSTRY / SECTOR",
                                        "COUNTRY",
                                        "MARKET CAP",
                                        "EPS",
                                        "P/E",
                                        "DIVIDEND YIELD",
                                        "P/B",
                                        "ROE (%)",
                                        "ACTION",
                                    ] {
                                        header.col(|ui| {
                                            ui.label(
                                                RichText::new(h)
                                                    .size(11.0)
                                                    .strong()
                                                    .color(self.pal().text_muted),
                                            );
                                        });
                                    }
                                })
                                .body(|body| {
                                    body.rows(24.0, filtrados.len(), |mut row| {
                                        let a = filtrados[row.index()];
                                        row.col(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} ({})",
                                                    a.name, a.short_code
                                                ))
                                                .strong()
                                                .color(self.pal().text_primary),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(&a.sector);
                                        });
                                        row.col(|ui| {
                                            ui.label(&a.country);
                                        });
                                        row.col(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.2} bn", a.market_cap_bi))
                                                    .strong(),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.2}", a.eps))
                                                    .color(cor_variacao(a.eps)),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{:.2}", a.pe));
                                        });
                                        row.col(|ui| {
                                            ui.label(format!("{:.2}%", a.dy));
                                        });
                                        row.col(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.2}", a.pb))
                                                    .color(GREEN_UP),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:.1}%", a.roe))
                                                    .color(cor_variacao(a.roe)),
                                            );
                                        });
                                        row.col(|ui| {
                                            if ui.small_button("📈 Analyze").clicked() {
                                                acao_para_abrir = Some(a.short_code.clone());
                                            }
                                        });
                                    });
                                });
                        });
                });
            }
        }

        if let Some(tk) = acao_para_abrir {
            self.selecionar_acao(&tk, ctx);
            self.active_tab = ActiveTab::Stocks;
        }
    }
}

impl eframe::App for DaxEconomiaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized {
            self.initialized = true;
            self.last_indices_refresh = Instant::now();
            self.disparar_carga_inicial(ctx);
        }

        if !self.settings.offline_mode
            && self.last_indices_refresh.elapsed() >= Duration::from_secs(600)
        {
            self.renovar_dados_mercado_diario(ctx);
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        self.processar_mensagens_bridge();

        let pal = self.pal();

        // Top Bar with Global Indices Mini-Cards, Sparklines & Global Date Range
        egui::TopBottomPanel::top("top_dax1_bar")
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                self.renderizar_barra_superior_dax1(ui, ctx);
            });

        // Bottom Status Bar
        egui::TopBottomPanel::bottom("bottom_status_bar")
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mode_tag = if self.settings.offline_mode {
                        "📡 OFFLINE"
                    } else {
                        "🟢 ONLINE"
                    };
                    ui.label(
                        RichText::new(format!("{mode_tag}  |  Status: {}", self.status_message))
                            .size(11.5)
                            .color(pal.text_muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("DAX - Data Analysis Exchange (v0.3)  |  Rust (egui) ↔ Python (yfinance • FRED • World Bank)")
                                .size(11.0)
                                .color(pal.accent_cyan),
                        );
                    });
                });
            });

        // Fixed Left Sidebar (240px)
        egui::SidePanel::left("left_sidebar_dax_v02_fixed")
            .resizable(false)
            .exact_width(240.0)
            .min_width(240.0)
            .max_width(240.0)
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_sidebar)
                    .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                self.renderizar_menu_lateral(ui);
            });

        // Central Main Area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_primary)
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
                let largura_disponivel = (ui.available_width() - 16.0).max(580.0);

                if let Some(err) = &self.error_banner {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(90, 20, 35))
                        .stroke(Stroke::new(1.0_f32, RED_DOWN))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("⚠ {err}")).color(Color32::WHITE));
                            });
                        });
                    ui.add_space(6.0);
                }

                egui::ScrollArea::vertical()
                    .id_salt("main_central_vscroll_v02")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            match self.active_tab {
                                ActiveTab::Stocks => {
                                    self.renderizar_aba_acoes(ui, ctx, largura_disponivel)
                                }
                                ActiveTab::Forex => {
                                    self.renderizar_aba_moedas(ui, ctx, largura_disponivel)
                                }
                                ActiveTab::InterestRates => {
                                    self.renderizar_aba_juros(ui, ctx, largura_disponivel)
                                }
                                ActiveTab::MacroSocioeconomic => {
                                    self.renderizar_aba_macro(ui, ctx, largura_disponivel)
                                }
                                ActiveTab::DashboardScanner => self
                                    .renderizar_aba_dashboard_scanner(
                                        ui,
                                        ctx,
                                        largura_disponivel,
                                    ),
                            }
                            ui.add_space(18.0);
                        });
                    });
            });
    }
}
