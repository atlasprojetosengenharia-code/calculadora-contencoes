//! Aplicativo: navegação entre a tela de abertura e o ambiente de projeto.

use crate::projeto::Projeto;
use crate::resultado::Resultado;
use crate::sobre;
use crate::ui::{self, tema, widgets};
use eframe::egui::{self, RichText, Stroke};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Clone, Copy, PartialEq)]
enum Tela {
    #[default]
    Inicio,
    Projeto,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Etapa {
    #[default]
    Sondagem,
    Contencao,
    Resultados,
    Normas,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    proj: Projeto,
    caminho: Option<PathBuf>,
    tem_sessao: bool,
    /// Pasta com os PDFs de normas do usuário (Central de normas).
    pasta_normas: Option<PathBuf>,
    #[serde(skip)]
    central_normas: bool,
    #[serde(skip)]
    tela: Tela,
    #[serde(skip)]
    etapa: Etapa,
    #[serde(skip)]
    cache: Option<(String, Resultado)>,
    #[serde(skip)]
    msg: String,
    #[serde(skip)]
    atualiz: Option<crate::atualizacao::Verificador>,
}

impl App {
    pub fn novo(cc: &eframe::CreationContext<'_>) -> Self {
        tema::aplicar(&cc.egui_ctx);
        let mut app: App = cc.storage.and_then(|s| eframe::get_value::<App>(s, eframe::APP_KEY)).unwrap_or_default();
        app.atualiz = Some(crate::atualizacao::Verificador::iniciar(cc.egui_ctx.clone()));
        app
    }

    fn resultado(&mut self) -> Resultado {
        let chave = format!("{:?}", self.proj);
        if let Some((k, r)) = &self.cache {
            if *k == chave {
                return r.clone();
            }
        }
        let r = self.proj.calcular();
        self.cache = Some((chave, r.clone()));
        r
    }

    fn novo_projeto(&mut self) {
        self.proj = Projeto::default();
        self.caminho = None;
        self.tem_sessao = true;
        self.tela = Tela::Projeto;
        self.etapa = Etapa::Sondagem;
        self.msg = "Novo projeto criado.".into();
    }

    fn abrir(&mut self) {
        if let Some(arq) = rfd::FileDialog::new()
            .set_title("Abrir projeto")
            .add_filter("Projeto de contenção", &["ctc", "json"])
            .pick_file()
        {
            match Projeto::abrir(&arq) {
                Ok(p) => {
                    self.proj = p;
                    self.msg = format!("Projeto aberto: {}", arq.display());
                    self.caminho = Some(arq);
                    self.tem_sessao = true;
                    self.tela = Tela::Projeto;
                    self.etapa = Etapa::Contencao;
                }
                Err(e) => self.msg = format!("Erro ao abrir: {e}"),
            }
        }
    }

    fn salvar(&mut self, como: bool) {
        let destino = match (&self.caminho, como) {
            (Some(c), false) => Some(c.clone()),
            _ => rfd::FileDialog::new()
                .set_title("Salvar projeto")
                .set_file_name(format!("{}.ctc", self.proj.nome.trim()))
                .add_filter("Projeto de contenção", &["ctc"])
                .save_file(),
        };
        if let Some(c) = destino {
            match self.proj.salvar(&c) {
                Ok(_) => {
                    self.msg = format!("Projeto salvo em {}", c.display());
                    self.caminho = Some(c);
                }
                Err(e) => self.msg = format!("Erro ao salvar: {e}"),
            }
        }
    }

    fn abrir_manual(&mut self) {
        self.msg = match crate::manual::abrir() {
            Ok(c) => format!("Manual do usuário aberto: {}", c.display()),
            Err(e) => format!("Erro ao abrir o manual: {e}"),
        };
    }

    fn salvar_manual(&mut self) {
        if let Some(r) = crate::manual::salvar_copia() {
            self.msg = match r {
                Ok(c) => format!("Manual salvo em {}", c.display()),
                Err(e) => format!("Erro ao salvar o manual: {e}"),
            };
        }
    }

    fn tela_projeto(&mut self, ctx: &egui::Context) {
        let p = tema::paleta(ctx.style().visuals.dark_mode);

        // Barra superior
        egui::TopBottomPanel::top("barra")
            .frame(egui::Frame::new().fill(p.painel).inner_margin(egui::Margin::symmetric(16, 9)).stroke(Stroke::new(1.0_f32, p.borda)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    widgets::logo(ui, 30.0);
                    ui.label(RichText::new(sobre::NOME).font(tema::fonte_forte(17.0)));
                    ui.separator();
                    ui.label(RichText::new("Projeto").color(p.fraco));
                    ui.add(egui::TextEdit::singleline(&mut self.proj.nome).desired_width(240.0).font(tema::fonte_forte(14.5)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::widgets::global_theme_preference_switch(ui);
                        if let Some(v) = &self.atualiz {
                            crate::atualizacao::aviso(ui, &v.estado());
                        }
                        if ui.button("🏠 Início").clicked() {
                            self.tela = Tela::Inicio;
                        }
                        if ui.button("📚 Normas").on_hover_text("Central de normas ABNT").clicked() {
                            self.central_normas = true;
                        }
                        if ui.button("📘 Manual").on_hover_text("Manual do usuário (PDF)").clicked() {
                            self.abrir_manual();
                        }
                        ui.separator();
                        if ui.button("Salvar como…").clicked() {
                            self.salvar(true);
                        }
                        if ui.button("💾 Salvar").clicked() {
                            self.salvar(false);
                        }
                        if ui.button("📂 Abrir").clicked() {
                            self.abrir();
                        }
                        if ui.button("🗋 Novo").clicked() {
                            self.novo_projeto();
                        }
                    });
                });
            });

        // Barra de status
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::new().fill(p.painel).inner_margin(egui::Margin::symmetric(16, 5)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.msg).small().color(p.fraco));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("v{} · {}", sobre::versao_completa(), sobre::copyright())).small().color(p.fraco));
                        ui.hyperlink_to(RichText::new(sobre::site_curto()).small(), sobre::SITE);
                        if let Some(c) = &self.caminho {
                            ui.label(RichText::new(format!("{}   ·", c.display())).small().color(p.fraco));
                        }
                    });
                });
            });

        // Navegação lateral
        egui::SidePanel::left("nav")
            .exact_width(236.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.painel).inner_margin(egui::Margin::symmetric(12, 14)).stroke(Stroke::new(1.0_f32, p.borda)))
            .show(ctx, |ui| {
                ui.label(RichText::new("ETAPAS").small().color(p.fraco));
                ui.add_space(4.0);
                let itens = [
                    (Etapa::Sondagem, "1", "Sondagem SPT", "Camadas e parâmetros"),
                    (Etapa::Contencao, "2", "Contenção e geometria", "Tipo, dimensões, drenagem"),
                    (Etapa::Resultados, "3", "Resultados", "Verificações e memorial"),
                    (Etapa::Normas, "⚙", "Normas e fatores", "Coeficientes de segurança"),
                ];
                for (e, n, nome, desc) in itens {
                    if item_nav(ui, self.etapa == e, n, nome, desc) {
                        self.etapa = e;
                    }
                }
                ui.add_space(10.0);
                ui.separator();
                ui.label(RichText::new("DADOS DO PROJETO").small().color(p.fraco));
                ui.label("Local / obra");
                ui.add(egui::TextEdit::singleline(&mut self.proj.local).desired_width(f32::INFINITY));
                ui.label("Responsável técnico");
                ui.add(egui::TextEdit::singleline(&mut self.proj.responsavel).desired_width(f32::INFINITY));
                ui.add_space(8.0);
                ui.label(RichText::new("CONTENÇÃO").small().color(p.fraco));
                ui.label(RichText::new(self.proj.tipo.nome()).font(tema::fonte_forte(14.0)).color(p.destaque));
            });

        let res = self.resultado();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.fundo).inner_margin(egui::Margin::same(18)))
            .show(ctx, |ui| match self.etapa {
                Etapa::Sondagem => {
                    let h = self.proj.altura_contida();
                    if let Some(m) = ui::spt::mostrar(ui, &mut self.proj.sondagem, h) {
                        self.msg = m;
                    }
                }
                Etapa::Contencao => {
                    if ui::contencao::mostrar(ui, &mut self.proj, &res) {
                        self.etapa = Etapa::Resultados;
                    }
                }
                Etapa::Resultados => {
                    if let Some(m) = ui::resultados::mostrar(ui, &self.proj, &res) {
                        self.msg = m;
                    }
                }
                Etapa::Normas => ui::fatores::mostrar(ui, &mut self.proj.fatores),
            });
    }
}

fn item_nav(ui: &mut egui::Ui, ativo: bool, num: &str, nome: &str, desc: &str) -> bool {
    let p = tema::de(ui);
    let r = egui::Frame::new()
        .fill(if ativo { p.destaque_suave } else { egui::Color32::TRANSPARENT })
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (c, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
                ui.painter().circle_filled(c.center(), 13.0, if ativo { p.destaque } else { p.borda });
                ui.painter().text(c.center(), egui::Align2::CENTER_CENTER, num, tema::fonte_forte(13.0), egui::Color32::WHITE);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(nome).font(tema::fonte_forte(14.0)).color(if ativo { p.destaque } else { p.texto }));
                    ui.label(RichText::new(desc).size(11.5).color(p.fraco));
                });
            });
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    ui.add_space(2.0);
    r.clicked()
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.tela {
            Tela::Inicio => {
                let sessao = if self.tem_sessao { Some(self.proj.nome.clone()) } else { None };
                let estado = self.atualiz.as_ref().map(|v| v.estado());
                match ui::inicio::mostrar(ctx, sessao.as_deref(), estado.as_ref()) {
                    ui::inicio::Acao::Novo => self.novo_projeto(),
                    ui::inicio::Acao::Abrir => self.abrir(),
                    ui::inicio::Acao::Continuar => self.tela = Tela::Projeto,
                    ui::inicio::Acao::Manual => self.abrir_manual(),
                    ui::inicio::Acao::SalvarManual => self.salvar_manual(),
                    ui::inicio::Acao::Normas => self.central_normas = true,
                    ui::inicio::Acao::Nenhuma => {}
                }
            }
            Tela::Projeto => self.tela_projeto(ctx),
        }
        if self.central_normas {
            if let Some(m) = ui::normas_central::mostrar(ctx, &mut self.central_normas, &mut self.pasta_normas) {
                self.msg = m;
            }
        }
    }
}
