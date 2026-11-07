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
}

/// Textura de uma peça da marca na cor do tema (guardada no contexto).
pub fn textura(ctx: &egui::Context, peca: Peca, escuro: bool) -> egui::TextureHandle {
    let id = egui::Id::new(("marca", peca, escuro));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return t;
    }
    let m = match peca {
        Peca::Logotipo => logotipo(),
    };
    let cor = if escuro { CLARO } else { MARINHO };
    let t = ctx.load_texture(format!("marca_{peca:?}_{escuro}"), m.colorir(cor), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
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
    }
}
