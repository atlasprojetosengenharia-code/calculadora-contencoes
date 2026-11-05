//! Fatores de segurança e coeficientes normativos.

use super::contencao::tirante_tipo;
use super::tema;
use super::widgets::{cartao, grade, nota, num, titulo};
use crate::fatores::{Fatores, Nivel};
use crate::resultado::fmt;
use eframe::egui::{self, RichText};

pub fn mostrar(ui: &mut egui::Ui, f: &mut Fatores) {
    let p = tema::de(ui);
    ui.horizontal(|ui| {
        ui.heading("Normas e fatores de segurança");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Restaurar valores das normas").clicked() {
                *f = Fatores::default();
            }
        });
    });
    nota(ui, "Valores padrão: NBR 11682:2009, NBR 6122:2019, NBR 5629:2018, NBR 16920-2:2021 e NBR 6118:2023. Valem para o projeto inteiro.");
    ui.add_space(6.0);

    egui::ScrollArea::vertical().id_salt("rol_fatores").show(ui, |ui| {
        ui.columns(2, |c| {
            let ui = &mut c[0];
            cartao(ui, |ui| {
                titulo(ui, "NBR 11682 — Nível de segurança (Tab. 1, 2 e 3)");
                grade(ui, "niv", |ui| {
                    for (rotulo, campo, id) in [
                        ("Perda de vidas humanas", &mut f.nivel_vidas, "nv"),
                        ("Danos materiais e ambientais", &mut f.nivel_danos, "nd"),
                    ] {
                        ui.label(rotulo);
                        egui::ComboBox::from_id_salt(id).selected_text(campo.nome()).show_ui(ui, |ui| {
                            for n in [Nivel::Alto, Nivel::Medio, Nivel::Baixo] {
                                ui.selectable_value(campo, n, n.nome());
                            }
                        });
                        ui.end_row();
                    }
                });
                ui.checkbox(&mut f.majorar_variabilidade, "Grande variabilidade dos ensaios (+10 %, Nota 1)");
                ui.label(
                    RichText::new(format!("FS mínimo de estabilidade = {}", fmt(f.fs_global())))
                        .font(tema::fonte_forte(16.0))
                        .color(p.destaque),
                );
                nota(ui, "Válido para todos os carregamentos, inclusive sismo (7.3.7.1).");
            });
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Muros — estabilidade externa");
                grade(ui, "mur", |ui| {
                    num(ui, "FS deslizamento", &mut f.fs_deslizamento, "", 1.0..=5.0, 0.01);
                    num(ui, "FS tombamento", &mut f.fs_tombamento, "", 1.0..=5.0, 0.01);
                    num(ui, "FS capacidade de carga (NBR 6122)", &mut f.fs_capacidade_carga, "", 1.0..=6.0, 0.01);
                    num(ui, "Excentricidade máx. / B", &mut f.excentricidade_max, "", 0.05..=0.5, 0.001);
                    num(ui, "Área comprimida mín. / B", &mut f.area_comprimida_min, "", 0.3..=1.0, 0.01);
                    num(ui, "Sobrecarga mínima (7.3.3)", &mut f.sobrecarga_minima, "kPa", 0.0..=50.0, 0.5);
                });
                nota(ui, "Tombamento 2,0 e deslizamento 1,5: prática usual (a página 19 da NBR 11682 fornecida está ausente).");
                ui.add_space(4.0);
                ui.label(RichText::new("Caso sísmico").strong());
                grade(ui, "sis", |ui| {
                    num(ui, "FS deslizamento", &mut f.fs_deslizamento_sismo, "", 1.0..=5.0, 0.01);
                    num(ui, "FS tombamento", &mut f.fs_tombamento_sismo, "", 1.0..=5.0, 0.01);
                    num(ui, "FS capacidade de carga", &mut f.fs_capacidade_carga_sismo, "", 1.0..=6.0, 0.01);
                });
            });
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "Empuxo passivo e contenções embutidas");
                grade(ui, "emb", |ui| {
                    num(ui, "Redução do passivo — muros (NBR 6122 ≥ 2,0)", &mut f.reducao_passivo_muros, "", 1.0..=4.0, 0.01);
                    num(ui, "FS do passivo — embutidas (projeto)", &mut f.fs_passivo, "", 1.0..=4.0, 0.01);
                    num(ui, "Acréscimo de ficha — balanço", &mut f.acrescimo_ficha_balanco, "", 1.0..=2.0, 0.01);
                    num(ui, "Acréscimo de ficha — ancorada", &mut f.acrescimo_ficha_ancorada, "", 1.0..=2.0, 0.01);
                });
            });

            let ui = &mut c[1];
            cartao(ui, |ui| {
                titulo(ui, "NBR 5629:2018 — Tirantes");
                tirante_tipo(ui, f);
                grade(ui, "tir", |ui| {
                    num(ui, "Aço permanente: 0,9·fyk /", &mut f.fs_tirante_aco_perm, "", 1.0..=3.0, 0.01);
                    num(ui, "Aço provisório: 0,9·fyk /", &mut f.fs_tirante_aco_prov, "", 1.0..=3.0, 0.01);
                    num(ui, "Bulbo permanente (B.2.4.1)", &mut f.fs_tirante_bulbo_perm, "", 1.0..=4.0, 0.01);
                    num(ui, "Bulbo provisório (B.2.4.1)", &mut f.fs_tirante_bulbo_prov, "", 1.0..=4.0, 0.01);
                    num(ui, "Trecho livre mín. — rosca", &mut f.tirante_livre_rosca, "m", 0.0..=10.0, 0.1);
                    num(ui, "Trecho livre mín. — clavetes", &mut f.tirante_livre_clavete, "m", 0.0..=10.0, 0.1);
                    num(ui, "Folga além da cunha (× H)", &mut f.tirante_folga_h, "", 0.0..=0.5, 0.01);
                    num(ui, "Bulbo mínimo (projeto)", &mut f.tirante_bulbo_min, "m", 0.0..=10.0, 0.1);
                });
            });
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "NBR 16920-2:2021 — Solo grampeado");
                grade(ui, "gr", |ui| {
                    num(ui, "FS estabilidade interna (9.8)", &mut f.fs_grampo_interna, "", 1.0..=4.0, 0.01);
                    num(ui, "γm da armação (9.1)", &mut f.gamma_m_grampo, "", 1.0..=2.0, 0.01);
                });
            });
            ui.add_space(10.0);
            cartao(ui, |ui| {
                titulo(ui, "NBR 6118:2023 / NBR 8800 — Ponderação");
                grade(ui, "pond", |ui| {
                    num(ui, "γf (ações)", &mut f.gamma_f, "", 1.0..=2.0, 0.01);
                    num(ui, "γc (concreto)", &mut f.gamma_c, "", 1.0..=2.0, 0.01);
                    num(ui, "γs (aço CA)", &mut f.gamma_s, "", 1.0..=2.0, 0.01);
                    num(ui, "γa1 (aço estrutural)", &mut f.gamma_a1, "", 1.0..=2.0, 0.01);
                });
            });
        });
        ui.add_space(16.0);
    });
}
