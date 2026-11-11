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
                    // detalhes da drenagem (barbacã e canaleta) ao lado da seção
                    let mut desenho_dxf = res.desenho.clone();
                    crate::detalhes::anexar(&mut desenho_dxf, proj, res);
                    crate::barras::anexar(&mut desenho_dxf, &res.armaduras);
                    let dxf = crate::dxf::gerar(
                        &desenho_dxf,
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
            if ui
                .button("📄 Exportar memorial (.docx)")
                .on_hover_text("Documento do Word com margem superior folgada e cabeçalho vazio: aplique o papel timbrado do escritório")
                .clicked()
            {
                let nome = format!("memorial_{}.docx", proj.tipo.arquivo());
                if let Some(arq) = rfd::FileDialog::new()
                    .set_title("Salvar memorial de cálculo")
                    .set_file_name(&nome)
                    .add_filter("Documento do Word", &["docx"])
                    .save_file()
                {
                    let mut figuras = vec![("Seção transversal".to_string(), res.desenho.clone())];
                    if crate::detalhes::existem(proj) {
                        figuras.push(("Detalhes construtivos".to_string(), crate::detalhes::montar(proj, res, 1.0, 2)));
                    }
                    if !res.armaduras.is_empty() {
                        figuras.push(("Posições de armação isoladas (dobras e ganchos — NBR 6118)".to_string(), crate::barras::montar(&res.armaduras, 1.0, 2)));
                    }
                    let docx = crate::docx::gerar(res, &titulo_memorial, &crate::sobre::versao_completa(), &figuras);
                    msg = Some(match std::fs::write(&arq, docx) {
                        Ok(_) => {
                            let _ = std::process::Command::new("explorer").arg(&arq).spawn();
                            format!("Memorial salvo em {}", arq.display())
                        }
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
        ui.columns(super::widgets::n_colunas(ui), |c| {
            let ultima = c.len() - 1;
            cartao(&mut c[0], |ui| {
                titulo(ui, "Seção transversal");
                desenho::pintar(ui, &res.desenho, 430.0);
            });
            if ultima == 0 {
                c[0].add_space(10.0);
            }
            cartao(&mut c[ultima], |ui| {
                titulo(ui, "Verificações");
                lista_verificacoes(ui, res);
            });
        });
        if crate::detalhes::existem(proj) {
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Detalhes construtivos (sem escala)");
                let n = crate::detalhes::quantidade(proj);
                let nc = super::widgets::n_colunas(ui);
                for par in 0..n.div_ceil(nc) {
                    ui.columns(nc, |c| {
                        for (j, col) in c.iter_mut().enumerate() {
                            let i = nc * par + j;
                            if i < n {
                                desenho::pintar(col, &crate::detalhes::montar_um(proj, res, i), 300.0);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });
        }
        if !res.armaduras.is_empty() {
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Detalhamento do aço — tabela de armação");
                super::memorial::tabela_armacao(ui, &res.armaduras);
            });
        }
        if !res.armaduras.is_empty() {
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Barras de aço — posições isoladas (dobras e ganchos, NBR 6118)");
                let n = res.armaduras.len();
                let nc = super::widgets::n_colunas(ui);
                for par in 0..n.div_ceil(nc) {
                    ui.columns(nc, |c| {
                        for (j, col) in c.iter_mut().enumerate() {
                            let i = nc * par + j;
                            if i < n {
                                desenho::pintar(col, &crate::barras::montar_uma(&res.armaduras, i), 250.0);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
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
