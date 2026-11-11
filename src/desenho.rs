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
        Estilo::Armadura => (Color32::TRANSPARENT, s(2.2, Color32::from_rgb(200, 30, 60))),
        Estilo::ArmaduraSecao => (Color32::from_rgb(200, 30, 60), s(1.0, Color32::from_rgb(140, 10, 40))),
        Estilo::Reaterro => {
            let f = if escuro { Color32::from_rgb(120, 104, 70) } else { Color32::from_rgb(226, 204, 158) };
            (f, s(0.6, Color32::from_rgb(140, 110, 70)))
        }
        Estilo::Hachura => (
            Color32::TRANSPARENT,
            s(
                0.6,
                if escuro { Color32::from_rgba_unmultiplied(220, 220, 220, 90) } else { Color32::from_rgba_unmultiplied(70, 55, 35, 120) },
            ),
        ),
    }
}

/// Cor e traço do estilo no tema claro (rasterização para o .docx).
pub(crate) fn cor_claro(estilo: Estilo) -> (Color32, Stroke) {
    cor(estilo, false)
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

/// Cor em notação CSS rgba().
fn css(c: Color32) -> String {
    format!("rgba({},{},{},{:.3})", c.r(), c.g(), c.b(), c.a() as f32 / 255.0)
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Seção transversal como SVG autônomo (tema claro) — para o relatório HTML.
pub fn svg(d: &Desenho, largura: f64, altura: f64) -> String {
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
    pts.retain(|p| p[0].is_finite() && p[1].is_finite());
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {largura} {altura}" width="100%" role="img" aria-label="Seção transversal"><rect width="{largura}" height="{altura}" rx="6" fill="rgb(246,247,250)"/>"#
    );
    if pts.is_empty() {
        s.push_str("</svg>");
        return s;
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &pts {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let margem = 24.0;
    let (w, h) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    let k = ((largura - 2.0 * margem) / w).min((altura - 2.0 * margem) / h);
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let tela = |p: [f64; 2]| (largura / 2.0 + (p[0] - mx) * k, altura / 2.0 - (p[1] - my) * k);
    let lista = |v: &[[f64; 2]]| {
        v.iter()
            .map(|&q| {
                let (x, y) = tela(q);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    for p in &d.poligonos {
        let (f, st) = cor(p.estilo, false);
        let traco = if matches!(p.estilo, Estilo::Empuxo | Estilo::EmpuxoNegativo) {
            r#"stroke="none""#.to_string()
        } else {
            format!(r#"stroke="{}" stroke-width="{}""#, css(st.color), st.width)
        };
        s.push_str(&format!(r#"<polygon points="{}" fill="{}" {traco}/>"#, lista(&p.pontos), css(f)));
    }
    for l in &d.linhas {
        let (_, st) = cor(l.estilo, false);
        let tr = if l.tracejada { r#" stroke-dasharray="8 5""# } else { "" };
        s.push_str(&format!(
            r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round"{tr}/>"#,
            lista(&l.pontos),
            css(st.color),
            st.width
        ));
    }
    let (_, sc) = cor(Estilo::Carga, false);
    for (a, b) in &d.setas {
        let (ax, ay) = tela(*a);
        let (bx, by) = tela(*b);
        let (dx, dy) = (bx - ax, by - ay);
        let n = (dx * dx + dy * dy).sqrt().max(1e-6);
        let (ux, uy) = (dx / n, dy / n);
        let t = 6.0;
        s.push_str(&format!(
            r#"<line x1="{ax:.1}" y1="{ay:.1}" x2="{bx:.1}" y2="{by:.1}" stroke="{c}" stroke-width="{w}"/><polygon points="{bx:.1},{by:.1} {:.1},{:.1} {:.1},{:.1}" fill="{c}"/>"#,
            bx - t * ux + 0.5 * t * uy,
            by - t * uy - 0.5 * t * ux,
            bx - t * ux - 0.5 * t * uy,
            by - t * uy + 0.5 * t * ux,
            c = css(sc.color),
            w = sc.width
        ));
    }
    for t in &d.textos {
        let (x, y) = tela(t.pos);
        s.push_str(&format!(
            r#"<text x="{x:.1}" y="{y:.1}" font-size="12" text-anchor="middle" dominant-baseline="middle" fill="rgb(31,35,40)">{}</text>"#,
            esc(&t.texto)
        ));
    }
    let passo = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0].into_iter().find(|&m| m * k > 60.0).unwrap_or(50.0);
    let (bx, by) = (14.0, altura - 12.0);
    s.push_str(&format!(
        r#"<line x1="{bx}" y1="{by}" x2="{:.1}" y2="{by}" stroke="rgb(31,35,40)" stroke-width="2"/><text x="{:.1}" y="{by}" font-size="12" dominant-baseline="middle" fill="rgb(31,35,40)">{} m</text>"#,
        bx + passo * k,
        bx + passo * k + 6.0,
        crate::resultado::fmt(passo)
    ));
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn svg_basico() {
        let mut d = Desenho::default();
        d.retangulo(Estilo::Concreto, 0.0, 0.0, 1.0, 2.0);
        d.tracejada(Estilo::Agua, vec![[0.0, 1.0], [2.0, 1.0]]);
        d.seta([0.5, 3.0], [0.5, 2.0]);
        d.texto([0.5, 1.0], "H = 2 m <x>");
        let s = svg(&d, 400.0, 300.0);
        assert!(s.starts_with("<svg") && s.ends_with("</svg>"));
        assert!(s.contains("<polygon") && s.contains("stroke-dasharray") && s.contains("&lt;x&gt;"));
    }
}
