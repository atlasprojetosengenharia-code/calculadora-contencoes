//! Componentes reutilizáveis: cartões, campos numéricos, botões e logotipo.

use super::tema::{self, Paleta};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Shape, Stroke};
use std::ops::RangeInclusive;

/// Cartão com fundo, borda e cantos arredondados.
pub fn cartao<R>(ui: &mut egui::Ui, conteudo: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let p = tema::de(ui);
    egui::Frame::new()
        .fill(p.cartao)
        .stroke(Stroke::new(1.0_f32, p.borda))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), conteudo).inner
        })
}

/// Título de seção com barra de destaque.
pub fn titulo(ui: &mut egui::Ui, texto: &str) {
    let p = tema::de(ui);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(4.0, 18.0), Sense::hover());
        ui.painter().rect_filled(r, 2.0, p.destaque);
        ui.label(RichText::new(texto).font(tema::fonte_forte(16.0)).color(p.texto));
    });
    ui.add_space(2.0);
}

pub fn nota(ui: &mut egui::Ui, texto: &str) {
    let p = tema::de(ui);
    ui.label(RichText::new(texto).small().color(p.fraco));
}

/// Linha de grade "rótulo | valor com unidade".
pub fn num(ui: &mut egui::Ui, rotulo: &str, v: &mut f64, unid: &str, faixa: RangeInclusive<f64>, vel: f64) -> bool {
    ui.label(rotulo);
    let mut dv = egui::DragValue::new(v).speed(vel).range(faixa).max_decimals(3);
    if !unid.is_empty() {
        dv = dv.suffix(format!(" {unid}"));
    }
    let r = ui.add_sized([110.0, 24.0], dv);
    ui.end_row();
    r.changed()
}

pub fn grade(ui: &mut egui::Ui, id: &str, f: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([14.0, 6.0]).min_col_width(150.0).show(ui, f);
}

pub fn botao_primario(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    let p = tema::de(ui);
    ui.add(
        egui::Button::new(RichText::new(texto).color(Color32::WHITE).font(tema::fonte_forte(15.0)))
            .fill(p.destaque)
            .stroke(Stroke::NONE)
            .corner_radius(8.0)
            .min_size(egui::vec2(220.0, 42.0)),
    )
}

pub fn botao_secundario(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    let p = tema::de(ui);
    ui.add(
        egui::Button::new(RichText::new(texto).color(p.texto).font(tema::fonte_forte(15.0)))
            .fill(p.cartao)
            .stroke(Stroke::new(1.0_f32, p.borda))
            .corner_radius(8.0)
            .min_size(egui::vec2(220.0, 42.0)),
    )
}

/// Etiqueta colorida (status).
pub fn chip(ui: &mut egui::Ui, texto: &str, cor: Color32) {
    egui::Frame::new()
        .fill(cor.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0_f32, cor))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(texto).color(cor).font(tema::fonte_forte(13.0)));
        });
}

/// Logotipo vetorial: muro de contenção com empuxo.
pub fn pintar_logo(pt: &egui::Painter, r: Rect, p: &Paleta) {
    pt.rect_filled(r, r.width() * 0.22, p.destaque);
    let m = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    let branco = Color32::WHITE;
    let solo = Color32::from_rgba_unmultiplied(255, 255, 255, 70);
    pt.add(Shape::convex_polygon(vec![m(0.56, 0.30), m(0.86, 0.30), m(0.86, 0.78), m(0.60, 0.78)], solo, Stroke::NONE));
    pt.add(Shape::convex_polygon(vec![m(0.16, 0.78), m(0.60, 0.78), m(0.56, 0.24), m(0.42, 0.24)], branco, Stroke::NONE));
    pt.line_segment([m(0.10, 0.78), m(0.90, 0.78)], Stroke::new(r.width() * 0.035, branco));
    for (i, y) in [0.42, 0.54, 0.66].into_iter().enumerate() {
        let len = 0.10 + 0.04 * i as f32;
        let a = m(0.86 - 0.02, y);
        let b = m(0.86 - 0.02 - len, y);
        pt.line_segment([a, b], Stroke::new(r.width() * 0.025, branco));
        pt.line_segment([b, m(0.86 - 0.02 - len + 0.04, y - 0.035)], Stroke::new(r.width() * 0.025, branco));
        pt.line_segment([b, m(0.86 - 0.02 - len + 0.04, y + 0.035)], Stroke::new(r.width() * 0.025, branco));
    }
}

pub fn logo(ui: &mut egui::Ui, tam: f32) {
    let p = tema::de(ui);
    let (r, _) = ui.allocate_exact_size(egui::vec2(tam, tam), Sense::hover());
    pintar_logo(ui.painter(), r, &p);
}

/// Croqui de referência do muro de gravidade (letras A–F e H).
pub fn croqui_gravidade(ui: &mut egui::Ui) {
    let p = tema::de(ui);
    let (r, _) = ui.allocate_exact_size(egui::vec2(220.0, 150.0), Sense::hover());
    let pt = ui.painter();
    pt.rect_filled(r, 8.0, p.entrada);
    let m = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    let cor = Color32::from_rgb(230, 200, 60);
    let s = Stroke::new(1.6_f32, p.texto);
    let pts = [
        m(0.15, 0.88), m(0.72, 0.88), m(0.72, 0.74), m(0.68, 0.74),
        m(0.60, 0.14), m(0.48, 0.14), m(0.22, 0.74), m(0.15, 0.74),
    ];
    for i in 0..pts.len() {
        pt.line_segment([pts[i], pts[(i + 1) % pts.len()]], s);
    }
    let t = |pt: &egui::Painter, pos: Pos2, txt: &str| {
        pt.text(pos, egui::Align2::CENTER_CENTER, txt, tema::fonte_forte(13.0), cor);
    };
    t(pt, m(0.19, 0.69), "A");
    t(pt, m(0.30, 0.42), "B");
    t(pt, m(0.54, 0.08), "C");
    t(pt, m(0.70, 0.42), "D");
    t(pt, m(0.76, 0.69), "E");
    t(pt, m(0.78, 0.82), "F");
    t(pt, m(0.44, 0.95), "Base");
    pt.line_segment([m(0.86, 0.14), m(0.86, 0.74)], Stroke::new(1.0_f32, cor));
    pt.line_segment([m(0.83, 0.14), m(0.89, 0.14)], Stroke::new(1.0_f32, cor));
    pt.line_segment([m(0.83, 0.74), m(0.89, 0.74)], Stroke::new(1.0_f32, cor));
    t(pt, m(0.93, 0.44), "H");
}
