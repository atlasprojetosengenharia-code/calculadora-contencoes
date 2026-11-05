//! Renderização da seção transversal (coordenadas em metros → tela).

use crate::resultado::{Desenho, Estilo};
use eframe::egui::{self, Color32, Pos2, Sense, Shape, Stroke};

fn cor(estilo: Estilo, escuro: bool) -> (Color32, Stroke) {
    let s = |w: f32, c: Color32| Stroke::new(w, c);
    match estilo {
        Estilo::Concreto => {
            let f = if escuro { Color32::from_rgb(125, 125, 132) } else { Color32::from_rgb(178, 178, 182) };
            (f, s(1.2, Color32::from_rgb(60, 60, 66)))
        }
        Estilo::Aco => (Color32::from_rgb(70, 90, 150), s(1.2, Color32::from_rgb(30, 40, 90))),
        Estilo::Solo => {
            let f = if escuro { Color32::from_rgb(112, 90, 58) } else { Color32::from_rgb(214, 185, 132) };
            (f, s(0.6, Color32::from_rgb(140, 110, 70)))
        }
        Estilo::SoloFundacao => {
            let f = if escuro { Color32::from_rgb(88, 72, 50) } else { Color32::from_rgb(180, 150, 105) };
            (f, s(0.6, Color32::from_rgb(120, 95, 60)))
        }
        Estilo::Agua => (Color32::TRANSPARENT, s(1.8, Color32::from_rgb(40, 130, 230))),
        Estilo::Tirante => (Color32::TRANSPARENT, s(2.0, Color32::from_rgb(210, 40, 40))),
        Estilo::Bulbo => (Color32::TRANSPARENT, s(6.0, Color32::from_rgb(130, 35, 35))),
        Estilo::Empuxo => (
            Color32::from_rgba_unmultiplied(230, 60, 60, 110),
            s(0.8, Color32::from_rgb(200, 40, 40)),
        ),
        Estilo::EmpuxoNegativo => (
            Color32::from_rgba_unmultiplied(60, 120, 230, 110),
            s(0.8, Color32::from_rgb(40, 90, 200)),
        ),
        Estilo::Carga => (Color32::from_rgb(235, 130, 40), s(1.5, Color32::from_rgb(200, 100, 20))),
        Estilo::Ruptura => (Color32::TRANSPARENT, s(1.6, Color32::from_rgb(210, 0, 140))),
        Estilo::Momento => (Color32::TRANSPARENT, s(2.0, Color32::from_rgb(130, 50, 200))),
        Estilo::Cota => (Color32::TRANSPARENT, s(1.0, Color32::from_gray(120))),
        Estilo::Dreno => {
            let f = if escuro { Color32::from_rgb(92, 104, 112) } else { Color32::from_rgb(196, 204, 210) };
            (f, s(1.0, Color32::from_rgb(70, 160, 200)))
        }
        Estilo::Barbaca => (Color32::from_rgb(40, 140, 220), s(3.0, Color32::from_rgb(40, 140, 220))),
    }
}

pub fn pintar(ui: &mut egui::Ui, d: &Desenho, altura: f32) {
    let largura = ui.available_width();
    let (resp, painter) = ui.allocate_painter(egui::vec2(largura, altura), Sense::hover());
    let rect = resp.rect;
    let escuro = ui.visuals().dark_mode;
    let fundo = if escuro { Color32::from_rgb(28, 30, 34) } else { Color32::from_rgb(246, 247, 250) };
    painter.rect_filled(rect, 4.0, fundo);

    // limites
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for p in &d.poligonos {
        pts.extend(&p.pontos);
    }
    for l in &d.linhas {
        pts.extend(&l.pontos);
    }
    for (a, b) in &d.setas {
        pts.push(*a);
        pts.push(*b);
    }
    for t in &d.textos {
        pts.push(t.pos);
    }
    let pts: Vec<[f64; 2]> = pts.into_iter().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
    if pts.is_empty() {
        return;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &pts {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let margem = 24.0f64;
    let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    let s = ((rect.width() as f64 - 2.0 * margem) / w).min((rect.height() as f64 - 2.0 * margem) / h);
    let cx = rect.center().x as f64;
    let cy = rect.center().y as f64;
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let tela = |p: [f64; 2]| Pos2::new((cx + (p[0] - mx) * s) as f32, (cy - (p[1] - my) * s) as f32);

    let painter = painter.with_clip_rect(rect);
    for p in &d.poligonos {
        let (f, mut st) = cor(p.estilo, escuro);
        if matches!(p.estilo, Estilo::Empuxo | Estilo::EmpuxoNegativo) {
            // faixas do diagrama sem contorno (o contorno é desenhado como linha)
            st = Stroke::NONE;
        }
        let v: Vec<Pos2> = p.pontos.iter().map(|&q| tela(q)).collect();
        painter.add(Shape::convex_polygon(v, f, st));
    }
    for l in &d.linhas {
        let (_, st) = cor(l.estilo, escuro);
        let v: Vec<Pos2> = l.pontos.iter().map(|&q| tela(q)).collect();
        if l.tracejada {
            painter.extend(Shape::dashed_line(&v, st, 8.0, 5.0));
        } else {
            painter.add(Shape::line(v, st));
        }
    }
    let (_, st_carga) = cor(Estilo::Carga, escuro);
    for (a, b) in &d.setas {
        let pa = tela(*a);
        let pb = tela(*b);
        painter.arrow(pa, pb - pa, st_carga);
    }
    let cor_texto = ui.visuals().strong_text_color();
    for t in &d.textos {
        painter.text(tela(t.pos), egui::Align2::CENTER_CENTER, &t.texto, egui::FontId::proportional(13.0), cor_texto);
    }
    // escala gráfica
    let passo = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0].into_iter().find(|&m| m * s > 60.0).unwrap_or(50.0);
    let base = Pos2::new(rect.left() + 14.0, rect.bottom() - 12.0);
    let fim = base + egui::vec2((passo * s) as f32, 0.0);
    painter.line_segment([base, fim], Stroke::new(2.0_f32, cor_texto));
    painter.text(fim + egui::vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, format!("{passo} m"), egui::FontId::proportional(12.0), cor_texto);
}
