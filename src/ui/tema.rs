//! Tema visual: paleta, fontes (Segoe UI no Windows), espaçamentos e cantos.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

pub const FORTE: &str = "forte";

#[derive(Clone, Copy)]
pub struct Paleta {
    pub fundo: Color32,
    pub painel: Color32,
    pub cartao: Color32,
    pub borda: Color32,
    pub texto: Color32,
    pub fraco: Color32,
    pub destaque: Color32,
    pub destaque_suave: Color32,
    pub ok: Color32,
    pub falha: Color32,
    pub alerta: Color32,
    pub entrada: Color32,
}

pub fn paleta(escuro: bool) -> Paleta {
    let c = Color32::from_rgb;
    if escuro {
        Paleta {
            fundo: c(13, 17, 23),
            painel: c(20, 25, 32),
            cartao: c(26, 32, 41),
            borda: c(44, 52, 64),
            texto: c(230, 237, 243),
            fraco: c(139, 148, 158),
            destaque: c(56, 139, 253),
            destaque_suave: c(28, 52, 86),
            ok: c(63, 185, 80),
            falha: c(248, 81, 73),
            alerta: c(214, 158, 46),
            entrada: c(15, 19, 25),
        }
    } else {
        Paleta {
            fundo: c(242, 244, 247),
            painel: c(255, 255, 255),
            cartao: c(255, 255, 255),
            borda: c(214, 220, 227),
            texto: c(31, 35, 40),
            fraco: c(101, 109, 118),
            destaque: c(9, 105, 218),
            destaque_suave: c(221, 234, 252),
            ok: c(26, 127, 55),
            falha: c(207, 34, 46),
            alerta: c(154, 103, 0),
            entrada: c(247, 249, 251),
        }
    }
}

pub fn de(ui: &egui::Ui) -> Paleta {
    paleta(ui.visuals().dark_mode)
}

fn visuais(escuro: bool) -> egui::Visuals {
    let p = paleta(escuro);
    let mut v = if escuro { egui::Visuals::dark() } else { egui::Visuals::light() };
    let r = CornerRadius::same(6);
    v.panel_fill = p.fundo;
    v.window_fill = p.cartao;
    v.window_stroke = Stroke::new(1.0_f32, p.borda);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.extreme_bg_color = p.entrada;
    v.faint_bg_color = if escuro { Color32::from_rgb(31, 38, 48) } else { Color32::from_rgb(244, 246, 249) };
    v.hyperlink_color = p.destaque;
    v.selection.bg_fill = p.destaque;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    let botao = if escuro { Color32::from_rgb(36, 44, 56) } else { Color32::from_rgb(238, 241, 245) };
    let botao_hover = if escuro { Color32::from_rgb(46, 56, 70) } else { Color32::from_rgb(226, 232, 239) };

    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.borda);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.texto);
    v.widgets.noninteractive.corner_radius = r;

    v.widgets.inactive.weak_bg_fill = botao;
    v.widgets.inactive.bg_fill = botao;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.borda);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.texto);
    v.widgets.inactive.corner_radius = r;

    v.widgets.hovered.weak_bg_fill = botao_hover;
    v.widgets.hovered.bg_fill = botao_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.destaque);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, p.texto);
    v.widgets.hovered.corner_radius = r;

    v.widgets.active.weak_bg_fill = p.destaque_suave;
    v.widgets.active.bg_fill = p.destaque_suave;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.destaque);
    v.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.texto);
    v.widgets.active.corner_radius = r;

    v.widgets.open.weak_bg_fill = botao_hover;
    v.widgets.open.corner_radius = r;
    v
}

fn fontes(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let carregar = |nome: &str| std::fs::read(format!("C:\\Windows\\Fonts\\{nome}")).ok();
    let mut prop = f.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(b) = carregar("segoeui.ttf") {
        f.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(b)));
        prop.insert(0, "segoe".into());
    }
    let mut forte = prop.clone();
    if let Some(b) = carregar("segoeuisb.ttf").or_else(|| carregar("seguisb.ttf")) {
        f.font_data.insert("segoe_sb".into(), Arc::new(FontData::from_owned(b)));
        forte.insert(0, "segoe_sb".into());
    }
    f.families.insert(FontFamily::Proportional, prop);
    f.families.insert(FontFamily::Name(FORTE.into()), forte);
    ctx.set_fonts(f);
}

pub fn aplicar(ctx: &egui::Context) {
    fontes(ctx);
    ctx.set_visuals_of(egui::Theme::Dark, visuais(true));
    ctx.set_visuals_of(egui::Theme::Light, visuais(false));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 7.0);
        s.spacing.button_padding = egui::vec2(12.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.spacing.indent = 16.0;
        s.text_styles = [
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(22.0, FontFamily::Name(FORTE.into()))),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
    });
}

pub fn fonte_forte(tam: f32) -> FontId {
    FontId::new(tam, FontFamily::Name(FORTE.into()))
}
