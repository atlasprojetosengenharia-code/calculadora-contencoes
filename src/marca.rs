//! Identidade visual da Atlas Projetos e Engenharia: cores oficiais e
//! logotipos embutidos como máscaras de transparência (a cor é aplicada na
//! hora: azul-marinho no tema claro, claro no tema escuro).
//!
//! Arquivos em `assets/marca/*.alpha`: "ALFA" + largura (u16 LE) + altura
//! (u16 LE) + 1 byte de opacidade por pixel. Gerados a partir de
//! "Atlas - Identidade Visual / Imagens / Sem Fundo".

use eframe::egui;

/// Azul-marinho institucional (#13253E).
pub const MARINHO: [u8; 3] = [19, 37, 62];
/// Cinza-claro institucional (#E8E8E8).
pub const CLARO: [u8; 3] = [232, 232, 232];

/// Máscara de opacidade de um logotipo.
pub struct Mascara {
    pub largura: usize,
    pub altura: usize,
    pub alfa: &'static [u8],
}

fn ler(b: &'static [u8]) -> Mascara {
    assert!(b.len() > 8 && &b[..4] == b"ALFA", "máscara inválida");
    let w = u16::from_le_bytes([b[4], b[5]]) as usize;
    let h = u16::from_le_bytes([b[6], b[7]]) as usize;
    assert_eq!(b.len(), 8 + w * h, "tamanho da máscara");
    Mascara { largura: w, altura: h, alfa: &b[8..] }
}

/// Logotipo principal (símbolo + ATLAS + PROJETOS E ENGENHARIA).
pub fn logotipo() -> Mascara {
    ler(include_bytes!("../assets/marca/logotipo.alpha"))
}

/// Símbolo (prédio em traço).
pub fn simbolo() -> Mascara {
    ler(include_bytes!("../assets/marca/simbolo.alpha"))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Peca {
    Logotipo,
    Simbolo,
}

impl Mascara {
    /// Imagem colorida (RGBA não pré-multiplicado).
    pub fn colorir(&self, cor: [u8; 3]) -> egui::ColorImage {
        let mut rgba = Vec::with_capacity(self.alfa.len() * 4);
        for &a in self.alfa {
            rgba.extend_from_slice(&[cor[0], cor[1], cor[2], a]);
        }
        egui::ColorImage::from_rgba_unmultiplied([self.largura, self.altura], &rgba)
    }

    /// Opacidade média num bloco (reamostragem por caixa), coordenadas 0–1.
    fn amostra(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let (w, h) = (self.largura as f64, self.altura as f64);
        let (i0, i1) = ((x0 * w) as usize, ((x1 * w).ceil() as usize).min(self.largura));
        let (j0, j1) = ((y0 * h) as usize, ((y1 * h).ceil() as usize).min(self.altura));
        let (mut s, mut n) = (0u64, 0u64);
        for j in j0..j1.max(j0 + 1).min(self.altura) {
            for i in i0..i1.max(i0 + 1).min(self.largura) {
                s += self.alfa[j * self.largura + i] as u64;
                n += 1;
            }
        }
        if n == 0 { 0.0 } else { s as f64 / n as f64 / 255.0 }
    }
}

/// Textura de uma peça da marca na cor do tema (guardada no contexto).
pub fn textura(ctx: &egui::Context, peca: Peca, escuro: bool) -> egui::TextureHandle {
    let id = egui::Id::new(("marca", peca, escuro));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let m = match peca {
        Peca::Logotipo => logotipo(),
        Peca::Simbolo => simbolo(),
    };
    let cor = if escuro { CLARO } else { MARINHO };
    let t = ctx.load_texture(format!("marca_{peca:?}_{escuro}"), m.colorir(cor), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// Desenha uma peça da marca com a altura dada (mantém a proporção).
pub fn mostrar(ui: &mut egui::Ui, peca: Peca, altura: f32) -> egui::Response {
    let t = textura(ui.ctx(), peca, ui.visuals().dark_mode);
    let [w, h] = t.size();
    let tam = egui::vec2(altura * w as f32 / h as f32, altura);
    ui.add(egui::Image::new((t.id(), tam)))
}

/// Ícone da janela: símbolo claro sobre quadrado azul-marinho arredondado.
pub fn icone(n: usize) -> egui::IconData {
    let s = simbolo();
    let mut rgba = vec![0u8; n * n * 4];
    let r = 0.20;
    // símbolo ocupa ~70 % do quadrado, centralizado
    let lado = 0.70;
    let asp = s.largura as f64 / s.altura as f64;
    let (sw, sh) = if asp >= 1.0 { (lado, lado / asp) } else { (lado * asp, lado) };
    let (sx, sy) = ((1.0 - sw) / 2.0, (1.0 - sh) / 2.0);
    for j in 0..n {
        for i in 0..n {
            let (x, y) = ((i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64);
            let dx = (x - x.clamp(r, 1.0 - r)).abs();
            let dy = (y - y.clamp(r, 1.0 - r)).abs();
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let passo = 1.0 / n as f64;
            let u0 = (x - passo / 2.0 - sx) / sw;
            let v0 = (y - passo / 2.0 - sy) / sh;
            let a = if (0.0..1.0).contains(&u0) && (0.0..1.0).contains(&v0) {
                s.amostra(u0, v0, (u0 + passo / sw).min(1.0), (v0 + passo / sh).min(1.0))
            } else {
                0.0
            };
            // reforça o traço fino em tamanhos pequenos
            let a = (a * 2.2).min(1.0);
            let c: Vec<u8> = (0..3).map(|k| (MARINHO[k] as f64 * (1.0 - a) + 245.0 * a).round() as u8).collect();
            rgba[(j * n + i) * 4..(j * n + i) * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mascaras_validas() {
        let l = super::logotipo();
        let s = super::simbolo();
        assert!(l.largura > 100 && l.altura > 100 && s.largura > 100);
        // há traço (opaco) e fundo (transparente)
        assert!(l.alfa.iter().any(|&a| a > 200) && l.alfa.iter().any(|&a| a == 0));
        let ic = super::icone(64);
        assert_eq!(ic.rgba.len(), 64 * 64 * 4);
        // o símbolo aparece em claro sobre o marinho no centro do ícone
        assert!(ic.rgba.chunks(4).any(|p| p[0] > 200));
    }
}
