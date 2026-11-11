//! Bloco B — Tipo de contenção, geometria essencial, parâmetros específicos,
//! drenagem e pré-visualização em tempo real.

use super::tema;
use super::widgets::{cartao, chip, croqui_gravidade, grade, nota, num, titulo};
use crate::concreto::Concreto;
use crate::desenho;
use crate::drenagem::{Condicao, Drenagem, MaterialFiltro, TipoAreia, TipoBrita};
use crate::reaterro::Reaterro;
use crate::spt::{Sondagem, TIPOS};
use crate::empuxo::{ParamEmpuxo, Teoria};
use crate::fatores::Fatores;
use crate::projeto::{Projeto, TipoContencao, TIPOS_CONTENCAO};
use crate::resultado::{fmt, Resultado};
use crate::solo::{Cargas, CriterioAtrito, SoloFundacao};
use crate::tipos::atirantada::{AcoTirante, CortinaAtirantada, Diagrama};
use crate::tipos::embutida::{Apoio, EstacaPrancha, ParedeEmbutida, ParedeEstacas};
use crate::tipos::flexao::MuroFlexao;
use crate::tipos::grampeado::SoloGrampeado;
use crate::alvenaria::CLASSES_ARGAMASSA;
use std::sync::{Arc, Mutex};
use crate::tipos::gravidade::{Material, MuroGravidade, MATERIAIS};
use eframe::egui::{self, RichText, Stroke};

/// Retorna `true` se o usuário pediu para ver os resultados.
pub fn mostrar(ui: &mut egui::Ui, proj: &mut Projeto, res: &Resultado) -> bool {
    let mut ver_resultados = false;
    let p = tema::de(ui);

    ui.heading("Contenção e geometria");
    ui.add_space(4.0);
    // Seletor de tipo
    ui.horizontal_wrapped(|ui| {
        for t in TIPOS_CONTENCAO {
            let sel = proj.tipo == t;
            let resp = egui::Frame::new()
                .fill(if sel { p.destaque_suave } else { p.cartao })
                .stroke(Stroke::new(if sel { 2.0_f32 } else { 1.0 }, if sel { p.destaque } else { p.borda }))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(if ui.ctx().screen_rect().width() < 1250.0 { 150.0 } else { 178.0 });
                    ui.set_height(44.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(RichText::new(t.nome()).font(tema::fonte_forte(14.5)).color(if sel { p.destaque } else { p.texto }));
                        ui.add(egui::Label::new(RichText::new(t.descricao()).size(11.5).color(p.fraco)).wrap());
                    });
                })
                .response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                proj.tipo = t;
            }
        }
    });
    ui.add_space(8.0);

    // janela estreita: entradas e pré-visualização empilhadas, numa só rolagem
    if ui.available_width() < 820.0 {
        egui::ScrollArea::vertical().id_salt("rol_contencao_estreita").show(ui, |ui| {
            if entradas(ui, proj, res) {
                if let Some(m) = proj.pre_dimensionar() {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("predim_msg", proj.tipo.arquivo())), m));
                }
            }
            ui.add_space(10.0);
            cartao(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    titulo(ui, "Pré-visualização");
                    if res.atende() {
                        chip(ui, "ATENDE", p.ok);
                    } else {
                        chip(ui, "NÃO ATENDE", p.falha);
                    }
                    if ui.button("Ver resultados completos ➡").clicked() {
                        ver_resultados = true;
                    }
                });
                desenho::pintar(ui, &res.desenho, 320.0);
                ui.add_space(6.0);
                lista_verificacoes(ui, res);
            });
        });
        return ver_resultados;
    }

    let largura_entradas = 410.0f32.min(ui.available_width() * 0.42);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(largura_entradas, ui.available_height()), egui::Layout::top_down(egui::Align::Min), |ui| {
            egui::ScrollArea::vertical().id_salt("rol_entradas").show(ui, |ui| {
                if entradas(ui, proj, res) {
                    if let Some(m) = proj.pre_dimensionar() {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(("predim_msg", proj.tipo.arquivo())), m));
                    }
                }
            });
        });
        ui.add_space(12.0);
        ui.vertical(|ui| {
            cartao(ui, |ui| {
                ui.horizontal(|ui| {
                    titulo(ui, "Pré-visualização");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Ver resultados completos ➡").clicked() {
                            ver_resultados = true;
                        }
                        if res.atende() {
                            chip(ui, "ATENDE", p.ok);
                        } else {
                            chip(ui, "NÃO ATENDE", p.falha);
                        }
                    });
                });
                let alt = (ui.available_height() * 0.62).max(300.0);
                desenho::pintar(ui, &res.desenho, alt);
                ui.add_space(6.0);
                egui::ScrollArea::vertical().id_salt("rol_verif_prev").show(ui, |ui| {
                    lista_verificacoes(ui, res);
                });
            });
        });
    });
    ver_resultados
}

pub fn lista_verificacoes(ui: &mut egui::Ui, res: &Resultado) {
    let p = tema::de(ui);
    egui::Grid::new("verif_compacta").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
        for v in &res.verificacoes {
            ui.label(&v.nome);
            ui.label(RichText::new(v.expressao()).color(super::memorial::cor_status(v, &p)));
            super::memorial::etiqueta(ui, v);
            ui.end_row();
        }
    });
}

/// Campos de entrada. Retorna `true` se o pré-dimensionamento deve ser aplicado.
fn entradas(ui: &mut egui::Ui, proj: &mut Projeto, res: &Resultado) -> bool {
    let otimizando = acompanhar_otimizacao(ui.ctx(), proj);
    let tipo = proj.tipo;
    let mut pre_dim = false;
    let mut otimizar_pedido = false;
    let Projeto {
        geometria, empuxo, cargas, drenagem, fatores, gravidade, flexao, atirantada, prancha, estacas, grampeado, sondagem, reaterro, pre_dim_auto, ..
    } = proj;

    cartao(ui, |ui| {
        titulo(ui, "Geometria essencial");
        grade(ui, "geo", |ui| {
            if num(ui, tipo.rotulo_altura(), &mut geometria.h, "m", 0.5..=30.0, 0.05) && *pre_dim_auto {
                pre_dim = true;
            }
            num(ui, "Inclinação do talude de montante β", &mut geometria.beta, "°", 0.0..=45.0, 0.25);
            if geometria.beta > 0.0 {
                ui.label("");
                ui.label(RichText::new(format!("≈ 1 : {} (V:H)", fmt(1.0 / geometria.beta.to_radians().tan()))).small());
                ui.end_row();
            }
            num(ui, "Sobrecarga na crista q", &mut geometria.q, "kN/m²", 0.0..=500.0, 0.5);
            match tipo.rotulo_engaste() {
                Some(r) => {
                    num(ui, r, &mut geometria.engaste, "m", 0.0..=15.0, 0.05);
                }
                None => {
                    super::widgets::rotulo(ui, "Profundidade de engastamento");
                    let d = res.verificacoes.iter().find(|v| v.nome.starts_with("Ficha de projeto")).map(|v| v.valor);
                    ui.label(RichText::new(match d {
                        Some(d) if tipo != TipoContencao::Grampeado => format!("calculada: {} m", fmt(d)),
                        _ => "não se aplica".into(),
                    }).color(tema::de(ui).destaque));
                    ui.end_row();
                }
            }
        });
        if let Some(regra) = tipo.regra_predim() {
            ui.horizontal_wrapped(|ui| {
                super::widgets::caixa(ui, pre_dim_auto, "Pré-dimensionar ao alterar H");
                if ui.small_button("↻ Aplicar pré-dimensionamento").on_hover_text(regra).clicked() {
                    pre_dim = true;
                }
            });
            nota(ui, regra);
        }
        let chave_msg = egui::Id::new(("predim_msg", tipo.arquivo()));
        if let Some(m) = ui.ctx().data(|d| d.get_temp::<String>(chave_msg)) {
            let p = tema::de(ui);
            let nao_atende = m.contains("Ainda não atende") || m.contains("nenhuma das") || m.contains("descartado");
            let cor = if nao_atende { p.alerta } else { p.ok };
            ui.label(RichText::new(m).small().color(cor));
        }
        ui.horizontal_wrapped(|ui| {
            if otimizando {
                ui.spinner();
                ui.label(RichText::new("Otimizando: avaliando as alternativas…").small());
            } else {
                let rotulo = if res.atende() { "⚙ Otimizar custo" } else { "⚙ Otimizar: solução econômica que atende" };
                if ui
                    .small_button(rotulo)
                    .on_hover_text(
                        "Avalia as alternativas usuais deste tipo (base, dente, passivo, embutimento, níveis de tirantes, malha e barras dos grampos, \
                         apoio, ficha, perfil do catálogo VPA…), ajusta espessuras e placas e aplica a mais econômica que aprova TODAS as verificações, \
                         inclusive a estabilidade global",
                    )
                    .clicked()
                {
                    otimizar_pedido = true;
                }
            }
        });
        if geometria.q + 1e-9 < fatores.sobrecarga_minima {
            ui.label(RichText::new(format!("⚠ q abaixo do mínimo de {} kPa (NBR 11682 7.3.3)", fmt(fatores.sobrecarga_minima))).small().color(tema::de(ui).alerta));
        }
        ui.add_space(4.0);
        teoria(ui, empuxo);
        if tipo == TipoContencao::Grampeado {
            let (g, ph, c) = sondagem.medias(geometria.h);
            nota(ui, &format!("Solo (médias da sondagem até H): γ = {} ; φ' = {}° ; c' = {} kPa", fmt(g), fmt(ph), fmt(c)));
        } else {
            nota(ui, &match sondagem.modo {
                crate::spt::ModoSolo::Completo => {
                    format!("Solo contido: sondagem {} ({} camadas).", sondagem.identificacao, sondagem.camadas.len())
                }
                crate::spt::ModoSolo::Simplificado => format!(
                    "Solo: entrada simplificada — {} com NSPT = {} (homogêneo).",
                    sondagem.simples_tipo.nome(),
                    fmt(sondagem.simples_nspt)
                ),
            });
        }
    });
    ui.add_space(10.0);

    cartao(ui, |ui| {
        titulo(ui, &format!("Parâmetros — {}", tipo.nome()));
        match tipo {
            TipoContencao::Gravidade => det_gravidade(ui, gravidade, geometria.h + geometria.engaste.max(0.0) - gravidade.f),
            TipoContencao::Flexao => det_flexao(ui, flexao),
            TipoContencao::Atirantada => det_atirantada(ui, atirantada, fatores, geometria.h, empuxo),
            TipoContencao::Prancha => det_prancha(ui, prancha, empuxo),
            TipoContencao::Estacas => det_estacas(ui, estacas, empuxo),
            TipoContencao::Grampeado => det_grampeado(ui, grampeado),
        }
    });
    ui.add_space(10.0);

    if matches!(tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
        cartao(ui, |ui| {
            titulo(ui, "Reaterro do tardoz e solo de apoio");
            let z_base = geometria.h + geometria.engaste.max(0.0);
            ui_reaterro(ui, reaterro, sondagem, z_base);
        });
        ui.add_space(10.0);
    }

    cartao(ui, |ui| {
        titulo(ui, "Drenagem");
        ui_drenagem(ui, drenagem, tipo);
    });
    ui.add_space(10.0);

    cartao(ui, |ui| {
        titulo(ui, "Cargas adicionais");
        ui_cargas(ui, cargas);
    });
    ui.add_space(20.0);
    if otimizar_pedido {
        // roda em segundo plano: a janela continua respondendo durante a avaliação
        let base = proj.clone();
        let base_json = serde_json::to_string(&base).unwrap_or_default();
        let estado: Arc<Mutex<EstadoOtim>> = Arc::new(Mutex::new(EstadoOtim::Rodando));
        let e2 = estado.clone();
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let custo_base = crate::otimizar::custo(&base);
            let o = crate::otimizar::otimizar(&base);
            if let Ok(mut e) = e2.lock() {
                *e = EstadoOtim::Pronto(Box::new((custo_base, o)));
            }
            ctx.request_repaint();
        });
        ui.ctx().data_mut(|d| d.insert_temp(id_otim(), TarefaOtim { estado, base_json, tipo }));
    }
    pre_dim
}

/// Estado da otimização em segundo plano.
enum EstadoOtim {
    Rodando,
    Pronto(Box<(f64, crate::otimizar::Otimizacao)>),
}

#[derive(Clone)]
struct TarefaOtim {
    estado: Arc<Mutex<EstadoOtim>>,
    /// Projeto no momento do pedido: o resultado só é aplicado se nada mudou desde então.
    base_json: String,
    tipo: TipoContencao,
}

fn id_otim() -> egui::Id {
    egui::Id::new("otimizacao_em_andamento")
}

/// Verifica a otimização em segundo plano; aplica o resultado quando pronto.
/// Retorna `true` enquanto ainda estiver rodando.
fn acompanhar_otimizacao(ctx: &egui::Context, proj: &mut Projeto) -> bool {
    let Some(t) = ctx.data(|d| d.get_temp::<TarefaOtim>(id_otim())) else { return false };
    let pronto = match t.estado.lock() {
        Ok(mut e) => match std::mem::replace(&mut *e, EstadoOtim::Rodando) {
            EstadoOtim::Pronto(r) => Some(r),
            EstadoOtim::Rodando => None,
        },
        Err(_) => None,
    };
    let Some(r) = pronto else {
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
        return true;
    };
    ctx.data_mut(|d| d.remove::<TarefaOtim>(id_otim()));
    let (custo_base, o) = *r;
    let mut msg = o.mensagem(custo_base);
    if serde_json::to_string(&*proj).unwrap_or_default() != t.base_json {
        msg = "Otimização descartada: o projeto foi alterado durante o cálculo. Clique em Otimizar novamente.".into();
    } else if let Some(m) = o.melhor {
        *proj = m.projeto;
        proj.nota_otimizacao = msg.clone();
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("predim_msg", t.tipo.arquivo())), msg));
    false
}

fn teoria(ui: &mut egui::Ui, e: &mut ParamEmpuxo) {
    ui.horizontal(|ui| {
        super::widgets::rotulo(ui, "Teoria de empuxo:");
        ui.selectable_value(&mut e.teoria, Teoria::Rankine, " Rankine ");
        ui.selectable_value(&mut e.teoria, Teoria::Coulomb, " Coulomb ");
    });
    if e.teoria == Teoria::Coulomb {
        grade(ui, "delta", |ui| {
            num(ui, "Atrito solo-estrutura δ", &mut e.delta, "°", 0.0..=45.0, 0.25);
        });
    }
}

fn fundacao(ui: &mut egui::Ui, f: &mut SoloFundacao, onde: &str) {
    egui::CollapsingHeader::new("Contato base-solo de fundação").id_salt(onde).show(ui, |ui| {
        nota(ui, &format!("γ, φ' e c' da fundação: camada da sondagem {onde}."));
        ui.horizontal(|ui| {
            super::widgets::rotulo(ui, "Atrito na base:");
            ui.selectable_value(&mut f.criterio_atrito, CriterioAtrito::AnguloReduzido, " tan(k·φ') ");
            ui.selectable_value(&mut f.criterio_atrito, CriterioAtrito::TangenteReduzida, " k·tg φ' (Marchetti) ");
        });
        grade(ui, &format!("fund_{onde}"), |ui| {
            let rot = if f.criterio_atrito == CriterioAtrito::AnguloReduzido { "k = δ base / φ'" } else { "k (f = k·tg φ')" };
            num(ui, rot, &mut f.frac_delta_base, "", 0.0..=1.0, 0.01);
            num(ui, "Adesão / c'", &mut f.frac_adesao, "", 0.0..=1.0, 0.01);
        });
        ui.horizontal_wrapped(|ui| {
            super::widgets::rotulo(ui, "Predefinições:");
            if ui.small_button("⅔φ (usual)").clicked() {
                f.criterio_atrito = CriterioAtrito::AnguloReduzido;
                f.frac_delta_base = 2.0 / 3.0;
            }
            if ui.small_button("0,67·tg φ").clicked() {
                f.criterio_atrito = CriterioAtrito::TangenteReduzida;
                f.frac_delta_base = 0.67;
            }
            if ui.small_button("0,9·tg φ (base compacta)").clicked() {
                f.criterio_atrito = CriterioAtrito::TangenteReduzida;
                f.frac_delta_base = 0.9;
            }
        });
        nota(ui, &format!(
            "Coeficiente de atrito resultante: {}. Marchetti 3.1: f = 0,67·tg φ a tg φ ; c' = 0,5c a 0,67c ; recomenda Ep = 0 (solo à frente pode ser erodido).",
            crate::resultado::fmt(f.coef_atrito())
        ));
    }).header_response.on_hover_ui(|ui| { super::widgets::dica_ui(ui, "Contato base-solo de fundação"); });
}

fn concreto(ui: &mut egui::Ui, id: &str, c: &mut Concreto) {
    egui::CollapsingHeader::new("Concreto e aço (NBR 6118:2023)").id_salt(id).show(ui, |ui| {
        grade(ui, id, |ui| {
            num(ui, "fck", &mut c.fck, "MPa", 20.0..=50.0, 1.0);
            num(ui, "fyk", &mut c.fyk, "MPa", 250.0..=600.0, 5.0);
            let mut cob = c.cobrimento * 100.0;
            num(ui, "Cobrimento", &mut cob, "cm", 1.0..=10.0, 0.1);
            c.cobrimento = cob / 100.0;
            super::widgets::rotulo(ui, "Peso específico");
            ui.label(RichText::new(format!("{} kN/m³  🔒", fmt(crate::materiais::CONCRETO_ARMADO))).strong());
            ui.end_row();
        });
        nota(ui, "Concreto armado: 25 kN/m³ — NBR 6120:2019, Tab. 1 (valor fixo).");
    }).header_response.on_hover_ui(|ui| { super::widgets::dica_ui(ui, "Concreto e aço (NBR 6118:2023)"); });
}

fn passivo_subpressao(ui: &mut egui::Ui, passivo: &mut bool, frac: &mut f64, sub: &mut bool) {
    super::widgets::caixa(ui, passivo, "Considerar empuxo passivo frontal");
    if *passivo {
        grade(ui, "frac_pas", |ui| {
            num(ui, "   Fração mobilizada (≤ 1/redução)", frac, "", 0.0..=1.0, 0.01);
        });
    }
    super::widgets::caixa(ui, sub, "Considerar subpressão na base");
}

/// Dente de base (chave de cisalhamento).
fn dente_ui(ui: &mut egui::Ui, dente: &mut bool, h: &mut f64, b: &mut f64) {
    super::widgets::caixa(ui, dente, "Dente de base (chave de cisalhamento)");
    if *dente {
        grade(ui, "dente", |ui| {
            num(ui, "   Profundidade do dente", h, "m", 0.10..=2.0, 0.01);
            num(ui, "   Largura do dente", b, "m", 0.15..=1.5, 0.01);
        });
    }
}

/// Banco de alvenarias: tensões admissíveis automáticas por material.
fn alvenaria_ui(ui: &mut egui::Ui, m: &mut MuroGravidade) {
    let p = tema::de(ui);
    egui::CollapsingHeader::new("Alvenaria — tensões admissíveis").id_salt("alv").default_open(true).show(ui, |ui| {
        match m.material {
            Material::ConcretoSimples | Material::ConcretoCiclopico => {
                grade(ui, "alv_c", |ui| {
                    num(ui, "fck do concreto (C15 a C40)", &mut m.fck_alvenaria, "MPa", 15.0..=40.0, 0.5);
                });
            }
            Material::PedraArgamassada => {
                ui.horizontal(|ui| {
                    super::widgets::rotulo(ui, "Argamassa (resist. média):");
                    egui::ComboBox::from_id_salt("argamassa").selected_text(m.argamassa.nome()).show_ui(ui, |ui| {
                        for a in CLASSES_ARGAMASSA {
                            ui.selectable_value(&mut m.argamassa, a, a.nome());
                        }
                    });
                });
            }
            Material::Gabiao | Material::PedraSeca => {}
        }
        super::widgets::caixa(ui, &mut m.personalizar_alvenaria, "Informar valores do projetista");
        if m.personalizar_alvenaria {
            grade(ui, "alv_p", |ui| {
                num(ui, "σ admissível (compressão)", &mut m.sigma_adm, "MPa", 0.05..=20.0, 0.05);
                num(ui, "Tração admissível nas juntas", &mut m.tracao_adm, "MPa", 0.0..=2.0, 0.01);
                num(ui, "Cisalhamento admissível", &mut m.tau_adm, "MPa", 0.0..=2.0, 0.01);
            });
        }
        let pr = m.alvenaria();
        egui::Grid::new("alv_res").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
            let linha = |ui: &mut egui::Ui, n: &str, v: String| {
                ui.label(n);
                ui.label(RichText::new(v).font(tema::fonte_forte(14.5)).color(p.destaque));
                ui.end_row();
            };
            linha(ui, "σadm (compressão)", format!("{} MPa", fmt(pr.sigma_adm)));
            linha(ui, "σt,adm (tração nas juntas)", format!("{} MPa", fmt(pr.tracao_adm)));
            linha(
                ui,
                "τadm (cisalhamento)",
                format!(
                    "{} + {}·σ{} MPa",
                    format!("{:.3}", pr.c0).replace('.', ","),
                    format!("{:.3}", pr.mu).replace('.', ","),
                    if pr.tau_max.is_finite() { format!(" ≤ {}", fmt(pr.tau_max)) } else { String::new() }
                ),
            );
        });
        nota(ui, &pr.origem);
    }).header_response.on_hover_ui(|ui| { super::widgets::dica_ui(ui, "Alvenaria — tensões admissíveis"); });
}

fn det_gravidade(ui: &mut egui::Ui, m: &mut MuroGravidade, h: f64) {
    ui.horizontal(|ui| {
        croqui_gravidade(ui);
        ui.vertical(|ui| {
            super::widgets::rotulo(ui, "Material:");
            egui::ComboBox::from_id_salt("material").selected_text(m.material.nome()).width(170.0).show_ui(ui, |ui| {
                for mat in MATERIAIS {
                    ui.selectable_value(&mut m.material, mat, format!("{}  —  {} kN/m³", mat.nome(), fmt(mat.peso())));
                }
            });
            ui.label(RichText::new(format!("γ = {} kN/m³  🔒", fmt(m.material.peso()))).font(tema::fonte_forte(15.0)).color(tema::de(ui).destaque));
            ui.add_space(4.0);
            let base = m.ressalto_frente + m.proj_frente + m.crista + m.proj_tardoz + m.ressalto_tras;
            ui.label(RichText::new(format!("Base = {} m", fmt(base))).strong());
            ui.label(RichText::new(format!("Altura total = {} m", fmt(h + m.f))).strong());
            ui.label(RichText::new(format!("Base / altura = {}", fmt(base / (h + m.f)))).small());
        });
    });
    grade(ui, "grav", |ui| {
        num(ui, "A — ressalto frontal", &mut m.ressalto_frente, "m", 0.0..=3.0, 0.01);
        num(ui, "B — projeção do paramento frontal", &mut m.proj_frente, "m", 0.0..=10.0, 0.01);
        num(ui, "C — largura da crista", &mut m.crista, "m", 0.2..=5.0, 0.01);
        num(ui, "D — projeção do tardoz", &mut m.proj_tardoz, "m", -2.0..=5.0, 0.01);
        num(ui, "E — ressalto posterior", &mut m.ressalto_tras, "m", 0.0..=3.0, 0.01);
        num(ui, "F — altura da base", &mut m.f, "m", 0.0..=3.0, 0.01);
    });
    nota(ui, &format!("γ fixo: {}.", m.material.origem()));
    alvenaria_ui(ui, m);
    passivo_subpressao(ui, &mut m.considerar_passivo, &mut m.frac_passivo, &mut m.subpressao);
    dente_ui(ui, &mut m.dente, &mut m.dente_h, &mut m.dente_b);
    egui::CollapsingHeader::new("Detalhamento construtivo (juntas e coroamento)").id_salt("grav_det").show(ui, |ui| {
        grade(ui, "grav_det_g", |ui| {
            num(ui, "Espaçamento das juntas de dilatação", &mut m.junta_espac, "m", 3.0..=30.0, 0.5);
        });
        super::widgets::caixa(ui, &mut m.coroamento, "Viga de coroamento em concreto armado");
        if m.coroamento {
            grade(ui, "grav_det_c", |ui| {
                num(ui, "Altura da viga de coroamento", &mut m.coroamento_h, "m", 0.10..=0.80, 0.01);
            });
        }
    }).header_response.on_hover_ui(|ui| { super::widgets::dica_ui(ui, "Detalhamento construtivo (juntas e coroamento)"); });
    fundacao(ui, &mut m.fundacao, "na cota da base (z = H + F)");
}

fn det_flexao(ui: &mut egui::Ui, m: &mut MuroFlexao) {
    use crate::tipos::misto::{Sistema, BLOCOS, FBK};
    ui.horizontal_wrapped(|ui| {
        super::widgets::rotulo(ui, "Sistema estrutural");
        egui::ComboBox::from_id_salt("sistema_flexao").selected_text(m.sistema.nome()).width(300.0).show_ui(ui, |ui| {
            for s in [Sistema::ConcretoArmado, Sistema::Misto] {
                ui.selectable_value(&mut m.sistema, s, s.nome());
            }
        })
        .response
        .on_hover_text(super::dicas::dica("Sistema estrutural").unwrap_or_default());
    });
    if m.sistema == Sistema::Misto {
        let mi = &mut m.misto;
        grade(ui, "flex_misto", |ui| {
            super::widgets::rotulo(ui, "Bloco estrutural");
            let atual = BLOCOS.iter().find(|b| (b.0 - mi.bloco).abs() < 1e-6).map(|b| b.1).unwrap_or("—");
            egui::ComboBox::from_id_salt("bloco_misto").selected_text(atual).width(240.0).show_ui(ui, |ui| {
                for (t, nome) in BLOCOS {
                    ui.selectable_value(&mut mi.bloco, t, nome);
                }
            });
            ui.end_row();
            super::widgets::rotulo(ui, "Resistência do bloco fbk");
            egui::ComboBox::from_id_salt("fbk_misto").selected_text(format!("{} MPa", fmt(mi.fbk))).width(120.0).show_ui(ui, |ui| {
                for f in FBK {
                    ui.selectable_value(&mut mi.fbk, f, format!("{} MPa", fmt(f)));
                }
            });
            ui.end_row();
            num(ui, "Espaçamento entre pilares (eixos)", &mut mi.espac_pilar, "m", 1.5..=6.0, 0.05);
            num(ui, "Largura do pilar (ao longo do muro)", &mut mi.pilar_b, "m", 0.14..=0.80, 0.01);
            num(ui, "Altura da seção do pilar (para o aterro)", &mut mi.pilar_h, "m", 0.14..=1.20, 0.01);
            num(ui, "Canaletas armadas a cada", &mut mi.esp_canaleta, "m", 0.20..=1.20, 0.20);
        });
        ui.label(RichText::new("Na estabilidade, a parede entra com espessura e peso equivalentes (blocos + pilares distribuídos).").small().color(tema::de(ui).fraco));
    }
    grade(ui, "flex", |ui| {
        if m.sistema == Sistema::ConcretoArmado {
            num(ui, "Espessura da parede (topo)", &mut m.t_topo, "m", 0.12..=1.5, 0.01);
            num(ui, "Espessura da parede (base)", &mut m.t_base, "m", 0.12..=2.0, 0.01);
        }
        num(ui, "Espessura da sapata", &mut m.h_sapata, "m", 0.2..=2.0, 0.01);
        num(ui, "Comprimento da ponta", &mut m.b_ponta, "m", 0.0..=6.0, 0.01);
        num(ui, "Comprimento do talão", &mut m.b_talao, "m", 0.0..=10.0, 0.01);
    });
    passivo_subpressao(ui, &mut m.considerar_passivo, &mut m.frac_passivo, &mut m.subpressao);
    dente_ui(ui, &mut m.dente, &mut m.dente_h, &mut m.dente_b);
    fundacao(ui, &mut m.fundacao, "na cota da base (z = H)");
    concreto(ui, "flex_c", &mut m.concreto);
}

pub fn tirante_tipo(ui: &mut egui::Ui, f: &mut Fatores) {
    use crate::fatores::Fixacao;
    ui.horizontal(|ui| {
        super::widgets::rotulo(ui, "Tirante:");
        ui.radio_value(&mut f.tirante_permanente, true, "Permanente");
        ui.radio_value(&mut f.tirante_permanente, false, "Provisório (< 2 anos)");
    });
    ui.horizontal(|ui| {
        super::widgets::rotulo(ui, "Fixação da cabeça:");
        ui.radio_value(&mut f.tirante_fixacao, Fixacao::Rosca, "Rosca");
        ui.radio_value(&mut f.tirante_fixacao, Fixacao::Clavete, "Clavetes");
    });
}

fn det_atirantada(ui: &mut egui::Ui, c: &mut CortinaAtirantada, f: &mut Fatores, h: f64, emp: &ParamEmpuxo) {
    grade(ui, "cort", |ui| {
        num(ui, "Espessura da cortina", &mut c.espessura, "m", 0.12..=1.0, 0.01);
        num(ui, "Placa de ancoragem (lado)", &mut c.placa, "m", 0.1..=0.8, 0.01);
        num(ui, "Altura da viga de coroamento", &mut c.viga_h, "m", 0.3..=1.5, 0.01);
        if emp.teoria == Teoria::Coulomb {
            num(ui, "δ no lado passivo", &mut c.delta_passivo, "°", 0.0..=30.0, 0.25);
        }
    });
    ui.horizontal(|ui| {
        super::widgets::rotulo(ui, "Diagrama:");
        ui.radio_value(&mut c.diagrama, Diagrama::Aparente, "Aparente (Terzaghi-Peck)");
        ui.radio_value(&mut c.diagrama, Diagrama::Teorico, "Teórico");
    });
    ui.separator();
    ui.label(RichText::new("Tirantes (NBR 5629)").strong());
    tirante_tipo(ui, f);
    let mut remover = None;
    for (i, z) in c.niveis.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("Nível {} a", i + 1));
            ui.add(egui::DragValue::new(z).speed(0.05).range(0.1..=30.0).suffix(" m"));
            if *z >= h {
                ui.label(RichText::new("abaixo de H — ignorado").small().color(tema::de(ui).alerta));
            }
            if ui.small_button("✖").clicked() {
                remover = Some(i);
            }
        });
    }
    if let Some(i) = remover {
        c.niveis.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.button("➕ Nível").clicked() {
            let z = c.niveis.last().map(|z| z + 2.5).unwrap_or(1.5);
            c.niveis.push(z.min(h - 0.5));
        }
        if ui.button("Distribuir uniformemente").on_hover_text("Reparte os níveis atuais ao longo de H").clicked() {
            let n = c.niveis.len().max(1);
            let passo = h / (n as f64 + 0.5);
            c.niveis = (0..n).map(|i| ((i as f64 + 0.6) * passo * 100.0).round() / 100.0).collect();
        }
    });
    grade(ui, "tir", |ui| {
        num(ui, "Espaçamento horizontal", &mut c.espac_h, "m", 0.5..=6.0, 0.05);
        num(ui, "Inclinação", &mut c.inclinacao, "°", 0.0..=45.0, 0.5);
        let mut d = c.diam_furo * 100.0;
        num(ui, "Diâmetro do furo", &mut d, "cm", 5.0..=40.0, 0.5);
        c.diam_furo = d / 100.0;
        num(ui, "qs (atrito lateral do bulbo)", &mut c.qs, "kPa", 10.0..=1000.0, 1.0);
    });
    super::widgets::caixa(ui, &mut c.auto_dim, "Dimensionar o aço de cada tirante pela tração (NBR 5629)");
    if c.auto_dim {
        ui.horizontal(|ui| {
            super::widgets::rotulo(ui, "   Tipo de aço:");
            egui::ComboBox::from_id_salt("aco_tir").selected_text(c.tipo_aco.nome()).show_ui(ui, |ui| {
                for t in [AcoTirante::Barra, AcoTirante::Cordoalha] {
                    ui.selectable_value(&mut c.tipo_aco, t, t.nome());
                }
            });
        });
        nota(ui, "A menor opção comercial com 0,9·fyk·As/FS ≥ Ft é adotada em cada nível (ver o memorial).");
    } else {
        grade(ui, "tir_aco", |ui| {
            num(ui, "fyk do aço do tirante", &mut c.fyk_tirante, "MPa", 250.0..=1900.0, 5.0);
            num(ui, "Área de aço do tirante", &mut c.area_aco, "cm²", 0.5..=50.0, 0.05);
        });
    }
    concreto(ui, "cort_c", &mut c.concreto);
}

fn det_parede(ui: &mut egui::Ui, p: &mut ParedeEmbutida, emp: &ParamEmpuxo) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut p.apoio, Apoio::Balanco, " Em balanço ");
        ui.selectable_value(&mut p.apoio, Apoio::Ancorada, " Com uma linha de tirante/estronca ");
    });
    grade(ui, "par", |ui| {
        if p.apoio == Apoio::Ancorada {
            num(ui, "Profundidade do apoio", &mut p.prof_apoio, "m", 0.1..=20.0, 0.05);
            num(ui, "Inclinação do tirante", &mut p.incl_apoio, "°", 0.0..=45.0, 0.5);
            num(ui, "Espaçamento horizontal", &mut p.espac_apoio, "m", 0.5..=10.0, 0.05);
        }
        if emp.teoria == Teoria::Coulomb {
            num(ui, "δ no lado passivo", &mut p.delta_passivo, "°", 0.0..=30.0, 0.25);
        }
        num(ui, "Acréscimo construtivo de ficha", &mut p.ficha_adicional, "m", 0.0..=15.0, 0.05);
    });
    ui.horizontal(|ui| {
        super::widgets::caixa(ui, &mut p.usar_na_frente, "NA no lado escavado a");
        ui.add_enabled(p.usar_na_frente, egui::DragValue::new(&mut p.prof_na_frente).speed(0.05).range(0.0..=60.0).suffix(" m"));
    });
}

fn det_prancha(ui: &mut egui::Ui, e: &mut EstacaPrancha, emp: &ParamEmpuxo) {
    det_parede(ui, &mut e.parede, emp);
    ui.separator();
    use crate::perfis_prancha::{self as cat, Catalogo};
    let atual = e.catalogo();
    let texto = match atual {
        Some(Catalogo::Aco(p)) => format!("{} — W {} cm³/m, {} kg/m²", p.nome, fmt(p.w), fmt(p.peso_parede)),
        Some(Catalogo::Pvc(p)) => format!("{} (PVC) — Madm {} kN·m/m", p.nome, fmt(p.m_adm)),
        None => "Personalizado".into(),
    };
    ui.horizontal(|ui| {
        super::widgets::rotulo(ui, "Perfil (catálogo VPA):");
        egui::ComboBox::from_id_salt("perfil_prancha").width(260.0).selected_text(texto).height(420.0).show_ui(ui, |ui| {
            ui.label(RichText::new("Aço — perfis U").strong());
            for p in cat::ACO.iter().filter(|p| p.forma == cat::Forma::U) {
                if ui.selectable_label(e.nome_perfil == p.nome, format!("{}   W {} cm³/m · {} kg/m²", p.nome, fmt(p.w), fmt(p.peso_parede))).clicked() {
                    e.nome_perfil = p.nome.into();
                    e.w_perfil = p.w;
                }
            }
            ui.separator();
            ui.label(RichText::new("Aço — perfis Z").strong());
            for p in cat::ACO.iter().filter(|p| p.forma == cat::Forma::Z) {
                if ui.selectable_label(e.nome_perfil == p.nome, format!("{}   W {} cm³/m · {} kg/m²", p.nome, fmt(p.w), fmt(p.peso_parede))).clicked() {
                    e.nome_perfil = p.nome.into();
                    e.w_perfil = p.w;
                }
            }
            ui.separator();
            ui.label(RichText::new("Aço — perfis W (pequenas contenções)").strong());
            for p in cat::ACO.iter().filter(|p| p.forma == cat::Forma::W) {
                if ui.selectable_label(e.nome_perfil == p.nome, format!("{}   W {} cm³/m · {} kg/m²", p.nome, fmt(p.w), fmt(p.peso_parede))).clicked() {
                    e.nome_perfil = p.nome.into();
                    e.w_perfil = p.w;
                }
            }
            ui.separator();
            ui.label(RichText::new("PVC VANIL®").strong());
            for p in cat::PVC.iter() {
                if ui.selectable_label(e.nome_perfil == p.nome, format!("{}   Madm {} kN·m/m · W {} cm³/m", p.nome, fmt(p.m_adm), fmt(p.w))).clicked() {
                    e.nome_perfil = p.nome.into();
                    e.w_perfil = p.w;
                }
            }
            ui.separator();
            if ui.selectable_label(atual.is_none(), "Personalizado (informar W)").clicked() && atual.is_some() {
                e.nome_perfil = "Perfil personalizado".into();
            }
        });
    });
    match atual {
        Some(Catalogo::Aco(p)) => nota(
            ui,
            &format!(
                "{} {}: b = {} mm · h = {} mm · t = {} mm · I = {} cm⁴/m · {} kg/m por estaca.",
                p.nome, p.forma.nome(), fmt(p.b), fmt(p.h), fmt(p.t), fmt(p.inercia), fmt(p.peso_estaca)
            ),
        ),
        Some(Catalogo::Pvc(p)) => nota(
            ui,
            &format!(
                "{} {}: b = {} mm · h = {} mm · t = {} mm · I = {} cm⁴/m. Verificado pelo momento admissível do fabricante.",
                p.nome, p.forma.nome(), fmt(p.b), fmt(p.h), fmt(p.t), fmt(p.inercia)
            ),
        ),
        None => {
            ui.horizontal(|ui| {
                super::widgets::rotulo(ui, "Nome:");
                ui.add(egui::TextEdit::singleline(&mut e.nome_perfil).desired_width(180.0));
            });
        }
    }
    let pvc = matches!(atual, Some(Catalogo::Pvc(_)));
    grade(ui, "pran", |ui| {
        if atual.is_none() {
            num(ui, "Módulo resistente W", &mut e.w_perfil, "cm³/m", 50.0..=10000.0, 5.0);
        }
        num(ui, "Altura da viga de travamento", &mut e.viga_h, "m", 0.3..=1.5, 0.01);
        if !pvc {
            super::widgets::rotulo(ui, "Aço do perfil");
            let nome_aco = cat::ACOS.iter().find(|a| (a.1 - e.fy).abs() < 0.01).map(|a| a.0.to_string()).unwrap_or_else(|| format!("Outro (fy {} MPa)", fmt(e.fy)));
            egui::ComboBox::from_id_salt("aco_prancha").width(230.0).selected_text(nome_aco).show_ui(ui, |ui| {
                for (nome, fy) in cat::ACOS {
                    ui.selectable_value(&mut e.fy, fy, nome);
                }
            });
            ui.end_row();
            num(ui, "fy do aço", &mut e.fy, "MPa", 200.0..=500.0, 5.0);
        }
    });
    if !pvc {
        nota(ui, "O catálogo VPA não informa a classe do aço: confirme o fy com o fornecedor.");
    }
}

fn det_estacas(ui: &mut egui::Ui, e: &mut ParedeEstacas, emp: &ParamEmpuxo) {
    det_parede(ui, &mut e.parede, emp);
    ui.separator();
    grade(ui, "est", |ui| {
        num(ui, "Diâmetro", &mut e.diametro, "m", 0.2..=2.0, 0.01);
        num(ui, "Espaçamento (eixo a eixo)", &mut e.espacamento, "m", 0.15..=5.0, 0.01);
        num(ui, "Altura da viga de travamento", &mut e.viga_h, "m", 0.3..=1.5, 0.01);
        super::widgets::rotulo(ui, "Nº de barras longitudinais");
        ui.add(egui::DragValue::new(&mut e.n_barras).range(6..=40));
        ui.end_row();
    });
    concreto(ui, "est_c", &mut e.concreto);
}

fn det_grampeado(ui: &mut egui::Ui, g: &mut SoloGrampeado) {
    super::widgets::caixa(ui, &mut g.auto_dim, "Dimensionar barra e comprimento pela tração (NBR 16920-2)");
    if g.auto_dim {
        nota(ui, "L e Ø da barra são calculados (tração por grampo, arrancamento e cunha); os campos abaixo ficam bloqueados.");
    }
    grade(ui, "gram", |ui| {
        num(ui, "Inclinação da face (c/ vertical)", &mut g.incl_face, "°", 0.0..=45.0, 0.5);
        num(ui, "Espaçamento horizontal Sh", &mut g.sh, "m", 0.5..=3.0, 0.05);
        num(ui, "Espaçamento vertical Sv", &mut g.sv, "m", 0.5..=3.0, 0.05);
        num(ui, "Profundidade do 1º grampo", &mut g.z1, "m", 0.2..=3.0, 0.05);
        if g.auto_dim {
            super::widgets::rotulo(ui, "Comprimento L");
            ui.label(RichText::new("calculado pela tração").weak());
            ui.end_row();
        } else {
            num(ui, "Comprimento L", &mut g.comprimento, "m", 1.0..=30.0, 0.1);
        }
        num(ui, "Inclinação (abaixo da horizontal)", &mut g.incl_grampo, "°", 0.0..=45.0, 0.5);
        let mut d = g.diam_furo * 100.0;
        num(ui, "Diâmetro do furo", &mut d, "cm", 5.0..=30.0, 0.5);
        g.diam_furo = d / 100.0;
        num(ui, "qs (arrancamento)", &mut g.qs, "kPa", 10.0..=500.0, 1.0);
        if g.auto_dim {
            super::widgets::rotulo(ui, "Diâmetro da barra");
            ui.label(RichText::new("calculado pela tração").weak());
            ui.end_row();
        } else {
            num(ui, "Diâmetro da barra", &mut g.diam_barra, "mm", 10.0..=50.0, 0.5);
        }
        num(ui, "fyk da barra", &mut g.fyk_barra, "MPa", 250.0..=1000.0, 5.0);
        let mut e = g.espessura_face * 100.0;
        num(ui, "Espessura da face", &mut e, "cm", 5.0..=40.0, 0.5);
        g.espessura_face = e / 100.0;
        num(ui, "Placa da cabeça (lado)", &mut g.placa, "m", 0.1..=0.5, 0.01);
    });
    concreto(ui, "gram_c", &mut g.concreto);
}

fn ui_reaterro(ui: &mut egui::Ui, r: &mut Reaterro, s: &Sondagem, z_base: f64) {
    super::widgets::caixa(ui, &mut r.usar, "Especificar o material do reaterro compactado");
    if r.usar {
        ui.horizontal(|ui| {
            super::widgets::rotulo(ui, "   Tipo de solo:");
            egui::ComboBox::from_id_salt("rea_tipo").selected_text(r.tipo.nome()).show_ui(ui, |ui| {
                for t in TIPOS {
                    ui.selectable_value(&mut r.tipo, t, t.nome());
                }
            });
        });
        grade(ui, "rea", |ui| {
            num(ui, "   NSPT equivalente do reaterro", &mut r.nspt, "golpes", 1.0..=40.0, 0.5);
            num(ui, "   Grau de compactação mínimo", &mut r.gc_min, "% PN", 85.0..=105.0, 0.5);
            num(ui, "   Espessura das camadas", &mut r.camada_cm, "cm", 10.0..=40.0, 1.0);
            num(ui, "   Faixa de compactação leve junto à estrutura", &mut r.faixa_leve_m, "m", 0.0..=3.0, 0.1);
        });
        let (g, _, c, phi) = r.parametros(s);
        nota(ui, &format!("Tardoz: γ = {} kN/m³ ; φ' = {}° ; c' = {} kPa (correlações com o SPT, como na sondagem)", fmt(g), fmt(phi), fmt(c)));
    } else {
        nota(ui, "Sem especificação: o tardoz usa o perfil da sondagem.");
    }
    if let Some(c) = s.camada_em(z_base + 0.01) {
        nota(
            ui,
            &format!(
                "Solo de apoio da base (z = {} m): {} — NSPT = {} ; γ = {} ; φ' = {}° ; c' = {} kPa",
                fmt(z_base),
                c.tipo.nome().to_lowercase(),
                fmt(c.nspt),
                fmt(c.gamma),
                fmt(c.phi),
                fmt(c.c)
            ),
        );
    }
}

fn ui_drenagem(ui: &mut egui::Ui, d: &mut Drenagem, tipo: TipoContencao) {
    ui.label("Condição adotada no cálculo:");
    for c in [Condicao::Drenada, Condicao::Parcial, Condicao::SemDrenagem] {
        ui.radio_value(&mut d.condicao, c, c.nome());
    }
    if d.condicao == Condicao::Parcial {
        grade(ui, "dren_p", |ui| {
            num(ui, "   Lâmina d'água / altura contida", &mut d.fracao_parcial, "", 0.0..=1.0, 0.01);
        });
    }
    if d.condicao == Condicao::SemDrenagem {
        grade(ui, "dren_s", |ui| {
            num(ui, "   Lâmina d'água / altura contida", &mut d.fracao_sem, "", 0.0..=1.0, 0.01);
        });
        nota(ui, "Sem saída, a água acumula atrás da estrutura: 1,0 = aterro saturado até a crista (hidrostática). O NA da sondagem vale se for mais alto.");
    }
    if d.condicao == Condicao::Drenada && !d.eficaz(tipo) {
        ui.label(
            RichText::new("⚠ 'Drenada' exige camada drenante + barbacãs (ou DHP): sem isso o cálculo usa 'sem drenagem'.")
                .small()
                .color(tema::de(ui).alerta),
        );
    }
    let com_face = !matches!(tipo, TipoContencao::Prancha | TipoContencao::Estacas);
    if !com_face {
        nota(ui, "Para estaca-prancha e parede de estacas, a drenagem atua apenas na poropressão adotada.");
        return;
    }
    ui.separator();
    super::widgets::caixa(ui, &mut d.barbacas, "Barbacãs");
    if d.barbacas {
        grade(ui, "barb", |ui| {
            num(ui, "   Diâmetro (tubo PVC)", &mut d.diam_barbaca_mm, "mm", 32.0..=200.0, 1.0);
            num(ui, "   Espaçamento horizontal", &mut d.esp_h, "m", 0.5..=5.0, 0.05);
            num(ui, "   Espaçamento vertical", &mut d.esp_v, "m", 0.3..=5.0, 0.05);
            num(ui, "   Altura da 1ª fila", &mut d.altura_1a_fila, "m", 0.1..=3.0, 0.05);
            num(ui, "   Declividade para fora", &mut d.declividade, "%", 0.0..=20.0, 0.5);
        });
    }
    super::widgets::caixa(ui, &mut d.filtro, "Filtro / camada drenante no tardoz");
    if d.filtro {
        ui.horizontal(|ui| {
            super::widgets::rotulo(ui, "   Material:");
            egui::ComboBox::from_id_salt("filtro").selected_text(d.material_filtro.nome()).show_ui(ui, |ui| {
                for m in [MaterialFiltro::BritaGeotextil, MaterialFiltro::Geocomposto, MaterialFiltro::AreiaGrossa] {
                    ui.selectable_value(&mut d.material_filtro, m, m.nome());
                }
            });
        });
        match d.material_filtro {
            MaterialFiltro::AreiaGrossa => {
                ui.horizontal(|ui| {
                    super::widgets::rotulo(ui, "   Tipo de areia:");
                    egui::ComboBox::from_id_salt("areia").selected_text(d.areia.nome()).show_ui(ui, |ui| {
                        for a in [TipoAreia::Grossa, TipoAreia::Media] {
                            ui.selectable_value(&mut d.areia, a, format!("{} ({})", a.nome(), a.faixa()));
                        }
                    });
                });
            }
            MaterialFiltro::BritaGeotextil => {
                ui.horizontal(|ui| {
                    super::widgets::rotulo(ui, "   Brita:");
                    egui::ComboBox::from_id_salt("brita").selected_text(d.brita.nome()).show_ui(ui, |ui| {
                        for b in [TipoBrita::N1, TipoBrita::N2, TipoBrita::N3] {
                            ui.selectable_value(&mut d.brita, b, format!("{} ({})", b.nome(), b.faixa()));
                        }
                    });
                });
            }
            MaterialFiltro::Geocomposto => {}
        }
        grade(ui, "filt", |ui| {
            num(ui, "   Espessura", &mut d.esp_filtro, "m", 0.02..=1.5, 0.01);
            num(ui, "   Gramatura do geotêxtil", &mut d.gramatura, "g/m²", 100.0..=600.0, 10.0);
        });
    }
    ui.horizontal(|ui| {
        super::widgets::caixa(ui, &mut d.dreno_pe, "Dreno de pé (tubo perfurado) Ø");
        ui.add_enabled(d.dreno_pe, egui::DragValue::new(&mut d.diam_dreno_pe_mm).speed(1.0).range(50.0..=300.0).suffix(" mm"));
    });
    super::widgets::caixa(ui, &mut d.canaleta_crista, "Canaleta de crista");
    if matches!(tipo, TipoContencao::Atirantada | TipoContencao::Grampeado) {
        super::widgets::caixa(ui, &mut d.dhp, "Drenos sub-horizontais profundos (DHP)");
        if d.dhp {
            grade(ui, "dhp", |ui| {
                num(ui, "   Comprimento", &mut d.dhp_comprimento, "m", 1.0..=40.0, 0.1);
                num(ui, "   Espaçamento horizontal", &mut d.dhp_espacamento, "m", 0.5..=20.0, 0.1);
            });
        }
    }
}

fn ui_cargas(ui: &mut egui::Ui, c: &mut Cargas) {
    super::widgets::caixa(ui, &mut c.usar_faixa, "Carga em faixa (Boussinesq)");
    if c.usar_faixa {
        grade(ui, "faixa", |ui| {
            num(ui, "   Intensidade", &mut c.q_faixa, "kPa", 0.0..=1000.0, 0.5);
            num(ui, "   Distância à contenção", &mut c.faixa_dist, "m", 0.0..=50.0, 0.05);
            num(ui, "   Largura da faixa", &mut c.faixa_larg, "m", 0.1..=50.0, 0.05);
        });
    }
    super::widgets::caixa(ui, &mut c.usar_sismo, "Ação sísmica (Mononobe-Okabe)");
    if c.usar_sismo {
        grade(ui, "sismo", |ui| {
            num(ui, "   kh (ah/g)", &mut c.kh, "", 0.0..=0.5, 0.005);
            num(ui, "   kv", &mut c.kv, "", -0.3..=0.3, 0.005);
        });
    }
}
