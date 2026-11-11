//! Janela de visualização 3D da contenção: renderização por software
//! (triângulos ordenados por profundidade, sombreamento difuso) com órbita,
//! zoom, vistas prontas e exportação IFC.

use super::tema;
use crate::modelo3d::{Classe, Material, Modelo3d, Tri};
use crate::projeto::Projeto;
use crate::resultado::{fmt, Resultado};
use eframe::egui::{self, Color32, Pos2, RichText, Sense, Shape, Stroke};

/// Estado da câmera e opções da janela (não é salvo no projeto).
#[derive(Clone, Debug)]
pub struct Camera {
    pub azimute: f32,
    pub elevacao: f32,
    pub zoom: f32,
    pub comprimento: f64,
    pub terreno: bool,
    pub reforcos: bool,
    pub drenagem: bool,
    /// Maciço de solo só até a metade do trecho (vista em corte).
    pub corte: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self { azimute: -0.75, elevacao: 0.45, zoom: 1.0, comprimento: 10.0, terreno: true, reforcos: true, drenagem: true, corte: true }
    }
}

struct Prim {
    pts: [Pos2; 3],
    prof: f32,
    cor: Color32,
}

fn cor_material(m: Material, luz: f32, escuro: bool) -> Color32 {
    let ([r, g, b], t) = m.cor();
    let k = luz.clamp(0.0, 1.15);
    let alfa = ((1.0 - t) * 255.0) as u8;
    let _ = escuro;
    Color32::from_rgba_unmultiplied((r as f32 * k * 255.0) as u8, (g as f32 * k * 255.0) as u8, (b as f32 * k * 255.0) as u8, alfa)
}

/// Divide triângulos maiores que `s` (melhora a ordenação do pintor e permite textura).
fn subdividir(t: Tri, s: f64, prof: u32, saida: &mut Vec<Tri>) {
    let d = |a: [f64; 3], b: [f64; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let l = [d(t[0], t[1]), d(t[1], t[2]), d(t[2], t[0])];
    let (i, lm) = l.iter().enumerate().fold((0, 0.0), |acc, (k, &v)| if v > acc.1 { (k, v) } else { acc });
    if lm <= s || prof == 0 {
        saida.push(t);
        return;
    }
    let (a, b, c) = (t[i], t[(i + 1) % 3], t[(i + 2) % 3]);
    let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
    subdividir([a, m, c], s, prof - 1, saida);
    subdividir([m, b, c], s, prof - 1, saida);
}

/// Ruído determinístico em [−1, 1] para a célula da textura.
fn ruido(p: [f64; 3], grao: f64) -> f32 {
    let k = |v: f64| (v / grao).floor() as i64;
    let mut h = (k(p[0]).wrapping_mul(73_856_093) ^ k(p[1]).wrapping_mul(19_349_663) ^ k(p[2]).wrapping_mul(83_492_791)) as u64;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    ((h % 2001) as f32 / 1000.0) - 1.0
}

/// Desenha o modelo na área `rect`.
fn renderizar(painter: &egui::Painter, rect: egui::Rect, m: &Modelo3d, cam: &Camera, escuro: bool) {
    let (a, b) = m.caixa();
    let centro = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
    let raio = (((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() / 2.0).max(0.5) as f32;
    let (sa, ca) = cam.azimute.sin_cos();
    let (se, ce) = cam.elevacao.sin_cos();
    let dist = 3.2 * raio;
    let foco = 0.5 * rect.width().min(rect.height()) * 2.6 * cam.zoom;
    // espaço da câmera: rotação em torno do eixo vertical (y) e depois do horizontal
    let cam_pt = |v: [f64; 3]| -> [f32; 3] {
        let (x, y, z) = ((v[0] - centro[0]) as f32, (v[1] - centro[1]) as f32, (v[2] - centro[2]) as f32);
        let x1 = ca * x + sa * z;
        let z1 = -sa * x + ca * z;
        let y2 = ce * y - se * z1;
        let z2 = se * y + ce * z1;
        [x1, y2, z2]
    };
    let proj = |c: [f32; 3]| -> Pos2 {
        let d = (dist - c[2]).max(0.05 * raio);
        rect.center() + egui::vec2(foco * c[0] / d, -foco * c[1] / d)
    };
    // luz difusa vinda do observador, de cima e da esquerda
    let luz = {
        let l = [-0.35f32, 0.6, 0.72];
        let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        [l[0] / n, l[1] / n, l[2] / n]
    };
    // tamanho máximo dos triângulos (ordem do pintor estável e textura visível)
    let passo = (raio as f64 / 14.0).clamp(0.15, 1.5);
    let mut prims: Vec<Prim> = Vec::new();
    for pc in &m.pecas {
        if (pc.classe == Classe::Terreno && !cam.terreno) || (matches!(pc.classe, Classe::Tirante | Classe::Grampo | Classe::Placa | Classe::Cabeca) && !cam.reforcos)
            || (matches!(pc.classe, Classe::Tubo | Classe::Filtro | Classe::Canaleta) && !cam.drenagem) {
            continue;
        }
        let (tex, grao) = (pc.material.textura() as f32, pc.material.grao());
        let mut tris: Vec<Tri> = Vec::new();
        for t in pc.forma.triangulos(10) {
            subdividir(t, passo, 12, &mut tris);
        }
        for t in tris {
            let c = [cam_pt(t[0]), cam_pt(t[1]), cam_pt(t[2])];
            if c.iter().any(|q| dist - q[2] <= 0.05 * raio) {
                continue;
            }
            let u = [c[1][0] - c[0][0], c[1][1] - c[0][1], c[1][2] - c[0][2]];
            let w = [c[2][0] - c[0][0], c[2][1] - c[0][1], c[2][2] - c[0][2]];
            let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if nl < 1e-12 {
                continue;
            }
            let difusa = ((n[0] * luz[0] + n[1] * luz[1] + n[2] * luz[2]) / nl).abs();
            prims.push(Prim {
                pts: [proj(c[0]), proj(c[1]), proj(c[2])],
                prof: (c[0][2] + c[1][2] + c[2][2]) / 3.0,
                cor: {
                    let cm = [(t[0][0] + t[1][0] + t[2][0]) / 3.0, (t[0][1] + t[1][1] + t[2][1]) / 3.0, (t[0][2] + t[1][2] + t[2][2]) / 3.0];
                    cor_material(pc.material, (0.40 + 0.60 * difusa) * (1.0 + tex * ruido(cm, grao)), escuro)
                },
            });
        }
    }
    // algoritmo do pintor: do mais distante para o mais próximo
    prims.sort_by(|p, q| p.prof.total_cmp(&q.prof));
    let mut malha = egui::Mesh::default();
    for p in &prims {
        let i = malha.vertices.len() as u32;
        for q in p.pts {
            malha.colored_vertex(q, p.cor);
        }
        malha.add_triangle(i, i + 1, i + 2);
    }
    painter.add(Shape::mesh(malha));
}

/// Eixos e escala no canto da vista.
fn legenda(ui: &mut egui::Ui, m: &Modelo3d) {
    let p = tema::de(ui);
    let mut mats: Vec<Material> = Vec::new();
    for pc in &m.pecas {
        if !mats.contains(&pc.material) {
            mats.push(pc.material);
        }
    }
    ui.horizontal_wrapped(|ui| {
        for mt in mats {
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
            ui.painter().rect_filled(r, 2.0, cor_material(mt, 1.0, false));
            ui.label(RichText::new(mt.nome()).small().color(p.fraco));
            ui.add_space(6.0);
        }
    });
}

/// Mostra a janela 3D. Retorna uma mensagem para a barra de status.
pub fn janela(ctx: &egui::Context, aberta: &mut bool, cam: &mut Camera, proj: &Projeto, res: &Resultado) -> Option<String> {
    let mut msg = None;
    let tela = ctx.screen_rect();
    let mut aberta_local = *aberta;
    egui::Window::new("🧊 Modelo 3D da contenção")
        .open(&mut aberta_local)
        .default_size([(tela.width() * 0.7).min(1000.0), (tela.height() * 0.75).min(700.0)])
        .min_size([360.0, 300.0])
        .max_size(tela.size() - egui::vec2(16.0, 16.0))
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            let m = Modelo3d::montar(proj, res, cam.comprimento, cam.corte);
            ui.horizontal_wrapped(|ui| {
                ui.label("Trecho");
                ui.add(egui::DragValue::new(&mut cam.comprimento).range(2.0..=60.0).speed(0.2).suffix(" m"));
                ui.checkbox(&mut cam.terreno, "Solo");
                ui.add_enabled(cam.terreno, egui::Checkbox::new(&mut cam.corte, "Corte no maciço"));
                ui.checkbox(&mut cam.reforcos, "Tirantes/grampos");
                ui.checkbox(&mut cam.drenagem, "Drenagem");
                ui.separator();
                if ui.button("Frontal").clicked() {
                    (cam.azimute, cam.elevacao) = (-std::f32::consts::FRAC_PI_2, 0.05);
                }
                if ui.button("Lateral").clicked() {
                    (cam.azimute, cam.elevacao) = (0.0, 0.05);
                }
                if ui.button("Isométrica").clicked() {
                    *cam = Camera { comprimento: cam.comprimento, terreno: cam.terreno, reforcos: cam.reforcos, drenagem: cam.drenagem, corte: cam.corte, ..Camera::default() };
                }
                ui.separator();
                if ui.button("💾 Exportar IFC…").on_hover_text("IFC4 — abre em Revit, BIMvision, Solibri, BlenderBIM etc.").clicked() {
                    let nome = if proj.nome.trim().is_empty() { "contencao".to_string() } else { proj.nome.trim().to_string() };
                    if let Some(c) = rfd::FileDialog::new().set_title("Exportar IFC").set_file_name(format!("{nome}.ifc")).add_filter("IFC", &["ifc"]).save_file() {
                        msg = Some(match std::fs::write(&c, crate::ifc::gerar(&Modelo3d::montar(proj, res, cam.comprimento, false), proj)) {
                            Ok(_) => format!("IFC exportado: {}", c.display()),
                            Err(e) => format!("Erro ao exportar IFC: {e}"),
                        });
                    }
                }
            });
            legenda(ui, &m);
            let alt = (ui.available_height() - 22.0).max(160.0);
            let (resp, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), alt), Sense::click_and_drag());
            let p = tema::de(ui);
            let escuro = ui.visuals().dark_mode;
            painter.rect_filled(resp.rect, 8.0, if escuro { Color32::from_rgb(28, 32, 38) } else { Color32::from_rgb(236, 240, 245) });
            if resp.dragged() {
                let d = resp.drag_delta();
                cam.azimute -= d.x * 0.008;
                cam.elevacao = (cam.elevacao + d.y * 0.008).clamp(-1.45, 1.45);
            }
            if resp.hovered() {
                let rolagem = ui.input(|i| i.smooth_scroll_delta.y);
                if rolagem != 0.0 {
                    cam.zoom = (cam.zoom * (rolagem * 0.002).exp()).clamp(0.2, 8.0);
                }
            }
            if resp.double_clicked() {
                cam.zoom = 1.0;
            }
            renderizar(&painter.with_clip_rect(resp.rect), resp.rect, &m, cam, escuro);
            painter.rect_stroke(resp.rect, 8.0, Stroke::new(1.0_f32, p.borda), egui::StrokeKind::Inside);
            ui.label(
                RichText::new(format!(
                    "Arraste para girar · role para aproximar · duplo clique restaura o zoom · H livre = {} m · trecho de {} m",
                    fmt(proj.geometria.h),
                    fmt(m.comprimento)
                ))
                .small()
                .color(p.fraco),
            );
            for n in &m.notas {
                ui.label(RichText::new(format!("⚠ {n}")).small().color(p.alerta));
            }
        });
    *aberta = aberta_local;
    msg
}
