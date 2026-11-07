// Oculta a janela de console no executável de release (Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod alvenaria;
mod app;
mod armadura;
mod atualizacao;
mod concreto;
mod desenho;
mod drenagem;
mod dxf;
mod empuxo;
mod estabilidade;
mod fatores;
mod manual;
mod marca;
mod materiais;
mod memorial_html;
mod normas;
mod pdf;
mod projeto;
mod resultado;
mod sobre;
mod solo;
mod spt;
mod tipos;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 860.0])
            .with_min_inner_size([1100.0, 680.0])
            .with_title(sobre::NOME)
            .with_icon(std::sync::Arc::new(marca::icone(64))),
        ..Default::default()
    };
    eframe::run_native(sobre::NOME, opcoes, Box::new(|cc| Ok(Box::new(app::App::novo(cc)))))
}
