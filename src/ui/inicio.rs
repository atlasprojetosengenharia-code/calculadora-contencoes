//! Tela de abertura: apresentação, versão, autoria e direitos autorais.

use super::tema;
use super::widgets::{botao_primario, botao_secundario, cartao, pintar_logo};
use crate::sobre;
use eframe::egui::{self, RichText, Stroke};

pub enum Acao {
    Nenhuma,
    Novo,
    Abrir,
    Continuar,
    Manual,
    SalvarManual,
    Normas,
}

pub fn mostrar(ctx: &egui::Context, sessao_anterior: Option<&str>, atualizacao: Option<&crate::atualizacao::Estado>) -> Acao {
    let mut acao = Acao::Nenhuma;

    egui::TopBottomPanel::bottom("rodape_inicio")
        .frame(egui::Frame::new().fill(tema::paleta(ctx.style().visuals.dark_mode).painel).inner_margin(egui::Margin::symmetric(24, 12)))
        .show(ctx, |ui| {
            let p = tema::de(ui);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Criado por").color(p.fraco));
                ui.label(RichText::new(sobre::CRIADOR).strong());
                ui.label(RichText::new("·").color(p.fraco));
                ui.label(RichText::new(sobre::EMPRESA).strong());
                ui.label(RichText::new("·").color(p.fraco));
                ui.hyperlink_to(sobre::site_curto(), sobre::SITE);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_switch(ui);
                    ui.label(RichText::new(sobre::copyright()).color(p.fraco));
                });
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        let p = tema::de(ui);
        // fundo decorativo: linhas de nível suaves
        let r = ui.max_rect();
        let pt = ui.painter();
        for i in 0..14 {
            let y = r.top() + r.height() * (0.15 + i as f32 * 0.065);
            let cor = p.destaque.gamma_multiply(0.05);
            let pts: Vec<egui::Pos2> = (0..=40)
                .map(|k| {
                    let x = r.left() + r.width() * k as f32 / 40.0;
                    egui::pos2(x, y + 18.0 * ((k as f32 * 0.35 + i as f32).sin()))
                })
                .collect();
            pt.add(egui::Shape::line(pts, Stroke::new(1.5_f32, cor)));
        }
        // marca d'água: logotipo da Atlas Projetos e Engenharia, bem suave, no fundo
        let t = crate::marca::textura(ui.ctx(), crate::marca::Peca::Logotipo, ui.visuals().dark_mode);
        let [tw, th] = t.size();
        let alt = r.height() * 0.62;
        let tam = egui::vec2(alt * tw as f32 / th as f32, alt);
        let ret = egui::Rect::from_center_size(r.center(), tam);
        let opac = if ui.visuals().dark_mode { 0.045 } else { 0.06 };
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        pt.image(t.id(), ret, uv, egui::Color32::WHITE.gamma_multiply(opac));

        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.10).max(20.0));
            let (lr, _) = ui.allocate_exact_size(egui::vec2(116.0, 116.0), egui::Sense::hover());
            pintar_logo(ui.painter(), lr, &p);
            ui.add_space(18.0);
            ui.label(RichText::new(sobre::NOME).font(tema::fonte_forte(38.0)).color(p.texto));
            ui.label(RichText::new(sobre::SUBTITULO).size(18.0).color(p.fraco));
            ui.add_space(8.0);
            ui.label(RichText::new(format!("Versão {}", sobre::versao_completa())).color(p.destaque).font(tema::fonte_forte(13.0)));
            ui.add_space(4.0);
            ui.label(RichText::new(format!("{} · {}", sobre::CRIADOR, sobre::EMPRESA)).size(13.5).color(p.texto));
            if let Some(e) = atualizacao {
                ui.add_space(6.0);
                crate::atualizacao::aviso(ui, e);
            }
            ui.add_space(30.0);

            ui.allocate_ui_with_layout(egui::vec2(470.0, 50.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if botao_primario(ui, "Iniciar novo projeto").clicked() {
                    acao = Acao::Novo;
                }
                ui.add_space(14.0);
                if botao_secundario(ui, "Carregar projeto existente").clicked() {
                    acao = Acao::Abrir;
                }
            });
            ui.add_space(12.0);
            ui.allocate_ui_with_layout(egui::vec2(470.0, 30.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button(RichText::new("📘 Manual do usuário (PDF)").size(14.0)).on_hover_text("Tutorial passo a passo: SPT, geometria, estabilidade, detalhamento e exportações").clicked() {
                    acao = Acao::Manual;
                }
                if ui.small_button("Salvar PDF…").on_hover_text("Salvar uma cópia do manual").clicked() {
                    acao = Acao::SalvarManual;
                }
                ui.add_space(10.0);
                if ui.button(RichText::new("📚 Central de normas").size(14.0)).on_hover_text("Normas ABNT usadas nos cálculos").clicked() {
                    acao = Acao::Normas;
                }
            });
            if let Some(nome) = sessao_anterior {
                ui.add_space(10.0);
                if ui.link(RichText::new(format!("↺  Continuar último projeto: {nome}")).size(14.0)).clicked() {
                    acao = Acao::Continuar;
                }
            }

            ui.add_space(36.0);
            ui.allocate_ui_with_layout(egui::vec2(760.0, 120.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.columns(3, |c| {
                    let itens = [
                        ("Sondagem SPT", "Importe ou digite o perfil; γ, c' e φ' por correlações ajustáveis."),
                        ("6 tipos de contenção", "Gravidade, flexão, atirantada, estaca-prancha, estacas e solo grampeado."),
                        ("Normas ABNT", "NBR 11682, 6122, 5629, 16920-2 e 6118:2023 com memorial de cálculo."),
                    ];
                    for (i, (t, d)) in itens.iter().enumerate() {
                        cartao(&mut c[i], |ui| {
                            ui.label(RichText::new(*t).font(tema::fonte_forte(15.0)).color(p.destaque));
                            ui.label(RichText::new(*d).size(13.0).color(p.fraco));
                        });
                    }
                });
            });
            ui.add_space(16.0);
            ui.label(RichText::new(sobre::NORMAS.join("   ·   ")).small().color(p.fraco));
        });
    });
    acao
}
