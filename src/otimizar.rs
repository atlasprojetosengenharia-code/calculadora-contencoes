//! Otimização: procura, entre as alternativas usuais do tipo de contenção, a mais
//! econômica que aprova TODAS as verificações (estabilidade, estrutura e global).
//!
//! Cada alternativa recebe primeiro a geometria (base, dente, níveis, malha,
//! apoio, ficha…) e depois um ajuste estrutural local (espessuras, placas,
//! barras, perfil do catálogo) até as verificações de seção atenderem. O custo é
//! um índice de consumo de material por metro de contenção, suficiente para
//! ordenar soluções do mesmo tipo. As alternativas são avaliadas em paralelo.

use crate::perfis_prancha;
use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::{fmt, Resultado};
use crate::tipos::embutida::Apoio;
use crate::tipos::grampeado::BARRAS_GRAMPO;

/// Uma alternativa avaliada.
#[derive(Clone, Debug)]
pub struct Alternativa {
    pub descricao: String,
    pub custo: f64,
    pub atende: bool,
    pub pendentes: Vec<String>,
    pub projeto: Projeto,
}

/// Resultado da otimização.
#[derive(Clone, Debug)]
pub struct Otimizacao {
    /// Melhor alternativa que atende (a de menor custo), se houver.
    pub melhor: Option<Alternativa>,
    /// Alternativa de menor número de pendências, quando nenhuma atende.
    pub mais_proxima: Option<Alternativa>,
    pub avaliadas: usize,
}

impl Otimizacao {
    /// Mensagem para a interface e o memorial.
    pub fn mensagem(&self, custo_base: f64) -> String {
        match (&self.melhor, &self.mais_proxima) {
            (Some(m), _) => {
                let var = if custo_base > 1e-9 {
                    format!(" (consumo de material {:+.0} % em relação à solução anterior)", 100.0 * (m.custo / custo_base - 1.0))
                } else {
                    String::new()
                };
                format!("Otimização: entre {} alternativas, a mais econômica que atende é {}{}.", self.avaliadas, m.descricao, var)
            }
            (None, Some(p)) => format!(
                "Otimização: nenhuma das {} alternativas atende. A mais próxima ({}) ainda falha em: {}. Reveja os parâmetros do solo ou mude o tipo de contenção.",
                self.avaliadas,
                p.descricao,
                p.pendentes.join(", ")
            ),
            _ => "Otimização: nenhuma alternativa pôde ser avaliada.".into(),
        }
    }
}

fn nome_curto(nome: &str) -> String {
    nome.split(" — ").next().unwrap_or(nome).to_string()
}

/// Todas as verificações que não atendem.
fn falhas(r: &Resultado) -> Vec<String> {
    r.verificacoes.iter().filter(|v| !v.ok()).map(|v| nome_curto(&v.nome)).collect()
}

fn pendentes(p: &Projeto) -> Vec<String> {
    falhas(&p.calcular())
}

/// Índice de custo (consumo de material por metro de contenção).
pub fn custo(p: &Projeto) -> f64 {
    match p.tipo {
        TipoContencao::Gravidade => {
            let m = p.preparar_gravidade();
            let g = m.geo();
            let corpo = 0.5 * (m.crista + (g.x_pt - g.x_pf)) * m.h;
            let base = g.base * m.f;
            let dente = if m.dente { m.dente_b * m.dente_h } else { 0.0 };
            // alvenaria/concreto ciclópico ≈ 1,0 por m³ ; escavação da base ≈ 0,08 por m²
            corpo + base + dente + 0.08 * g.base * m.df
        }
        TipoContencao::Flexao => {
            let m = p.preparar_flexao();
            let b = m.b_ponta + m.t_base + m.b_talao;
            let hp = m.h - m.h_sapata;
            let sapata = b * m.h_sapata;
            let dente = if m.dente { m.dente_b * m.dente_h } else { 0.0 };
            // concreto armado ≈ 1,6 por m³ (fôrma + aço) ; escavação ≈ 0,08 por m²
            let parede = match m.sistema {
                // misto: alvenaria estrutural ≈ 0,7 por m³ de bloco (blocos, graute, aço das
                // canaletas) + pilares em concreto armado distribuídos no espaçamento
                crate::tipos::misto::Sistema::Misto => {
                    let mi = &m.misto;
                    let s = mi.espac_pilar.max(0.5);
                    let mut c = 0.7 * mi.bloco * hp + 1.6 * mi.pilar_b * mi.pilar_h * hp / s;
                    if mi.fundacao_estacas() {
                        // vigas intermediárias (contagem pelo resultado), blocos e 2 estacas por pilar
                        let r = p.calcular_local();
                        let n_vigas = r.armaduras.iter().find(|a| a.elemento.starts_with("Vigas intermediárias")).map(|a| {
                            a.elemento.split(['(', ')']).nth(1).and_then(|x| x.trim().parse::<f64>().ok()).unwrap_or(1.0)
                        }).unwrap_or(0.0);
                        let l_est = r.verificacoes.iter().find(|v| v.nome.starts_with("Estaca: comprimento")).map(|v| v.valor).unwrap_or(6.0);
                        let (_, lb, bb, hb) = mi.geometria_bloco();
                        let ap = std::f64::consts::PI * mi.diam_estaca * mi.diam_estaca / 4.0;
                        c += 1.6 * mi.viga_b * mi.viga_h * n_vigas + 1.6 * (lb * bb * hb + 2.0 * ap * l_est) / s;
                    }
                    c
                }
                _ => 1.6 * 0.5 * (m.t_topo + m.t_base) * hp,
            };
            parede + 1.6 * (sapata + dente) + 0.08 * b * m.df
        }
        TipoContencao::Atirantada => {
            let c = p.preparar_atirantada();
            let r = p.calcular_local();
            let l_tir: f64 = r.reforcos.iter().map(|f| f.comprimento).sum();
            // painel 1,6 por m³ ; tirante ≈ 0,12 por metro (por metro de cortina: ÷ espaçamento)
            1.6 * c.espessura * (c.h + c.ficha) + 0.12 * l_tir / c.espac_h.max(0.1)
        }
        TipoContencao::Grampeado => {
            let g = p.preparar_grampeado();
            let n = ((g.h - g.z1) / g.sv.max(0.3)).floor().max(0.0) + 1.0;
            // grampo ≈ 0,06 por metro (+ até 30 % para barras maiores) por m de face ; face 1,6 por m³
            let barra = 1.0 + 0.3 * ((g.diam_barra - 16.0) / 16.0).max(0.0);
            0.06 * barra * n * g.comprimento / g.sh.max(0.3) + 1.6 * g.espessura_face * g.h
        }
        TipoContencao::Prancha => {
            let e = p.preparar_prancha();
            let r = p.calcular_local();
            let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(2.0 * p.geometria.h);
            // aço da parede: 1,0 por 100 kg ; tirante/estronca ≈ 0,33 por metro (estimado H + 3 m) + longarina 0,5 por m
            let aco = 0.01 * e.peso_parede() * lt;
            let apoio = if e.parede.apoio == Apoio::Ancorada { 0.33 * (p.geometria.h + 3.0) / e.parede.espac_apoio.max(0.5) + 0.5 } else { 0.0 };
            aco + apoio
        }
        TipoContencao::Estacas => {
            let r = p.calcular_local();
            let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(2.0 * p.geometria.h);
            let e = &p.estacas;
            let apoio = if e.parede.apoio == Apoio::Ancorada { 0.33 * (p.geometria.h + 3.0) / e.parede.espac_apoio.max(0.5) + 0.5 } else { 0.0 };
            1.6 * std::f64::consts::PI * e.diametro * e.diametro / 4.0 * lt / e.espacamento.max(0.2) + apoio
        }
    }
}

/// Ajuste estrutural local (sem a estabilidade global, que a geometria resolve):
/// aumenta espessuras, placas ou o perfil até as verificações de seção atenderem.
fn ajustar_estrutura(p: &mut Projeto) {
    for _ in 0..30 {
        let r = p.calcular_local();
        let f: Vec<String> = falhas(&r);
        let tem = |s: &str| f.iter().any(|n| n.contains(s));
        let mudou = match p.tipo {
            TipoContencao::Gravidade => p.alargar_crista_se_preciso(&f),
            TipoContencao::Flexao => {
                let h = p.geometria.h;
                let m = &mut p.flexao;
                let mut ch = false;
                if (tem("Talão") || tem("Ponta")) && m.h_sapata < (0.20 * h).max(0.5) {
                    m.h_sapata = crate::projeto::arred(m.h_sapata + 0.05);
                    ch = true;
                }
                if tem("Parede") && m.t_base < (0.20 * h).max(0.5) {
                    m.t_base = crate::projeto::arred(m.t_base + 0.05);
                    ch = true;
                }
                // sistema misto: pilar mais alto; painel com canaletas mais próximas,
                // bloco mais largo ou mais resistente
                let mi = &mut m.misto;
                if (tem("Pilar: espaçamento") || tem("Pilar: Ø longitudinal ≤")) && mi.pilar_b < 0.60 {
                    mi.pilar_b = crate::projeto::arred(mi.pilar_b + 0.05);
                    ch = true;
                }
                if tem("Bloco: tirante") {
                    if let Some(d) = crate::tipos::misto::DIAM_ESTACA.iter().find(|d| **d > mi.diam_estaca + 1e-9) {
                        mi.diam_estaca = *d;
                        ch = true;
                    }
                }
                if tem("Pilar") && mi.pilar_h < 0.80 {
                    mi.pilar_h = crate::projeto::arred(mi.pilar_h + 0.05);
                    ch = true;
                }
                if tem("Painel") {
                    if mi.bloco < 0.19 {
                        mi.bloco = 0.19;
                        ch = true;
                    } else if let Some(f) = crate::tipos::misto::FBK.iter().find(|f| **f > mi.fbk + 1e-9) {
                        mi.fbk = *f;
                        ch = true;
                    }
                }
                if tem("Vigas intermediárias") && mi.viga_b < 0.60 {
                    mi.viga_b = crate::projeto::arred(mi.viga_b + 0.05);
                    ch = true;
                }
                if tem("Viga baldrame") && mi.baldrame_h < 1.0 {
                    mi.baldrame_h = crate::projeto::arred(mi.baldrame_h + 0.10);
                    ch = true;
                }
                if tem("Estaca") && !tem("comprimento") {
                    if let Some(d) = crate::tipos::misto::DIAM_ESTACA.iter().find(|d| **d > mi.diam_estaca + 1e-9) {
                        mi.diam_estaca = *d;
                        ch = true;
                    }
                }
                ch
            }
            TipoContencao::Atirantada => {
                let c = &mut p.atirantada;
                if tem("Punção") && c.placa < 0.50 {
                    c.placa = crate::projeto::arred(c.placa + 0.05);
                    true
                } else if (tem("Punção") || tem("Painel")) && c.espessura < 0.50 {
                    c.espessura = crate::projeto::arred(c.espessura + 0.05);
                    true
                } else {
                    false
                }
            }
            TipoContencao::Grampeado => {
                let g = &mut p.grampeado;
                if tem("Face: punção") && g.placa < 0.40 {
                    g.placa = crate::projeto::arred(g.placa + 0.05);
                    true
                } else if tem("Face") && g.espessura_face < 0.25 {
                    g.espessura_face = crate::projeto::arred(g.espessura_face + 0.025);
                    true
                } else {
                    false
                }
            }
            TipoContencao::Prancha => {
                // perfil de aço mais leve do catálogo VPA com W ≥ W necessário
                let w_nec = r.verificacoes.iter().find(|v| v.nome.starts_with("W do perfil")).map(|v| v.limite);
                match w_nec.and_then(perfis_prancha::aco_mais_leve) {
                    Some(perf) if perf.nome != p.prancha.nome_perfil => {
                        p.prancha.nome_perfil = perf.nome.into();
                        p.prancha.w_perfil = perf.w;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        if !mudou {
            break;
        }
    }
}

fn avaliar(mut p: Projeto, descricao: String) -> Alternativa {
    ajustar_estrutura(&mut p);
    let pend = pendentes(&p);
    let descricao = match p.tipo {
        TipoContencao::Prancha => format!("{descricao} ; perfil {}", p.prancha.nome_perfil),
        // misto: valores finais depois do ajuste estrutural (pilar e estacas)
        TipoContencao::Flexao if p.flexao.sistema == crate::tipos::misto::Sistema::Misto => {
            let mi = &p.flexao.misto;
            let base = descricao.split(" ; estacas").next().unwrap_or(&descricao).to_string();
            let est = if mi.fundacao_estacas() { format!(" ; estacas Ø{:.0} cm", 100.0 * mi.diam_estaca) } else { String::new() };
            format!("{base}{est} ; pilares {:.0}×{:.0} cm", 100.0 * mi.pilar_b, 100.0 * mi.pilar_h)
        }
        _ => descricao,
    };
    Alternativa { custo: custo(&p), atende: pend.is_empty(), pendentes: pend, descricao, projeto: p }
}

/// Gera as alternativas (projeto + descrição) a partir do projeto atual.
fn candidatos(base: &Projeto) -> Vec<(Projeto, String)> {
    let h = base.geometria.h;
    let mut v: Vec<(Projeto, String)> = Vec::new();
    match base.tipo {
        TipoContencao::Flexao if base.flexao.sistema == crate::tipos::misto::Sistema::Misto && base.flexao.misto.fundacao_estacas() => {
            // misto em estacas: espaçamento dos pilares, diâmetro das estacas e embutimento
            for s in [1.5, 2.0, 2.5, 3.0] {
                for d in crate::tipos::misto::DIAM_ESTACA {
                    for embut in [0.0, 0.5] {
                        let mut p = base.clone();
                        p.flexao.misto.espac_pilar = s;
                        p.flexao.misto.diam_estaca = d;
                        p.geometria.engaste += embut;
                        let emb = if embut > 0.0 { format!(" ; embutimento {} m", fmt(p.geometria.engaste)) } else { String::new() };
                        v.push((p, format!("pilares a cada {} m ; estacas Ø{} cm{emb}", fmt(s), fmt(100.0 * d))));
                    }
                }
            }
        }
        TipoContencao::Gravidade | TipoContencao::Flexao => {
            let flexao = base.tipo == TipoContencao::Flexao;
            for k in [0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 1.00] {
                for dente in [false, true] {
                    for passivo in [false, true] {
                        for embut in [0.0, 0.5] {
                            let mut p = base.clone();
                            p.aplicar_proporcoes_pub(k);
                            p.geometria.engaste += embut;
                            let emb_total = p.geometria.engaste;
                            if flexao {
                                p.flexao.dente = dente;
                                if dente {
                                    p.flexao.dente_h = (0.15 * h).clamp(0.3, 1.0);
                                    p.flexao.dente_b = p.flexao.t_base.max(0.3);
                                }
                                p.flexao.considerar_passivo = passivo;
                            } else {
                                p.gravidade.dente = dente;
                                if dente {
                                    p.gravidade.dente_h = (0.15 * h).clamp(0.3, 1.0);
                                    p.gravidade.dente_b = (0.5 * p.gravidade.f).max(0.3);
                                }
                                p.gravidade.considerar_passivo = passivo;
                            }
                            let d = format!(
                                "base = {} % de H{}{}{}",
                                (k * 100.0).round(),
                                if dente { " + dente de base" } else { "" },
                                if passivo { " + passivo frontal (reduzido, NBR 6122 7.6.3)" } else { "" },
                                if embut > 0.0 { format!(" + embutimento {} m", fmt(emb_total)) } else { String::new() }
                            );
                            if flexao && p.flexao.sistema == crate::tipos::misto::Sistema::Misto {
                                // sistema misto: também o espaçamento entre pilares
                                for s in [1.5, 2.0, 3.0, 4.0] {
                                    let mut q = p.clone();
                                    q.flexao.misto.espac_pilar = s;
                                    v.push((q, format!("{d} ; pilares a cada {} m", fmt(s))));
                                }
                            } else {
                                v.push((p, d));
                            }
                        }
                    }
                }
            }
        }
        TipoContencao::Atirantada => {
            let n_base = base.atirantada.niveis.len().max(1);
            let combos: Vec<(usize, f64)> = (n_base..=(n_base + 2).min(6)).flat_map(|n| [3.0, 2.5, 2.0, 1.5].map(|e| (n, e))).collect();
            // o pré-dimensionamento da ficha (várias análises) roda em paralelo
            let pre = em_paralelo(combos, |(n, esp)| {
                let mut p = base.clone();
                let passo = h / (n as f64 + 0.5);
                p.atirantada.niveis = (0..n).map(|i| ((i as f64 + 0.6) * passo * 100.0).round() / 100.0).collect();
                p.atirantada.espac_h = esp;
                p.atirantada.auto_dim = true;
                p.pre_dimensionar();
                // nível inferior mais próximo da base reduz a reação da ficha
                let mut q = p.clone();
                if let Some(ult) = q.atirantada.niveis.last_mut() {
                    *ult = (h - 0.6).max(0.5);
                }
                q.pre_dimensionar();
                (n, esp, p, q)
            });
            for (n, esp, p, q) in pre {
                let f0 = p.geometria.engaste;
                let fq = q.geometria.engaste;
                v.push((p.clone(), format!("{n} nível(is) de tirantes a cada {} m, ficha {} m", fmt(esp), fmt(f0))));
                v.push((q, format!("{n} nível(is) a cada {} m, último a 0,60 m da base, ficha {} m", fmt(esp), fmt(fq))));
                for ficha in [f0 + 1.0, f0 + 2.0, (1.0 * h).max(f0 + 3.0)] {
                    let mut w = p.clone();
                    w.geometria.engaste = ficha;
                    v.push((w, format!("{n} nível(is) a cada {} m, ficha {} m", fmt(esp), fmt(ficha))));
                }
            }
        }
        TipoContencao::Grampeado => {
            for s in [2.0, 1.75, 1.5, 1.25, 1.0] {
                for d in [0.10, 0.125, 0.15] {
                    let mut p = base.clone();
                    p.grampeado.sh = s;
                    p.grampeado.sv = s;
                    p.grampeado.z1 = (0.5 * s).max(0.5);
                    p.grampeado.diam_furo = d;
                    p.grampeado.auto_dim = true;
                    p.manter_malha = true;
                    p.pre_dimensionar();
                    let g0 = p.preparar_grampeado();
                    let (l0, b0) = (g0.comprimento, g0.diam_barra);
                    v.push((p.clone(), format!("malha {} × {} m, furo Ø {} cm (L e barra pela tração)", fmt(s), fmt(s), fmt(d * 100.0))));
                    // barras maiores e grampos mais longos que o mínimo da tração (estabilidade global)
                    for &barra in BARRAS_GRAMPO.iter().filter(|&&b| b >= b0) {
                        for extra in [0.0, 1.0, 2.0, 3.0, 4.0] {
                            if barra == b0 && extra == 0.0 {
                                continue;
                            }
                            let mut w = p.clone();
                            w.grampeado.auto_dim = false;
                            w.grampeado.comprimento = l0 + extra;
                            w.grampeado.diam_barra = barra;
                            v.push((
                                w,
                                format!("malha {} × {} m, furo Ø {} cm, barra Ø {} mm, L = {} m", fmt(s), fmt(s), fmt(d * 100.0), fmt(barra), fmt(l0 + extra)),
                            ));
                        }
                    }
                }
            }
        }
        TipoContencao::Prancha | TipoContencao::Estacas => {
            let prancha = base.tipo == TipoContencao::Prancha;
            let apoios = [(Apoio::Balanco, 0.0), (Apoio::Ancorada, 0.20 * h), (Apoio::Ancorada, 0.30 * h), (Apoio::Ancorada, 0.40 * h)];
            for (apoio, prof) in apoios {
                for extra in [0.0, 0.25 * h, 0.5 * h] {
                    let rot_apoio = if apoio == Apoio::Balanco { "em balanço".to_string() } else { format!("tirante a {} m", fmt(prof.max(0.5))) };
                    let rot_ficha = if extra > 0.0 { format!(" ; ficha + {} m", fmt(extra)) } else { String::new() };
                    if prancha {
                        let mut p = base.clone();
                        p.prancha.parede.apoio = apoio;
                        p.prancha.parede.prof_apoio = prof.max(0.5);
                        p.prancha.parede.ficha_adicional = extra;
                        // começa pelo perfil de aço mais leve; o ajuste sobe até atender
                        if let Some(perf) = perfis_prancha::aco_por_peso().first() {
                            p.prancha.nome_perfil = perf.nome.into();
                            p.prancha.w_perfil = perf.w;
                        }
                        v.push((p, format!("{rot_apoio}{rot_ficha}")));
                    } else {
                        for d in [0.40, 0.50, 0.60, 0.80, 1.00] {
                            for esp_k in [1.0, 1.5, 2.0] {
                                let mut p = base.clone();
                                p.estacas.parede.apoio = apoio;
                                p.estacas.parede.prof_apoio = prof.max(0.5);
                                p.estacas.parede.ficha_adicional = extra;
                                p.estacas.diametro = d;
                                p.estacas.espacamento = d * esp_k;
                                v.push((p, format!("{rot_apoio}{rot_ficha} ; estacas Ø {} m a cada {} m", fmt(d), fmt(d * esp_k))));
                            }
                        }
                    }
                }
            }
        }
    }
    v
}

/// Aplica `f` a cada item em paralelo (uma fatia por núcleo), preservando a ordem.
fn em_paralelo<T: Send, U: Send>(itens: Vec<T>, f: impl Fn(T) -> U + Sync) -> Vec<U> {
    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 16);
    let tam = itens.len().div_ceil(n).max(1);
    let mut lotes: Vec<Vec<T>> = Vec::new();
    let mut it = itens.into_iter().peekable();
    while it.peek().is_some() {
        lotes.push(it.by_ref().take(tam).collect());
    }
    let f = &f;
    std::thread::scope(|s| {
        let tarefas: Vec<_> = lotes.into_iter().map(|lote| s.spawn(move || lote.into_iter().map(f).collect::<Vec<_>>())).collect();
        tarefas.into_iter().flat_map(|t| t.join().unwrap_or_default()).collect()
    })
}

/// Avalia as alternativas em paralelo.
fn alternativas(base: &Projeto) -> Vec<Alternativa> {
    em_paralelo(candidatos(base), |(p, d)| avaliar(p, d))
}

/// Procura a alternativa mais econômica que atende. Não altera `base`.
pub fn otimizar(base: &Projeto) -> Otimizacao {
    let alts = alternativas(base);
    let avaliadas = alts.len();
    let ord = |a: &f64, b: &f64| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal);
    let melhor = alts.iter().filter(|a| a.atende).min_by(|a, b| ord(&a.custo, &b.custo)).cloned();
    let mais_proxima = alts.iter().min_by(|a, b| a.pendentes.len().cmp(&b.pendentes.len()).then(ord(&a.custo, &b.custo))).cloned();
    Otimizacao { melhor, mais_proxima, avaliadas }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn otimizador_encontra_solucao_que_atende_ou_explica() {
        // em modo debug (CI) só dois tipos: sem otimização o Bishop é ~20x mais lento
        let tipos: Vec<TipoContencao> = if cfg!(debug_assertions) {
            vec![TipoContencao::Gravidade, TipoContencao::Prancha]
        } else {
            crate::projeto::TIPOS_CONTENCAO.to_vec()
        };
        for tipo in tipos {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let o = otimizar(&p);
            assert!(o.avaliadas >= 4, "{}", tipo.nome());
            let msg = o.mensagem(custo(&p));
            assert!(msg.starts_with("Otimização:"), "{msg}");
            if let Some(m) = &o.melhor {
                // a melhor alternativa atende a TODAS as verificações do resultado completo
                assert!(m.projeto.calcular().atende(), "{}: a melhor alternativa deve atender", tipo.nome());
            }
            eprintln!("{}: {msg}", tipo.nome());
        }
    }

    #[test]
    fn prancha_escolhe_perfil_do_catalogo() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Prancha;
        p.geometria.h = 4.0;
        let o = otimizar(&p);
        let m = o.melhor.expect("prancha de 4 m deve ter solução");
        let nome = &m.projeto.prancha.nome_perfil;
        assert!(perfis_prancha::buscar(nome).is_some(), "perfil fora do catálogo: {nome}");
        assert!(m.descricao.contains("VPA"), "{}", m.descricao);
    }

    #[test]
    fn custo_cresce_com_a_base() {
        let mut a = Projeto::default();
        a.tipo = TipoContencao::Gravidade;
        a.geometria.h = 4.0;
        a.aplicar_proporcoes_pub(0.5);
        let mut b = a.clone();
        b.aplicar_proporcoes_pub(0.8);
        assert!(custo(&b) > custo(&a));
    }
}

#[cfg(test)]
mod diagnostico {
    use super::*;
    /// Varredura de todos os tipos e alturas:
    /// cargo test --release diag_otimizador -- --ignored --nocapture
    #[test]
    #[ignore]
    fn diag_otimizador() {
        for tipo in crate::projeto::TIPOS_CONTENCAO {
            for h in [3.0, 5.0, 8.0] {
                let mut p = Projeto::default();
                p.tipo = tipo;
                p.geometria.h = h;
                p.pre_dimensionar();
                let falhas0 = pendentes(&p);
                let t0 = std::time::Instant::now();
                let o = otimizar(&p);
                eprintln!("=== {} H={h}: falhas iniciais {:?}\n  ({:?}) {}", tipo.nome(), falhas0, t0.elapsed(), o.mensagem(custo(&p)));
                if let Some(m) = &o.melhor {
                    assert!(m.projeto.calcular().atende());
                }
            }
        }
    }
}

#[cfg(test)]
mod testes_muro_baixo {
    use super::*;

    /// Muros de gravidade baixos (com a sobrecarga padrão de 20 kPa) têm solução que atende.
    #[test]
    fn muro_de_gravidade_baixo_tem_solucao() {
        for h in [1.0, 1.5, 2.0] {
            let mut p = Projeto::default();
            p.tipo = TipoContencao::Gravidade;
            p.geometria.h = h;
            p.pre_dimensionar();
            let o = otimizar(&p);
            let m = o.melhor.clone().unwrap_or_else(|| panic!("H = {h}: {}", o.mensagem(custo(&p))));
            assert!(m.projeto.calcular().atende(), "H = {h}");
        }
    }
}
