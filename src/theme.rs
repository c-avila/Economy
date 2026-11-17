//! Color palette, visual components (Badges, Cards, Sparklines), and Dark/Light theme support.

use eframe::egui::{self, Color32, Pos2, RichText, Rounding, Stroke, Ui, Vec2};

// Core Dark Mode palette
pub const BG_PRIMARY: Color32 = Color32::from_rgb(13, 17, 26);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(18, 23, 35);
pub const BG_CARD: Color32 = Color32::from_rgb(24, 31, 46);
pub const BG_CARD_ALT: Color32 = Color32::from_rgb(30, 39, 58);
pub const BG_HERO_BLUE: Color32 = Color32::from_rgb(9, 75, 130);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(45, 56, 78);

pub const GREEN_UP: Color32 = Color32::from_rgb(0, 214, 114);
pub const RED_DOWN: Color32 = Color32::from_rgb(235, 38, 86);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 158, 11);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(241, 245, 249);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);

/// Dynamic theme palette supporting both Dark Mode and Light Mode.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub bg_primary: Color32,
    pub bg_sidebar: Color32,
    pub bg_card: Color32,
    pub bg_card_alt: Color32,
    pub border_subtle: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub accent_cyan: Color32,
    pub green_up: Color32,
}

pub fn palette(dark: bool) -> ThemePalette {
    if dark {
        ThemePalette {
            bg_primary: BG_PRIMARY,
            bg_sidebar: BG_SIDEBAR,
            bg_card: BG_CARD,
            bg_card_alt: BG_CARD_ALT,
            border_subtle: BORDER_SUBTLE,
            text_primary: TEXT_LIGHT,
            text_muted: TEXT_MUTED,
            accent_cyan: ACCENT_CYAN,
            green_up: GREEN_UP,
        }
    } else {
        ThemePalette {
            bg_primary: Color32::from_rgb(241, 245, 249),
            bg_sidebar: Color32::from_rgb(226, 232, 240),
            bg_card: Color32::from_rgb(255, 255, 255),
            bg_card_alt: Color32::from_rgb(248, 250, 252),
            border_subtle: Color32::from_rgb(203, 213, 225),
            text_primary: Color32::from_rgb(15, 23, 42),
            text_muted: Color32::from_rgb(71, 85, 105),
            accent_cyan: Color32::from_rgb(2, 132, 199),
            green_up: Color32::from_rgb(5, 150, 105),
        }
    }
}

/// Applies the DAX Dark or Light theme to the `egui` context.
pub fn aplicar_tema_dax(ctx: &egui::Context, dark: bool) {
    let pal = palette(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = pal.bg_primary;
    visuals.window_fill = pal.bg_card;
    visuals.faint_bg_color = pal.bg_card_alt;
    visuals.extreme_bg_color = if dark {
        Color32::from_rgb(10, 14, 22)
    } else {
        Color32::from_rgb(226, 232, 240)
    };
    visuals.widgets.noninteractive.bg_fill = pal.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, pal.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, pal.text_primary);

    visuals.widgets.inactive.bg_fill = pal.bg_card_alt;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, pal.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, pal.text_primary);

    visuals.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(38, 52, 77)
    } else {
        Color32::from_rgb(224, 242, 254)
    };
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2_f32, pal.green_up);

    visuals.widgets.active.bg_fill = pal.green_up;
    visuals.selection.bg_fill = if dark {
        Color32::from_rgb(14, 116, 144)
    } else {
        Color32::from_rgb(14, 165, 233)
    };
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    ctx.set_visuals(visuals);
}

/// Returns green for positive variations and red for negative ones.
pub fn cor_variacao(valor: f64) -> Color32 {
    if valor >= 0.0 {
        GREEN_UP
    } else {
        RED_DOWN
    }
}

/// Renders a colored badge (e.g., `+2.95%` on green or `-1.10%` on red).
pub fn badge_variacao(ui: &mut Ui, pct: f64) {
    let bg = if pct >= 0.0 { GREEN_UP } else { RED_DOWN };
    let sinal = if pct > 0.0 { "+" } else { "" };
    let texto = format!("{sinal}{pct:.2}%");

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

/// Draws a mini trend line (Sparkline) directly on the `egui` canvas.
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

    let y_base = pontos.first().map(|p| p.y).unwrap_or(rect.center().y);
    painter.line_segment(
        [Pos2::new(rect.left(), y_base), Pos2::new(rect.right(), y_base)],
        Stroke::new(0.6_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 50)),
    );

    painter.add(egui::Shape::line(pontos, Stroke::new(1.8_f32, cor_linha)));
}

/// Formats numbers with currency prefix.
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
    let s = format!("{valor:.2}");
    format!("{prefixo} {s}")
}

/// Standard card frame respecting the active Dark/Light theme.
pub fn card_frame_mode(dark: bool) -> egui::Frame {
    let pal = palette(dark);
    egui::Frame::none()
        .fill(pal.bg_card)
        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
}

/// Highlighted blue card frame for Current Price / US Fed Rate.
pub fn hero_blue_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_HERO_BLUE)
        .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::same(12.0))
}

/// Renders a horizontal legend bar positioned BELOW the chart.
pub fn renderizar_legenda_abaixo(ui: &mut Ui, itens: &[(Color32, String)]) {
    if itens.is_empty() {
        return;
    }
    let pal = palette(ui.visuals().dark_mode);
    ui.add_space(4.0);
    egui::Frame::none()
        .fill(pal.bg_card_alt)
        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
        .rounding(Rounding::same(5.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(14.0, 5.0);
                ui.label(
                    RichText::new("Legend:")
                        .size(11.0)
                        .strong()
                        .color(pal.text_muted),
                );
                for (cor, rotulo) in itens {
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        "▬ ",
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(13.0),
                            color: *cor,
                            ..Default::default()
                        },
                    );
                    job.append(
                        rotulo,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(11.5),
                            color: pal.text_primary,
                            ..Default::default()
                        },
                    );
                    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                }
            });
        });
}
