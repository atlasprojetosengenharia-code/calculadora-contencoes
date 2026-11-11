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
        Self { azimute: -0.75, elevacao: 0.45, zoom: 1.0, comprimento: 10.0, terreno: true, reforcos: true, drenagem: true, corte: false }
    }
}

/// Triângulo pronto para desenhar: vértices no mundo, cor base e normal.
struct TriCena {
    v: Tri,
    cor: [f32; 3],
    /// Intensidade difusa da luz (fixa no mundo), já calculada.
    luz: f32,
    classe: Classe,
}

/// Cena montada uma vez por projeto/opções (o desenho de cada quadro só projeta e ordena).
pub struct Cena {
    chave: u64,
    tris: Vec<TriCena>,
    centro: [f64; 3],
    raio: f32,
    pub modelo: Modelo3d,
}

/// Divide triângulos maiores que `s` (ordem do pintor estável). Feito só na montagem.
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

/// Cor de cada material (tom médio do material real); a bolsa de brita mostra a
/// manta geotêxtil por fora e a brita nas faces cortadas.
fn cor_de(m: Material, normal: [f64; 3]) -> [f32; 3] {
    if m == Material::Brita && normal[2].abs() <= 0.9 {
        return [0.86, 0.86, 0.87]; // manta geotêxtil
    }
    let ([r, g, b], _) = m.cor();
    [r as f32, g as f32, b as f32]
}

impl Cena {
    fn montar(chave: u64, modelo: Modelo3d) -> Self {
        let (a, b) = modelo.caixa();
        let centro = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
        let raio = (((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() / 2.0).max(0.5);
        // luz fixa no mundo (de cima, à esquerda e da frente)
        let luz = {
            let l = [-0.45f64, 0.80, -0.40];
            let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            [l[0] / n, l[1] / n, l[2] / n]
        };
        // só os blocos grandes (solo, muro) são subdivididos, e com passo largo
        let passo = (raio / 7.0).clamp(0.4, 2.5);
        let mut tris = Vec::new();
        for pc in &modelo.pecas {
            let grandes = matches!(pc.classe, Classe::Terreno | Classe::Muro | Classe::Sapata | Classe::Viga);
            let mut ts = Vec::new();
            for t in pc.forma.triangulos(8) {
                if grandes { subdividir(t, passo, 8, &mut ts) } else { ts.push(t) }
            }
            for t in ts {
                let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
                let w = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
                let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
                let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if nl < 1e-14 {
                    continue;
                }
                let n = [n[0] / nl, n[1] / nl, n[2] / nl];
                let difusa = (n[0] * luz[0] + n[1] * luz[1] + n[2] * luz[2]).abs() as f32;
                tris.push(TriCena { v: t, cor: cor_de(pc.material, n), luz: 0.45 + 0.60 * difusa, classe: pc.classe });
            }
        }
        Cena { chave, tris, centro, raio: raio as f32, modelo }
    }

    /// Projeta, ordena (pintor) e desenha numa única malha.
    fn desenhar(&self, painter: &egui::Painter, rect: egui::Rect, cam: &Camera) {
        let (sa, ca) = cam.azimute.sin_cos();
        let (se, ce) = cam.elevacao.sin_cos();
        let raio = self.raio;
        let dist = 3.2 * raio;
        let foco = 0.5 * rect.width().min(rect.height()) * 2.6 * cam.zoom;
        let c0 = self.centro;
        let cam_pt = |v: [f64; 3]| -> [f32; 3] {
            let (x, y, z) = ((v[0] - c0[0]) as f32, (v[1] - c0[1]) as f32, (v[2] - c0[2]) as f32);
            let x1 = ca * x + sa * z;
            let z1 = -sa * x + ca * z;
            [x1, ce * y - se * z1, se * y + ce * z1]
        };
        let centro_tela = rect.center();
        let mut prims: Vec<(f32, [Pos2; 3], Color32)> = Vec::with_capacity(self.tris.len());
        for t in &self.tris {
            if (t.classe == Classe::Terreno && !cam.terreno)
                || (matches!(t.classe, Classe::Tirante | Classe::Grampo | Classe::Placa | Classe::Cabeca) && !cam.reforcos)
                || (matches!(t.classe, Classe::Tubo | Classe::Filtro | Classe::Canaleta) && !cam.drenagem)
            {
                continue;
            }
            let c = [cam_pt(t.v[0]), cam_pt(t.v[1]), cam_pt(t.v[2])];
            if c.iter().any(|q| dist - q[2] <= 0.05 * raio) {
                continue;
            }
            let p = |q: [f32; 3]| {
                let d = dist - q[2];
                centro_tela + egui::vec2(foco * q[0] / d, -foco * q[1] / d)
            };
            let k = t.luz;
            let q = |x: f32| (x * k * 255.0).clamp(0.0, 255.0) as u8;
            prims.push(((c[0][2] + c[1][2] + c[2][2]) / 3.0, [p(c[0]), p(c[1]), p(c[2])], Color32::from_rgb(q(t.cor[0]), q(t.cor[1]), q(t.cor[2]))));
        }
        prims.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut malha = egui::Mesh::default();
        malha.reserve_triangles(prims.len());
        malha.reserve_vertices(prims.len() * 3);
        for (_, pts, cor) in prims {
            let i = malha.vertices.len() as u32;
            for q in pts {
                malha.colored_vertex(q, cor);
            }
            malha.add_triangle(i, i + 1, i + 2);
        }
        painter.add(Shape::mesh(malha));
    }
}

/// Cena em cache: remonta só quando o projeto, o trecho ou o corte mudam.
fn cena(ctx: &egui::Context, proj: &Projeto, res: &Resultado, cam: &Camera) -> std::sync::Arc<Cena> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{proj:?}").hash(&mut h);
    cam.comprimento.to_bits().hash(&mut h);
    cam.corte.hash(&mut h);
    let chave = h.finish();
    let id = egui::Id::new("cena_3d");
    if let Some(c) = ctx.data(|d| d.get_temp::<std::sync::Arc<Cena>>(id)) {
        if c.chave == chave {
            return c;
        }
    }
    let c = std::sync::Arc::new(Cena::montar(chave, Modelo3d::montar(proj, res, cam.comprimento, cam.corte)));
    ctx.data_mut(|d| d.insert_temp(id, c.clone()));
    c
}

fn cor_material(m: Material, luz: f32) -> Color32 {
    let ([r, g, b], _) = m.cor();
    let q = |x: f64| (x as f32 * luz * 255.0).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(q(r), q(g), q(b))
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
            ui.painter().rect_filled(r, 2.0, cor_material(mt, 1.0));
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
            let cn = cena(ctx, proj, res, cam);
            let m = &cn.modelo;
            ui.horizontal_wrapped(|ui| {
                ui.label("Trecho");
                ui.add(egui::DragValue::new(&mut cam.comprimento).range(2.0..=60.0).speed(0.2).suffix(" m"));
                ui.checkbox(&mut cam.terreno, "Solo");
                ui.add_enabled(cam.terreno, egui::Checkbox::new(&mut cam.corte, "Corte no maciço")).on_hover_text("Mostra o solo só em metade do trecho, para ver tirantes, grampos e drenagem por dentro");
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
            legenda(ui, m);
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
            cn.desenhar(&painter.with_clip_rect(resp.rect), resp.rect, cam);
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
