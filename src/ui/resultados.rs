//! Resultados: situação geral, croqui, verificações, avisos e memorial.

use super::contencao::lista_verificacoes;
use super::tema;
use super::widgets::{cartao, chip, titulo};
use crate::desenho;
use crate::projeto::Projeto;
use crate::resultado::Resultado;
use eframe::egui::{self, RichText};

pub fn mostrar(ui: &mut egui::Ui, proj: &Projeto, res: &Resultado) -> Option<String> {
    let mut msg = None;
    let p = tema::de(ui);
    let titulo_memorial = format!("{} — {}", proj.nome, proj.tipo.nome());

    ui.horizontal(|ui| {
        ui.heading("Resultados");
        ui.add_space(10.0);
        if res.atende() {
            chip(ui, "ATENDE A TODAS AS VERIFICAÇÕES", p.ok);
        } else {
            let n = res.verificacoes.iter().filter(|v| !v.ok()).count();
            chip(ui, &format!("NÃO ATENDE — {n} verificação(ões)"), p.falha);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("📐 Exportar DXF (AutoCAD)")
                .on_hover_text("Seção, armaduras, tabela de armação e notas em DXF R12 (metros). No AutoCAD: abrir e “Salvar como” .dwg.")
                .clicked()
            {
                let nome = format!("detalhamento_{}.dxf", proj.tipo.arquivo());
                if let Some(arq) = rfd::FileDialog::new()
                    .set_title("Exportar detalhamento em DXF")
                    .set_file_name(&nome)
                    .add_filter("Desenho DXF (AutoCAD)", &["dxf"])
                    .save_file()
                {
                    let dxf = crate::dxf::gerar(
                        &res.desenho,
                        &res.armaduras,
                        &titulo_memorial,
                        &super::memorial::notas_detalhamento(res),
                        &crate::sobre::versao_completa(),
                    );
                    msg = Some(match std::fs::write(&arq, dxf) {
                        Ok(_) => format!("DXF salvo em {}", arq.display()),
                        Err(e) => format!("Erro ao salvar: {e}"),
                    });
                }
            }
            if ui.button("🌐 Exportar relatório (.html)").on_hover_text("Relatório visual com seção, fórmulas, normas e status — imprima ou salve em PDF pelo navegador").clicked() {
                let nome = format!("memorial_{}.html", proj.tipo.arquivo());
                if let Some(arq) = rfd::FileDialog::new()
                    .set_title("Salvar relatório do memorial de cálculo")
                    .set_file_name(&nome)
                    .add_filter("Página HTML", &["html"])
                    .save_file()
                {
                    let html = crate::memorial_html::gerar(res, &titulo_memorial, &crate::sobre::versao_completa());
                    msg = Some(match std::fs::write(&arq, html) {
                        Ok(_) => {
                            // abre no navegador padrão
                            let _ = std::process::Command::new("explorer").arg(&arq).spawn();
                            format!("Relatório salvo em {}", arq.display())
                        }
                        Err(e) => format!("Erro ao salvar: {e}"),
                    });
                }
            }
            if ui.button("💾 Exportar memorial (.txt)").clicked() {
                let nome = format!("memorial_{}.txt", proj.tipo.arquivo());
                if let Some(arq) = rfd::FileDialog::new()
                    .set_title("Salvar memorial de cálculo")
                    .set_file_name(&nome)
                    .add_filter("Texto", &["txt"])
                    .save_file()
                {
                    msg = Some(match std::fs::write(&arq, res.texto_completo(&titulo_memorial)) {
                        Ok(_) => format!("Memorial salvo em {}", arq.display()),
                        Err(e) => format!("Erro ao salvar: {e}"),
                    });
                }
            }
            if ui.button("📋 Copiar memorial").clicked() {
                ui.ctx().copy_text(res.texto_completo(&titulo_memorial));
                msg = Some("Memorial copiado para a área de transferência.".into());
            }
        });
    });
    ui.add_space(8.0);

    egui::ScrollArea::vertical().id_salt("rol_resultados").show(ui, |ui| {
        ui.columns(2, |c| {
            cartao(&mut c[0], |ui| {
                titulo(ui, "Seção transversal");
                desenho::pintar(ui, &res.desenho, 430.0);
            });
            cartao(&mut c[1], |ui| {
                titulo(ui, "Verificações");
                lista_verificacoes(ui, res);
            });
        });
        if !res.armaduras.is_empty() {
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Detalhamento do aço — tabela de armação");
                super::memorial::tabela_armacao(ui, &res.armaduras);
            });
        }
        if !res.avisos.is_empty() {
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Avisos");
                for a in &res.avisos {
                    ui.label(RichText::new(format!("⚠  {a}")).color(p.alerta));
                }
            });
        }
        ui.add_space(10.0);
        cartao(ui, |ui| {
            egui::CollapsingHeader::new(RichText::new("Memorial de cálculo").font(tema::fonte_forte(16.0)))
                .default_open(true)
                .show(ui, |ui| {
                    super::memorial::mostrar(ui, res);
                });
        });
        ui.add_space(16.0);
    });
    msg
}
