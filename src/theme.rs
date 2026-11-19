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

/// Euclidean distance in screen pixels from point `p` to line segment `[a, b]`.
pub fn distancia_ponto_segmento(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq <= 1e-5 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + t * ab;
    p.distance(proj)
}

/// Checks if the mouse pointer is directly over the plotted line (`<= raio_px` in screen pixels).
/// Returns `Some((nearest_data_index, pixel_distance_to_line, screen_pos_of_data_point))` only when
/// the cursor is hovering over the line.
pub fn encontrar_ponto_serie_sob_mouse(
    plot_ui: &egui_plot::PlotUi,
    pontos: &[[f64; 2]],
    raio_px: f32,
) -> Option<(usize, f32, Pos2)> {
    if pontos.is_empty() {
        return None;
    }
    let plot_rect = plot_ui.response().rect;
    let mouse_pos = plot_ui.ctx().pointer_hover_pos()?;
    if !plot_rect.contains(mouse_pos) {
        return None;
    }

    let screen_pts: Vec<Pos2> = pontos
        .iter()
        .map(|pt| plot_ui.screen_from_plot(egui_plot::PlotPoint::new(pt[0], pt[1])))
        .collect();

    if screen_pts.len() == 1 {
        let d = mouse_pos.distance(screen_pts[0]);
        return if d <= raio_px {
            Some((0, d, screen_pts[0]))
        } else {
            None
        };
    }

    let mut menor_dist_seg = f32::INFINITY;
    let mut seg_mais_proximo = 0_usize;

    for i in 0..(screen_pts.len() - 1) {
        let d = distancia_ponto_segmento(mouse_pos, screen_pts[i], screen_pts[i + 1]);
        if d < menor_dist_seg {
            menor_dist_seg = d;
            seg_mais_proximo = i;
        }
    }

    if menor_dist_seg > raio_px {
        return None;
    }

    // Choose the closer data vertex of the hovered segment (or neighbor by X)
    let p_a = screen_pts[seg_mais_proximo];
    let p_b = screen_pts[seg_mais_proximo + 1];
    let idx_escolhido = if (mouse_pos.x - p_a.x).abs() <= (mouse_pos.x - p_b.x).abs() {
        seg_mais_proximo
    } else {
        seg_mais_proximo + 1
    };

    Some((idx_escolhido, menor_dist_seg, screen_pts[idx_escolhido]))
}

/// Draws an edge-aware floating tooltip showing ONLY the hovered variable's name, value, and date.
/// Automatically flips position (left/right/above/below) when the mouse is near any border of the chart
/// so the tooltip never disappears or gets clipped.
pub fn desenhar_tooltip_serie_borda_segura(
    plot_ui: &egui_plot::PlotUi,
    anchor_screen: Pos2,
    cor_serie: Color32,
    titulo_serie: &str,
    linha_valor: &str,
    linha_data: &str,
) {
    let ctx = plot_ui.ctx();
    let plot_rect = plot_ui.response().rect;
    let screen_rect = ctx.screen_rect();
    let mouse_pos = ctx.pointer_hover_pos().unwrap_or(anchor_screen);
    let dark = ctx.style().visuals.dark_mode;
    let pal = palette(dark);

    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("▬ {titulo_serie}\n"),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.5),
            color: cor_serie,
            ..Default::default()
        },
    );
    job.append(
        &format!("{linha_valor}\n"),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(12.5),
            color: pal.text_primary,
            ..Default::default()
        },
    );
    job.append(
        &format!("📅 {linha_data}"),
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::proportional(11.5),
            color: pal.text_muted,
            ..Default::default()
        },
    );

    let galley = ctx.fonts(|f| f.layout_job(job));
    let pad = Vec2::new(16.0, 12.0);
    let box_size = galley.size() + pad;

    // Default position: top-right of the mouse pointer
    let mut x = mouse_pos.x + 14.0;
    let mut y = mouse_pos.y - box_size.y - 10.0;

    // Smart border proximity checks against both plot_rect and window screen_rect
    let limite_dir = plot_rect.right().min(screen_rect.right() - 6.0);
    let limite_esq = plot_rect.left().max(screen_rect.left() + 6.0);
    let limite_topo = plot_rect.top().max(screen_rect.top() + 6.0);
    let limite_fundo = plot_rect.bottom().min(screen_rect.bottom() - 6.0);

    // Near right edge -> flip tooltip to the left of the cursor
    if x + box_size.x > limite_dir - 6.0 {
        x = mouse_pos.x - box_size.x - 14.0;
    }
    // Near left edge -> clamp inside left border
    if x < limite_esq + 4.0 {
        x = limite_esq + 4.0;
    }
    // Near top edge -> flip tooltip below the cursor
    if y < limite_topo + 4.0 {
        y = mouse_pos.y + 16.0;
    }
    // Near bottom edge -> shift tooltip above bottom border
    if y + box_size.y > limite_fundo - 4.0 {
        y = (mouse_pos.y - box_size.y - 12.0).max(limite_topo + 4.0);
    }

    let layer_id = egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("dax_chart_smart_edge_tooltip"),
    );
    let painter = ctx.layer_painter(layer_id);

    // Highlighted dot on the exact hovered point of the curve
    if plot_rect.contains(anchor_screen) {
        painter.circle_filled(anchor_screen, 5.5, cor_serie);
        painter.circle_stroke(anchor_screen, 5.5, Stroke::new(1.5_f32, Color32::WHITE));
    }

    let box_rect = egui::Rect::from_min_size(Pos2::new(x, y), box_size);
    let bg_fill = if dark {
        Color32::from_rgba_unmultiplied(15, 21, 33, 246)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 248)
    };
    painter.rect(
        box_rect,
        Rounding::same(6.0),
        bg_fill,
        Stroke::new(1.4_f32, cor_serie),
    );
    painter.galley(
        box_rect.min + Vec2::new(8.0, 6.0),
        galley,
        pal.text_primary,
    );
}
