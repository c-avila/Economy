//! Interface Gráfica Principal (Rust + `egui` + `eframe` + `egui_plot` + `egui_extras`).
//! Implementa as 5 abas obrigatórias (a, b, c, d, e) com inspiração visual nas imagens Dax1 a Dax4.

use crate::bridge::{BridgeMessage, PythonBridge};
use crate::config::UserSettings;
use crate::models::{
    ForexResponse, InterestResponse, MacroResponse, MarketOverviewResponse, StockDetailResponse,
};
use crate::theme::{
    aplicar_tema_dax, badge_variacao, card_frame, cor_variacao, desenhar_sparkline, formatar_moeda,
    hero_blue_frame, ACCENT_CYAN, ACCENT_ORANGE, BG_CARD, BG_CARD_ALT, BG_SIDEBAR, BORDER_SUBTLE,
    GREEN_UP, RED_DOWN, TEXT_LIGHT, TEXT_MUTED,
};
use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Bar, BarChart, BoxElem, BoxPlot, BoxSpread, Legend, Line, Plot, PlotPoints};
use std::time::{Duration, Instant};

/// Abas principais da aplicação conforme item 1 e 6(c) da especificação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    /// (a) Analisar ações de empresas (preços históricos, OHLCV, candles, estatísticas - Dax3)
    Stocks,
    /// (b) Cotações de moedas e Dólar Americano Index (DXY)
    Forex,
    /// (c) Taxas de juros de diferentes países (Fed, Selic, BoE, BCE, BoJ)
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

    // Mensagens de status e notificações
    pub status_message: String,
    pub error_banner: Option<String>,
    pub last_indices_refresh: Instant,
}

impl DaxEconomiaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        aplicar_tema_dax(&cc.egui_ctx);
        let settings = UserSettings::carregar();
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
            ticker_input: ticker_inicial,
            stock_chart_mode: StockChartMode::Candlestick,
            show_sma20: true,
            show_sma50: true,
            forex_base100_mode: true,
            conv_amount: 1000.0,
            conv_from: "USD".to_string(),
            conv_to: "BRL".to_string(),
            selected_wb_indicator: "gdp_total_bi".to_string(),
            dashboard_mode: DashboardViewMode::CardsDax2,
            scanner_search: String::new(),
            scanner_country: "Todos os países".to_string(),
            filter_min_mcap: 0.0,
            filter_max_pe: 100.0,
            filter_min_dy: 0.0,
            filter_min_roe: -50.0,
            status_message: "Iniciando plataforma e conectando ao motor de dados Python...".into(),
            error_banner: None,
            last_indices_refresh: Instant::now(),
        }
    }

    /// Dispara o carregamento inicial de todas as seções em background.
    fn disparar_carga_inicial(&mut self, ctx: &egui::Context) {
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
                        "Cotações cambiais e DXY atualizados ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.forex_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::InterestLoaded(data) => {
                    self.status_message = format!(
                        "Taxas de juros carregadas ({}) — {}",
                        data.source_status, data.updated_at
                    );
                    self.interest_data = Some(data);
                    self.error_banner = None;
                }
                BridgeMessage::MacroLoaded(data) => {
                    self.status_message = format!(
                        "Dados World Bank carregados ({}) — {}",
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

        // Linha de cabeçalho da barra superior: título + contador de 10 minutos + botão Atualizar Agora
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

        // Grade dos 11 cards das bolsas mundiais (Pontuação à esquerda | % e Gráfico à direita)
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(7.0, 6.0);

            if let Some(ov) = &self.overview_data {
                for item in &ov.top_bar {
                    egui::Frame::none()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(9.0, 5.0))
                        .show(ui, |ui| {
                            ui.set_min_width(142.0);
                            ui.set_max_width(156.0);
                            ui.vertical(|ui| {
                                // Linha 1: Nome do Índice (extremo esquerdo) | Variação % (extremo direito)
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
                                            let sinal = if item.change_pct > 0.0 { "+" } else { "" };
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

                                // Linha 2: Pontuação e País (extremo esquerdo) | Sparkline Gráfico (extremo direito)
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
                                                Vec2::new(56.0, 22.0),
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

        if let Some(cod) = indice_clicado {
            self.selecionar_acao(&cod, ctx);
            self.active_tab = ActiveTab::Stocks;
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // MENU LATERAL DE NAVEGAÇÃO (INSPIRADO EM DAX2.PNG - TRADEMAP)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_menu_lateral(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("📊 DAX")
                    .size(21.0)
                    .strong()
                    .color(GREEN_UP),
            );
            ui.label(
                RichText::new("Economia")
                    .size(21.0)
                    .strong()
                    .color(TEXT_LIGHT),
            );
        });
        ui.label(
            RichText::new("Plataforma Quantitativa & Macro")
                .size(11.5)
                .color(TEXT_MUTED),
        );
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

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
                "Comparativo USD vs EUR, GBP, JPY, CNY, BRL e Índice DXY=",
            ),
            (
                ActiveTab::InterestRates,
                "🏦 (c) Taxas de Juros (BCs)",
                "Fed Funds (EUA), Selic (Brasil), BCE, BoE e BoJ + Juro Real",
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
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    let btn = ui.add_sized(
                        [ui.available_width(), 26.0],
                        egui::Button::new(
                            RichText::new(titulo)
                                .size(13.5)
                                .strong()
                                .color(if selecionado { Color32::WHITE } else { TEXT_LIGHT }),
                        )
                        .frame(false),
                    );
                    if btn.on_hover_text(dica).clicked() {
                        self.active_tab = tab;
                    }
                });
            ui.add_space(4.0);
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(
            RichText::new("⭐ Ativos Favoritos")
                .size(12.5)
                .strong()
                .color(ACCENT_CYAN),
        );
        ui.add_space(4.0);

        let favs_clone = self.settings.favorite_tickers.clone();
        let mut ticker_clicado: Option<String> = None;

        ui.horizontal_wrapped(|ui| {
            for fav in &favs_clone {
                let ativo_atual = self.settings.selected_ticker.eq_ignore_ascii_case(fav);
                let btn = ui.add(
                    egui::Button::new(
                        RichText::new(fav)
                            .size(11.5)
                            .strong()
                            .color(if ativo_atual { Color32::BLACK } else { TEXT_LIGHT }),
                    )
                    .fill(if ativo_atual { GREEN_UP } else { BG_CARD_ALT }),
                );
                if btn.clicked() {
                    ticker_clicado = Some(fav.clone());
                }
            }
        });

        if let Some(tk) = ticker_clicado {
            self.selecionar_acao(&tk, ui.ctx());
            self.active_tab = ActiveTab::Stocks;
        }

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new("🔒 LGPD: Dados 100% locais (SQLite + JSON)")
                    .size(10.5)
                    .color(TEXT_MUTED),
            );
            if self.bridge.active_tasks > 0 {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(format!(
                            "Processando {} tarefa(s) no Python...",
                            self.bridge.active_tasks
                        ))
                        .size(11.5)
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
    fn renderizar_aba_acoes(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Barra de controles: Busca de Ticker, Favoritar, Período e Moeda (como em Dax3)
        card_frame().show(ui, |ui| {
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

                // Botão de Favorito
                let tk_upper = self.settings.selected_ticker.to_uppercase();
                let ja_favorito = self
                    .settings
                    .favorite_tickers
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(&tk_upper));
                let label_fav = if ja_favorito {
                    "★ Favorito"
                } else {
                    "☆ Favoritar"
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
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Carregando série histórica OHLCV e múltiplos fundamentalistas...")
                        .size(15.0)
                        .color(TEXT_MUTED),
                );
            });
            return;
        };

        let m = &dados.metrics;
        let moeda = &dados.display_currency;

        // Cabeçalho da Empresa e Botões de Exportação Plotly / Matplotlib
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} — {}", dados.short_code, dados.company_name))
                            .size(20.0)
                            .strong()
                            .color(TEXT_LIGHT),
                    );
                    badge_variacao(ui, m.change_day_pct);
                });
                ui.label(
                    RichText::new(format!(
                        "Setor: {}  |  País: {}  |  Moeda Exibida: {} (Fator Câmbio: {:.4})",
                        dados.sector, dados.country, dados.display_currency, dados.fx_factor
                    ))
                    .size(12.0)
                    .color(TEXT_MUTED),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("📊 Abrir Relatório Interativo (Plotly HTML)")
                    .on_hover_text("Gera e abre um dashboard HTML interativo com Candlesticks e Volume no navegador")
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
                if ui
                    .button("🖼 Exportar PNG (Matplotlib)")
                    .on_hover_text("Salva imagem PNG de alta resolução com preços, médias móveis e volume")
                    .clicked()
                {
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

        // 5 Cards de Indicadores Superiores idênticos ao layout da imagem Dax3.png
        ui.columns(5, |cols| {
            // Card 1: VALOR ATUAL (Destaque Azul Dax3)
            hero_blue_frame().show(&mut cols[0], |ui| {
                ui.set_min_height(78.0);
                ui.label(RichText::new("VALOR ATUAL").size(11.0).color(TEXT_LIGHT));
                ui.add_space(4.0);
                ui.label(
                    RichText::new(formatar_moeda(m.current_price, moeda))
                        .size(22.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                let seta = if m.change_day_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta} {:.2}% (hoje)", m.change_day_pct).replace('.', ","),
                    )
                    .size(12.0)
                    .strong()
                    .color(cor_variacao(m.change_day_pct)),
                );
            });

            // Card 2: MIN. 52 SEMANAS + MIN. MÊS
            card_frame().show(&mut cols[1], |ui| {
                ui.set_min_height(78.0);
                ui.label(RichText::new("MIN. 52 SEMANAS").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(formatar_moeda(m.min_52w, moeda))
                        .size(19.0)
                        .strong()
                        .color(TEXT_LIGHT),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new(format!("MIN. MÊS: {}", formatar_moeda(m.min_month, moeda)))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            // Card 3: MÁX. 52 SEMANAS + MÁX. MÊS
            card_frame().show(&mut cols[2], |ui| {
                ui.set_min_height(78.0);
                ui.label(RichText::new("MÁX. 52 SEMANAS").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(formatar_moeda(m.max_52w, moeda))
                        .size(19.0)
                        .strong()
                        .color(TEXT_LIGHT),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new(format!("MÁX. MÊS: {}", formatar_moeda(m.max_month, moeda)))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );
            });

            // Card 4: DIVIDEND YIELD + ÚLTIMOS 12 MESES
            card_frame().show(&mut cols[3], |ui| {
                ui.set_min_height(78.0);
                ui.label(RichText::new("DIVIDEND YIELD").size(11.0).color(TEXT_MUTED));
                ui.label(
                    RichText::new(format!("{:.2}%", m.dividend_yield_pct).replace('.', ","))
                        .size(19.0)
                        .strong()
                        .color(ACCENT_CYAN),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new(format!(
                        "ÚLT. 12M: {}",
                        formatar_moeda(m.dividends_12m, moeda)
                    ))
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
            });

            // Card 5: VALORIZAÇÃO (12M) + MÊS ATUAL
            card_frame().show(&mut cols[4], |ui| {
                ui.set_min_height(78.0);
                ui.label(RichText::new("VALORIZAÇÃO (12M)").size(11.0).color(TEXT_MUTED));
                let seta12 = if m.change_12m_pct >= 0.0 { "⬆" } else { "⬇" };
                ui.label(
                    RichText::new(
                        format!("{seta12} {:.2}%", m.change_12m_pct).replace('.', ","),
                    )
                    .size(19.0)
                    .strong()
                    .color(cor_variacao(m.change_12m_pct)),
                );
                ui.add_space(3.0);
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

        // Gráfico Interativo de Preços (Candlestick OHLC ou Linha + Médias Móveis SMA 20/50)
        card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "COTAÇÃO DE {} ({})",
                        dados.short_code, dados.display_currency
                    ))
                    .size(14.5)
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
                ui.checkbox(&mut self.show_sma20, "Média Móvel 20d (SMA20)");
                ui.checkbox(&mut self.show_sma50, "Média Móvel 50d (SMA50)");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "P/L: {:.2}  |  P/VP: {:.2}  |  ROE: {:.1}%  |  Volatilidade Anual: {:.1}%",
                            m.pe_ratio, m.pb_ratio, m.roe_pct, m.volatility_annual_pct
                        ))
                        .size(11.5)
                        .color(TEXT_MUTED),
                    );
                });
            });

            ui.add_space(4.0);
            let candles = &dados.candles;
            let datas_plot: Vec<String> = candles.iter().map(|c| c.date.clone()).collect();

            Plot::new("stock_main_plot")
                .height(300.0)
                .legend(Legend::default())
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
        });

        ui.add_space(8.0);

        // Tabela de Histórico OHLCV (egui_extras::TableBuilder)
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("📋 Série Histórica OHLCV (Pregões mais recentes no topo)")
                    .size(13.5)
                    .strong()
                    .color(TEXT_LIGHT),
            );
            ui.add_space(4.0);

            let candles_rev: Vec<_> = dados.candles.iter().rev().collect();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(100.0))
                .column(Column::initial(105.0))
                .column(Column::initial(105.0))
                .column(Column::initial(105.0))
                .column(Column::initial(110.0))
                .column(Column::initial(105.0))
                .column(Column::remainder())
                .max_scroll_height(190.0)
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
                        let cor_candle = if c.close >= c.open { GREEN_UP } else { RED_DOWN };
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
                                RichText::new(format!("{:.2}", c.close).replace('.', ","))
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
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (b): COTAÇÕES DE MOEDAS E ÍNDICE DÓLAR (DXY=)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_moedas(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let Some(fx) = &self.forex_data else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Carregando cotações de moedas e índice DXY...").size(15.0));
            });
            return;
        };

        // Conversor Cambial Interativo + Controles de Período
        card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("💱 Conversor de Moedas em Tempo Real:")
                        .size(13.5)
                        .strong()
                        .color(ACCENT_CYAN),
                );
                ui.add(
                    egui::DragValue::new(&mut self.conv_amount)
                        .speed(10.0)
                        .range(0.0..=1_000_000_000.0)
                        .prefix("$ "),
                );

                egui::ComboBox::from_id_salt("cb_conv_from")
                    .selected_text(&self.conv_from)
                    .width(75.0)
                    .show_ui(ui, |ui| {
                        for m in ["USD", "BRL", "EUR", "GBP", "JPY", "CNY", "CHF"] {
                            ui.selectable_value(&mut self.conv_from, m.to_string(), m);
                        }
                    });

                ui.label(RichText::new("➡").strong());

                egui::ComboBox::from_id_salt("cb_conv_to")
                    .selected_text(&self.conv_to)
                    .width(75.0)
                    .show_ui(ui, |ui| {
                        for m in ["BRL", "USD", "EUR", "GBP", "JPY", "CNY", "CHF"] {
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
                    RichText::new(format!(
                        "= {}",
                        formatar_moeda(convertido, &self.conv_to)
                    ))
                    .size(16.0)
                    .strong()
                    .color(GREEN_UP),
                );

                ui.separator();
                ui.checkbox(
                    &mut self.forex_base100_mode,
                    "Comparar em Base 100 (Performance Relativa)",
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (cod, rot) in [("1mo", "1M"), ("6mo", "6M"), ("1y", "1A"), ("5y", "5A")] {
                        if ui
                            .selectable_label(self.settings.selected_period == cod, rot)
                            .clicked()
                        {
                            self.settings.selected_period = cod.to_string();
                            let pairs = self.settings.selected_forex_pairs.clone();
                            self.bridge.request_forex(&pairs, cod, false, ctx.clone());
                        }
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Cards de Destaque para DXY= e Pares Principais
        ui.horizontal_wrapped(|ui| {
            for par in &fx.pairs {
                card_frame().show(ui, |ui| {
                    ui.set_min_width(195.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&par.display_code)
                                    .size(14.5)
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
                        ui.label(RichText::new(&par.name).size(11.0).color(TEXT_MUTED));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:.4}", par.current_rate).replace('.', ","))
                                    .size(18.0)
                                    .strong()
                                    .color(ACCENT_CYAN),
                            );
                            let spark: Vec<f64> =
                                par.history.iter().rev().take(25).rev().map(|h| h.close).collect();
                            desenhar_sparkline(
                                ui,
                                &spark,
                                Vec2::new(75.0, 26.0),
                                par.change_period_pct >= 0.0,
                            );
                        });
                        ui.label(
                            RichText::new(format!(
                                "No período: {:+.2}%  |  Vol.: {:.1}%",
                                par.change_period_pct, par.volatility_annual_pct
                            ))
                            .size(10.5)
                            .color(cor_variacao(par.change_period_pct)),
                        );
                    });
                });
            }
        });

        ui.add_space(8.0);

        // Gráfico Comparativo do Dólar (DXY= vs Pares Globais)
        card_frame().show(ui, |ui| {
            let titulo_grafico = if self.forex_base100_mode {
                "📈 Evolução Comparativa Cambial & DXY (Índice Base 100 = Início do Período)"
            } else {
                "📈 Séries Históricas Nominais de Câmbio e Índice DXY="
            };
            ui.label(RichText::new(titulo_grafico).size(14.0).strong().color(TEXT_LIGHT));
            ui.add_space(4.0);

            let cores = [
                GREEN_UP,
                ACCENT_CYAN,
                ACCENT_ORANGE,
                Color32::from_rgb(244, 114, 182),
                Color32::from_rgb(167, 139, 250),
                Color32::from_rgb(250, 204, 21),
            ];

            Plot::new("forex_comparison_plot")
                .height(275.0)
                .legend(Legend::default())
                .show(ui, |plot_ui| {
                    for (idx_p, par) in fx.pairs.iter().enumerate() {
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
        });

        ui.add_space(8.0);

        // Tabela Detalhada de Moedas (egui_extras::TableBuilder)
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("📋 Tabela Comparativa de Moedas e Índice Dólar (DXY)")
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(4.0);

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(95.0))
                .column(Column::initial(220.0))
                .column(Column::initial(110.0))
                .column(Column::initial(100.0))
                .column(Column::initial(100.0))
                .column(Column::initial(100.0))
                .column(Column::initial(100.0))
                .column(Column::remainder())
                .max_scroll_height(180.0)
                .header(22.0, |mut header| {
                    for h in [
                        "Ticker",
                        "Descrição do Par",
                        "Região/País",
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
                                RichText::new(format!("{:.4}", p.current_rate).replace('.', ","))
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
                            ui.label(format!("{:.2} / {:.2}", p.low_period, p.high_period));
                        });
                        row.col(|ui| {
                            ui.label(format!("{:.2}%", p.volatility_annual_pct));
                        });
                    });
                });
        });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (c): TAXAS DE JUROS GLOBAIS (FED, SELIC, BCE, BOE, BOJ)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_juros(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("🏦 Selecionar Bancos Centrais / Países:")
                        .size(13.5)
                        .strong()
                        .color(ACCENT_CYAN),
                );

                let opcoes = [
                    ("BR", "Brasil (SELIC - BCB)"),
                    ("US", "EUA (Fed Funds - DFF)"),
                    ("EU", "Zona do Euro (BCE - GKLIB3M)"),
                    ("GB", "Reino Unido (BoE - GBRBOE1M)"),
                    ("JP", "Japão (BoJ / JCC)"),
                    ("CN", "China (PBoC LPR)"),
                ];

                let mut alterou = false;
                for (cod, nome) in opcoes {
                    let mut marcado = self
                        .settings
                        .selected_interest_countries
                        .iter()
                        .any(|c| c == cod);
                    if ui.checkbox(&mut marcado, nome).changed() {
                        if marcado {
                            self.settings.selected_interest_countries.push(cod.to_string());
                        } else if self.settings.selected_interest_countries.len() > 1 {
                            self.settings
                                .selected_interest_countries
                                .retain(|c| c != cod);
                        }
                        alterou = true;
                    }
                }

                if alterou {
                    let _ = self.settings.salvar();
                    let c_list = self.settings.selected_interest_countries.clone();
                    self.bridge.request_interest(&c_list, 2019, false, ctx.clone());
                }
            });
        });

        ui.add_space(8.0);

        let Some(juros) = &self.interest_data else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Carregando taxas de juros dos Bancos Centrais...").size(15.0));
            });
            return;
        };

        // Cards com Taxa Básica Nominal, Inflação CPI e Juro Real Ex-Post
        ui.horizontal_wrapped(|ui| {
            for item in &juros.countries {
                card_frame().show(ui, |ui| {
                    ui.set_min_width(210.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&item.country_name)
                                    .size(14.5)
                                    .strong()
                                    .color(TEXT_LIGHT),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(&item.series_code)
                                            .size(11.0)
                                            .color(ACCENT_CYAN),
                                    );
                                },
                            );
                        });
                        ui.label(RichText::new(&item.central_bank).size(11.0).color(TEXT_MUTED));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(
                                    format!("{:.2}% a.a.", item.current_rate_pct).replace('.', ","),
                                )
                                .size(20.0)
                                .strong()
                                .color(GREEN_UP),
                            );
                            ui.label(
                                RichText::new(format!("({:+.0} bps)", item.change_bps))
                                    .size(11.5)
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
                });
            }
        });

        ui.add_space(8.0);

        // Gráfico de Curvas Históricas de Juros
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("📈 Trajetória das Taxas Básicas de Juros (% ao ano)")
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(4.0);

            let cores = [
                GREEN_UP,
                ACCENT_CYAN,
                ACCENT_ORANGE,
                Color32::from_rgb(244, 114, 182),
                Color32::from_rgb(167, 139, 250),
                Color32::from_rgb(250, 204, 21),
            ];

            Plot::new("interest_rates_plot")
                .height(270.0)
                .legend(Legend::default())
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
                                .name(format!("{} — {} ({})", c.country_name, c.series_code, c.central_bank))
                                .color(cores[idx % cores.len()])
                                .width(2.3_f32),
                        );
                    }
                });
        });

        ui.add_space(8.0);

        // Tabela Comparativa de Política Monetária
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new("📋 Quadro Comparativo de Política Monetária e Juro Real")
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(4.0);

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(125.0))
                .column(Column::initial(190.0))
                .column(Column::initial(100.0))
                .column(Column::initial(110.0))
                .column(Column::initial(110.0))
                .column(Column::initial(110.0))
                .column(Column::remainder())
                .max_scroll_height(180.0)
                .header(22.0, |mut header| {
                    for h in [
                        "País / Bloco",
                        "Banco Central",
                        "Série / Código",
                        "Taxa Nominal",
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
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (d): VARIÁVEIS SOCIOECONÔMICAS ENTRE PAÍSES (WORLD BANK API)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_macro(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let Some(macro_resp) = &self.macro_data else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Carregando indicadores socioeconômicos do Banco Mundial (World Bank)...")
                        .size(15.0),
                );
            });
            return;
        };

        card_frame().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("🌍 Indicador em Destaque no Gráfico:")
                        .size(13.5)
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
                ui.label(RichText::new("Países Comparados:").strong());
                let paises_disp = [
                    ("BRA", "Brasil"),
                    ("USA", "Estados Unidos"),
                    ("CHN", "China"),
                    ("DEU", "Alemanha"),
                    ("GBR", "Reino Unido"),
                    ("JPN", "Japão"),
                ];
                let mut mudou = false;
                for (iso, nome) in paises_disp {
                    let mut marcado = self
                        .settings
                        .selected_macro_countries
                        .iter()
                        .any(|c| c == iso);
                    if ui.checkbox(&mut marcado, nome).changed() {
                        if marcado {
                            self.settings.selected_macro_countries.push(iso.to_string());
                        } else if self.settings.selected_macro_countries.len() > 1 {
                            self.settings
                                .selected_macro_countries
                                .retain(|c| c != iso);
                        }
                        mudou = true;
                    }
                }

                if mudou {
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

        // Gráfico de Evolução Histórica e Ranking em Barras do Indicador Selecionado
        let ind_meta = macro_resp
            .indicators_catalog
            .iter()
            .find(|i| i.key == self.selected_wb_indicator);
        let nome_ind = ind_meta
            .map(|i| format!("{} ({}) — Código WB: {}", i.name, i.unit, i.wb_code))
            .unwrap_or_else(|| self.selected_wb_indicator.clone());

        ui.columns(2, |cols| {
            card_frame().show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(format!("📈 Série Histórica — {nome_ind}"))
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(4.0);

                let cores = [
                    GREEN_UP,
                    ACCENT_CYAN,
                    ACCENT_ORANGE,
                    Color32::from_rgb(244, 114, 182),
                    Color32::from_rgb(167, 139, 250),
                    Color32::from_rgb(250, 204, 21),
                ];

                Plot::new("macro_series_plot")
                    .height(255.0)
                    .legend(Legend::default())
                    .show(ui, |plot_ui| {
                        for (idx, c) in macro_resp.countries.iter().enumerate() {
                            if let Some(serie) = c.series.get(&self.selected_wb_indicator) {
                                let pts: PlotPoints = serie
                                    .iter()
                                    .map(|pt| [pt.year as f64, pt.value])
                                    .collect();
                                plot_ui.line(
                                    Line::new(pts)
                                        .name(format!("{} ({})", c.country_name, c.iso3))
                                        .color(cores[idx % cores.len()])
                                        .width(2.4_f32),
                                );
                            }
                        }
                    });
            });

            card_frame().show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(format!("📊 Comparativo Direto Recente — {nome_ind}"))
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(4.0);

                let nomes_paises: Vec<String> = macro_resp
                    .countries
                    .iter()
                    .map(|c| c.country_name.clone())
                    .collect();

                Plot::new("macro_bar_plot")
                    .height(255.0)
                    .x_axis_formatter(move |mark, _range| {
                        let i = mark.value.round() as isize;
                        if i >= 0 && (i as usize) < nomes_paises.len() {
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
                                    .fill(ACCENT_CYAN),
                            );
                        }
                        plot_ui.bar_chart(BarChart::new(barras));
                    });
            });
        });

        ui.add_space(8.0);

        // Tabela Consolidada com todas as 6 variáveis socioeconômicas lado a lado
        card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(
                    "📋 Painel Comparativo Multidimensional (Banco Mundial — Último Ano Disponível)",
                )
                .size(13.5)
                .strong(),
            );
            ui.add_space(4.0);

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(135.0))
                .column(Column::initial(120.0))
                .column(Column::initial(125.0))
                .column(Column::initial(115.0))
                .column(Column::initial(115.0))
                .column(Column::initial(125.0))
                .column(Column::remainder())
                .max_scroll_height(200.0)
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
                                RichText::new(format!("{} ({})", c.country_name, c.iso3)).strong(),
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
                                RichText::new(format!("{:.2}%", cpi).replace('.', ",")).color(
                                    if cpi > 5.0 { RED_DOWN } else { GREEN_UP },
                                ),
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
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ABA (e): DASHBOARD & SCANNER INTERATIVO (ESTILO DAX1, DAX2 E DAX4)
    // ═══════════════════════════════════════════════════════════════════════
    fn renderizar_aba_dashboard_scanner(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let Some(ov) = self.overview_data.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("Carregando Dashboard de Mercado, Altas/Baixas e Scanner...")
                        .size(15.0),
                );
            });
            return;
        };

        let mut acao_para_abrir: Option<String> = None;

        // 1. PAINEL DE ALTAS E BAIXAS (FIEL À IMAGEM DAX1.PNG)
        card_frame().show(ui, |ui| {
            ui.columns(2, |cols| {
                // Coluna da Esquerda: ▲ Altas
                cols[0].vertical(|ui| {
                    ui.label(
                        RichText::new("▲ Altas do Pregão")
                            .size(15.0)
                            .strong()
                            .color(GREEN_UP),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for item in &ov.gainers {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&item.code)
                                        .size(11.5)
                                        .strong()
                                        .color(TEXT_LIGHT),
                                );
                                let btn = ui.add(
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
                                    .min_size(Vec2::new(68.0, 24.0)),
                                );
                                if btn.on_hover_text("Clique para abrir os detalhes do ativo").clicked() {
                                    acao_para_abrir = Some(item.code.clone());
                                }
                            });
                        }
                    });
                });

                // Coluna da Direita: ▼ Baixas
                cols[1].vertical(|ui| {
                    ui.label(
                        RichText::new("▼ Baixas do Pregão")
                            .size(15.0)
                            .strong()
                            .color(RED_DOWN),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for item in &ov.losers {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&item.code)
                                        .size(11.5)
                                        .strong()
                                        .color(TEXT_LIGHT),
                                );
                                let btn = ui.add(
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
                                    .min_size(Vec2::new(68.0, 24.0)),
                                );
                                if btn.on_hover_text("Clique para abrir os detalhes do ativo").clicked() {
                                    acao_para_abrir = Some(item.code.clone());
                                }
                            });
                        }
                    });
                });
            });
        });

        ui.add_space(8.0);

        // 2. SELETOR DE MODO (CARDS DAX2 vs STOCKS SCANNER DAX4)
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Modo de Visualização:")
                    .size(13.5)
                    .strong()
                    .color(ACCENT_CYAN),
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::CardsDax2,
                "🗂 Cards com Sparklines (Estilo Dax2 / TradeMap)",
            );
            ui.selectable_value(
                &mut self.dashboard_mode,
                DashboardViewMode::ScannerDax4,
                "🎛 Stocks Scanner & Filtros (Estilo Dax4 / xStation)",
            );
        });

        ui.add_space(6.0);

        match self.dashboard_mode {
            DashboardViewMode::CardsDax2 => {
                // Grade de Cards estilo Dax2.png (3 colunas com Nome, Preço, Badge, Sparkline e 3 pílulas inferiores)
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let n_cols = 3;
                    for chunk in ov.assets.chunks(n_cols) {
                        ui.columns(n_cols, |cols| {
                            for (i, ativo) in chunk.iter().enumerate() {
                                card_frame().show(&mut cols[i], |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&ativo.short_code)
                                                    .size(17.0)
                                                    .strong()
                                                    .color(TEXT_LIGHT),
                                            );
                                            ui.label(
                                                RichText::new(&ativo.name)
                                                    .size(11.5)
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
                                                    .size(14.5)
                                                    .strong()
                                                    .color(TEXT_LIGHT),
                                                );
                                            },
                                        );
                                    });

                                    ui.add_space(6.0);
                                    let w = ui.available_width().max(180.0);
                                    desenhar_sparkline(
                                        ui,
                                        &ativo.sparkline,
                                        Vec2::new(w, 68.0),
                                        ativo.change_day_pct >= 0.0,
                                    );
                                    ui.add_space(6.0);

                                    // 3 Pílulas de rodapé como em Dax2: Mcap | Volume | Var 12M + Botão Abrir
                                    ui.horizontal(|ui| {
                                        egui::Frame::none()
                                            .fill(BG_CARD_ALT)
                                            .rounding(Rounding::same(4.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Mcap: {:.1}B",
                                                        ativo.market_cap_bi
                                                    ))
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                                );
                                            });

                                        egui::Frame::none()
                                            .fill(BG_CARD_ALT)
                                            .rounding(Rounding::same(4.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "DY: {:.2}%",
                                                        ativo.dy
                                                    ))
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                                );
                                            });

                                        egui::Frame::none()
                                            .fill(BG_CARD_ALT)
                                            .rounding(Rounding::same(4.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "12M: {:+.1}%",
                                                        ativo.change_12m_pct
                                                    ))
                                                    .size(10.5)
                                                    .color(cor_variacao(ativo.change_12m_pct)),
                                                );
                                            });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("🔍 Detalhes")
                                                    .clicked()
                                                {
                                                    acao_para_abrir =
                                                        Some(ativo.short_code.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                        });
                        ui.add_space(8.0);
                    }
                });
            }

            DashboardViewMode::ScannerDax4 => {
                // Painel de Filtros com Sliders idêntico ao conceito da imagem Dax4.png (Stocks Scanner)
                card_frame().show(ui, |ui| {
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
                                    ui.selectable_value(&mut self.scanner_country, p.to_string(), p);
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
                                RichText::new("Capitalização Mínima (Bi)").size(11.5).color(TEXT_MUTED),
                            );
                            ui.add(egui::Slider::new(&mut self.filter_min_mcap, 0.0..=1000.0).suffix(" Bi"));
                        });
                        cols[1].vertical(|ui| {
                            ui.label(RichText::new("P/E (Preço/Lucro Máx)").size(11.5).color(TEXT_MUTED));
                            ui.add(egui::Slider::new(&mut self.filter_max_pe, 2.0..=100.0));
                        });
                        cols[2].vertical(|ui| {
                            ui.label(RichText::new("Dividend Yield Mín (%)").size(11.5).color(TEXT_MUTED));
                            ui.add(egui::Slider::new(&mut self.filter_min_dy, 0.0..=20.0).suffix("%"));
                        });
                        cols[3].vertical(|ui| {
                            ui.label(RichText::new("ROE Mínimo (%)").size(11.5).color(TEXT_MUTED));
                            ui.add(egui::Slider::new(&mut self.filter_min_roe, -20.0..=80.0).suffix("%"));
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

                // Tabela Zebra-Striped estilo Dax4.png
                card_frame().show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::initial(160.0))
                        .column(Column::initial(160.0))
                        .column(Column::initial(105.0))
                        .column(Column::initial(110.0))
                        .column(Column::initial(75.0))
                        .column(Column::initial(75.0))
                        .column(Column::initial(95.0))
                        .column(Column::initial(75.0))
                        .column(Column::initial(80.0))
                        .column(Column::remainder())
                        .max_scroll_height(360.0)
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
                                    ui.label(RichText::new(h).size(11.5).strong().color(TEXT_MUTED));
                                });
                            }
                        })
                        .body(|body| {
                            body.rows(24.0, filtrados.len(), |mut row| {
                                let a = filtrados[row.index()];
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(format!("{} ({})", a.name, a.short_code))
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
                                        RichText::new(format!("{:.2} bn", a.market_cap_bi)).strong(),
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
                                        RichText::new(format!("{:.2}", a.pb)).color(GREEN_UP),
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

        // Atualização automática a cada 10 minutos (600 segundos) dos índices das bolsas mundiais
        if self.last_indices_refresh.elapsed() >= Duration::from_secs(600) {
            self.last_indices_refresh = Instant::now();
            let favs = self.settings.favorite_tickers.clone();
            self.bridge.request_overview(&favs, true, ctx.clone());
        }

        // Solicita redesenho periódico a cada 1s para atualizar o cronômetro de 10 minutos
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
                            RichText::new("Rust (egui/eframe) ↔ Python (yfinance • FRED • World Bank • SQLite)")
                                .size(11.0)
                                .color(ACCENT_CYAN),
                        );
                    });
                });
            });

        // Menu Lateral Esquerdo (Dax2)
        egui::SidePanel::left("left_sidebar_dax2")
            .resizable(false)
            .exact_width(245.0)
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::same(12.0)),
            )
            .show(ctx, |ui| {
                self.renderizar_menu_lateral(ui);
            });

        // Área Central Principal
        egui::CentralPanel::default().show(ctx, |ui| {
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

            egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                ActiveTab::Stocks => self.renderizar_aba_acoes(ui, ctx),
                ActiveTab::Forex => self.renderizar_aba_moedas(ui, ctx),
                ActiveTab::InterestRates => self.renderizar_aba_juros(ui, ctx),
                ActiveTab::MacroSocioeconomic => self.renderizar_aba_macro(ui, ctx),
                ActiveTab::DashboardScanner => self.renderizar_aba_dashboard_scanner(ui, ctx),
            });
        });
    }
}

