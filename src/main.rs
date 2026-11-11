// Oculta a janela de console no executável de release (Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alvenaria;
mod app;
mod armadura;
mod barras;
mod atualizacao;
mod concreto;
mod desenho;
mod detalhes;
mod docx;
mod drenagem;
mod dxf;
mod empuxo;
mod estabilidade;
mod fatores;
mod global;
mod ifc;
mod manual;
mod modelo3d;
mod marca;
mod materiais;
mod memorial_html;
mod normas;
mod otimizar;
mod pdf;
mod perfis_prancha;
mod projeto;
mod raster;
mod referencias;
mod reaterro;
mod resultado;
mod sobre;
mod solo;
mod spt;
mod tipos;
mod ui;

use eframe::egui;

/// Ícone da janela (64×64) gerado a partir do logotipo vetorial.
fn icone() -> egui::IconData {
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    let dentro = |p: &[(f32, f32)], x: f32, y: f32| {
        let mut s: i32 = 0;
        for i in 0..p.len() {
            let (x0, y0) = p[i];
            let (x1, y1) = p[(i + 1) % p.len()];
            let c = (x1 - x0) * (y - y0) - (y1 - y0) * (x - x0);
            s += if c >= 0.0 { 1 } else { -1 };
        }
        s.unsigned_abs() as usize == p.len()
    };
    let muro = [(0.16, 0.78), (0.42, 0.24), (0.56, 0.24), (0.60, 0.78)];
    let solo = [(0.56, 0.30), (0.86, 0.30), (0.86, 0.78), (0.60, 0.78)];
    for j in 0..n {
        for i in 0..n {
            let (x, y) = ((i as f32 + 0.5) / n as f32, (j as f32 + 0.5) / n as f32);
            // quadrado arredondado
            let r = 0.22;
            let dx = (x - x.clamp(r, 1.0 - r)).abs();
            let dy = (y - y.clamp(r, 1.0 - r)).abs();
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let mut c = [19u8, 37, 62, 255]; // azul-marinho da paleta
            if dentro(&solo, x, y) {
                c = [102, 113, 130, 255];
            }
            if dentro(&muro, x, y) || ((0.76..0.81).contains(&y) && (0.10..0.90).contains(&x)) {
                c = [255, 255, 255, 255];
            }
            rgba[(j * n + i) * 4..(j * n + i) * 4 + 4].copy_from_slice(&c);
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}

fn main() -> eframe::Result {
    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 860.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title(sobre::NOME)
            .with_icon(std::sync::Arc::new(icone())),
        ..Default::default()
    };
    eframe::run_native(sobre::NOME, opcoes, Box::new(|cc| Ok(Box::new(app::App::novo(cc)))))
}
