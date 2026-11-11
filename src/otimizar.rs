//! Otimização quando o pré-dimensionamento não atende: procura, entre as
//! alternativas usuais do tipo de contenção, a mais econômica que aprova todas
//! as verificações de estabilidade. O custo é um índice de consumo de material
//! por metro de contenção (m³ de concreto/alvenaria, m² de escavação, m de
//! grampo ou tirante), suficiente para ordenar soluções do mesmo tipo.

use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::fmt;

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
                let var = if custo_base > 1e-9 { format!(" (consumo de material {:+.0} % em relação ao pré-dimensionamento)", 100.0 * (m.custo / custo_base - 1.0)) } else { String::new() };
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

/// Verificações de estabilidade que a geometria pode resolver (as mesmas do pré-dimensionamento).
fn pendentes(p: &Projeto) -> Vec<String> {
    let r = p.calcular();
    r.verificacoes
        .iter()
        .filter(|v| match p.tipo {
            TipoContencao::Grampeado => !v.nome.starts_with("Face"),
            TipoContencao::Atirantada => v.nome.starts_with("Passivo") || v.nome.starts_with("Tirante") || v.nome.contains("estabilidade"),
            TipoContencao::Prancha | TipoContencao::Estacas => v.nome.contains("estabilidade") || v.nome.starts_with("W do perfil") || v.nome.starts_with("Taxa de armadura"),
            _ => v.nome.contains("(estático)") || v.nome.contains("estabilidade"),
        })
        .filter(|v| !v.ok())
        .map(|v| v.nome.split(" — ").next().unwrap_or(&v.nome).to_string())
        .collect()
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
            let parede = 0.5 * (m.t_topo + m.t_base) * (m.h - m.h_sapata);
            let sapata = b * m.h_sapata;
            let dente = if m.dente { m.dente_b * m.dente_h } else { 0.0 };
            // concreto armado ≈ 1,6 por m³ (fôrma + aço) ; escavação ≈ 0,08 por m²
            1.6 * (parede + sapata + dente) + 0.08 * b * m.df
        }
        TipoContencao::Atirantada => {
            let c = p.preparar_atirantada();
            let r = p.calcular();
            let l_tir: f64 = r.reforcos.iter().map(|f| f.comprimento).sum();
            // painel 1,6/m³ ; tirante ≈ 0,12 por metro por metro de cortina
            1.6 * c.espessura * (c.h + c.ficha) + 0.12 * l_tir / c.espac_h.max(0.1) * c.espac_h / c.espac_h.max(0.1)
        }
        TipoContencao::Grampeado => {
            let g = p.preparar_grampeado();
            let n = ((g.h - g.z1) / g.sv.max(0.3)).floor().max(0.0) + 1.0;
            // grampo ≈ 0,06 por metro por m² de face ; face 1,6/m³
            0.06 * n * g.comprimento / g.sh.max(0.3) + 1.6 * g.espessura_face * g.h
        }
        TipoContencao::Prancha => {
            let r = p.calcular();
            let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(2.0 * p.geometria.h);
            0.35 * lt * (p.prancha.w_perfil / 1000.0).max(0.5)
        }
        TipoContencao::Estacas => {
            let r = p.calcular();
            let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(2.0 * p.geometria.h);
            let e = &p.estacas;
            1.6 * std::f64::consts::PI * e.diametro * e.diametro / 4.0 * lt / e.espacamento.max(0.2)
        }
    }
}

fn avaliar(p: Projeto, descricao: String) -> Alternativa {
    let pend = pendentes(&p);
    Alternativa { custo: custo(&p), atende: pend.is_empty(), pendentes: pend, descricao, projeto: p }
}

/// Gera as alternativas do tipo de contenção a partir do projeto atual (já pré-dimensionado).
fn alternativas(base: &Projeto) -> Vec<Alternativa> {
    let h = base.geometria.h;
    let mut v = Vec::new();
    match base.tipo {
        TipoContencao::Gravidade => {
            for k in [0.40, 0.50, 0.60, 0.70, 0.80, 0.90] {
                for dente in [false, true] {
                    for passivo in [false, true] {
                        let mut p = base.clone();
                        p.aplicar_proporcoes_pub(k);
                        p.gravidade.dente = dente;
                        if dente {
                            p.gravidade.dente_h = (0.15 * h).clamp(0.3, 1.0);
                            p.gravidade.dente_b = (0.5 * p.gravidade.f).max(0.3);
                        }
                        p.gravidade.considerar_passivo = passivo;
                        v.push(avaliar(
                            p,
                            format!(
                                "base = {} % de H{}{}",
                                (k * 100.0).round(),
                                if dente { " + dente de base" } else { "" },
                                if passivo { " + passivo frontal (reduzido, NBR 6122 7.6.3)" } else { "" }
                            ),
                        ));
                    }
                }
            }
        }
        TipoContencao::Flexao => {
            for k in [0.40, 0.50, 0.60, 0.70, 0.80, 0.90] {
                for dente in [false, true] {
                    for passivo in [false, true] {
                        let mut p = base.clone();
                        p.aplicar_proporcoes_pub(k);
                        p.flexao.dente = dente;
                        if dente {
                            p.flexao.dente_h = (0.15 * h).clamp(0.3, 1.0);
                            p.flexao.dente_b = p.flexao.t_base.max(0.3);
                        }
                        p.flexao.considerar_passivo = passivo;
                        v.push(avaliar(
                            p,
                            format!(
                                "base = {} % de H{}{}",
                                (k * 100.0).round(),
                                if dente { " + dente de base" } else { "" },
                                if passivo { " + passivo frontal (reduzido, NBR 6122 7.6.3)" } else { "" }
                            ),
                        ));
                    }
                }
            }
        }
        TipoContencao::Atirantada => {
            let n_base = base.atirantada.niveis.len().max(1);
            for n in n_base..=(n_base + 2).min(6) {
                for esp in [3.0, 2.5, 2.0, 1.5] {
                    let mut p = base.clone();
                    let passo = h / (n as f64 + 0.5);
                    p.atirantada.niveis = (0..n).map(|i| ((i as f64 + 0.6) * passo * 100.0).round() / 100.0).collect();
                    p.atirantada.espac_h = esp;
                    p.atirantada.auto_dim = true;
                    p.pre_dimensionar();
                    let f0 = p.geometria.engaste;
                    v.push(avaliar(p.clone(), format!("{n} nível(is) de tirantes a cada {} m, ficha {} m", fmt(esp), fmt(f0))));
                    // nível inferior mais próximo da base reduz a reação da ficha; ficha maior também
                    let mut q = p.clone();
                    if let Some(ult) = q.atirantada.niveis.last_mut() {
                        *ult = (h - 0.6).max(0.5);
                    }
                    q.pre_dimensionar();
                    v.push(avaliar(q.clone(), format!("{n} nível(is) a cada {} m, último a {} m da base, ficha {} m", fmt(esp), fmt(0.6), fmt(q.geometria.engaste))));
                    for ficha in [f0 + 1.0, f0 + 2.0, (1.0 * h).max(f0 + 3.0)] {
                        let mut w = p.clone();
                        w.geometria.engaste = ficha;
                        v.push(avaliar(w, format!("{n} nível(is) a cada {} m, ficha {} m", fmt(esp), fmt(ficha))));
                    }
                }
            }
        }
        TipoContencao::Grampeado => {
            for s in [2.0, 1.5, 1.2, 1.0] {
                for qs_k in [1.0] {
                    let _ = qs_k;
                    let mut p = base.clone();
                    p.grampeado.sh = s;
                    p.grampeado.sv = s;
                    p.grampeado.auto_dim = true;
                    p.manter_malha = true;
                    for d in [0.10, 0.125, 0.15] {
                        let mut q = p.clone();
                        q.grampeado.diam_furo = d;
                        q.pre_dimensionar();
                        let l0 = q.preparar_grampeado().comprimento;
                        v.push(avaliar(q.clone(), format!("malha {} × {} m, furo Ø {} cm (L e barra pela tração)", fmt(s), fmt(s), fmt(d * 100.0))));
                        // grampos mais longos que o mínimo da tração (estabilidade global)
                        for extra in [1.0, 2.0, 3.0] {
                            let mut w = q.clone();
                            w.grampeado.auto_dim = false;
                            w.grampeado.comprimento = l0 + extra;
                            w.grampeado.diam_barra = q.preparar_grampeado().diam_barra;
                            v.push(avaliar(w, format!("malha {} × {} m, furo Ø {} cm, L = {} m", fmt(s), fmt(s), fmt(d * 100.0), fmt(l0 + extra))));
                        }
                    }
                }
            }
        }
        TipoContencao::Prancha => {
            use crate::tipos::embutida::Apoio;
            for (apoio, prof) in [(Apoio::Balanco, 0.0), (Apoio::Ancorada, 0.25 * h), (Apoio::Ancorada, 0.35 * h)] {
                for w in [1300.0, 1600.0, 2000.0, 2500.0, 3200.0] {
                    let mut p = base.clone();
                    p.prancha.parede.apoio = apoio;
                    p.prancha.parede.prof_apoio = prof.max(0.5);
                    p.prancha.w_perfil = w;
                    v.push(avaliar(
                        p,
                        format!(
                            "{} ; perfil W = {} cm³/m",
                            if apoio == Apoio::Balanco { "em balanço".to_string() } else { format!("tirante a {} m", fmt(prof.max(0.5))) },
                            fmt(w)
                        ),
                    ));
                }
            }
        }
        TipoContencao::Estacas => {
            use crate::tipos::embutida::Apoio;
            for (apoio, prof) in [(Apoio::Balanco, 0.0), (Apoio::Ancorada, 0.25 * h), (Apoio::Ancorada, 0.35 * h)] {
                for d in [0.40, 0.50, 0.60, 0.80, 1.00] {
                    for esp_k in [1.0, 1.5, 2.0] {
                        let mut p = base.clone();
                        p.estacas.parede.apoio = apoio;
                        p.estacas.parede.prof_apoio = prof.max(0.5);
                        p.estacas.diametro = d;
                        p.estacas.espacamento = d * esp_k;
                        v.push(avaliar(
                            p,
                            format!(
                                "{} ; estacas Ø {} m a cada {} m",
                                if apoio == Apoio::Balanco { "em balanço".to_string() } else { format!("tirante a {} m", fmt(prof.max(0.5))) },
                                fmt(d),
                                fmt(d * esp_k)
                            ),
                        ));
                    }
                }
            }
        }
    }
    v
}

/// Procura a alternativa mais econômica que atende. Não altera `base`.
pub fn otimizar(base: &Projeto) -> Otimizacao {
    let alts = alternativas(base);
    let avaliadas = alts.len();
    let melhor = alts.iter().filter(|a| a.atende).min_by(|a, b| a.custo.partial_cmp(&b.custo).unwrap_or(std::cmp::Ordering::Equal)).cloned();
    let mais_proxima = alts
        .iter()
        .min_by(|a, b| a.pendentes.len().cmp(&b.pendentes.len()).then(a.custo.partial_cmp(&b.custo).unwrap_or(std::cmp::Ordering::Equal)))
        .cloned();
    Otimizacao { melhor, mais_proxima, avaliadas }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn otimizador_encontra_solucao_que_atende_ou_explica() {
        for tipo in crate::projeto::TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let o = otimizar(&p);
            assert!(o.avaliadas >= 4, "{}", tipo.nome());
            let msg = o.mensagem(custo(&p));
            assert!(msg.starts_with("Otimização:"), "{msg}");
            if let Some(m) = &o.melhor {
                assert!(pendentes(&m.projeto).is_empty(), "{}: a melhor alternativa deve atender", tipo.nome());
                // nenhuma alternativa que atende é mais barata
                assert!(alternativas(&p).iter().filter(|a| a.atende).all(|a| a.custo >= m.custo - 1e-9));
            }
            eprintln!("{}: {msg}", tipo.nome());
        }
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

