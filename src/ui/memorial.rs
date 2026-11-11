//! Aba do memorial de cálculo: títulos, fórmulas passo a passo, caixas de
//! referência normativa e verificações com etiqueta de status.

use super::tema::{self, Paleta};
use super::widgets::titulo;
use crate::resultado::{Bloco, Criterio, Formula, RefNorma, Resultado, Verificacao};
use eframe::egui::{self, Color32, FontFamily, FontId, Label, Margin, Rect, RichText, Stroke};

/// Família de fonte para expressões matemáticas (ver `tema::fontes`).
pub const FONTE_MATEMATICA: &str = "matematica";

fn fonte_mat(tam: f32) -> FontId {
    FontId::new(tam, FontFamily::Name(FONTE_MATEMATICA.into()))
}

fn cor_norma(escuro: bool) -> Color32 {
    if escuro { Color32::from_rgb(163, 113, 247) } else { Color32::from_rgb(130, 80, 223) }
}

/// Cor e texto da etiqueta de status de uma verificação.
pub fn cor_status(v: &Verificacao, p: &Paleta) -> Color32 {
    match v.criterio {
        Criterio::Info => p.fraco,
        _ if v.ok() => p.ok,
        _ => p.falha,
    }
}

/// Etiqueta de status: [ ATENDIDO ] / [ NÃO ATENDIDO ] / [ INFORMATIVO ].
pub fn etiqueta(ui: &mut egui::Ui, v: &Verificacao) {
    let p = tema::de(ui);
    let cor = cor_status(v, &p);
    let simbolo = match v.criterio {
        Criterio::Info => "•",
        _ if v.ok() => "✔",
        _ => "✖",
    };
    egui::Frame::new()
        .fill(cor.gamma_multiply(0.16))
        .stroke(Stroke::new(1.0_f32, cor))
        .corner_radius(12.0)
        .inner_margin(Margin::symmetric(9, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(format!("{simbolo} {}", v.status())).color(cor).font(tema::fonte_forte(12.0)));
        });
}

/// Caixa com barra colorida à esquerda.
fn caixa(ui: &mut egui::Ui, fundo: Color32, borda: Color32, barra: Color32, conteudo: impl FnOnce(&mut egui::Ui)) {
    let resp = egui::Frame::new()
        .fill(fundo)
        .stroke(Stroke::new(1.0_f32, borda))
        .corner_radius(8.0)
        .inner_margin(Margin { left: 16, right: 12, top: 8, bottom: 8 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            conteudo(ui);
        });
    let r = resp.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(r.min, egui::vec2(4.0, r.height())), 2.0, barra);
}

fn formula(ui: &mut egui::Ui, f: &Formula, p: &Paleta) {
    caixa(ui, p.entrada, p.borda, p.destaque, |ui| {
        ui.label(RichText::new(&f.titulo).font(tema::fonte_forte(13.5)).color(p.texto));
        ui.add_space(2.0);
        let passo = |ui: &mut egui::Ui, rotulo: &str, texto: &str, destaque: bool| {
            ui.horizontal_top(|ui| {
                ui.add_sized(
                    [104.0, 20.0],
                    Label::new(RichText::new(rotulo).font(tema::fonte_forte(11.0)).color(p.fraco)),
                );
                let mut t = RichText::new(texto).font(fonte_mat(16.0));
                t = if destaque { t.color(p.destaque).strong() } else { t.color(p.texto) };
                ui.add(Label::new(t).wrap());
            });
        };
        passo(ui, "EQUAÇÃO", &f.equacao, false);
        if !f.substituicao.is_empty() {
            passo(ui, "SUBSTITUIÇÃO", &f.substituicao, false);
        }
        passo(ui, "RESULTADO", &f.resultado, true);
    });
}

fn norma(ui: &mut egui::Ui, r: &RefNorma, p: &Paleta) {
    let cor = cor_norma(ui.visuals().dark_mode);
    caixa(ui, cor.gamma_multiply(0.10), cor.gamma_multiply(0.45), cor, |ui| {
        ui.label(RichText::new(format!("§  {} — {}", r.norma, r.item)).font(tema::fonte_forte(13.0)).color(cor));
        ui.add(Label::new(RichText::new(r.requisito).color(p.texto)).wrap());
    });
}

fn verificacao(ui: &mut egui::Ui, v: &Verificacao, p: &Paleta) {
    let cor = cor_status(v, p);
    egui::Frame::new()
        .fill(cor.gamma_multiply(0.07))
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&v.nome).color(p.texto));
                ui.label(RichText::new(v.expressao()).font(fonte_mat(16.0)).color(cor).strong());
                ui.label(RichText::new("→").color(p.fraco));
                etiqueta(ui, v);
            });
        });
}

/// Tabela de armação (detalhamento do aço).
pub fn tabela_armacao(ui: &mut egui::Ui, arm: &[crate::armadura::Armadura]) {
    let p = tema::de(ui);
    egui::ScrollArea::horizontal().id_salt("tab_arm_h").show(ui, |ui| {
        egui::Grid::new("tab_armacao").num_columns(9).striped(true).spacing([14.0, 5.0]).show(ui, |ui| {
            for c in crate::armadura::CABECALHO {
                ui.label(RichText::new(c).font(tema::fonte_forte(13.0)).color(p.fraco));
            }
            ui.end_row();
            for a in arm {
                for (i, c) in a.celulas().into_iter().enumerate() {
                    let t = RichText::new(c);
                    ui.label(if i == 0 || i == 2 { t.strong().color(p.destaque) } else { t });
                }
                ui.end_row();
            }
        });
    });
    ui.label(RichText::new("CA-50 ; comprimentos aproximados — conferir dobras, cobrimentos e emendas no projeto executivo.").small().color(p.fraco));
}

/// Linhas de texto da seção de detalhamento construtivo (notas do DXF).
pub fn notas_detalhamento(res: &Resultado) -> Vec<String> {
    let mut dentro = false;
    let mut v = Vec::new();
    for b in &res.blocos {
        match b {
            Bloco::Titulo(t) => dentro = t.starts_with("Detalhamento construtivo"),
            Bloco::Texto(t) if dentro && !t.is_empty() => v.push(t.clone()),
            _ => {}
        }
    }
    v
}

/// Desenha o memorial completo.
pub fn mostrar(ui: &mut egui::Ui, res: &Resultado) {
    let p = tema::de(ui);
    let id = ui.make_persistent_id("memorial_filtros");
    let (mut ver_formulas, mut ver_normas) = ui.data_mut(|d| *d.get_persisted_mut_or(id, (true, true)));
    ui.horizontal(|ui| {
        super::widgets::caixa(ui, &mut ver_formulas, "Fórmulas passo a passo");
        super::widgets::caixa(ui, &mut ver_normas, "Referências normativas");
        ui.label(RichText::new("Normas: sínteses dos requisitos — consulte o texto integral da ABNT.").small().color(p.fraco));
    });
    ui.data_mut(|d| d.insert_persisted(id, (ver_formulas, ver_normas)));
    ui.add_space(4.0);

    for b in &res.blocos {
        match b {
            Bloco::Titulo(t) => {
                ui.add_space(10.0);
                titulo(ui, t);
            }
            Bloco::Texto(t) if t.is_empty() => {}
            Bloco::Texto(t) if t.contains("  ") => {
                ui.label(RichText::new(t).monospace().color(p.texto));
            }
            Bloco::Texto(t) => {
                ui.add(Label::new(RichText::new(t).color(p.texto)).wrap());
            }
            Bloco::Formula(f) if ver_formulas => {
                ui.add_space(2.0);
                formula(ui, f, &p);
                ui.add_space(2.0);
            }
            Bloco::Norma(r) if ver_normas => {
                ui.add_space(2.0);
                norma(ui, r, &p);
                ui.add_space(2.0);
            }
            Bloco::Formula(_) | Bloco::Norma(_) => {}
            Bloco::Verif(i) => verificacao(ui, &res.verificacoes[*i], &p),
        }
    }
    if !res.referencias.is_empty() {
        ui.add_space(10.0);
        titulo(ui, "Referências bibliográficas");
        for r in &res.referencias {
            ui.add(Label::new(RichText::new(r).color(p.texto)).wrap());
            ui.add_space(3.0);
        }
    }
}
