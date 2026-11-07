//! Central de normas: referências das normas ABNT usadas nos cálculos, links
//! para o Catálogo ABNT e abertura dos PDFs que o usuário já possui (as normas
//! são protegidas por direitos autorais e não acompanham o programa).

use super::tema;
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};

pub struct Norma {
    pub codigo: &'static str,
    pub titulo: &'static str,
    /// Trechos que o nome do arquivo PDF deve conter (minúsculas).
    pub chaves: &'static [&'static str],
    pub usos: &'static str,
}

pub const NORMAS: [Norma; 6] = [
    Norma {
        codigo: "ABNT NBR 11682:2009",
        titulo: "Estabilidade de encostas",
        chaves: &["11682"],
        usos: "Tabelas 3 e 4 (fatores de segurança de muros), 7.3.3 (sobrecarga mínima de 20 kPa), \
               7.3.3.1 e 7.3.3.2 (terço central), drenagem e investigação geotécnica (Anexo D).",
    },
    Norma {
        codigo: "ABNT NBR 6122:2019",
        titulo: "Projeto e execução de fundações",
        chaves: &["6122"],
        usos: "Tabela 1 (FS global 3,0 na capacidade de carga), 7.6.2 (área comprimida ≥ 2/3 da base), \
               7.6.3 (empuxo passivo reduzido por no mínimo 2,0).",
    },
    Norma {
        codigo: "ABNT NBR 6118:2023",
        titulo: "Projeto de estruturas de concreto",
        chaves: &["6118"],
        usos: "Flexão e cisalhamento (seções 17 e 19), ancoragem e emendas (9.4), armaduras mínimas (17.3.5), \
               detalhamento de pilares e estacas (18.4) e concreto simples (seção 24).",
    },
    Norma {
        codigo: "ABNT NBR 5629:2018",
        titulo: "Tirantes ancorados no terreno — Projeto e execução",
        chaves: &["5629"],
        usos: "4.5.2.1 (carga de trabalho do aço), B.2.4.1 (bulbo), B.2.7 (comprimento livre mínimo), \
               5.5.1 (início do bulbo a 3 m do paramento).",
    },
    Norma {
        codigo: "ABNT NBR 16920-1:2021",
        titulo: "Muros e taludes em solos reforçados — Parte 1: Solos reforçados em aterro",
        chaves: &["16920", "parte-1"],
        usos: "Referência complementar para estruturas em solo reforçado.",
    },
    Norma {
        codigo: "ABNT NBR 16920-2:2021",
        titulo: "Muros e taludes em solos reforçados — Parte 2: Solos grampeados",
        chaves: &["16920", "parte-2"],
        usos: "9.1 (γm da armação), 9.3 (paramento), 9.8 (FS da estabilidade interna), \
               10.2.3 e 10.3 (cobrimento da calda e proteção da cabeça).",
    },
];

pub const CATALOGO_ABNT: &str = "https://www.abntcatalogo.com.br";

/// Endereço de busca da norma no Catálogo ABNT.
pub fn busca(n: &Norma) -> String {
    let termo = n.codigo.split(':').next().unwrap_or(n.codigo).replace(' ', "+");
    format!("https://www.google.com/search?q={termo}+site%3Aabntcatalogo.com.br")
}

/// PDF da norma na pasta do usuário (pelo número no nome do arquivo).
pub fn arquivo_local(pasta: &Path, n: &Norma) -> Option<PathBuf> {
    let mut achados: Vec<PathBuf> = std::fs::read_dir(pasta)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let nome = p.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            nome.ends_with(".pdf") && n.chaves.iter().all(|c| nome.contains(c))
        })
        .collect();
    achados.sort();
    achados.into_iter().next()
}

/// Janela da central. Retorna uma mensagem para a barra de status, se houver.
pub fn mostrar(ctx: &egui::Context, aberta: &mut bool, pasta: &mut Option<PathBuf>) -> Option<String> {
    let mut msg = None;
    egui::Window::new("📚 Central de normas")
        .open(aberta)
        .default_width(720.0)
        .default_height(560.0)
        .collapsible(false)
        .show(ctx, |ui| {
            let p = tema::de(ui);
            ui.label(
                RichText::new(
                    "Normas ABNT usadas nos cálculos. As normas são protegidas por direitos autorais e não acompanham \
                     o programa: adquira-as no Catálogo ABNT ou aponte a pasta onde estão os seus PDFs.",
                )
                .color(p.fraco),
            );
            ui.horizontal_wrapped(|ui| {
                ui.hyperlink_to("🔗 Catálogo ABNT", CATALOGO_ABNT);
                ui.separator();
                ui.label("Pasta dos PDFs:");
                ui.label(RichText::new(pasta.as_ref().map(|c| c.display().to_string()).unwrap_or_else(|| "não definida".into())).strong());
                if ui.button("📁 Escolher pasta…").clicked() {
                    if let Some(d) = rfd::FileDialog::new().set_title("Pasta com as normas em PDF").pick_folder() {
                        *pasta = Some(d);
                    }
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for n in &NORMAS {
                    super::widgets::cartao(ui, |ui| {
                        ui.label(RichText::new(n.codigo).font(tema::fonte_forte(15.0)).color(p.destaque));
                        ui.label(RichText::new(n.titulo).strong());
                        ui.label(RichText::new(format!("Uso no programa: {}", n.usos)).size(12.5).color(p.fraco));
                        ui.horizontal(|ui| {
                            ui.hyperlink_to("🔎 Buscar no Catálogo ABNT", busca(n));
                            if let Some(d) = pasta.as_deref() {
                                match arquivo_local(d, n) {
                                    Some(arq) => {
                                        if ui.button("📄 Abrir PDF local").on_hover_text(arq.display().to_string()).clicked() {
                                            msg = Some(match std::process::Command::new("explorer").arg(&arq).spawn() {
                                                Ok(_) => format!("Abrindo {}", arq.display()),
                                                Err(e) => format!("Erro ao abrir: {e}"),
                                            });
                                        }
                                    }
                                    None => {
                                        ui.label(RichText::new("PDF não encontrado na pasta").small().color(p.fraco));
                                    }
                                }
                            }
                        });
                    });
                    ui.add_space(6.0);
                }
            });
        });
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localiza_pdf_pelo_numero() {
        let dir = std::env::temp_dir().join(format!("normas_teste_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["nbr-61182023-concreto.pdf", "abnt-nbr-16920-2021-parte-2-grampeados.pdf", "nbr-6122-fundacoes.pdf"] {
            std::fs::write(dir.join(f), b"%PDF").unwrap();
        }
        let achar = |c: &str| NORMAS.iter().find(|n| n.codigo == c).and_then(|n| arquivo_local(&dir, n));
        assert!(achar("ABNT NBR 6118:2023").is_some());
        assert!(achar("ABNT NBR 6122:2019").is_some());
        assert!(achar("ABNT NBR 16920-2:2021").is_some());
        assert!(achar("ABNT NBR 16920-1:2021").is_none());
        assert!(achar("ABNT NBR 5629:2018").is_none());
        assert!(busca(&NORMAS[0]).contains("ABNT+NBR+11682"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
