//! Paleta de cores, componentes visuais (Badges, Cards e Sparklines) inspirados nas imagens Dax1 a Dax4.

use eframe::egui::{self, Color32, Pos2, RichText, Rounding, Stroke, Ui, Vec2};

// Cores principais do tema Dark Financeiro (TradeMap / xStation / StatusInvest Dark)
pub const BG_PRIMARY: Color32 = Color32::from_rgb(13, 17, 26);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(18, 23, 35);
pub const BG_CARD: Color32 = Color32::from_rgb(24, 31, 46);
pub const BG_CARD_ALT: Color32 = Color32::from_rgb(30, 39, 58);
pub const BG_HERO_BLUE: Color32 = Color32::from_rgb(9, 75, 130); // Destaque azul "VALOR ATUAL" da imagem Dax3
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(45, 56, 78);

pub const GREEN_UP: Color32 = Color32::from_rgb(0, 214, 114); // Verde vibrante das imagens Dax1 e Dax2
pub const RED_DOWN: Color32 = Color32::from_rgb(235, 38, 86); // Vermelho/Carmesim das imagens Dax1 e Dax2
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 158, 11);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(241, 245, 249);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

/// Aplica o tema escuro profissional inspirado nas imagens Dax1, Dax2 e Dax4.
pub fn aplicar_tema_dax(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_PRIMARY;
    visuals.window_fill = BG_CARD;
    visuals.faint_bg_color = Color32::from_rgb(19, 25, 38);
    visuals.extreme_bg_color = Color32::from_rgb(10, 14, 22);
    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);

    visuals.widgets.inactive.bg_fill = BG_CARD_ALT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_LIGHT);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 52, 77);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2_f32, GREEN_UP);

    visuals.widgets.active.bg_fill = Color32::from_rgb(0, 168, 90);
    visuals.selection.bg_fill = Color32::from_rgb(14, 116, 144);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);
}

/// Retorna a cor verde para variações positivas e vermelha para negativas.
pub fn cor_variacao(valor: f64) -> Color32 {
    if valor >= 0.0 {
        GREEN_UP
    } else {
        RED_DOWN
    }
}

/// Renderiza um badge colorido estilo Dax1 / Dax2 (ex.: `+2,95%` em fundo verde ou `-1,10%` em fundo vermelho).
pub fn badge_variacao(ui: &mut Ui, pct: f64) {
    let bg = if pct >= 0.0 { GREEN_UP } else { RED_DOWN };
    let sinal = if pct > 0.0 { "+" } else { "" };
    let texto = format!("{sinal}{pct:.2}%").replace('.', ",");

    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(4.0))
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(texto)
                    .color(Color32::from_rgb(10, 14, 20))
                    .strong()
                    .size(11.5),
            );
        });
}

/// Desenha uma mini-linha de tendência (Sparkline) diretamente no canvas do `egui` (como em Dax1 e Dax2).
pub fn desenhar_sparkline(ui: &mut Ui, valores: &[f64], tamanho: Vec2, positivo: bool) {
    let (rect, _resp) = ui.allocate_exact_size(tamanho, egui::Sense::hover());
    if !ui.is_rect_visible(rect) || valores.len() < 2 {
        return;
    }

    let painter = ui.painter_at(rect);
    let min_v = valores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let amplitude = (max_v - min_v).max(1e-6);

    let cor_linha = if positivo { GREEN_UP } else { RED_DOWN };
    let n = valores.len();
    let mut pontos: Vec<Pos2> = Vec::with_capacity(n);

    for (i, &val) in valores.iter().enumerate() {
        let x = rect.left() + (i as f32 / (n - 1) as f32) * rect.width();
        let norm_y = ((val - min_v) / amplitude) as f32;
        let y = rect.bottom() - 3.0 - norm_y * (rect.height() - 6.0);
        pontos.push(Pos2::new(x, y));
    }

    // Linha de base sutil
    let y_base = pontos.first().map(|p| p.y).unwrap_or(rect.center().y);
    painter.line_segment(
        [Pos2::new(rect.left(), y_base), Pos2::new(rect.right(), y_base)],
        Stroke::new(0.6_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 50)),
    );

    painter.add(egui::Shape::line(pontos, Stroke::new(1.8_f32, cor_linha)));
}

/// Formata números no padrão brasileiro (vírgula decimal) com sufixo ou prefixo opcional.
pub fn formatar_moeda(valor: f64, moeda: &str) -> String {
    let prefixo = match moeda.to_uppercase().as_str() {
        "BRL" => "R$",
        "USD" => "US$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        "CNY" => "¥",
        _ => moeda,
    };
    let s = format!("{valor:.2}").replace('.', ",");
    format!("{prefixo} {s}")
}

/// Formata um painel de card padrão com borda arredondada estilo TradeMap (Dax2).
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_CARD)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
}

/// Retângulo de card azul destaque para o "VALOR ATUAL" da imagem Dax3.
pub fn hero_blue_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_HERO_BLUE)
        .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::same(12.0))
}
