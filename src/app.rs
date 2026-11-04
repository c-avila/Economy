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
use crate::config::{UserSettings, CATALOGO_MOEDAS_PAISES};
use crate::models::{
    ForexResponse, InterestResponse, MacroResponse, MarketOverviewResponse, StockDetailResponse,
};
use crate::theme::{
    aplicar_tema_dax, badge_variacao, card_frame, cor_variacao, desenhar_sparkline, formatar_moeda,
    hero_blue_frame, renderizar_legenda_abaixo, ACCENT_CYAN, ACCENT_ORANGE, BG_CARD, BG_CARD_ALT,
    BG_SIDEBAR, BORDER_SUBTLE, GREEN_UP, RED_DOWN, TEXT_LIGHT, TEXT_MUTED,
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
        aplicar_tema_dax(&cc.egui_ctx);
        let mut settings = UserSettings::carregar();
        settings.sincronizar_paises_com_moedas();
        let ticker_inicial = settings.selected_ticker.clone();

        Self {
            settings,
            bridge: PythonBridge::new(),
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
            dashboard_mode: DashboardViewMode::CardsDax2,
            scanner_search: String::new(),
            scanner_country: "Todos os países".to_string(),
            filter_min_mcap: 0.0,
            filter_max_pe: 100.0,
            filter_min_dy: 0.0,
            filter_min_roe: -50.0,
            status_message: "Iniciando DAX - Data Analysis Exchange (v0.2)...".into(),
            error_banner: None,
            last_indices_refresh: Instant::now(),
        }
    }

    /// Dispara o carregamento inicial de todas as seções em background.
    fn disparar_carga_inicial(&mut self, ctx: &egui::Context) {
        self.settings.sincronizar_paises_com_moedas();
        let favs = self.settings.favorite_tickers.clone();
        let ticker = self.settings.selected_ticker.clone();
        let period = self.settings.selected_period.clone();
        let curr = self.settings.display_currency.clone();
        let pairs = self.settings.selected_forex_pairs.clone();
        let int_countries = self.settings.selected_interest_countries.clone();
        let macro_countries = self.settings.selected_macro_countries.clone();

        self.bridge.request_overview(&favs, false, ctx.clone());
        self.bridge
            .request_stock(&ticker, &period, &curr, false, ctx.clone());
        self.bridge
            .request_forex(&pairs, &period, false, ctx.clone());
        self.bridge
            .request_interest(&int_countries, 2019, false, ctx.clone());
        self.bridge.request_macro(
            &macro_countries,
            self.settings.macro_start_year,
            self.settings.macro_end_year,
            false,
            ctx.clone(),
        );
    }

    /// Sincroniza os países das Abas (c) e (d) com as moedas selecionadas na Aba (b) e recarrega os dados.
    fn atualizar_moedas_e_sincronizar_paises(&mut self, ctx: &egui::Context) {
        self.settings.sincronizar_paises_com_moedas();
        let _ = self.settings.salvar();

        let pairs = self.settings.selected_forex_pairs.clone();
        let period = self.settings.selected_period.clone();
        let int_countries = self.settings.selected_interest_countries.clone();
        let macro_countries = self.settings.selected_macro_countries.clone();

        self.bridge
            .request_forex(&pairs, &period, false, ctx.clone());
        self.bridge
            .request_interest(&int_countries, 2019, false, ctx.clone());
        self.bridge.request_macro(
            &macro_countries,
            self.settings.macro_start_year,
            self.settings.macro_end_year,
            false,
            ctx.clone(),
        );
    }

    /// Processa mensagens assíncronas recebidas da ponte Python.
    fn processar_mensagens_bridge(&mut self) {
        for msg in self.bridge.poll_messages() {
            match msg {
                BridgeMessage::OverviewLoaded(data) => {
                    self.status_message = format!(
                        "Visão geral atualizada ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.overview_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::StockLoaded(data) => {
                    self.status_message = format!(
                        "Ação {} carregada em {} ({}) — {}",
                        data.short_code, data.display_currency, data.source_status, data.updated_at
                    );
                    self.stock_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::ForexLoaded(data) => {
                    self.status_message = format!(
                        "Moedas e DXY atualizados ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.forex_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::InterestLoaded(data) => {
                    self.status_message = format!(
                        "Taxas de juros sincronizadas ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.interest_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::MacroLoaded(data) => {
                    self.status_message = format!(
                        "Dados World Bank sincronizados ({}) — {}",
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
                BridgeMessage::Error(err) => {
                    self.error_banner = Some(err.clone());
                    self.status_message = "Atenção: verifique o aviso exibido no topo.".into();
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
                RichText::new("🌐 ÍNDICES DAS PRINCIPAIS BOLSAS MUNDIAIS")
                    .size(12.0)
                    .strong()
                    .color(ACCENT_CYAN),
            );
            ui.label(
                RichText::new(
                    "(Brasil • Wall Street • Nasdaq • Japão • China • Alemanha • Inglaterra • França • Espanha • Argentina)",
                )
                .size(11.0)
                .color(TEXT_MUTED),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(
                        RichText::new("🔄 Atualizar Agora")
                            .size(12.0)
                            .strong()
                            .color(GREEN_UP),
                    )
                    .on_hover_text("Atualiza imediatamente os índices das bolsas mundiais e demais cotações")
                    .clicked()
                {
                    self.last_indices_refresh = Instant::now();
                    let favs = self.settings.favorite_tickers.clone();
                    let ticker = self.settings.selected_ticker.clone();
                    let period = self.settings.selected_period.clone();
                    let curr = self.settings.display_currency.clone();
                    self.bridge.request_overview(&favs, true, ctx.clone());
                    self.bridge
                        .request_stock(&ticker, &period, &curr, true, ctx.clone());
                    self.atualizar_moedas_e_sincronizar_paises(ctx);
                }

                ui.separator();

                let ultima_hora = self
                    .overview_data
                    .as_ref()
                    .map(|o| o.updated_at.as_str())
                    .unwrap_or("--:--:--");
                ui.label(
                    RichText::new(format!(
                        "⏱ Auto (10 min): {:02}:{:02}  |  Última: {}",
                        min_rest, seg_rest, ultima_hora
                    ))
                    .size(11.0)
                    .color(TEXT_MUTED),
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
                                .fill(BG_CARD)
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
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
                                                            .color(TEXT_LIGHT),
                                                    )
                                                    .frame(false),
                                                )
                                                .on_hover_text(format!(
                                                    "Clique para abrir o gráfico detalhado de {} ({})",
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
                                                            format!("{sinal}{:.2}%", item.change_pct)
                                                                .replace('.', ","),
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
                                                        .color(ACCENT_CYAN),
                                                );
                                                ui.label(
                                                    RichText::new(&item.subtitle)
                                                        .size(10.0)
                                                        .color(TEXT_MUTED),
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
                                "Carregando bolsas mundiais (IBOV, Wall St, Nasdaq, Nikkei, Xangai, DAX, FTSE, CAC, IBEX, Merval)...",
                            )
                            .color(TEXT_MUTED),
                        );
                    }
                });
            });

        if let Some(cod) = indice_clicado {
            self.selecionar_acao(&cod, ctx);
            self.active_tab = ActiveTab::Stocks;
        }
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
                RichText::new("v0.2")
                    .size(12.5)
                    .strong()
                    .color(ACCENT_CYAN),
            );
        });
        ui.label(
            RichText::new("Data Analysis Exchange")
                .size(13.5)
                .strong()
                .color(TEXT_LIGHT),
        );
        ui.label(
            RichText::new("Ações • Câmbio DXY • Juros • Macro")
                .size(11.0)
                .color(TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        let abas = [
            (
                ActiveTab::DashboardScanner,
                "⚡ (e) Dashboard & Scanner",
                "Altas/Baixas (Dax1), Cards (Dax2) e Stocks Scanner (Dax4)",
            ),
            (
                ActiveTab::Stocks,
                "📈 (a) Ações & Candles",
                "Análise OHLCV, Candlesticks, Médias Móveis e Múltiplos (Dax3)",
            ),
            (
                ActiveTab::Forex,
                "💵 (b) Moedas & Dólar (DXY)",
                "Até 10 moedas, 3 destaques vs USD, filtro de curvas e DXY=",
            ),
            (
                ActiveTab::InterestRates,
                "🏦 (c) Taxas de Juros (BCs)",
                "Taxas atuais de todos os países selecionados em (b) + Juro Real",
            ),
            (
                ActiveTab::MacroSocioeconomic,
                "🌍 (d) Socioeconomia (WB)",
                "PIB, PIB per capita, Inflação, Desemprego, Câmbio e População",
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
                                .color(if selecionado { Color32::WHITE } else { TEXT_LIGHT }),
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
            RichText::new("⭐ Ativos Favoritos (Clique ✖ p/ remover)")
                .size(11.5)
                .strong()
                .color(ACCENT_CYAN),
        );
        ui.add_space(4.0);

        // Campo rápido para inserir novo favorito direto na barra lateral
        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.new_fav_input)
                    .desired_width(155.0)
                    .hint_text("Novo (ex: WEGE3)"),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (enter || ui.button("➕").on_hover_text("Adicionar aos favoritos").clicked())
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
                                            BG_CARD_ALT
                                        }),
                                    );
                                    if btn_open
                                        .on_hover_text(format!("Abrir análise de {fav}"))
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
                                        .fill(BG_CARD),
                                    );
                                    if btn_del
                                        .on_hover_text(format!(
                                            "Remover {fav} dos Ativos Favoritos"
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
            self.status_message = format!("Ativo '{rem}' removido dos favoritos.");
        }

        if let Some(tk) = ticker_clicado {
            self.selecionar_acao(&tk, ui.ctx());
            self.active_tab = ActiveTab::Stocks;
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new("🔒 LGPD: Dados 100% locais (SQLite + JSON)")
                    .size(10.0)
                    .color(TEXT_MUTED),
            );
            if self.bridge.active_tasks > 0 {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!(
                            "Processando {} tarefa(s)...",
                            self.bridge.active_tasks
                        ))
                        .size(11.0)
                        .color(ACCENT_ORANGE),
                    );
                });
            }
        });
    }

    /// Atualiza a ação selecionada, persiste em `user_settings.json` e dispara busca no Python.
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

        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("🔍 Ativo / Ticker:").strong());
                let resp_input = ui.add(
                    egui::TextEdit::singleline(&mut self.ticker_input)
                        .desired_width(110.0)
                        .hint_text("Ex: PETR4, AAPL"),
                );
                if (resp_input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    || ui.button(RichText::new("Consultar").strong()).clicked()
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
                    "★ Remover Favorito"
                } else {
                    "☆ Adicionar Favorito"
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
                ui.label(RichText::new("Período:").strong());
                let periodos = [
                    ("5d", "5 DIAS"),
                    ("1mo", "30 DIAS"),
                    ("6mo", "6 MESES"),
                    ("1y", "1 ANO"),
                    ("5y", "5 ANOS"),
                ];
                for (cod, rotulo) in periodos {
                    let sel = self.settings.selected_period == cod;
                    if ui.selectable_label(sel, rotulo).clicked() {
                        self.settings.selected_period = cod.to_string();
                        let _ = self.settings.salvar();
                        self.bridge.request_stock(
                            &self.settings.selected_ticker,
                            &self.settings.selected_period,
                            &self.settings.display_currency,
                            false,
                            ctx.clone(),
                        );
                    }
                }

                ui.separator();
                ui.label(RichText::new("Moeda:").strong());
                let moedas = [
                    ("ORIGINAL", "Moeda Nativa"),
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
                            &self.settings.display_currency,
                            false,
                            ctx.clone(),
                        );
                    }
                }
            });
        });

        ui.add_space(8.0);

        let Some(dados) = &self.stock_data else {
            ui.label(
                RichText::new("Carregando série histórica OHLCV e múltiplos fundamentalistas...")
                    .size(15.0)
                    .color(TEXT_MUTED),
            );
            return;
        };

        let m = &dados.metrics;
        let moeda = &dados.display_currency;

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{} — {}", dados.short_code, dados.company_name))
                    .size(20.0)
                    .strong()
                    .color(TEXT_LIGHT),
            );
            badge_variacao(ui, m.change_day_pct);
            ui.label(
                RichText::new(format!(
                    " | Setor: {} | País: {} | Moeda: {} (Câmbio: {:.4})",
                    dados.sector, dados.country, dados.display_currency, dados.fx_factor
                ))
                .size(12.0)
                .color(TEXT_MUTED),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("📊 Relatório Interativo (Plotly HTML)")
                    .clicked()
                {
                    self.bridge.request_export_chart(
                        &dados.ticker,
                        &dados.period,
                        &dados.display_currency,
                        "plotly",
                        ctx.clone(),
                    );
                }
                if ui.button("🖼 Exportar PNG (Matplotlib)").clicked() {
                    self.bridge.request_export_chart(
                        &dados.ticker,
                        &dados.period,
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
                ui.label(RichText::new("VALOR ATUAL").size(11.0).color(TEXT_LIGHT));
                ui.label(
                    RichText::new(formatar_moeda(m.current_price, moeda))
                        .size(20.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                let seta = if m.change_day_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta} {:.2}% (hoje)", m.change_day_pct).replace('.', ","),
                    )
                    .size(11.5)
                    .strong()
                    .color(cor_variacao(m.change_day_pct)),
                );
            });

            card_frame().show(&mut cols[1], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("MIN. 52 SEMANAS").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(formatar_moeda(m.min_52w, moeda))
                        .size(18.0)
                        .strong()
                        .color(TEXT_LIGHT),
                );
                ui.label(
                    RichText::new(format!("MIN. MÊS: {}", formatar_moeda(m.min_month, moeda)))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            card_frame().show(&mut cols[2], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("MÁX. 52 SEMANAS").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(formatar_moeda(m.max_52w, moeda))
                        .size(18.0)
                        .strong()
                        .color(TEXT_LIGHT),
                );
                ui.label(
                    RichText::new(format!("MÁX. MÊS: {}", formatar_moeda(m.max_month, moeda)))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            card_frame().show(&mut cols[3], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("DIVIDEND YIELD").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(format!("{:.2}%", m.dividend_yield_pct).replace('.', ","))
                        .size(18.0)
                        .strong()
                        .color(ACCENT_CYAN),
                );
                ui.label(
                    RichText::new(format!(
                        "ÚLT. 12M: {}",
                        formatar_moeda(m.dividends_12m, moeda)
                    ))
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
            });

            card_frame().show(&mut cols[4], |ui| {
                ui.set_min_height(74.0);
                ui.label(RichText::new("VALORIZAÇÃO (12M)").size(11.0).color(TEXT_MUTED));
                let seta12 = if m.change_12m_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta12} {:.2}%", m.change_12m_pct).replace('.', ","),
                    )
                    .size(18.0)
                    .strong()
                    .color(cor_variacao(m.change_12m_pct)),
                );
                ui.label(
                    RichText::new(
                        format!("MÊS ATUAL: {:.2}%", m.change_month_pct).replace('.', ","),
                    )
                    .size(11.0)
                    .color(cor_variacao(m.change_month_pct)),
                );
            });
        });

        ui.add_space(8.0);

        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "COTAÇÃO DE {} ({})",
                        dados.short_code, dados.display_currency
                    ))
                    .size(14.0)
                    .strong()
                    .color(ACCENT_CYAN),
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
                    "📈 Linha Contínua (Dax3)",
                );
                ui.separator();
                ui.checkbox(&mut self.show_sma20, "SMA 20");
                ui.checkbox(&mut self.show_sma50, "SMA 50");
            });

            ui.add_space(4.0);
            let candles = &dados.candles;
            let datas_plot: Vec<String> = candles.iter().map(|c| c.date.clone()).collect();

            Plot::new("stock_main_plot")
                .width((largura_max - 36.0).max(400.0))
                .height(265.0)
                .allow_scroll(false)
                .x_axis_label("Data / Ano")
                .x_axis_formatter(move |mark, _range| {
                    let idx = mark.value.round() as isize;
                    if idx >= 0 && (idx as usize) < datas_plot.len() {
                        datas_plot[idx as usize].clone()
                    } else {
                        String::new()
                    }
                })
                .show(ui, |plot_ui| {
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
                        }
                        StockChartMode::AreaLine => {
                            let pts_close: PlotPoints = candles
                                .iter()
                                .enumerate()
                                .map(|(i, c)| [i as f64, c.close])
                                .collect();
                            let piso = m.min_52w * 0.96;
                            plot_ui.line(
                                Line::new(pts_close)
                                    .name("Fechamento")
                                    .color(ACCENT_ORANGE)
                                    .width(2.4_f32)
                                    .fill(piso as f32),
                            );
                        }
                    }

                    if self.show_sma20 {
                        let pts_sma20: PlotPoints = candles
                            .iter()
                            .enumerate()
                            .map(|(i, c)| [i as f64, c.sma20])
                            .collect();
                        plot_ui.line(
                            Line::new(pts_sma20)
                                .name("SMA 20")
                                .color(ACCENT_CYAN)
                                .width(1.6_f32),
                        );
                    }

                    if self.show_sma50 {
                        let pts_sma50: PlotPoints = candles
                            .iter()
                            .enumerate()
                            .map(|(i, c)| [i as f64, c.sma50])
                            .collect();
                        plot_ui.line(
                            Line::new(pts_sma50)
                                .name("SMA 50")
                                .color(Color32::from_rgb(250, 204, 21))
                                .width(1.6_f32),
                        );
                    }
                });

            // Legenda posicionada abaixo do gráfico de Ações (sem sobrepor as linhas/candles)
            let mut legenda_acoes: Vec<(Color32, String)> = Vec::new();
            match self.stock_chart_mode {
                StockChartMode::Candlestick => {
                    legenda_acoes.push((GREEN_UP, "Candle de Alta (Fechamento ≥ Abertura)".to_string()));
                    legenda_acoes.push((RED_DOWN, "Candle de Baixa (Fechamento < Abertura)".to_string()));
                }
                StockChartMode::AreaLine => {
                    legenda_acoes.push((
                        ACCENT_ORANGE,
                        format!("Preço de Fechamento ({})", dados.short_code),
                    ));
                }
            }
            if self.show_sma20 {
                legenda_acoes.push((ACCENT_CYAN, "SMA 20 (Média Móvel 20 períodos)".to_string()));
            }
            if self.show_sma50 {
                legenda_acoes.push((
                    Color32::from_rgb(250, 204, 21),
                    "SMA 50 (Média Móvel 50 períodos)".to_string(),
                ));
            }
            renderizar_legenda_abaixo(ui, &legenda_acoes);
        });

        ui.add_space(8.0);

        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("📋 Série Histórica OHLCV (Pregões mais recentes no topo)")
                    .size(13.5)
                    .strong()
                    .color(TEXT_LIGHT),
            );
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
                                "Data",
                                "Abertura",
                                "Máxima",
                                "Mínima",
                                "Fechamento",
                                "SMA 20",
                                "Volume Negociado",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(col).strong().color(ACCENT_CYAN));
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
                                    ui.label(format!("{:.2}", c.open).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.high).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.low).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(
                                            format!("{:.2}", c.close).replace('.', ","),
                                        )
                                        .strong()
                                        .color(cor_candle),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", c.sma20).replace('.', ","));
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
            ui.label(RichText::new("Carregando cotações de moedas e índice DXY...").size(15.0));
            return;
        };

        // 1. PAINEL DE CONVERSOR CAMBIAL + GERENCIADOR DE ATÉ 10 MOEDAS (INSERIR / RETIRAR)
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("💱 Conversor Cambial:")
                        .size(13.0)
                        .strong()
                        .color(ACCENT_CYAN),
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
                ui.label(RichText::new("Período:").strong());
                for (cod, rot) in [("1mo", "1M"), ("6mo", "6M"), ("1y", "1A"), ("5y", "5A")] {
                    if ui
                        .selectable_label(self.settings.selected_period == cod, rot)
                        .clicked()
                    {
                        self.settings.selected_period = cod.to_string();
                        let _ = self.settings.salvar();
                        let pairs = self.settings.selected_forex_pairs.clone();
                        self.bridge.request_forex(&pairs, cod, false, ctx.clone());
                    }
                }
            });

            ui.separator();

            // Gerenciador de Inserção / Retirada de Moedas (Limite de 10 moedas)
            let qtd_moedas = self.settings.total_moedas_sem_dxy();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "🌐 Moedas Selecionadas ({qtd_moedas} / 10 máx) — Sincroniza países com (c) Juros e (d) Socioeconomia:"
                    ))
                    .size(12.5)
                    .strong()
                    .color(ACCENT_CYAN),
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
                        egui::Button::new(RichText::new("➕ Inserir Moeda").strong()),
                    )
                    .on_hover_text("Adiciona a moeda escolhida (limite máximo de 10 moedas)")
                    .clicked()
                {
                    self.settings
                        .selected_forex_pairs
                        .push(self.currency_to_add.clone());
                    self.atualizar_moedas_e_sincronizar_paises(ctx);
                }

                if qtd_moedas >= 10 {
                    ui.label(
                        RichText::new("(Limite de 10 moedas atingido)")
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
                                RichText::new(rotulo_botao).size(11.0).color(TEXT_LIGHT),
                            )
                            .fill(BG_CARD_ALT),
                        )
                        .on_hover_text("Clique para retirar esta moeda da lista")
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
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "📌 Destaques de Variação vs Dólar Americano (USD) — Escolha até 3 Moedas:",
                    )
                    .size(13.0)
                    .strong()
                    .color(ACCENT_CYAN),
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
                    ui.label(RichText::new(format!("Moeda {}:", slot + 1)).size(11.5));
                    let atual_txt = if self.settings.forex_spotlight_three[slot].is_empty() {
                        "(Nenhuma)".to_string()
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
                                    "(Nenhuma)",
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
                        .fill(BG_CARD_ALT)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(&mut cols[i], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&par.display_code)
                                        .size(13.5)
                                        .strong()
                                        .color(TEXT_LIGHT),
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
                                    .color(TEXT_MUTED),
                            );
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(
                                        format!("{:.4}", par.current_rate).replace('.', ","),
                                    )
                                    .size(16.5)
                                    .strong()
                                    .color(ACCENT_CYAN),
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
                            ui.label(
                                RichText::new(format!(
                                    "Variação no período vs USD: {:+.2}%",
                                    par.change_period_pct
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
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                let titulo_grafico = if self.forex_base100_mode {
                    "📈 Evolução Comparativa Cambial & DXY (Base 100 = Início do Período)"
                } else {
                    "📈 Evolução Comparativa Cambial & DXY (Cotação Nominal)"
                };
                ui.label(
                    RichText::new(titulo_grafico)
                        .size(13.5)
                        .strong()
                        .color(TEXT_LIGHT),
                );
                ui.separator();
                ui.checkbox(&mut self.forex_base100_mode, "Escala Base 100");
            });

            // Checkboxes para ativar ou desativar cada moeda que está sendo comparada no gráfico
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Ativar/Desativar no gráfico:")
                        .size(11.5)
                        .strong()
                        .color(ACCENT_CYAN),
                );

                if ui.small_button("Marcar Todas").clicked() {
                    self.settings.disabled_forex_chart_pairs.clear();
                    let _ = self.settings.salvar();
                }
                if ui.small_button("Desmarcar Todas").clicked() {
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
                ACCENT_CYAN,
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
                .x_axis_label("Data / Ano")
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
                    for (idx_p, par) in fx.pairs.iter().enumerate() {
                        let desativado = self
                            .settings
                            .disabled_forex_chart_pairs
                            .iter()
                            .any(|d| d.eq_ignore_ascii_case(&par.ticker));
                        if desativado {
                            continue;
                        }
                        let pts: PlotPoints = par
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
                        plot_ui.line(
                            Line::new(pts)
                                .name(format!("{} ({})", par.display_code, par.country))
                                .color(cores[idx_p % cores.len()])
                                .width(2.0_f32),
                        );
                    }
                });

            // Legenda abaixo do gráfico de Moedas & DXY
            renderizar_legenda_abaixo(ui, &legenda_moedas);
        });

        ui.add_space(8.0);

        // 4. TABELA DETALHADA DE MOEDAS CONTIDA NA JANELA COM SCROLL
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("📋 Tabela Comparativa de Moedas e Índice Dólar (DXY)")
                    .size(13.5)
                    .strong(),
            );
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
                                "Descrição do Par",
                                "Região / País",
                                "Cotação Atual",
                                "Var. Dia (%)",
                                "Var. Período",
                                "Mín / Máx",
                                "Volatilidade (a.a.)",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(ACCENT_CYAN));
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
                                            format!("{:.4}", p.current_rate).replace('.', ","),
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
    fn renderizar_aba_juros(&mut self, ui: &mut Ui, _ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "🏦 Taxas Básicas de Juros (BCs) — Países sincronizados com as moedas selecionadas no item (b) Moedas & Dólar (DXY):",
                    )
                    .size(13.0)
                    .strong()
                    .color(ACCENT_CYAN),
                );
            });
        });

        ui.add_space(8.0);

        let Some(juros) = &self.interest_data else {
            ui.label(
                RichText::new("Carregando taxas de juros dos Bancos Centrais...").size(15.0),
            );
            return;
        };

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
                        card_frame()
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
                                    ui.label(
                                        RichText::new(format!("Série: {}", item.series_code))
                                            .size(10.5)
                                            .color(ACCENT_CYAN),
                                    );
                                },
                            );
                        });
                        ui.label(
                            RichText::new(&item.central_bank)
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Taxa Atual:")
                                    .size(11.5)
                                    .color(TEXT_LIGHT),
                            );
                            ui.label(
                                RichText::new(
                                    format!("{:.2}% a.a.", item.current_rate_pct)
                                        .replace('.', ","),
                                )
                                .size(18.5)
                                .strong()
                                .color(GREEN_UP),
                            );
                            ui.label(
                                RichText::new(format!("({:+.0} bps)", item.change_bps))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "Inflação CPI: {:.2}%  |  Juro Real: {:+.2}%",
                                item.inflation_cpi_pct, item.real_interest_pct
                            ))
                            .size(11.0)
                            .color(if item.real_interest_pct >= 0.0 {
                                ACCENT_CYAN
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
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("📈 Trajetória Histórica das Taxas Básicas de Juros (% ao ano)")
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(4.0);

            // Extrai as datas (YYYY-MM) da série mais longa para rotular o eixo dos anos
            let datas_juros: Vec<String> = juros
                .countries
                .iter()
                .max_by_key(|c| c.history.len())
                .map(|c| c.history.iter().map(|h| h.date.clone()).collect())
                .unwrap_or_default();

            let cores = [
                GREEN_UP,
                ACCENT_CYAN,
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
                .x_axis_label("Data / Ano")
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
                    for (idx, c) in juros.countries.iter().enumerate() {
                        let pts: PlotPoints = c
                            .history
                            .iter()
                            .enumerate()
                            .map(|(i, h)| [i as f64, h.rate])
                            .collect();
                        plot_ui.line(
                            Line::new(pts)
                                .name(format!(
                                    "{} — {:.2}% ({})",
                                    c.country_name, c.current_rate_pct, c.series_code
                                ))
                                .color(cores[idx % cores.len()])
                                .width(2.3_f32),
                        );
                    }
                });

            // Legenda abaixo do gráfico de Taxas de Juros
            renderizar_legenda_abaixo(ui, &legenda_juros);
        });

        ui.add_space(8.0);

        // 3. TABELA COMPARATIVA DE POLÍTICA MONETÁRIA CONTIDA NA JANELA COM SCROLL
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new("📋 Quadro Comparativo de Política Monetária e Juro Real")
                    .size(13.5)
                    .strong(),
            );
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
                                "País / Bloco",
                                "Banco Central",
                                "Série / Código",
                                "Taxa Atual",
                                "Taxa Anterior",
                                "Inflação (CPI)",
                                "Juro Real Ex-Post",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(ACCENT_CYAN));
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
    // ABA (d): VARIÁVEIS SOCIOECONÔMICAS (WORLD BANK) — SINCRONIZADA COM (b)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_macro(&mut self, ui: &mut Ui, ctx: &egui::Context, largura_max: f32) {
        ui.set_max_width(largura_max);

        let Some(macro_resp) = &self.macro_data else {
            ui.label(
                RichText::new(
                    "Carregando indicadores socioeconômicos do Banco Mundial (World Bank)...",
                )
                .size(15.0),
            );
            return;
        };

        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("🌍 Indicador Socioeconômico:")
                        .size(13.0)
                        .strong()
                        .color(ACCENT_CYAN),
                );

                for ind in &macro_resp.indicators_catalog {
                    let sel = self.selected_wb_indicator == ind.key;
                    if ui.selectable_label(sel, &ind.name).clicked() {
                        self.selected_wb_indicator = ind.key.clone();
                    }
                }
            });

            ui.separator();

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(
                        "🔗 Países sincronizados com o item (b) Moedas & Dólar (DXY)  |  Intervalo de Anos:",
                    )
                    .size(12.0)
                    .strong()
                    .color(TEXT_LIGHT),
                );

                let mut mudou_ano = false;
                ui.label("Início:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.settings.macro_start_year)
                            .range(2000..=self.settings.macro_end_year - 1),
                    )
                    .changed()
                {
                    mudou_ano = true;
                }
                ui.label("Final:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.settings.macro_end_year)
                            .range(self.settings.macro_start_year + 1..=2025),
                    )
                    .changed()
                {
                    mudou_ano = true;
                }

                if mudou_ano && ui.button("Aplicar Período").clicked() {
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
        });

        ui.add_space(8.0);

        let ind_meta = macro_resp
            .indicators_catalog
            .iter()
            .find(|i| i.key == self.selected_wb_indicator);
        let nome_ind = ind_meta
            .map(|i| format!("{} ({})", i.name, i.unit))
            .unwrap_or_else(|| self.selected_wb_indicator.clone());

        // Descobre o ano de início real e o ano final real presentes nos dados carregados
        let mut ano_min = macro_resp.start_year;
        let mut ano_max = macro_resp.end_year;
        for c in &macro_resp.countries {
            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                if let Some(primeiro) = serie.first() {
                    ano_min = ano_min.min(primeiro.year);
                }
                if let Some(ultimo) = serie.last() {
                    ano_max = ano_max.max(ultimo.year);
                }
            }
        }

        let cores_macro = [
            GREEN_UP,
            ACCENT_CYAN,
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

        ui.columns(2, |cols| {
            card_frame().show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(format!(
                        "📈 Série Histórica ({ano_min} a {ano_max}) — {nome_ind}"
                    ))
                    .size(13.0)
                    .strong(),
                );
                ui.add_space(4.0);

                Plot::new("macro_series_plot_v02")
                    .height(245.0)
                    .allow_scroll(false)
                    .x_grid_spacer(uniform_grid_spacer(|_| [1.0, 2.0, 5.0]))
                    .x_axis_label(format!("Ano (Início: {ano_min} — Final: {ano_max})"))
                    .x_axis_formatter(move |mark, _range| {
                        let ano = mark.value.round() as i32;
                        if (mark.value - ano as f64).abs() < 0.05 {
                            if ano == ano_min {
                                format!("{ano} (Início)")
                            } else if ano == ano_max {
                                format!("{ano} (Final)")
                            } else {
                                format!("{ano}")
                            }
                        } else {
                            String::new()
                        }
                    })
                    .show(ui, |plot_ui| {
                        for (idx, c) in macro_resp.countries.iter().enumerate() {
                            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                                let pts: PlotPoints =
                                    serie.iter().map(|pt| [pt.year as f64, pt.value]).collect();
                                plot_ui.line(
                                    Line::new(pts)
                                        .name(format!("{} ({})", c.country_name, c.iso3))
                                        .color(cores_macro[idx % cores_macro.len()])
                                        .width(2.4_f32),
                                );
                            }
                        }
                    });

                // Legenda posicionada abaixo do gráfico de Série Histórica
                renderizar_legenda_abaixo(ui, &legenda_macro);
            });

            card_frame().show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(format!(
                        "📊 Comparativo Recente ({ano_max}) — {nome_ind}"
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

                Plot::new("macro_bar_plot_v02")
                    .height(245.0)
                    .allow_scroll(false)
                    .x_grid_spacer(uniform_grid_spacer(|_| [1.0, 1.0, 1.0]))
                    .x_axis_label(format!("Países Comparados (Dados: {ano_min}–{ano_max})"))
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
                        for (idx, c) in macro_resp.countries.iter().enumerate() {
                            let val = c
                                .latest
                                .get(&self.selected_wb_indicator)
                                .copied()
                                .unwrap_or(0.0);
                            barras.push(
                                Bar::new(idx as f64, val)
                                    .name(&c.country_name)
                                    .width(0.55)
                                    .fill(cores_macro[idx % cores_macro.len()]),
                            );
                        }
                        plot_ui.bar_chart(BarChart::new(barras));
                    });

                // Legenda posicionada abaixo do gráfico de Barras Comparativo
                renderizar_legenda_abaixo(ui, &legenda_macro);
            });
        });

        ui.add_space(8.0);

        // Tabela Consolidada com todas as 6 variáveis socioeconômicas lado a lado
        card_frame().show(ui, |ui| {
            ui.set_max_width(largura_max - 20.0);
            ui.label(
                RichText::new(format!(
                    "📋 Painel Comparativo Multidimensional (Banco Mundial — Período {ano_min} a {ano_max})"
                ))
                .size(13.5)
                .strong(),
            );
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
                                "País (ISO-3)",
                                "PIB Total (US$ Bi)",
                                "PIB per Capita (US$)",
                                "Inflação CPI (%)",
                                "Desemprego (%)",
                                "Câmbio Oficial (/US$)",
                                "População (Milhões)",
                            ] {
                                header.col(|ui| {
                                    ui.label(RichText::new(h).strong().color(ACCENT_CYAN));
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
                                    ui.label(format!("US$ {:.1} Bi", gdp).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(format!("US$ {:.0}", gdppc));
                                });
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{:.2}%", cpi).replace('.', ","))
                                            .color(if cpi > 5.0 { RED_DOWN } else { GREEN_UP }),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}%", unemp).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.2}", fx).replace('.', ","));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.1} M", pop).replace('.', ","));
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
                RichText::new("Carregando Dashboard de Mercado, Altas/Baixas e Scanner...")
                    .size(15.0),
            );
            return;
        };

        let mut acao_para_abrir: Option<String> = None;

        // 1. PAINEL DE ALTAS E BAIXAS (DAX1.PNG) — DUAS COLUNAS SEPARADAS COM GRADE DE 4 COLUNAS (SEM SOBREPOSIÇÃO)
        ui.columns(2, |cols| {
            card_frame().show(&mut cols[0], |ui| {
                ui.set_min_height(112.0);
                ui.label(
                    RichText::new("⬆ Altas do Pregão")
                        .size(15.0)
                        .strong()
                        .color(GREEN_UP),
                );
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
                                            .color(TEXT_LIGHT),
                                    );
                                    let btn = ui.add_sized(
                                        [76.0, 24.0],
                                        egui::Button::new(
                                            RichText::new(
                                                format!("+{:.2}%", item.change_pct.abs())
                                                    .replace('.', ","),
                                            )
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::from_rgb(10, 15, 22)),
                                        )
                                        .fill(GREEN_UP)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                    if btn
                                        .on_hover_text("Clique para abrir os detalhes do ativo")
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

            card_frame().show(&mut cols[1], |ui| {
                ui.set_min_height(112.0);
                ui.label(
                    RichText::new("⬇ Baixas do Pregão")
                        .size(15.0)
                        .strong()
                        .color(RED_DOWN),
                );
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
                                            .color(TEXT_LIGHT),
                                    );
                                    let btn = ui.add_sized(
                                        [76.0, 24.0],
                                        egui::Button::new(
                                            RichText::new(
                                                format!("-{:.2}%", item.change_pct.abs())
                                                    .replace('.', ","),
                                            )
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                        )
                                        .fill(RED_DOWN)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                    if btn
                                        .on_hover_text("Clique para abrir os detalhes do ativo")
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
                RichText::new("Modo de Visualização:")
                    .size(13.5)
                    .strong()
                    .color(ACCENT_CYAN),
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::CardsDax2,
                "📊 Cards com Sparklines (Estilo Dax2 / TradeMap)",
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::ScannerDax4,
                "🔍 Stocks Scanner & Filtros (Estilo Dax4 / xStation)",
            );
        });

        ui.add_space(6.0);

        match self.dashboard_mode {
            DashboardViewMode::CardsDax2 => {
                let n_cols = 3;
                for chunk in ov.assets.chunks(n_cols) {
                    ui.columns(n_cols, |cols| {
                        for (i, ativo) in chunk.iter().enumerate() {
                            card_frame().show(&mut cols[i], |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            RichText::new(&ativo.short_code)
                                                .size(16.5)
                                                .strong()
                                                .color(TEXT_LIGHT),
                                        );
                                        ui.label(
                                            RichText::new(&ativo.name)
                                                .size(11.0)
                                                .color(TEXT_MUTED),
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
                                                .color(TEXT_LIGHT),
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
                                            .color(TEXT_MUTED),
                                    );
                                    ui.label(
                                        RichText::new(format!("DY: {:.2}%", ativo.dy))
                                            .size(10.5)
                                            .color(TEXT_MUTED),
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

                                    if ui.small_button("🔍 Detalhes").clicked() {
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
                card_frame().show(ui, |ui| {
                    ui.set_max_width(largura_max - 20.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("País:").strong());
                        egui::ComboBox::from_id_salt("scanner_country_cb")
                            .selected_text(&self.scanner_country)
                            .show_ui(ui, |ui| {
                                for p in [
                                    "Todos os países",
                                    "Brasil",
                                    "Estados Unidos",
                                    "Alemanha",
                                    "Países Baixos",
                                ] {
                                    ui.selectable_value(
                                        &mut self.scanner_country,
                                        p.to_string(),
                                        p,
                                    );
                                }
                            });

                        ui.separator();
                        ui.label(RichText::new("Procurar instrumento:").strong());
                        ui.add(
                            egui::TextEdit::singleline(&mut self.scanner_search)
                                .desired_width(150.0)
                                .hint_text("Nome ou ticker..."),
                        );

                        if ui.button("Limpar Filtros").clicked() {
                            self.scanner_country = "Todos os países".to_string();
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
                                RichText::new("Capitalização Mínima (Bi)")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.filter_min_mcap, 0.0..=1000.0)
                                    .suffix(" Bi"),
                            );
                        });
                        cols[1].vertical(|ui| {
                            ui.label(
                                RichText::new("P/E (Preço/Lucro Máx)")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(egui::Slider::new(&mut self.filter_max_pe, 2.0..=100.0));
                        });
                        cols[2].vertical(|ui| {
                            ui.label(
                                RichText::new("Dividend Yield Mín (%)")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.filter_min_dy, 0.0..=20.0).suffix("%"),
                            );
                        });
                        cols[3].vertical(|ui| {
                            ui.label(
                                RichText::new("ROE Mínimo (%)")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
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
                        let ok_pais = self.scanner_country == "Todos os países"
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

                card_frame().show(ui, |ui| {
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
                                        "NOME COMPLETO",
                                        "INDÚSTRIA / SETOR",
                                        "PAÍS",
                                        "CAPITALIZAÇÃO",
                                        "EPS (LPA)",
                                        "P/E (P/L)",
                                        "DIVIDEND YIELD",
                                        "P/B (P/VP)",
                                        "ROE (%)",
                                        "AÇÃO",
                                    ] {
                                        header.col(|ui| {
                                            ui.label(
                                                RichText::new(h)
                                                    .size(11.0)
                                                    .strong()
                                                    .color(TEXT_MUTED),
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
                                                .color(TEXT_LIGHT),
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
                                            if ui.small_button("📈 Analisar").clicked() {
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

        if self.last_indices_refresh.elapsed() >= Duration::from_secs(600) {
            self.last_indices_refresh = Instant::now();
            let favs = self.settings.favorite_tickers.clone();
            self.bridge.request_overview(&favs, true, ctx.clone());
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        self.processar_mensagens_bridge();

        // Barra Superior com Mini-Cards e Sparklines (Dax1)
        egui::TopBottomPanel::top("top_dax1_bar")
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                self.renderizar_barra_superior_dax1(ui, ctx);
            });

        // Barra Inferior de Status
        egui::TopBottomPanel::bottom("bottom_status_bar")
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("🟢 Status: {}", self.status_message))
                            .size(11.5)
                            .color(TEXT_MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("DAX - Data Analysis Exchange (v0.2)  |  Rust (egui) ↔ Python (yfinance • FRED • World Bank)")
                                .size(11.0)
                                .color(ACCENT_CYAN),
                        );
                    });
                });
            });

        // Menu Lateral Esquerdo com Largura Estritamente Fixa (240px) sem espaço vazio lateral
        egui::SidePanel::left("left_sidebar_dax_v02_fixed")
            .resizable(false)
            .exact_width(240.0)
            .min_width(240.0)
            .max_width(240.0)
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                self.renderizar_menu_lateral(ui);
            });

        // Área Central Principal colada imediatamente à barra lateral esquerda, com barra de rolagem sempre visível
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(crate::theme::BG_PRIMARY)
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
