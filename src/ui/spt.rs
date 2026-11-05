//! Bloco A — Sondagem SPT: tabela de camadas, importação e perfil gráfico.

use super::tema;
use super::widgets::{cartao, nota, titulo};
use crate::resultado::fmt;
use crate::spt::{compacidade, importar, CamadaSpt, CondicaoCoesivos, MetodoPhi, Sondagem, TipoSolo, TIPOS};
use eframe::egui::{self, Color32, RichText, Stroke};

/// Lê arquivo de texto aceitando UTF-8 ou Windows-1252/Latin-1 (CSV do Excel).
fn ler_texto(caminho: &std::path::Path) -> Result<String, String> {
    let b = std::fs::read(caminho).map_err(|e| e.to_string())?;
    Ok(match String::from_utf8(b.clone()) {
        Ok(s) => s,
        Err(_) => b.iter().map(|&c| c as char).collect(),
    })
}

/// Retorna mensagem de status, se houver.
pub fn mostrar(ui: &mut egui::Ui, s: &mut Sondagem, h_contencao: f64) -> Option<String> {
    let mut msg = None;
    let mut mudou = false;
    let p = tema::de(ui);

    ui.horizontal(|ui| {
        ui.heading("Sondagem SPT");
        ui.add_space(12.0);
        ui.label("Identificação:");
        ui.add(egui::TextEdit::singleline(&mut s.identificacao).desired_width(110.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑 Limpar").on_hover_text("Remove todas as camadas").clicked() {
                s.camadas.clear();
                mudou = true;
            }
            if ui.button("⇊ Agrupar camadas iguais").on_hover_text("Une camadas consecutivas com mesma descrição").clicked() {
                s.agrupar();
                mudou = true;
            }
            if ui.button("➕ Camada").clicked() {
                let ate = s.camadas.last().map(|c| c.ate + 1.0).unwrap_or(1.0);
                let ult = s.camadas.last().cloned().unwrap_or_else(|| CamadaSpt::nova(1.0, 5.0, "Areia"));
                s.camadas.push(CamadaSpt { ate, manual: false, ..ult });
                mudou = true;
            }
            if ui
                .add(egui::Button::new(RichText::new("📂 Importar CSV / TXT").color(Color32::WHITE)).fill(p.destaque))
                .clicked()
            {
                if let Some(arq) = rfd::FileDialog::new()
                    .set_title("Importar sondagem SPT")
                    .add_filter("Sondagem (CSV, TXT)", &["csv", "txt"])
                    .pick_file()
                {
                    match ler_texto(&arq).and_then(|t| importar(&t)) {
                        Ok((camadas, na)) => {
                            let n = camadas.len();
                            s.camadas = camadas;
                            if let Some(z) = na {
                                s.usar_na = true;
                                s.prof_na = z;
                            }
                            mudou = true;
                            msg = Some(format!("{n} registros importados de {}", arq.display()));
                        }
                        Err(e) => msg = Some(format!("Erro na importação: {e}")),
                    }
                }
            }
        });
    });
    ui.add_space(6.0);

    let largura_grafico = 300.0f32.min(ui.available_width() * 0.32);
    ui.horizontal_top(|ui| {
        let larg_tab = ui.available_width() - largura_grafico - 12.0;
        ui.allocate_ui_with_layout(egui::vec2(larg_tab, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical().id_salt("spt_rol").show(ui, |ui| {
                cartao(ui, |ui| {
                    titulo(ui, "Camadas");
                    if tabela(ui, s) {
                        mudou = true;
                    }
                    if s.camadas.is_empty() {
                        nota(ui, "Nenhuma camada. Use “➕ Camada” ou importe um arquivo.");
                    }
                });
                ui.add_space(10.0);
                cartao(ui, |ui| {
                    titulo(ui, "Nível d'água e correlações");
                    ui.horizontal(|ui| {
                        mudou |= ui.checkbox(&mut s.usar_na, "Nível d'água encontrado a").changed();
                        mudou |= ui
                            .add_enabled(s.usar_na, egui::DragValue::new(&mut s.prof_na).speed(0.05).range(0.0..=100.0).suffix(" m"))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("φ' de solos granulares:");
                        egui::ComboBox::from_id_salt("metodo_phi").selected_text(s.metodo_phi.nome()).width(260.0).show_ui(ui, |ui| {
                            for m in [MetodoPhi::Teixeira1996, MetodoPhi::Godoy1983] {
                                mudou |= ui.selectable_value(&mut s.metodo_phi, m, m.nome()).changed();
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Solos coesivos:");
                        mudou |= ui.radio_value(&mut s.coesivos, CondicaoCoesivos::Drenada, "Drenado (longo prazo)").changed();
                        mudou |= ui
                            .radio_value(&mut s.coesivos, CondicaoCoesivos::NaoDrenada, "Não drenado: Su = 10·N, φ = 0")
                            .changed();
                    });
                    if s.coesivos == CondicaoCoesivos::Drenada {
                        ui.horizontal(|ui| {
                            ui.label("   c' adotado");
                            mudou |= ui.add(egui::DragValue::new(&mut s.c_drenado).speed(0.5).range(0.0..=200.0).suffix(" kPa")).changed();
                            ui.label("φ' adotado");
                            mudou |= ui.add(egui::DragValue::new(&mut s.phi_drenado).speed(0.25).range(0.0..=45.0).suffix(" °")).changed();
                        });
                    }
                    nota(ui, "γ por Godoy (1972). Correlações são estimativas: confirme com ensaios (NBR 11682 Anexo D). Clique em “Auto” para ajustar valores manualmente.");
                });
                ui.add_space(10.0);
                cartao(ui, |ui| {
                    titulo(ui, "Formato do arquivo de importação");
                    nota(ui, "Uma linha por registro, separador ; (aceita vírgula decimal), tabulação ou vírgula:");
                    ui.label(RichText::new("prof. inicial ; prof. final ; NSPT ; descrição").monospace());
                    ui.label(RichText::new("profundidade ; NSPT ; descrição      (ensaio metro a metro)").monospace());
                    ui.label(RichText::new("NA ; 3,50                           (nível d'água)").monospace());
                    nota(ui, "Cabeçalhos e linhas iniciadas por # são ignorados. Descrição vazia repete a anterior.");
                });
            });
        });
        ui.add_space(12.0);
        ui.allocate_ui_with_layout(egui::vec2(largura_grafico, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
            cartao(ui, |ui| {
                titulo(ui, "Perfil");
                perfil(ui, s, h_contencao);
            });
        });
    });

    if mudou {
        s.normalizar();
        s.recalcular();
    }
    msg
}

fn tabela(ui: &mut egui::Ui, s: &mut Sondagem) -> bool {
    let mut mudou = false;
    let mut remover = None;
    let p = tema::de(ui);
    egui::ScrollArea::horizontal().id_salt("spt_h").show(ui, |ui| {
        egui::Grid::new("tab_spt").num_columns(13).striped(true).spacing([8.0, 5.0]).show(ui, |ui| {
            for t in ["#", "De (m)", "Até (m)", "NSPT", "Descrição", "Tipo", "Estado", "γ", "γsat", "c' (kPa)", "φ' (°)", "", ""] {
                ui.label(RichText::new(t).font(tema::fonte_forte(13.0)).color(p.fraco));
            }
            ui.end_row();
            let n = s.camadas.len();
            for i in 0..n {
                let de = s.de(i);
                let c = &mut s.camadas[i];
                let [r, g, b] = c.tipo.cor();
                let (q, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(q, 4.0, Color32::from_rgb(r, g, b));
                ui.painter().text(q.center(), egui::Align2::CENTER_CENTER, format!("{}", i + 1), egui::FontId::proportional(11.0), Color32::BLACK);
                ui.label(fmt(de));
                mudou |= ui
                    .add_sized([64.0, 22.0], egui::DragValue::new(&mut c.ate).speed(0.05).range(de + 0.05..=200.0).max_decimals(2))
                    .changed();
                mudou |= ui.add_sized([48.0, 22.0], egui::DragValue::new(&mut c.nspt).speed(0.2).range(0.0..=100.0).max_decimals(0)).changed();
                let rd = ui.add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut c.descricao).hint_text("ex.: Areia fina argilosa"));
                if rd.changed() {
                    if let Some(t) = TipoSolo::da_descricao(&c.descricao) {
                        c.tipo = t;
                    }
                    mudou = true;
                }
                egui::ComboBox::from_id_salt(("tipo", i)).selected_text(c.tipo.nome()).width(120.0).show_ui(ui, |ui| {
                    for t in TIPOS {
                        mudou |= ui.selectable_value(&mut c.tipo, t, t.nome()).changed();
                    }
                });
                ui.label(RichText::new(compacidade(c.tipo, c.nspt)).color(p.fraco));
                let campos: [(&mut f64, f64, RangeF); 4] = [
                    (&mut c.gamma, 0.1, 10.0..=30.0),
                    (&mut c.gamma_sat, 0.1, 10.0..=30.0),
                    (&mut c.c, 0.5, 0.0..=500.0),
                    (&mut c.phi, 0.25, 0.0..=50.0),
                ];
                let mut editou = false;
                for (v, vel, faixa) in campos {
                    let r = ui.add_sized(
                        [56.0, 22.0],
                        egui::DragValue::new(v).speed(vel).range(faixa).max_decimals(1).custom_formatter(|x, _| format!("{x:.1}")),
                    );
                    editou |= r.changed();
                }
                if editou {
                    c.manual = true;
                    mudou = true;
                }
                let (txt, cor) = if c.manual { ("Manual", p.alerta) } else { ("Auto", p.ok) };
                if ui
                    .add(egui::Button::new(RichText::new(txt).color(cor).size(12.0)).min_size(egui::vec2(58.0, 20.0)))
                    .on_hover_text("Auto = parâmetros por correlação com o SPT. Manual = valores digitados. Clique para alternar.")
                    .clicked()
                {
                    c.manual = !c.manual;
                    mudou = true;
                }
                if ui.small_button("✖").on_hover_text("Remover camada").clicked() {
                    remover = Some(i);
                }
                ui.end_row();
            }
        });
    });
    if let Some(i) = remover {
        s.camadas.remove(i);
        mudou = true;
    }
    mudou
}

type RangeF = std::ops::RangeInclusive<f64>;

/// Perfil estratigráfico com NSPT, NA e altura da contenção.
fn perfil(ui: &mut egui::Ui, s: &Sondagem, h: f64) {
    let p = tema::de(ui);
    let alt = (ui.available_height() - 10.0).clamp(300.0, 620.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), alt), egui::Sense::hover());
    let pt = ui.painter_at(r);
    let zmax = s.camadas.last().map(|c| c.ate).unwrap_or(10.0).max(h + 1.0).max(1.0);
    let topo = r.top() + 16.0;
    let fundo = r.bottom() - 8.0;
    let y = |z: f64| topo + (fundo - topo) * (z / zmax) as f32;
    let x0 = r.left() + 34.0;
    let x_col = x0 + (r.width() - 34.0) * 0.42;
    let nmax = s.camadas.iter().map(|c| c.nspt).fold(10.0, f64::max);

    // eixo de profundidade
    let passo = if zmax > 20.0 { 5.0 } else if zmax > 8.0 { 2.0 } else { 1.0 };
    let mut z = 0.0;
    while z <= zmax + 1e-6 {
        pt.line_segment([egui::pos2(x0 - 4.0, y(z)), egui::pos2(r.right(), y(z))], Stroke::new(0.5_f32, p.borda));
        pt.text(egui::pos2(x0 - 6.0, y(z)), egui::Align2::RIGHT_CENTER, format!("{z:.0}"), egui::FontId::proportional(11.0), p.fraco);
        z += passo;
    }
    pt.text(egui::pos2(r.left() + 2.0, r.top() + 2.0), egui::Align2::LEFT_TOP, "z (m)", egui::FontId::proportional(11.0), p.fraco);
    pt.text(egui::pos2(x_col + 4.0, r.top() + 2.0), egui::Align2::LEFT_TOP, "NSPT", egui::FontId::proportional(11.0), p.fraco);

    for (i, c) in s.camadas.iter().enumerate() {
        let (z0, z1) = (s.de(i), c.ate);
        let [rr, gg, bb] = c.tipo.cor();
        let caixa = egui::Rect::from_min_max(egui::pos2(x0, y(z0)), egui::pos2(x_col - 4.0, y(z1)));
        pt.rect_filled(caixa, 2.0, Color32::from_rgb(rr, gg, bb));
        pt.rect_stroke(caixa, 2.0, Stroke::new(0.8_f32, Color32::from_black_alpha(90)), egui::StrokeKind::Inside);
        if caixa.height() > 14.0 {
            pt.text(caixa.center(), egui::Align2::CENTER_CENTER, c.tipo.nome(), egui::FontId::proportional(11.0), Color32::from_rgb(25, 25, 25));
        }
        let w = ((r.right() - x_col - 26.0) as f64 * (c.nspt / nmax)) as f32;
        let barra = egui::Rect::from_min_max(egui::pos2(x_col, y(z0) + 2.0), egui::pos2(x_col + w.max(2.0), y(z1) - 2.0));
        pt.rect_filled(barra, 2.0, p.destaque.gamma_multiply(0.75));
        pt.text(egui::pos2(barra.right() + 4.0, barra.center().y), egui::Align2::LEFT_CENTER, format!("{:.0}", c.nspt), egui::FontId::proportional(11.0), p.texto);
    }
    if let Some(na) = s.na() {
        let yy = y(na);
        pt.line_segment([egui::pos2(x0, yy), egui::pos2(r.right(), yy)], Stroke::new(1.5_f32, Color32::from_rgb(40, 140, 230)));
        let tri = vec![egui::pos2(x0 + 8.0, yy - 9.0), egui::pos2(x0 + 18.0, yy - 9.0), egui::pos2(x0 + 13.0, yy - 1.0)];
        pt.add(egui::Shape::convex_polygon(tri, Color32::from_rgb(40, 140, 230), Stroke::NONE));
        pt.text(egui::pos2(r.right() - 2.0, yy - 8.0), egui::Align2::RIGHT_CENTER, format!("▽ NA {} m", fmt(na)), egui::FontId::proportional(11.0), Color32::from_rgb(40, 140, 230));
    }
    if h > 0.0 {
        let yy = y(h);
        pt.extend(egui::Shape::dashed_line(&[egui::pos2(x0, yy), egui::pos2(r.right(), yy)], Stroke::new(1.5_f32, p.falha), 6.0, 4.0));
        pt.text(egui::pos2(r.right() - 2.0, yy + 8.0), egui::Align2::RIGHT_CENTER, format!("H = {} m", fmt(h)), egui::FontId::proportional(11.0), p.falha);
    }
}
