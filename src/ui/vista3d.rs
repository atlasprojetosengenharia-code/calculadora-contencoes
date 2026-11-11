//! Janela de visualização 3D da contenção: renderização por software
//! (triângulos ordenados por profundidade, sombreamento difuso) com órbita,
//! zoom, vistas prontas e exportação IFC.

use super::tema;
use crate::modelo3d::{Classe, Material, Modelo3d, Tri};
use crate::projeto::Projeto;
use crate::resultado::{fmt, Resultado};
use eframe::egui::{self, Color32, RichText, Sense, Stroke};

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
        Self { azimute: 0.85, elevacao: 0.40, zoom: 1.0, comprimento: 10.0, terreno: true, reforcos: true, drenagem: true, corte: false }
    }
}

/// Triângulo pronto para desenhar: vértices no mundo, cor já iluminada e peça.
struct TriCena {
    v: Tri,
    cor: [u8; 3],
    peca: u32,
    classe: Classe,
    /// Textura (tipo, ladrilho em m, eixo de projeção 0=x 1=y 2=z) e luz × tinta.
    tex: Option<(super::texturas::Tex, f32, u8, [f32; 3])>,
}

/// Cena montada uma vez por projeto/opções; a imagem é rasterizada com
/// z-buffer (sem sobreposições) só quando a câmera ou a janela mudam.
pub struct Cena {
    chave: u64,
    tris: Vec<TriCena>,
    centro: [f64; 3],
    raio: f32,
    pub modelo: Modelo3d,
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

/// Textura, ladrilho (m) e tinta de cada material; `None` = cor lisa (aço, PVC,
/// cordoalha e peças metálicas seguem a modelagem sem textura).
fn textura_de(m: Material, normal: [f64; 3]) -> Option<(super::texturas::Tex, f64, [f32; 3])> {
    use super::texturas::Tex;
    let branco = [1.0, 1.0, 1.0];
    Some(match m {
        Material::Concreto => (Tex::Concreto, 1.6, branco),
        Material::Bulbo => (Tex::Concreto, 0.8, [0.95, 0.95, 0.92]),
        Material::Pedra => (Tex::Pedra, 1.4, branco),
        Material::Gabiao => (Tex::Gabiao, 1.0, branco),
        // bolsa de brita envolta em manta: manta por fora, brita nas faces cortadas
        Material::Brita => {
            if normal[2].abs() > 0.9 { (Tex::Brita, 0.45, branco) } else { (Tex::Geotextil, 0.6, branco) }
        }
        Material::Areia => (Tex::Areia, 0.8, branco),
        Material::Geocomposto => (Tex::Geotextil, 0.6, [0.25, 0.25, 0.27]),
        Material::SoloArgila => (Tex::Argila, 2.2, branco),
        Material::SoloAreia => (Tex::Areia, 1.6, branco),
        Material::SoloSilte => (Tex::Silte, 2.0, branco),
        Material::SoloAterro => (Tex::SoloEscuro, 1.6, branco),
        Material::Reaterro => (Tex::SoloEscuro, 1.6, [1.12, 1.05, 1.0]),
        _ => return None,
    })
}

impl Cena {
    fn montar(chave: u64, modelo: Modelo3d) -> Self {
        let (a, b) = modelo.caixa();
        let centro = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
        let raio = (((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() / 2.0).max(0.5);
        // luz fixa no mundo (de cima, à esquerda e da frente) + ambiente
        let luz = {
            let l = [-0.45f64, 0.80, -0.40];
            let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            [l[0] / n, l[1] / n, l[2] / n]
        };
        let mut tris = Vec::new();
        for (ip, pc) in modelo.pecas.iter().enumerate() {
            for t in pc.forma.triangulos(14) {
                let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
                let w = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
                let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
                let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if nl < 1e-14 {
                    continue;
                }
                let n = [n[0] / nl, n[1] / nl, n[2] / nl];
                let k = 0.42 + 0.62 * (n[0] * luz[0] + n[1] * luz[1] + n[2] * luz[2]).abs() as f32;
                let c = cor_de(pc.material, n);
                let q = |x: f32| (x * k * 255.0).clamp(0.0, 255.0) as u8;
                let tex = textura_de(pc.material, n).map(|(tx, esc, tinta)| {
                    let (ax, ay, az) = (n[0].abs(), n[1].abs(), n[2].abs());
                    let eixo = if ax >= ay && ax >= az { 0 } else if ay >= az { 1 } else { 2 };
                    (tx, esc as f32, eixo, [tinta[0] * k, tinta[1] * k, tinta[2] * k])
                });
                tris.push(TriCena { v: t, cor: [q(c[0]), q(c[1]), q(c[2])], peca: ip as u32 + 1, classe: pc.classe, tex });
            }
        }
        Cena { chave, tris, centro, raio: raio as f32, modelo }
    }

    /// Rasteriza a cena (z-buffer, contornos nas arestas das peças) numa imagem `w × h`.
    fn rasterizar(&self, w: usize, h: usize, cam: &Camera, fundo: Color32, escala: f32) -> egui::ColorImage {
        let (sa, ca) = cam.azimute.sin_cos();
        let (se, ce) = cam.elevacao.sin_cos();
        let raio = self.raio;
        let dist = 3.2 * raio;
        let foco = 0.5 * (w.min(h) as f32) * 3.4 * cam.zoom;
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let c0 = self.centro;
        // passagem adiada: guarda o triângulo e as coordenadas baricêntricas (corrigidas
        // pela perspectiva) e só depois colore cada pixel visível uma vez
        let mut tri_px = vec![u32::MAX; w * h];
        let mut bar = vec![[0.0f32; 2]; w * h];
        let mut prof = vec![0.0f32; w * h]; // 1/d (maior = mais perto)
        let mut id = vec![0u32; w * h];
        for (it, t) in self.tris.iter().enumerate() {
            if (t.classe == Classe::Terreno && !cam.terreno)
                || (matches!(t.classe, Classe::Tirante | Classe::Grampo | Classe::Placa | Classe::Cabeca) && !cam.reforcos)
                || (matches!(t.classe, Classe::Tubo | Classe::Filtro | Classe::Canaleta) && !cam.drenagem)
            {
                continue;
            }
            // vértices na tela (x, y) e 1/d
            let mut p = [[0.0f32; 3]; 3];
            let mut atras = false;
            for k in 0..3 {
                let v = t.v[k];
                let (x, y, z) = ((v[0] - c0[0]) as f32, (v[1] - c0[1]) as f32, (v[2] - c0[2]) as f32);
                let x1 = ca * x + sa * z;
                let z1 = -sa * x + ca * z;
                let (y2, z2) = (ce * y - se * z1, se * y + ce * z1);
                let d = dist - z2;
                if d <= 0.05 * raio {
                    atras = true;
                    break;
                }
                p[k] = [cx + foco * x1 / d, cy - foco * y2 / d, 1.0 / d];
            }
            if atras {
                continue;
            }
            // o solo recua um pouco: peças encostadas nele não "brigam" pelo pixel
            let vies = if t.classe == Classe::Terreno { 0.9995 } else { 1.0 };
            let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
            if area.abs() < 1e-6 {
                continue;
            }
            let x0 = p.iter().map(|q| q[0]).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
            let x1 = (p.iter().map(|q| q[0]).fold(f32::NEG_INFINITY, f32::max).ceil() as isize).min(w as isize - 1);
            let y0 = p.iter().map(|q| q[1]).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
            let y1 = (p.iter().map(|q| q[1]).fold(f32::NEG_INFINITY, f32::max).ceil() as isize).min(h as isize - 1);
            if x1 < 0 || y1 < 0 {
                continue;
            }
            let inv = 1.0 / area;
            for yy in y0..=y1 as usize {
                let py = yy as f32 + 0.5;
                for xx in x0..=x1 as usize {
                    let px = xx as f32 + 0.5;
                    let w0 = ((p[1][0] - px) * (p[2][1] - py) - (p[1][1] - py) * (p[2][0] - px)) * inv;
                    let w1 = ((p[2][0] - px) * (p[0][1] - py) - (p[2][1] - py) * (p[0][0] - px)) * inv;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -1e-5 || w1 < -1e-5 || w2 < -1e-5 {
                        continue;
                    }
                    let z = (w0 * p[0][2] + w1 * p[1][2] + w2 * p[2][2]) * vies;
                    let i = yy * w + xx;
                    if z > prof[i] {
                        prof[i] = z;
                        tri_px[i] = it as u32;
                        bar[i] = [w0 * p[0][2] / z * vies, w1 * p[1][2] / z * vies];
                        id[i] = t.peca;
                    }
                }
            }
        }
        // contornos: troca de peça ou salto de profundidade entre vizinhos
        let linha = (escala.round() as usize).max(1);
        let cor: Vec<Color32> = (0..w * h)
            .map(|i| {
                let Some(t) = self.tris.get(tri_px[i] as usize) else { return fundo };
                match t.tex {
                    None => Color32::from_rgb(t.cor[0], t.cor[1], t.cor[2]),
                    Some((tx, esc, eixo, tinta)) => {
                        let [b0, b1] = bar[i];
                        let b2 = 1.0 - b0 - b1;
                        let pw = |k: usize| (b0 as f64 * t.v[0][k] + b1 as f64 * t.v[1][k] + b2 as f64 * t.v[2][k]) as f32;
                        let (s, r) = match eixo { 0 => (pw(2), pw(1)), 1 => (pw(0), pw(2)), _ => (pw(0), pw(1)) };
                        let c = super::texturas::amostrar(super::texturas::pixels(tx), s / esc, -r / esc);
                        let q = |x: u8, k: f32| (x as f32 * k).clamp(0.0, 255.0) as u8;
                        Color32::from_rgb(q(c[0], tinta[0]), q(c[1], tinta[1]), q(c[2], tinta[2]))
                    }
                }
            })
            .collect();
        let mut saida = cor.clone();
        for yy in 0..h {
            for xx in 0..w {
                let i = yy * w + xx;
                if id[i] == 0 {
                    continue;
                }
                let mut borda = false;
                for (dx, dy) in [(linha as isize, 0isize), (0, linha as isize), (-(linha as isize), 0), (0, -(linha as isize))] {
                    let (nx, ny) = (xx as isize + dx, yy as isize + dy);
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    // só desenha o contorno no pixel mais próximo da câmera
                    if id[j] != id[i] && prof[i] >= prof[j] {
                        borda = true;
                        break;
                    }
                    if id[j] == id[i] && (prof[i] - prof[j]).abs() > 0.02 * prof[i] {
                        borda = true;
                        break;
                    }
                }
                if borda {
                    let c = cor[i];
                    saida[i] = Color32::from_rgb((c.r() as f32 * 0.45) as u8, (c.g() as f32 * 0.45) as u8, (c.b() as f32 * 0.45) as u8);
                }
            }
        }
        egui::ColorImage { size: [w, h], pixels: saida }
    }

    /// Desenha a cena em `rect`: re-rasteriza só se a câmera, as opções ou o tamanho mudaram.
    fn desenhar(&self, ui: &egui::Ui, painter: &egui::Painter, rect: egui::Rect, cam: &Camera, fundo: Color32) {
        use std::hash::{Hash, Hasher};
        let ppp = ui.ctx().pixels_per_point();
        // resolução limitada (~1,6 Mpx) para manter o programa leve
        let mut esc = ppp;
        let px = (rect.width() * esc) * (rect.height() * esc);
        if px > 1.6e6 {
            esc *= (1.6e6 / px).sqrt();
        }
        let (w, h) = (((rect.width() * esc) as usize).max(16), ((rect.height() * esc) as usize).max(16));
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        self.chave.hash(&mut hs);
        (w, h, cam.azimute.to_bits(), cam.elevacao.to_bits(), cam.zoom.to_bits(), cam.terreno, cam.reforcos, cam.drenagem).hash(&mut hs);
        fundo.to_array().hash(&mut hs);
        let chave = hs.finish();
        let id = egui::Id::new("imagem_3d");
        let atual: Option<(u64, egui::TextureHandle)> = ui.ctx().data(|d| d.get_temp(id));
        let tex = match atual {
            Some((k, t)) if k == chave => t,
            outro => {
                let img = self.rasterizar(w, h, cam, fundo, esc);
                let t = match outro {
                    Some((_, mut t)) => {
                        t.set(img, egui::TextureOptions::LINEAR);
                        t
                    }
                    None => ui.ctx().load_texture("imagem_3d", img, egui::TextureOptions::LINEAR),
                };
                ui.ctx().data_mut(|d| d.insert_temp(id, (chave, t.clone())));
                t
            }
        };
        painter.image(tex.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
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
                    (cam.azimute, cam.elevacao) = (std::f32::consts::FRAC_PI_2, 0.05);
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
            let fundo = if escuro { Color32::from_rgb(28, 32, 38) } else { Color32::from_rgb(236, 240, 245) };
            painter.rect_filled(resp.rect, 8.0, fundo);
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
            cn.desenhar(ui, &painter.with_clip_rect(resp.rect), resp.rect, cam, fundo);
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

#[cfg(test)]
mod amostra {
    use super::*;
    use crate::projeto::TIPOS_CONTENCAO;

    /// Grava PNGs do 3D de cada tipo (conferência visual): PASTA_3D=pasta.
    #[test]
    #[ignore]
    fn imagens() {
        let Ok(pasta) = std::env::var("PASTA_3D") else { return };
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let r = p.calcular();
            for (nome, corte) in [("cheio", false), ("corte", true)] {
                let cena = Cena::montar(1, Modelo3d::montar(&p, &r, 10.0, corte));
                let cam = if corte { Camera { corte, ..Camera::default() } } else { Camera { azimute: 1.25, elevacao: 0.25, ..Camera::default() } };
                let img = cena.rasterizar(900, 600, &cam, Color32::from_rgb(236, 240, 245), 1.0);
                let px: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
                let im = crate::raster::Imagem { w: 900, h: 600, px };
                std::fs::write(format!("{pasta}/{}_{nome}.png", tipo.arquivo()), im.png()).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod amostra_calha {
    use super::*;
    #[test]
    #[ignore]
    fn calha() {
        let Ok(pasta) = std::env::var("PASTA_3D") else { return };
        let mut p = Projeto::default();
        p.tipo = crate::projeto::TipoContencao::Flexao;
        p.geometria.h = 4.0;
        p.pre_dimensionar();
        let r = p.calcular();
        let gd = p.geo_drenagem();
        println!("canaletas {:?}", gd.canaletas);
        let m = p.secao_global(&r);
        println!("sup {:?}", m.sup);
        let cena = Cena::montar(1, Modelo3d::montar(&p, &r, 10.0, false));
        let img = cena.rasterizar(900, 600, &Camera { azimute: 0.15, elevacao: 0.2, zoom: 1.6, ..Camera::default() }, Color32::from_rgb(236, 240, 245), 1.0);
        let px: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
        std::fs::write(format!("{pasta}/calha.png"), crate::raster::Imagem { w: 900, h: 600, px }.png()).unwrap();
    }
}

#[cfg(test)]
mod amostra_gravidade {
    use super::*;
    use crate::tipos::gravidade::Material as M;
    #[test]
    #[ignore]
    fn gravidade() {
        let Ok(pasta) = std::env::var("PASTA_3D") else { return };
        for (nome, mat, dente) in [("pedra", M::PedraArgamassada, false), ("ciclopico_dente", M::ConcretoCiclopico, true), ("gabiao", M::Gabiao, false)] {
            let mut p = Projeto::default();
            p.tipo = crate::projeto::TipoContencao::Gravidade;
            p.geometria.h = 3.0;
            p.gravidade.material = mat;
            p.pre_dimensionar();
            p.gravidade.dente = dente;
            let r = p.calcular();
            for (v, az, el, corte) in [("lado", 0.15f32, 0.15f32, true), ("frente", 0.85, 0.35, false), ("secao", 0.0, 0.0, false)] {
                let cena = Cena::montar(1, Modelo3d::montar(&p, &r, 8.0, corte));
                let img = cena.rasterizar(900, 600, &Camera { azimute: az, elevacao: el, zoom: 1.3, corte, ..Camera::default() }, Color32::from_rgb(236, 240, 245), 1.0);
                let px: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
                std::fs::write(format!("{pasta}/grav_{nome}_{v}.png"), crate::raster::Imagem { w: 900, h: 600, px }.png()).unwrap();
            }
        }
    }
}
