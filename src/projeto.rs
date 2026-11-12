//! Projeto: dados gerais, sondagem SPT, geometria essencial, cargas,
//! drenagem, fatores e parâmetros específicos de cada tipo de contenção.
//! Salvo em arquivo `.ctc` (JSON).

use crate::drenagem::Drenagem;
use crate::reaterro::Reaterro;
use crate::empuxo::{rad, ParamEmpuxo};
use crate::fatores::Fatores;
use crate::resultado::{fmt, Bloco, Formula, Resultado};
use crate::solo::{Cargas, SoloFundacao};
use crate::spt::{CondicaoCoesivos, Sondagem};
use crate::tipos::atirantada::CortinaAtirantada;
use crate::tipos::embutida::{EstacaPrancha, ParedeEstacas};
use crate::tipos::flexao::MuroFlexao;
use crate::tipos::grampeado::SoloGrampeado;
use crate::tipos::gravidade::MuroGravidade;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TipoContencao {
    Gravidade,
    Flexao,
    Atirantada,
    Prancha,
    Estacas,
    Grampeado,
}

pub const TIPOS_CONTENCAO: [TipoContencao; 6] = [
    TipoContencao::Gravidade,
    TipoContencao::Flexao,
    TipoContencao::Atirantada,
    TipoContencao::Prancha,
    TipoContencao::Estacas,
    TipoContencao::Grampeado,
];

impl TipoContencao {
    pub fn nome(&self) -> &'static str {
        match self {
            TipoContencao::Gravidade => "Muro de gravidade",
            TipoContencao::Flexao => "Muro de flexão",
            TipoContencao::Atirantada => "Cortina atirantada",
            TipoContencao::Prancha => "Estaca-prancha",
            TipoContencao::Estacas => "Parede de estacas",
            TipoContencao::Grampeado => "Solo grampeado",
        }
    }
    pub fn descricao(&self) -> &'static str {
        match self {
            TipoContencao::Gravidade => "Pedra argamassada, concreto ciclópico, gabião",
            TipoContencao::Flexao => "Concreto armado em L / T invertido",
            TipoContencao::Atirantada => "Painel de concreto com tirantes (NBR 5629)",
            TipoContencao::Prancha => "Perfis metálicos cravados",
            TipoContencao::Estacas => "Estacas justapostas, secantes ou espaçadas",
            TipoContencao::Grampeado => "Grampos + face em concreto projetado",
        }
    }
    pub fn rotulo_altura(&self) -> &'static str {
        match self {
            TipoContencao::Grampeado => "Altura livre da face H",
            _ => "Altura livre frontal H",
        }
    }
    /// Regra de pré-dimensionamento da geometria a partir de H, se houver.
    pub fn regra_predim(&self) -> Option<&'static str> {
        match self {
            TipoContencao::Gravidade => Some(
                "Marchetti (2007), 2.1 — Fig. 38: base de 40 % a 70 % da altura total (a menor que atende), crista máx(30 cm; 10 % H), altura da base 13 % H, paramento 1:12, ressalto frontal 0,5·F. Embutimento Df = máx(base + 30 cm; 0,10·Ht; 0,40 m) — Marchetti ex. 4.1 e NBR 16920-1 D.6.1; na divisa com vizinho, NBR 6122 7.7.2 pede ≥ 1,5 m.",
            ),
            TipoContencao::Flexao => Some(
                "Marchetti (2007), 2.2 — Fig. 41: base de 40 % a 70 % de H (a menor que atende), parede 20 cm no topo e 9 % H na base, sapata 9 % H, ponta 11 % H. Embutimento Df = máx(sapata + 30 cm; 0,10·H; 0,40 m) — Marchetti e NBR 16920-1 D.6.1; na divisa com vizinho, NBR 6122 7.7.2 pede ≥ 1,5 m.",
            ),
            TipoContencao::Grampeado => Some(
                "Prática corrente (não normativa): Sh = Sv = 1,5 m e 1º grampo a 0,75 m; barra e comprimento dimensionados pela tração dos grampos (ou L de 0,6 a 1,2·H, o menor que atende, com o dimensionamento automático desligado).",
            ),
            TipoContencao::Atirantada => Some(
                "Ficha: a menor (passo 0,25 m, mín. 0,50 m) em que o passivo reduzido menos o ativo abaixo da escavação equilibra a reação na base da cortina.",
            ),
            _ => None,
        }
    }
    /// Rótulo do campo de engastamento, ou `None` se calculado pelo programa.
    pub fn rotulo_engaste(&self) -> Option<&'static str> {
        match self {
            TipoContencao::Gravidade | TipoContencao::Flexao => Some("Embutimento (solo à frente) Df"),
            TipoContencao::Atirantada => Some("Ficha abaixo da escavação"),
            _ => None,
        }
    }
    pub fn arquivo(&self) -> &'static str {
        match self {
            TipoContencao::Gravidade => "muro_gravidade",
            TipoContencao::Flexao => "muro_flexao",
            TipoContencao::Atirantada => "cortina_atirantada",
            TipoContencao::Prancha => "estaca_prancha",
            TipoContencao::Estacas => "parede_estacas",
            TipoContencao::Grampeado => "solo_grampeado",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Geometria {
    /// Altura livre frontal (fachada visível, do terreno à frente até o topo), m.
    pub h: f64,
    /// `true` quando `h` é a altura livre frontal (projetos a partir da 0.3.4).
    /// Projetos antigos (sem o campo) guardavam a altura estrutural dos muros.
    #[serde(default)]
    pub h_livre: bool,
    /// Inclinação do talude de montante (crista), graus.
    pub beta: f64,
    /// Sobrecarga na crista (kPa).
    pub q: f64,
    pub engaste: f64,
}

impl Default for Geometria {
    fn default() -> Self {
        Self { h: 4.0, h_livre: true, beta: 0.0, q: 20.0, engaste: 0.6 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Projeto {
    pub nome: String,
    pub local: String,
    pub responsavel: String,
    pub tipo: TipoContencao,
    pub sondagem: Sondagem,
    /// Reaterro compactado do tardoz (muros); projetos antigos abrem sem especificação.
    #[serde(default = "Reaterro::legado")]
    pub reaterro: Reaterro,
    pub geometria: Geometria,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
    pub drenagem: Drenagem,
    pub fatores: Fatores,
    pub gravidade: MuroGravidade,
    pub flexao: MuroFlexao,
    pub atirantada: CortinaAtirantada,
    pub prancha: EstacaPrancha,
    pub estacas: ParedeEstacas,
    pub grampeado: SoloGrampeado,
    /// Preencher a geometria pelo pré-dimensionamento ao alterar a altura H.
    #[serde(default = "verdadeiro")]
    pub pre_dim_auto: bool,
    /// Registro da otimização aplicada (texto para o memorial).
    #[serde(default)]
    pub nota_otimizacao: String,
    /// Não redefinir a malha dos grampos no pré-dimensionamento (uso do otimizador). Não é salvo.
    #[serde(skip)]
    pub manter_malha: bool,
}

fn verdadeiro() -> bool {
    true
}

/// Arredonda para cima em múltiplos de 5 cm.
pub fn arred(x: f64) -> f64 {
    (x / 0.05 - 1e-9).ceil() * 0.05
}

/// Embutimento mínimo de muros com fundação direta (m): o maior entre
/// 30 cm de solo sobre a base (Marchetti, exemplo 4.1), 0,10 da altura
/// (NBR 16920-1 D.6.1 a, terreno à frente horizontal) e 0,40 m (D.6.1 c).
fn embutimento_minimo(altura: f64, espessura_base: f64) -> f64 {
    arred((espessura_base + 0.30).max(0.10 * altura).max(0.40))
}

impl Default for Projeto {
    fn default() -> Self {
        Self {
            nome: "Novo projeto".into(),
            local: String::new(),
            responsavel: String::new(),
            tipo: TipoContencao::Gravidade,
            sondagem: Sondagem::default(),
            reaterro: Reaterro::default(),
            geometria: Geometria::default(),
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
            drenagem: Drenagem::default(),
            fatores: Fatores::default(),
            gravidade: MuroGravidade::default(),
            flexao: MuroFlexao::default(),
            atirantada: CortinaAtirantada::default(),
            prancha: EstacaPrancha::default(),
            estacas: ParedeEstacas::default(),
            grampeado: SoloGrampeado::default(),
            pre_dim_auto: true,
            nota_otimizacao: String::new(),
            manter_malha: false,
        }
    }
}

impl Projeto {
    /// Aplica as proporções do pré-dimensionamento com a fração `k` da faixa
    /// variável (base dos muros, comprimento dos grampos).
    fn aplicar_proporcoes(&mut self, k: f64) {
        let h = self.geometria.h.max(0.5);
        match self.tipo {
            TipoContencao::Gravidade => {
                // Marchetti, Muros de Arrimo, 2.1 (Fig. 38), com Ht = altura total (corpo + base):
                // base 40–70 % Ht (k) ; topo 30 cm ou 8–15 % Ht → máx(0,30; 10 %) ;
                // altura da base 12–15 % Ht → 13 % ; paramento frontal 1:10 a 1:15 → 1:12 ;
                // ponta 0,5d a d → 0,5·F ; embutimento: ver embutimento_minimo
                // H = altura livre frontal: Ht = H + Df (Df depende de F, que depende de Ht)
                let m = &mut self.gravidade;
                let mut df = embutimento_minimo(1.15 * h, 0.15 * h);
                for _ in 0..30 {
                    m.f = arred(0.13 * (h + df));
                    df = embutimento_minimo(h + df, m.f);
                }
                let ht = h + df;
                m.f = arred(0.13 * ht);
                let base = arred(k * ht);
                m.crista = arred((0.10 * ht).max(0.30));
                m.proj_frente = arred((ht - m.f) / 12.0);
                m.ressalto_frente = arred(0.5 * m.f);
                m.ressalto_tras = arred((0.02 * ht).max(0.10));
                let resto = base - m.ressalto_frente - m.proj_frente - m.crista - m.ressalto_tras;
                m.proj_tardoz = (resto.max(0.0) / 0.05).round() * 0.05;
                self.geometria.engaste = embutimento_minimo(ht, m.f);
            }
            TipoContencao::Flexao => {
                // Marchetti, 2.2 (Fig. 41): topo 20 cm mín. ; parede na base 8–10 % H ;
                // sapata 8–10 % H ; ponta 10–12 % H ; base 40–70 % H (k) ;
                // 30 cm mínimos de solo sobre a ponta (embutimento_minimo)
                // H = altura livre frontal: altura total = H + Df
                let m = &mut self.flexao;
                let mut df = 0.40_f64.max(0.10 * h);
                for _ in 0..30 {
                    df = embutimento_minimo(h + df, arred((0.09 * (h + df)).max(0.25)));
                }
                let h = h + df;
                m.t_topo = 0.20;
                m.t_base = arred((0.09 * h).max(0.20));
                m.h_sapata = arred((0.09 * h).max(0.25));
                m.b_ponta = arred((0.11 * h).max(0.30));
                m.b_talao = arred((k * h - m.b_ponta - m.t_base).max(0.30));
                self.geometria.engaste = embutimento_minimo(h, m.h_sapata);
            }
            TipoContencao::Grampeado => {
                // prática corrente (não normativa): L = k·H (mín. 3 m), malha 1,5 × 1,5 m
                let g = &mut self.grampeado;
                g.comprimento = arred((k * h).max(3.0));
                if !self.manter_malha {
                    g.sh = 1.5;
                    g.sv = 1.5;
                    g.z1 = 0.75;
                }
            }
            _ => {}
        }
    }


    /// Muro de gravidade: alarga a crista (mantendo a base) quando há tração
    /// nas seções do corpo. Retorna `true` se alterou a geometria.
    pub fn alargar_crista_se_preciso(&mut self, falhas: &[String]) -> bool {
        if self.tipo != TipoContencao::Gravidade || !falhas.iter().any(|n| n.starts_with("Tensão mín. no corpo")) {
            return false;
        }
        let g = &mut self.gravidade;
        if g.crista >= 0.8 * (self.geometria.h + self.geometria.engaste) {
            return false;
        }
        // a base fica como está: a crista cresce sobre a projeção do tardoz
        if g.proj_tardoz < 0.05 - 1e-9 {
            return false;
        }
        g.crista = arred(g.crista + 0.05);
        g.proj_tardoz = ((g.proj_tardoz - 0.05).max(0.0) / 0.05).round() * 0.05;
        true
    }

    /// Alarga a crista até eliminar a tração no corpo (sem estabilidade global: rápido).
    fn ajustar_crista(&mut self) {
        for _ in 0..30 {
            let r = self.calcular_local();
            let f: Vec<String> = r.verificacoes.iter().filter(|v| !v.ok()).map(|v| v.nome.clone()).collect();
            if !self.alargar_crista_se_preciso(&f) {
                break;
            }
        }
    }
    /// Aplica as proporções com a fração `k` (uso do otimizador).
    pub fn aplicar_proporcoes_pub(&mut self, k: f64) {
        self.aplicar_proporcoes(k);
    }

    /// Pré-dimensionamento da geometria a partir da altura H: aplica as
    /// proporções de referência e escolhe, dentro da faixa recomendada, a
    /// menor base (ou o menor comprimento de grampo) que atende às
    /// verificações de estabilidade. Retorna um resumo, ou `None` se o tipo
    /// não tem regra (contenções embutidas: a ficha é calculada).
    pub fn pre_dimensionar(&mut self) -> Option<String> {
        if self.tipo == TipoContencao::Atirantada {
            return Some(self.pre_dimensionar_ficha());
        }
        if self.tipo == TipoContencao::Grampeado && self.grampeado.auto_dim {
            self.aplicar_proporcoes(0.6);
            return Some(
                "Pré-dimensionamento aplicado: malha 1,5 × 1,5 m e 1º grampo a 0,75 m; a barra e o comprimento são dimensionados pela tração dos grampos."
                    .into(),
            );
        }
        let (faixa, nome): (&[f64], &str) = match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao => {
                // 40–70 % (Marchetti); muros baixos com sobrecarga podem exigir até 100 %
                (&[0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70, 0.80, 0.90, 1.00], "base")
            }
            TipoContencao::Grampeado => (&[0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2], "comprimento dos grampos L"),
            _ => return None,
        };
        // verificações que a variação da faixa pode resolver (estabilidade)
        let pendentes = |p: &Projeto| -> Option<Vec<String>> {
            let r = p.calcular();
            let estab: Vec<_> = r
                .verificacoes
                .iter()
                .filter(|v| match p.tipo {
                    TipoContencao::Grampeado => !v.nome.starts_with("Face"),
                    _ => v.nome.contains("(estático)") || v.nome.starts_with("Tensão mín. no corpo"),
                })
                .collect();
            if estab.is_empty() {
                return None;
            }
            Some(estab.iter().filter(|v| !v.ok()).map(|v| v.nome.split(" — ").next().unwrap_or(&v.nome).to_string()).collect())
        };
        for &k in faixa {
            self.aplicar_proporcoes(k);
            self.ajustar_crista();
            if pendentes(self).is_some_and(|f| f.is_empty()) {
                let fora = if k > 0.70 + 1e-9 { " (acima dos 70 % usuais: em muros baixos a sobrecarga pesa mais que o solo)" } else { "" };
                let crista = if self.tipo == TipoContencao::Gravidade { format!(" ; crista {} m", crate::resultado::fmt(self.gravidade.crista)) } else { String::new() };
                return Some(format!(
                    "Pré-dimensionamento aplicado: {nome} = {} % de H{fora}{crista}, a menor que atende à estabilidade.",
                    (k * 100.0).round()
                ));
            }
        }
        // muro de gravidade sem dente: a faixa não resolveu o deslizamento → tenta com dente de base
        if self.tipo == TipoContencao::Gravidade && !self.gravidade.dente {
            self.gravidade.dente = true;
            for &k in faixa {
                self.aplicar_proporcoes(k);
                self.ajustar_crista();
                if pendentes(self).is_some_and(|f| f.is_empty()) {
                    return Some(format!(
                        "Pré-dimensionamento aplicado: base = {} % de H com dente de base (sem dente o deslizamento não atendia) ; crista {} m.",
                        (k * 100.0).round(),
                        crate::resultado::fmt(self.gravidade.crista)
                    ));
                }
            }
            self.gravidade.dente = false;
        }
        let k = *faixa.last().unwrap();
        self.aplicar_proporcoes(k);
        self.ajustar_crista();
        let falta = pendentes(self).unwrap_or_default().join(", ");
        let dica = match self.tipo {
            TipoContencao::Grampeado => "aumente o qs, o diâmetro do furo ou reduza o espaçamento",
            _ if falta.contains("deslizamento") => {
                "para o deslizamento, considere dente de base, atrito k·tg φ (Marchetti) ou reveja a sobrecarga"
            }
            _ => "ajuste a geometria manualmente",
        };
        Some(format!(
            "Pré-dimensionamento aplicado com {nome} = {} % de H (limite da faixa). Ainda não atende: {falta} — {dica}.",
            (k * 100.0).round()
        ))
    }

    /// Cortina atirantada: menor ficha que equilibra a reação na base.
    fn pre_dimensionar_ficha(&mut self) -> String {
        const NOME: &str = "Passivo reduzido − ativo na ficha";
        let atende = |p: &Projeto| p.calcular().verificacoes.iter().find(|v| v.nome.starts_with(NOME)).map(|v| v.ok());
        let h = self.geometria.h.max(1.0);
        let max = arred((0.6 * h).max(2.0));
        let mut d = 0.50;
        while d <= max + 1e-9 {
            self.geometria.engaste = d;
            if atende(self) == Some(true) {
                return format!("Pré-dimensionamento aplicado: ficha = {} m, a menor que equilibra a reação na base.", crate::resultado::fmt(d));
            }
            d += 0.25;
        }
        self.geometria.engaste = max;
        format!(
            "Pré-dimensionamento aplicado com ficha = {} m (0,6·H). Ainda não atende: equilíbrio da ficha — \
             apoie a base em rocha, acrescente um nível de tirante mais baixo ou reveja o solo abaixo da escavação.",
            crate::resultado::fmt(max)
        )
    }

    pub fn salvar(&self, caminho: &std::path::Path) -> Result<(), String> {
        let txt = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(caminho, txt).map_err(|e| e.to_string())
    }

    pub fn abrir(caminho: &std::path::Path) -> Result<Self, String> {
        let txt = std::fs::read_to_string(caminho).map_err(|e| e.to_string())?;
        let mut p: Self = serde_json::from_str(&txt).map_err(|e| format!("Arquivo inválido: {e}"))?;
        p.migrar();
        Ok(p)
    }

    /// Converte projetos antigos, que guardavam a altura estrutural dos muros,
    /// para a altura livre frontal.
    pub fn migrar(&mut self) {
        if !self.geometria.h_livre {
            let df = self.geometria.engaste.max(0.0);
            match self.tipo {
                TipoContencao::Gravidade => self.geometria.h = (self.geometria.h + self.gravidade.f - df).max(0.5),
                TipoContencao::Flexao => self.geometria.h = (self.geometria.h - df).max(0.5),
                _ => {}
            }
            self.geometria.h_livre = true;
        }
    }

    /// Altura total de solo contido pela estrutura (para drenagem / NA):
    /// nos muros, da base da fundação ao topo (altura livre + embutimento).
    pub fn altura_contida(&self) -> f64 {
        match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao => self.geometria.h + self.geometria.engaste.max(0.0),
            _ => self.geometria.h,
        }
    }

    fn cargas_calc(&self) -> Cargas {
        let mut c = self.cargas.clone();
        c.q_uniforme = self.geometria.q;
        c
    }

    fn empuxo_calc(&self) -> ParamEmpuxo {
        let mut e = self.empuxo.clone();
        e.beta = self.geometria.beta;
        e
    }

    fn perfil_calc(&self) -> crate::solo::Perfil {
        let na = self.na_calculo();
        // muros: o tardoz é reaterro compactado de material especificado
        if self.reaterro.usar && matches!(self.tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
            return self.reaterro.perfil(&self.sondagem, na);
        }
        self.sondagem.perfil(na)
    }

    /// Cota de assentamento da base (profundidade na sondagem, m).
    pub fn z_base(&self) -> f64 {
        self.altura_contida()
    }

    /// Seção do memorial com o SPT do solo de apoio e do tardoz.
    fn solos_de_projeto(&self) -> Vec<Bloco> {
        let t = |x: String| Bloco::Texto(x);
        let s = &self.sondagem;
        let camada = |z: f64| -> String {
            match s.camada_em(z) {
                Some(c) => format!(
                    "{} — {} (NSPT = {}, {}), γ = {} kN/m³, φ' = {}°, c' = {} kPa",
                    if c.descricao.is_empty() { c.tipo.nome().to_string() } else { c.descricao.clone() },
                    c.tipo.nome().to_lowercase(),
                    fmt(c.nspt),
                    crate::spt::compacidade(c.tipo, c.nspt),
                    fmt(c.gamma),
                    fmt(c.phi),
                    fmt(c.c)
                ),
                None => "não definido".into(),
            }
        };
        let medio = |de: f64, ate: f64| -> f64 {
            let n = 100;
            (0..n).filter_map(|i| s.camada_em(de + (ate - de) * (i as f64 + 0.5) / n as f64)).map(|c| c.nspt).sum::<f64>() / n as f64
        };
        let mut v = vec![Bloco::Titulo("Solos de projeto — apoio da contenção e tardoz".into())];
        match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao => {
                let z = self.z_base();
                v.push(t(format!("Solo de apoio da base (cota de assentamento a z = {} m na sondagem {}): {}", fmt(z), s.identificacao, camada(z + 0.01))));
            }
            TipoContencao::Atirantada | TipoContencao::Prancha | TipoContencao::Estacas => {
                let h = self.geometria.h;
                v.push(t(format!("Solo no pé da escavação (z = {} m na sondagem {}): {}", fmt(h), s.identificacao, camada(h + 0.01))));
                let ficha = if self.tipo == TipoContencao::Atirantada { self.geometria.engaste } else { 0.5 * h };
                v.push(t(format!("Solo da ficha (z = {} a {} m): NSPT médio = {}", fmt(h), fmt(h + ficha), fmt(medio(h, h + ficha)))));
            }
            TipoContencao::Grampeado => {
                v.push(t(format!("Solo do maciço grampeado (z = 0 a {} m): NSPT médio = {}", fmt(self.geometria.h), fmt(medio(0.0, self.geometria.h)))));
            }
        }
        match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao if self.reaterro.usar => {
                let (g, _, c, phi) = self.reaterro.parametros(s);
                v.push(t(format!(
                    "Solo do tardoz: reaterro compactado, {} (NSPT equivalente = {}, {}), γ = {} kN/m³, φ' = {}°, c' = {} kPa — ver o detalhamento do reaterro.",
                    self.reaterro.tipo.nome().to_lowercase(),
                    fmt(self.reaterro.nspt),
                    crate::spt::compacidade(self.reaterro.tipo, self.reaterro.nspt),
                    fmt(g),
                    fmt(phi),
                    fmt(c)
                )));
            }
            TipoContencao::Gravidade | TipoContencao::Flexao => {
                v.push(t(format!(
                    "Solo do tardoz: perfil da sondagem (z = 0 a {} m), NSPT médio = {} — reaterro compactado não especificado.",
                    fmt(self.altura_contida()),
                    fmt(medio(0.0, self.altura_contida()))
                )));
            }
            _ => {
                v.push(t(format!(
                    "Solo do tardoz: terreno natural da sondagem (z = 0 a {} m), NSPT médio = {}.",
                    fmt(self.altura_contida()),
                    fmt(medio(0.0, self.altura_contida()))
                )));
            }
        }
        v
    }

    fn fundacao_em(&self, z: f64, base: &SoloFundacao) -> SoloFundacao {
        let mut f = base.clone();
        if let Some(c) = self.sondagem.camada_em(z + 0.01) {
            f.gamma = c.gamma;
            f.gamma_sat = c.gamma_sat;
            f.phi = c.phi;
            f.c = c.c;
        }
        // NA natural da sondagem sob a fundação: a drenagem do aterro não o
        // rebaixa. O NA do aterro (condição de drenagem) entra no cálculo do muro.
        f.na_abaixo_base = self.sondagem.na().map(|n| n - z);
        f
    }

    pub fn preparar_gravidade(&self) -> MuroGravidade {
        let mut m = self.gravidade.clone();
        // corpo acima da base = altura livre + embutimento − altura da base
        m.h = (self.geometria.h + self.geometria.engaste.max(0.0) - m.f).max(0.3);
        m.df = self.geometria.engaste;
        m.aterro = self.perfil_calc();
        m.reaterro = self.reaterro.usar;
        m.fundacao = self.fundacao_em(m.h + m.f, &m.fundacao);
        m.cargas = self.cargas_calc();
        m.empuxo = self.empuxo_calc();
        m
    }

    pub fn preparar_flexao(&self) -> MuroFlexao {
        let mut m = self.flexao.clone();
        m.concreto.peso = crate::materiais::CONCRETO_ARMADO;
        // altura total (sapata incluída) = altura livre + embutimento
        m.h = self.geometria.h + self.geometria.engaste.max(0.0);
        // sistema misto: parede equivalente (alvenaria + pilares distribuídos) na estabilidade
        if m.sistema == crate::tipos::misto::Sistema::Misto {
            let t = m.misto.t_equivalente();
            m.t_topo = t;
            m.t_base = t;
            if m.misto.fundacao_estacas() {
                // sem sapata corrida: a "base" é a viga baldrame
                m.h_sapata = m.misto.baldrame_h;
                m.b_ponta = 0.0;
                m.b_talao = (m.misto.baldrame_b - t).max(0.0);
                m.dente = false;
            }
            // perfil SPT e cota do topo da fundação para as estacas
            m.misto.spt = self.sondagem.camadas_calc().iter().map(|c| (c.ate, c.nspt, c.tipo)).collect();
            m.misto.z_topo = m.h - m.h_sapata;
        }
        m.df = self.geometria.engaste;
        m.aterro = self.perfil_calc();
        m.reaterro = self.reaterro.usar;
        m.fundacao = self.fundacao_em(m.h, &m.fundacao);
        m.cargas = self.cargas_calc();
        m.empuxo = self.empuxo_calc();
        m
    }

    pub fn preparar_atirantada(&self) -> CortinaAtirantada {
        let mut c = self.atirantada.clone();
        c.concreto.peso = crate::materiais::CONCRETO_ARMADO;
        c.h = self.geometria.h;
        c.ficha = self.geometria.engaste;
        c.perfil = self.perfil_calc();
        c.cargas = self.cargas_calc();
        c.empuxo = self.empuxo_calc();
        c
    }

    fn preparar_parede(&self, p: &mut crate::tipos::embutida::ParedeEmbutida) {
        p.h = self.geometria.h;
        p.perfil = self.perfil_calc();
        p.cargas = self.cargas_calc();
        p.empuxo = self.empuxo_calc();
    }

    pub fn preparar_prancha(&self) -> EstacaPrancha {
        let mut e = self.prancha.clone();
        self.preparar_parede(&mut e.parede);
        e
    }

    pub fn preparar_estacas(&self) -> ParedeEstacas {
        let mut e = self.estacas.clone();
        e.concreto.peso = crate::materiais::CONCRETO_ARMADO;
        self.preparar_parede(&mut e.parede);
        e
    }

    pub fn preparar_grampeado(&self) -> SoloGrampeado {
        let mut g = self.grampeado.clone();
        g.concreto.peso = crate::materiais::CONCRETO_ARMADO;
        g.h = self.geometria.h;
        let (gm, phi, c) = self.sondagem.medias(g.h);
        g.gamma = gm;
        g.phi = phi;
        g.c = c;
        g.cargas = self.cargas_calc();
        g.empuxo = self.empuxo_calc();
        // água no maciço (condição de drenagem): altura acima do pé da escavação
        g.hw = self
            .na_calculo()
            .map(|n| (g.h - n).max(0.0))
            .unwrap_or(0.0);
        if g.auto_dim {
            g.dimensionar(&self.fatores);
        }
        g
    }

    /// Cálculo do tipo de contenção e da drenagem, sem a estabilidade global
    /// (rápido; uso do otimizador nos ajustes estruturais).
    pub fn calcular_local(&self) -> Resultado {
        let f = &self.fatores;
        let mut r = match self.tipo {
            TipoContencao::Gravidade => self.preparar_gravidade().calcular(f),
            TipoContencao::Flexao => self.preparar_flexao().calcular(f),
            TipoContencao::Atirantada => self.preparar_atirantada().calcular(f),
            TipoContencao::Prancha => self.preparar_prancha().calcular(f),
            TipoContencao::Estacas => self.preparar_estacas().calcular(f),
            TipoContencao::Grampeado => self.preparar_grampeado().calcular(f),
        };
        self.drenagem_no_resultado(&mut r);
        r
    }

    pub fn calcular(&self) -> Resultado {
        let f = &self.fatores;
        let mut r = self.calcular_local();
        self.estabilidade_global(&mut r);
        r.desenho.hachurar_solos();
        if self.reaterro.usar && matches!(self.tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
            r.titulo("Detalhamento construtivo — reaterro compactado do tardoz");
            for l in self.reaterro.especificacao(&self.sondagem) {
                r.linha(l);
            }
            if !self.reaterro.tipo.granular() {
                r.aviso("Reaterro coesivo: prefira material granular, que drena melhor e reduz o empuxo; confirme a drenagem do tardoz.");
            }
        } else if matches!(self.tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
            r.aviso("Reaterro do tardoz não especificado: adotado o perfil da sondagem. Defina o material compactado (tipo de solo, NSPT, grau de compactação).");
        }

        // Cabeçalho do memorial
        let t = |x: String| Bloco::Texto(x);
        let mut cab = vec![
            Bloco::Titulo("Identificação".into()),
            t(format!("Projeto: {}", self.nome)),
            t(format!("Local: {}", if self.local.is_empty() { "—" } else { &self.local })),
            t(format!("Responsável técnico: {}", if self.responsavel.is_empty() { "—" } else { &self.responsavel })),
            t(format!("Tipo de contenção: {}", self.tipo.nome())),
            t(match self.tipo {
                TipoContencao::Gravidade | TipoContencao::Flexao => format!(
                    "Altura livre frontal (fachada visível) H = {} m ; embutimento Df = {} m ; altura total da estrutura = {} m",
                    fmt(self.geometria.h),
                    fmt(self.geometria.engaste.max(0.0)),
                    fmt(self.altura_contida())
                ),
                _ => format!("Altura livre frontal (fachada visível) H = {} m", fmt(self.geometria.h)),
            }),
            Bloco::Titulo(match self.sondagem.modo {
                crate::spt::ModoSolo::Completo => format!("Sondagem {} (SPT)", self.sondagem.identificacao),
                crate::spt::ModoSolo::Simplificado => "Solo — entrada simplificada na base (SPT)".into(),
            }),
        ];
        let s = &self.sondagem;
        cab.push(t(format!(
            "NA da sondagem: {} ; NA adotado no cálculo: {}",
            s.na().map(|n| format!("{} m", fmt(n))).unwrap_or("não encontrado".into()),
            self
                .na_calculo()
                .map(|n| format!("{} m (condição: {})", fmt(n), self.drenagem.condicao_efetiva(self.tipo).nome()))
                .unwrap_or(format!("sem água (condição: {})", self.drenagem.condicao_efetiva(self.tipo).nome()))
        )));
        let (eq_phi, nome_phi) = match s.metodo_phi {
            crate::spt::MetodoPhi::Teixeira1996 => ("φ' = √(20·N) + 15  (≤ 45°)", "Teixeira (1996)"),
            crate::spt::MetodoPhi::Godoy1983 => ("φ' = 28 + 0,4·N  (≤ 45°)", "Godoy (1983)"),
        };
        cab.push(Bloco::Formula(Formula {
            titulo: format!("Correlações com o SPT — γ: Godoy (1972) ; φ' granulares: {nome_phi}"),
            equacao: match s.coesivos {
                CondicaoCoesivos::NaoDrenada => format!("{eq_phi} ;  coesivos: Su = 10·N (kPa), φ = 0"),
                CondicaoCoesivos::Drenada => format!("{eq_phi} ;  coesivos: c' e φ' adotados"),
            },
            substituicao: match s.coesivos {
                CondicaoCoesivos::NaoDrenada => "N = NSPT de cada camada (Teixeira e Godoy, 1996)".into(),
                CondicaoCoesivos::Drenada => format!("N = NSPT de cada camada ;  coesivos: c' = {} kPa, φ' = {}°", fmt(s.c_drenado), fmt(s.phi_drenado)),
            },
            resultado: "Parâmetros por camada na tabela abaixo (estimativas a confirmar por ensaios)".into(),
        }));
        cab.push(t(format!("{:<13} {:>5}  {:<34} {:>6} {:>6} {:>6} {:>6}  {}", "Prof. (m)", "NSPT", "Descrição", "γ", "γsat", "c'", "φ'", "origem")));
        if s.modo == crate::spt::ModoSolo::Simplificado {
            cab.push(t(format!(
                "Solo homogêneo adotado no maciço contido e na fundação: {} com NSPT = {} ({}).",
                s.simples_tipo.nome(),
                fmt(s.simples_nspt),
                crate::spt::compacidade(s.simples_tipo, s.simples_nspt)
            )));
        }
        let cs = s.camadas_calc();
        for (i, c) in cs.iter().enumerate() {
            let desc: String = c.descricao.chars().take(34).collect();
            let de = if i == 0 { 0.0 } else { cs[i - 1].ate };
            cab.push(t(format!(
                "{:<13} {:>5}  {:<34} {:>6} {:>6} {:>6} {:>6}  {}",
                format!("{}–{}", fmt(de), fmt(c.ate)),
                fmt(c.nspt),
                desc,
                fmt(c.gamma),
                fmt(c.gamma_sat),
                fmt(c.c),
                fmt(c.phi),
                if c.manual { "manual" } else { "correlação" }
            )));
        }
        if self.tipo == TipoContencao::Grampeado {
            let (g, p, c) = s.medias(self.geometria.h);
            cab.push(t(format!(
                "Solo grampeado: parâmetros médios até H — γ = {} ; φ' = {}° ; c' = {} kPa",
                fmt(g), fmt(p), fmt(c)
            )));
        }
        cab.extend(self.solos_de_projeto());
        if !self.nota_otimizacao.is_empty() {
            cab.push(Bloco::Titulo("Otimização da solução".into()));
            cab.push(t(self.nota_otimizacao.clone()));
            cab.push(t("Critério: menor índice de consumo de material (m³ de concreto/alvenaria, m de tirante ou grampo, perfil) entre as alternativas usuais do tipo que aprovam todas as verificações de estabilidade. Confirme custos unitários reais antes de decidir.".into()));
        }
        // Critérios de segurança adotados
        cab.push(Bloco::Titulo("Critérios de segurança adotados".into()));
        cab.push(Bloco::Norma(crate::normas::NBR11682_TAB3));
        cab.push(t(format!(
            "Níveis de segurança — vidas humanas: {} ; danos materiais e ambientais: {} → FS mínimo de estabilidade = {}{}",
            f.nivel_vidas.nome(),
            f.nivel_danos.nome(),
            fmt(f.fs_global()),
            if f.majorar_variabilidade { " (majorado 10 % — Nota 1)" } else { "" }
        )));
        // NBR 11682 7.3.7.3 — monitoramento obrigatório
        if matches!(
            self.tipo,
            TipoContencao::Atirantada | TipoContencao::Prancha | TipoContencao::Estacas | TipoContencao::Grampeado
        ) {
            r.aviso("Escavação: se a região de influência das deformações atingir obras existentes, o monitoramento com instrumentação geotécnica é obrigatório (NBR 11682 7.3.7.3 b).");
        }
        if self.altura_contida() > 30.0 {
            r.aviso("Altura > 30 m: em área urbana, o monitoramento com instrumentação geotécnica é obrigatório (NBR 11682 7.3.7.3 c).");
        }
        if s.tem_coesivo_padrao() {
            r.aviso("Camadas coesivas com c' e φ' padrão (condição drenada) — ajustar com base em ensaios (NBR 11682 Anexo D).");
        }
        r.aviso("Parâmetros obtidos por correlações com o SPT são estimativas; confirmar com ensaios de laboratório/campo.");
        self.quantitativos(&mut r);
        cab.append(&mut r.blocos);
        r.blocos = cab;
        r.referencias = crate::referencias::gerar(&r);
        r
    }

    /// Geometria da drenagem na seção (mesmas coordenadas do desenho e do
    /// modelo de estabilidade global): barbacãs, filtro, dreno de pé, canaletas e DHP.
    /// Cotas absolutas (na seção) das vigas intermediárias do muro misto — as mesmas do cálculo.
    pub fn niveis_vigas_misto(&self) -> Vec<f64> {
        let fx = self.preparar_flexao();
        if self.tipo != TipoContencao::Flexao || fx.sistema != crate::tipos::misto::Sistema::Misto {
            return Vec::new();
        }
        let hst = fx.h - fx.h_sapata;
        let ph = |z: f64| fx.empuxo.pressao_horizontal_total(&fx.aterro, &fx.cargas, 0.0, z);
        let p_y = |y: f64| ph((hst - y).max(0.0));
        fx.misto.niveis_vigas(&p_y, hst - crate::tipos::misto::H_RESPALDO, self.fatores.gamma_f, fx.concreto.fyk / 1.15).into_iter().map(|y| fx.h_sapata + y).collect()
    }

    pub fn geo_drenagem(&self) -> GeoDrenagem {
        let dr = &self.drenagem;
        let mut gd = GeoDrenagem::default();
        let tb = rad(self.geometria.beta).tan();
        use crate::drenagem::{BARBACA_ENTRADA, BARBACA_SAIDA};
        // barbacã: do tardoz (xt) à face (xf), com caimento para fora; o tubo entra
        // 20 cm no maciço (coleta) e sai 15 cm da face (pingadeira: a água não escorre no paramento)
        // saída do barbacã (geratriz inferior) ≥ terreno acabado à frente + 15 cm
        let y_frente = self.cota_terreno_frente();
        let barb = |gd: &mut GeoDrenagem, xt: f64, xf: f64, y: f64| {
            let i = dr.declividade / 100.0;
            let mut y = y;
            if let Some(yf) = y_frente {
                let saida = y - i * ((xt - xf).abs() + BARBACA_SAIDA) - dr.diam_barbaca_mm / 2000.0;
                y += (yf + crate::drenagem::FOLGA_SAIDA - saida).max(0.0);
            }
            let s = if xt >= xf { 1.0 } else { -1.0 };
            let t = [xt + s * BARBACA_ENTRADA, y + i * BARBACA_ENTRADA];
            let f = [xf - s * BARBACA_SAIDA, y - i * ((xt - xf).abs() + BARBACA_SAIDA)];
            gd.barbacas.push((t, f));
            gd.larguras.push((t[0] - f[0]).abs());
            gd.paredes.push(([xt, y], [xf, y - i * (xt - xf).abs()]));
        };
        match self.tipo {
            TipoContencao::Gravidade => {
                let m = self.preparar_gravidade();
                let g = m.geo();
                let y0 = m.df.max(m.f);
                for v in dr.filas(g.htot - y0) {
                    let y = y0 + v;
                    let (xl, xr) = m.faces(y);
                    barb(&mut gd, xr, xl, y);
                }
                let yt = g.htot - 0.3;
                gd.filtro = Some(([g.x_pt, m.f], [m.faces(yt).1, yt]));
                gd.altura_filtro = yt - m.f;
                if dr.canaleta_crista {
                    let x0 = g.x_cr + dr.esp_filtro + 0.30;
                    gd.canaletas.push([x0, g.htot + (x0 - g.x_cr) * tb]);
                    if m.df > 0.0 {
                        gd.canaletas.push([-0.45, m.df]);
                    }
                }
            }
            TipoContencao::Flexao => {
                let m = self.preparar_flexao();
                let (bp, tbs, tt, hs, h) = (m.b_ponta, m.t_base, m.t_topo, m.h_sapata, m.h);
                let misto = m.sistema == crate::tipos::misto::Sistema::Misto;
                // misto: o barbacã atravessa só o pano de alvenaria (espessura do bloco)
                let xr = |y: f64| if misto { bp + m.misto.bloco } else { bp + tbs - (tbs - tt) * ((y - hs) / (h - hs)).clamp(0.0, 1.0) };
                // misto: fora das vigas intermediárias e da viga de respaldo (folga de 5 cm)
                let vigas = if misto { self.niveis_vigas_misto() } else { Vec::new() };
                let folga = 0.5 * m.misto.viga_h + 0.05 + dr.diam_barbaca_mm / 2000.0;
                let livre = |y: f64| !misto || (vigas.iter().all(|yv| (y - yv).abs() > folga) && y < h - crate::tipos::misto::H_RESPALDO - 0.05);
                let y0 = m.df.max(hs);
                for v in dr.filas(h - y0) {
                    // fila que cai numa viga: desloca para o pano mais próximo (acima ou abaixo)
                    let y = y0 + v;
                    if let Some(y) = [y, y + 2.0 * folga, y - 2.0 * folga].into_iter().find(|&y| y > y0 + 0.05 && livre(y)) {
                        barb(&mut gd, xr(y), bp, y);
                    }
                }
                let yt = h - 0.3;
                gd.filtro = Some(([bp + tbs, hs], [xr(yt), yt]));
                gd.altura_filtro = yt - hs;
                if dr.canaleta_crista {
                    let x0 = bp + tt + dr.esp_filtro + 0.30;
                    gd.canaletas.push([x0, h + (x0 - bp - tt) * tb]);
                    if m.df > 0.0 {
                        gd.canaletas.push([-0.45, m.df]);
                    }
                }
            }
            TipoContencao::Atirantada => {
                let c = self.preparar_atirantada();
                for v in dr.filas(c.h) {
                    barb(&mut gd, 0.0, -c.espessura, -c.h + v);
                }
                gd.filtro = Some(([0.0, -c.h], [0.0, -0.3]));
                gd.altura_filtro = c.h - 0.3;
                if dr.canaleta_crista {
                    let x0 = dr.esp_filtro + 0.30;
                    gd.canaletas.push([x0, x0 * tb]);
                    gd.canaletas.push([-c.espessura - 0.45, -c.h]);
                }
                if dr.dhp {
                    let mut y = -c.h + 0.5;
                    while y < -0.5 {
                        gd.dhp.push(([0.0, y], [dr.dhp_comprimento, y + 0.05 * dr.dhp_comprimento]));
                        y += 4.0;
                    }
                }
            }
            TipoContencao::Grampeado => {
                let g = self.preparar_grampeado();
                let eta = rad(g.incl_face);
                let e = g.espessura_face.max(0.02 * g.h);
                for v in dr.filas(g.h) {
                    let xf = v * eta.tan();
                    barb(&mut gd, xf, xf - e, v);
                }
                let yt = g.h - 0.3;
                gd.filtro = Some(([0.0, 0.0], [yt * eta.tan(), yt]));
                gd.altura_filtro = yt;
                if dr.canaleta_crista {
                    let x_ft = g.h * eta.tan();
                    let x0 = x_ft + dr.esp_filtro + 0.30;
                    gd.canaletas.push([x0, g.h + (x0 - x_ft) * tb]);
                    gd.canaletas.push([-e - 0.45, 0.0]);
                }
                if dr.dhp {
                    let mut y = 0.5;
                    while y < g.h - 0.5 {
                        let xf = y * eta.tan();
                        gd.dhp.push(([xf, y], [xf + dr.dhp_comprimento, y + 0.05 * dr.dhp_comprimento]));
                        y += 4.0;
                    }
                }
            }
            TipoContencao::Prancha | TipoContencao::Estacas => {}
        }
        if !dr.filtro {
            gd.filtro = None;
        }
        // brita + geotêxtil: bolsas drenantes só nos barbacãs e envelope no dreno de pé
        if dr.material_filtro == crate::drenagem::MaterialFiltro::BritaGeotextil {
            gd.filtro_continuo = false;
            let l = crate::drenagem::BOLSA_LADO.max(dr.esp_filtro.min(0.5));
            if dr.filtro {
                for (t, _) in gd.paredes.clone() {
                    gd.bolsas.push([[t[0], t[1] - l / 2.0], [t[0] + l, t[1] - l / 2.0], [t[0] + l, t[1] + l / 2.0], [t[0], t[1] + l / 2.0]]);
                }
            }
            if let (Some((p0, _)), true) = (gd.filtro, dr.dreno_pe) {
                gd.envelope_pe = Some([p0, [p0[0] + l, p0[1]], [p0[0] + l, p0[1] + l], [p0[0], p0[1] + l]]);
            }
            gd.altura_filtro = 0.0;
        } else {
            gd.filtro_continuo = gd.filtro.is_some();
        }
        // dreno de pé ACIMA do terreno acabado à frente (reaterro frontal), com saídas
        // pelo paramento: geratriz inferior da saída ≥ terreno + 15 cm, escoamento livre
        // por gravidade (abaixo dele fica a lâmina residual considerada no empuxo)
        gd.y_frente = self.cota_terreno_frente();
        if let (Some((p0, _)), true, Some(yf)) = (gd.filtro, dr.dreno_pe, gd.y_frente) {
            let yf = if dr.pe_em_rede { f64::NEG_INFINITY } else { yf };
            use crate::drenagem::{BARBACA_SAIDA, FOLGA_SAIDA};
            let r_t = (dr.diam_dreno_pe_mm / 2000.0).max(0.03);
            let l_env = crate::drenagem::BOLSA_LADO.max(dr.esp_filtro.min(0.5));
            let (x, y_orig) = match gd.envelope_pe {
                Some(_) => (p0[0] + l_env / 2.0, p0[1] + l_env / 2.0),
                None => (p0[0] + dr.esp_filtro / 2.0, p0[1] + 1.4 * r_t),
            };
            let face = |y: f64| -> f64 {
                match self.tipo {
                    TipoContencao::Gravidade => self.preparar_gravidade().faces(y).0,
                    TipoContencao::Flexao => self.preparar_flexao().b_ponta,
                    TipoContencao::Atirantada => -self.preparar_atirantada().espessura,
                    TipoContencao::Grampeado => {
                        let g = self.preparar_grampeado();
                        y * rad(g.incl_face).tan() - g.espessura_face.max(0.02 * g.h)
                    }
                    _ => 0.0,
                }
            };
            let i = dr.declividade.max(1.0) / 100.0;
            let comp = (x - face(y_orig)).abs() + BARBACA_SAIDA;
            let y_c = y_orig.max(yf + FOLGA_SAIDA + r_t + i * comp);
            let xf = face(y_c);
            gd.dreno_pe = Some([x, y_c]);
            gd.saida_pe = (!dr.pe_em_rede).then_some(([x, y_c], [xf - BARBACA_SAIDA, y_c - i * ((x - xf).abs() + BARBACA_SAIDA)]));
            if gd.envelope_pe.is_some() {
                let h = l_env / 2.0;
                gd.envelope_pe = Some([[x - h, y_c - h], [x + h, y_c - h], [x + h, y_c + h], [x - h, y_c + h]]);
            }
        }
        gd
    }

    fn drenagem_no_resultado(&self, r: &mut Resultado) {
        let dr = &self.drenagem;
        let gd = self.geo_drenagem();
        let d = &mut r.desenho;
        for (t, f) in &gd.barbacas {
            d.linha(crate::resultado::Estilo::Barbaca, vec![*t, *f]);
        }
        if let (Some((p0, p1)), true) = (gd.filtro, gd.filtro_continuo) {
            let e = dr.esp_filtro;
            d.vazios.push(vec![p0, [p0[0] + e, p0[1]], [p1[0] + e, p1[1]], p1]);
            dr.faixa_filtro(d, p0, p1);
        }
        for b in gd.bolsas.iter().chain(gd.envelope_pe.iter()) {
            d.vazios.push(b.to_vec());
            d.poli(crate::resultado::Estilo::Dreno, b.to_vec());
            d.pedras(&b.to_vec(), 0.08);
        }
        // dreno de pé (na cota acima do reaterro frontal) e a saída através do paramento
        if let Some(c) = gd.dreno_pe {
            d.circulo(crate::resultado::Estilo::Barbaca, c, (dr.diam_dreno_pe_mm / 2000.0).max(0.03));
        }
        if let Some((a, b)) = gd.saida_pe {
            d.tracejada(crate::resultado::Estilo::Barbaca, vec![a, b]);
        }
        for c in &gd.canaletas {
            d.canaleta(c[0], c[1]);
        }
        for (a, b) in &gd.dhp {
            d.tracejada(crate::resultado::Estilo::Barbaca, vec![*a, *b]);
        }
        dr.relatar(r, self.altura_contida(), &gd.larguras, gd.altura_filtro, self.tipo);
        if !gd.bolsas.is_empty() || gd.envelope_pe.is_some() {
            dr.relatar_bolsas(r, gd.bolsas.len(), gd.envelope_pe.is_some());
        }
        // saídas acima do reaterro frontal (escoamento livre, sem afogamento nem colmatação)
        if let Some(yf) = gd.y_frente {
            use crate::drenagem::FOLGA_SAIDA;
            use crate::resultado::{fmt, Verificacao};
            let d_b = dr.diam_barbaca_mm / 2000.0;
            let folga_barb = gd.barbacas.iter().map(|(_, f)| f[1] - d_b - yf).fold(f64::INFINITY, f64::min);
            if folga_barb.is_finite() {
                r.verif(Verificacao::min("Barbacãs: saída acima do terreno acabado à frente (folga ≥ 15 cm)", folga_barb, FOLGA_SAIDA, "m"));
            }
            if let (Some(c), Some((_, b))) = (gd.dreno_pe, gd.saida_pe) {
                let r_t = (dr.diam_dreno_pe_mm / 2000.0).max(0.03);
                r.linha(format!(
                    "Dreno de pé com eixo a {} m (geratriz inferior {} m) e saídas Ø{} mm pelo paramento a cada {} m, descarga a {} m: acima do terreno acabado à frente ({} m), com escoamento livre para a canaleta de pé.",
                    fmt(c[1]), fmt(c[1] - r_t), format!("{:.0}", dr.diam_barbaca_mm), fmt(crate::drenagem::ESP_SAIDA_PE), fmt(b[1] - d_b), fmt(yf)
                ));
                r.verif(Verificacao::min("Dreno de pé: saída acima do terreno acabado à frente (folga ≥ 15 cm)", b[1] - d_b - yf, FOLGA_SAIDA, "m"));
            }
            if dr.dreno_pe && dr.pe_em_rede {
                r.linha("Dreno de pé na base, com declividade longitudinal ≥ 1 % até caixas de passagem/rede pluvial com fundo abaixo da geratriz inferior do dreno (descarga por gravidade, sem saída pelo paramento) ; sem lâmina residual no tardoz.");
            }
            let hres = self.lamina_residual();
            if hres > 1e-6 && dr.condicao_efetiva(self.tipo) != crate::drenagem::Condicao::SemDrenagem {
                r.formula(
                    "Lâmina d'água residual abaixo da saída mais baixa",
                    "hw,res = cota da saída mais baixa − base da altura contida ;  u = γw·(hw,res − z')  (empuxo e subpressão)",
                    format!("terreno à frente na cota {} m ; folga mínima das saídas {} m", fmt(yf), fmt(FOLGA_SAIDA)),
                    format!("hw,res = {} m — considerada no empuxo, na subpressão e na estabilidade global", fmt(hres)),
                );
            }
            r.verif(Verificacao::min("Barbacãs: declividade ≥ 3 % para fora (critério de projeto)", dr.declividade, 3.0, "%"));
            if dr.dreno_pe {
                r.verif(Verificacao::min("Dreno de pé: Ø ≥ 100 mm (critério de projeto)", dr.diam_dreno_pe_mm, 100.0, "mm"));
            }
            if dr.filtro && dr.material_filtro != crate::drenagem::MaterialFiltro::Geocomposto {
                r.verif(Verificacao::min("Geotêxtil não tecido: gramatura ≥ 200 g/m² (filtro contra colmatação)", dr.gramatura, 200.0, "g/m²"));
            }
            r.linha(
                "Reaterro frontal: solo compactado em camadas (≥ 95 % do Proctor normal, NBR 7182), superfície acabada com caimento ≥ 2 % para a canaleta de pé, sem cobrir nem obstruir as saídas de barbacãs e do dreno de pé ; camada superficial pouco permeável (solo argiloso ou grama) para reduzir a infiltração junto ao pé do muro.",
            );
        }
    }
}


impl Projeto {
    /// Quantitativos por metro de contenção: resumo de aço (por diâmetro) e demais
    /// materiais (concreto por elemento, fôrmas, alvenaria, graute, estacas, tirantes,
    /// grampos, perfis e drenagem). Valores teóricos, sem perdas.
    fn quantitativos(&self, r: &mut Resultado) {
        use crate::armadura::ContextoResumo;
        use crate::resultado::{fmt, Verificacao};
        let verif = |r: &Resultado, prefixo: &str| r.verificacoes.iter().find(|v| v.nome.starts_with(prefixo)).map(|v| v.valor);
        let f0 = |x: f64| format!("{x:.0}");
        // (material, elemento, quantidade, unidade)
        let mut q: Vec<(&str, String, f64, &str)> = Vec::new();
        let mut cx = ContextoResumo::default();
        match self.tipo {
            TipoContencao::Gravidade => {
                let g = self.preparar_gravidade();
                let gg = g.geo();
                let corpo = crate::estabilidade::centroide(&g.poligono()).0;
                let dente = if g.dente { g.dente_b * g.dente_h } else { 0.0 };
                q.push(("Corpo do muro", g.material.nome().to_string(), corpo + dente, "m³/m"));
                if matches!(g.material, crate::tipos::gravidade::Material::ConcretoCiclopico | crate::tipos::gravidade::Material::ConcretoSimples) {
                    q.push(("Fôrmas", "faces do corpo e da base".into(), (gg.htot - g.f) * 2.0 + 2.0 * g.f, "m²/m"));
                }
                q.push(("Concreto magro", "lastro e = 5 cm".into(), 0.05 * gg.base, "m³/m"));
            }
            TipoContencao::Flexao => {
                let m = self.preparar_flexao();
                let (bp, tb, tt, hs, h) = (m.b_ponta, m.t_base, m.t_topo, m.h_sapata, m.h);
                let b = bp + tb + m.b_talao;
                let hst = h - hs;
                cx.altura = hst;
                cx.largura_sapata = b;
                if m.sistema == crate::tipos::misto::Sistema::Misto {
                    let mi = &m.misto;
                    let s = mi.espac_pilar.max(0.5);
                    let n_vigas = self.niveis_vigas_misto().len() as f64;
                    let hr = crate::tipos::misto::H_RESPALDO;
                    cx.pilares_por_m = 1.0 / s;
                    let fck = m.concreto.fck;
                    q.push(("Concreto", format!("pilares {}×{} cm (C{})", f0(100.0 * mi.pilar_b), f0(100.0 * mi.pilar_h), f0(fck)), mi.pilar_b * mi.pilar_h * hst / s, "m³/m"));
                    q.push(("Concreto", format!("{} viga(s) intermediária(s) + respaldo", n_vigas), mi.viga_b * mi.viga_h * n_vigas + mi.bloco * hr, "m³/m"));
                    let area_alv = (hst - hr - n_vigas * mi.viga_h).max(0.0) * (1.0 - mi.pilar_b / s);
                    q.push(("Alvenaria estrutural", format!("bloco de concreto {} cm, fbk {} MPa", f0(100.0 * mi.bloco), f0(mi.fbk)), area_alv, "m²/m"));
                    q.push(("Blocos", "39 × 19 cm (12,5 blocos/m²)".into(), 12.5 * area_alv, "un/m"));
                    // graute nos furos armados: furo ≈ 0,13 × (t − 0,05) m a cada espaçamento das barras do painel
                    let s_v = r.armaduras.iter().find(|a| a.pos == "N2").and_then(|a| a.espac_cm).unwrap_or(40.0) / 100.0;
                    q.push(("Graute", "furos armados do painel (fgk ≥ 2·fbk)".into(), area_alv / 1.0 * 0.13 * (mi.bloco - 0.05) / s_v, "m³/m"));
                    q.push(("Fôrmas", "pilares (3 faces), vigas (2 faces)".into(), (2.0 * (mi.pilar_h - mi.bloco) + mi.pilar_b) * hst / s + 2.0 * mi.viga_h * n_vigas, "m²/m"));
                    if mi.fundacao_estacas() {
                        let (_, lb, bb, hb) = mi.geometria_bloco();
                        let l = verif(r, "Estaca: comprimento").unwrap_or(0.0);
                        let ap = std::f64::consts::PI * mi.diam_estaca * mi.diam_estaca / 4.0;
                        cx.estacas_por_m = 2.0 / s;
                        q.push(("Concreto", "viga baldrame".into(), mi.baldrame_b * mi.baldrame_h, "m³/m"));
                        q.push(("Concreto", format!("blocos {}×{}×{} m", fmt(lb), fmt(bb), fmt(hb)), lb * bb * hb / s, "m³/m"));
                        q.push(("Estacas escavadas", format!("Ø{} cm, L = {} m (2 por bloco)", f0(100.0 * mi.diam_estaca), fmt(l)), 2.0 * l / s, "m/m"));
                        q.push(("Concreto", "estacas".into(), 2.0 * ap * l / s, "m³/m"));
                        q.push(("Fôrmas", "baldrame (2 faces) e blocos".into(), 2.0 * mi.baldrame_h + 2.0 * (lb + bb) * hb / s, "m²/m"));
                    } else {
                        q.push(("Concreto", "sapata corrida".into(), b * hs, "m³/m"));
                        q.push(("Fôrmas", "laterais da sapata".into(), 2.0 * hs, "m²/m"));
                    }
                } else {
                    let dente = if m.dente { m.dente_b * m.dente_h } else { 0.0 };
                    q.push(("Concreto", format!("parede (C{})", f0(m.concreto.fck)), 0.5 * (tt + tb) * hst, "m³/m"));
                    q.push(("Concreto", "sapata + dente".into(), b * hs + dente, "m³/m"));
                    q.push(("Fôrmas", "parede (2 faces) e sapata".into(), 2.0 * hst + 2.0 * hs, "m²/m"));
                }
                q.push(("Concreto magro", "lastro e = 5 cm".into(), 0.05 * b, "m³/m"));
            }
            TipoContencao::Atirantada => {
                let c = self.preparar_atirantada();
                cx.altura = c.h + c.ficha;
                let l_tir: f64 = r.reforcos.iter().map(|f| f.comprimento).sum();
                q.push(("Concreto", format!("cortina e = {} m", fmt(c.espessura)), c.espessura * (c.h + c.ficha), "m³/m"));
                q.push(("Fôrmas", "face externa da cortina".into(), c.h, "m²/m"));
                q.push(("Tirantes", format!("{} nível(is) a cada {} m", r.reforcos.len(), fmt(c.espac_h)), l_tir / c.espac_h.max(0.1), "m/m"));
            }
            TipoContencao::Estacas => {
                let e = self.preparar_estacas();
                let lt = verif(r, "Comprimento total").unwrap_or(0.0);
                cx.estacas_por_m = 1.0 / e.espacamento.max(0.2);
                // telas do fechamento cobrem a altura escavada
                cx.altura = self.geometria.h;
                q.push(("Estacas", format!("Ø{} cm a cada {} m, L = {} m", f0(100.0 * e.diametro), fmt(e.espacamento), fmt(lt)), lt / e.espacamento.max(0.2), "m/m"));
                q.push(("Concreto", "estacas".into(), std::f64::consts::PI * e.diametro * e.diametro / 4.0 * lt / e.espacamento.max(0.2), "m³/m"));
                let bv = (e.diametro + 0.20).max(0.40);
                q.push(("Concreto", format!("viga de coroamento {}×{} m", fmt(bv), fmt(e.viga_h)), bv * e.viga_h, "m³/m"));
                if let Some((esp, ..)) = e.dimensionar_fechamento(self.fatores.gamma_f, self.fatores.gamma_c, self.fatores.gamma_s) {
                    q.push(("Concreto projetado", format!("fechamento entre estacas e = {} cm", f0(100.0 * esp)), esp * (self.geometria.h - e.viga_h).max(0.0), "m³/m"));
                }
            }
            TipoContencao::Prancha => {
                let e = self.preparar_prancha();
                let lt = verif(r, "Comprimento total").unwrap_or(0.0);
                q.push(("Estaca-prancha", format!("{} , L = {} m", e.nome_perfil, fmt(lt)), e.peso_parede() * lt, "kg/m"));
            }
            TipoContencao::Grampeado => {
                let g = self.preparar_grampeado();
                let n_lin = ((g.h - g.z1) / g.sv.max(0.3)).floor().max(0.0) + 1.0;
                let face = g.h / g.incl_face.to_radians().cos();
                cx.grampos_por_m = n_lin / g.sh.max(0.3);
                cx.altura = face;
                q.push(("Grampos", format!("{} linhas a cada {} m, L = {} m", n_lin, fmt(g.sh), fmt(g.comprimento)), n_lin * g.comprimento / g.sh.max(0.3), "m/m"));
                q.push(("Calda de cimento", format!("furos Ø{} cm", f0(100.0 * g.diam_furo)), n_lin * g.comprimento / g.sh.max(0.3) * std::f64::consts::PI * g.diam_furo * g.diam_furo / 4.0, "m³/m"));
                q.push(("Concreto projetado", format!("face e = {} m", fmt(g.espessura_face)), g.espessura_face * face, "m³/m"));
            }
        }
        // drenagem
        let gd = self.geo_drenagem();
        let dr = &self.drenagem;
        if !gd.barbacas.is_empty() {
            q.push(("Tubo PVC", format!("barbacãs Ø{} mm", f0(dr.diam_barbaca_mm)), gd.larguras.iter().sum::<f64>() / dr.esp_h.max(0.3), "m/m"));
        }
        if dr.dreno_pe && gd.filtro.is_some() {
            q.push(("Tubo PVC perfurado", format!("dreno de pé Ø{} mm", f0(dr.diam_dreno_pe_mm)), 1.0, "m/m"));
            if let Some((a, b)) = gd.saida_pe {
                q.push(("Tubo PVC", format!("saídas do dreno de pé Ø{} mm a cada {} m", f0(dr.diam_barbaca_mm), fmt(crate::drenagem::ESP_SAIDA_PE)), (a[0] - b[0]).abs() / crate::drenagem::ESP_SAIDA_PE, "m/m"));
            }
        }
        if !gd.canaletas.is_empty() {
            q.push(("Canaleta de concreto", "meia-cana / U (crista e pé)".into(), gd.canaletas.len() as f64, "m/m"));
        }
        if !gd.dhp.is_empty() {
            q.push(("DHP", format!("tubos de {} m", fmt(dr.dhp_comprimento)), gd.dhp.len() as f64 * dr.dhp_comprimento / dr.dhp_espacamento.max(0.5), "m/m"));
        }

        crate::armadura::resumo_por_metro(r, &cx);
        if q.is_empty() {
            return;
        }
        r.titulo("Resumo dos demais materiais por metro de contenção");
        r.linha(format!("{:<24} {:<52} {:>10} {:<6}", "Material", "Elemento", "Quant.", "Un."));
        for (m, e, v, u) in &q {
            r.linha(format!("{:<24} {:<52} {:>10} {:<6}", m, e.chars().take(52).collect::<String>(), fmt(*v), u));
        }
        let concreto: f64 = q.iter().filter(|x| x.0 == "Concreto").map(|x| x.2).sum();
        if concreto > 0.0 {
            r.linha(format!("Concreto estrutural total: {} m³/m.", fmt(concreto)));
            r.verif(Verificacao::info("Concreto estrutural por metro de contenção", concreto, "m³/m"));
            let aco = verif(r, "Aço total por metro").unwrap_or(0.0);
            if aco > 0.0 {
                r.verif(Verificacao::info("Taxa de aço (aço / concreto estrutural)", aco / concreto, "kg/m³"));
            }
        }
        r.linha("Valores teóricos (sem perdas). Brita, geotêxtil e areia da drenagem: ver a seção “Drenagem”.");
    }
}


impl Projeto {
    /// Cota do terreno acabado à frente da contenção (reaterro frontal / pé da escavação),
    /// nas coordenadas da seção de cada tipo; `None` nas paredes sem drenagem.
    pub fn cota_terreno_frente(&self) -> Option<f64> {
        match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao => Some(self.geometria.engaste.max(0.0)),
            TipoContencao::Atirantada => Some(-self.geometria.h),
            TipoContencao::Grampeado => Some(0.0),
            _ => None,
        }
    }

    /// Lâmina d'água residual no tardoz (m acima da base da altura contida): a água
    /// abaixo da saída mais baixa (geratriz inferior do dreno de pé ou saída da 1ª fila
    /// de barbacãs) não é drenada por gravidade. As saídas ficam ≥ 15 cm acima do terreno
    /// à frente; abaixo delas, a água permanece e entra no empuxo e na subpressão.
    pub fn lamina_residual(&self) -> f64 {
        use crate::drenagem::FOLGA_SAIDA;
        let dr = &self.drenagem;
        let Some(frente) = self.cota_terreno_frente() else { return 0.0 };
        // base da altura contida, nas mesmas coordenadas
        let (base, topo_fundacao) = match self.tipo {
            TipoContencao::Gravidade => (0.0, self.gravidade.f),
            TipoContencao::Flexao => {
                let hs = if self.flexao.sistema == crate::tipos::misto::Sistema::Misto && self.flexao.misto.fundacao_estacas() {
                    self.flexao.misto.baldrame_h
                } else {
                    self.flexao.h_sapata
                };
                (0.0, hs)
            }
            TipoContencao::Atirantada => (-self.geometria.h, -self.geometria.h),
            _ => (0.0, 0.0),
        };
        let saida = if dr.dreno_pe && dr.filtro && dr.pe_em_rede {
            return 0.0; // descarga longitudinal em caixa/rede abaixo do dreno: sem lâmina residual
        } else if dr.dreno_pe && dr.filtro {
            topo_fundacao.max(frente + FOLGA_SAIDA)
        } else if dr.barbacas {
            frente + dr.altura_1a_fila.max(FOLGA_SAIDA)
        } else {
            return 0.0;
        };
        (saida - base).max(0.0)
    }

    /// Nível d'água no tardoz adotado no cálculo (profundidade a partir do topo):
    /// condição de drenagem + lâmina residual abaixo das saídas + NA da sondagem.
    pub fn na_calculo(&self) -> Option<f64> {
        let altura = self.altura_contida();
        let na = self.drenagem.na_efetivo(self.sondagem.na(), altura, self.tipo);
        let hres = self.lamina_residual();
        if hres <= 1e-6 || self.drenagem.condicao_efetiva(self.tipo) == crate::drenagem::Condicao::SemDrenagem {
            return na;
        }
        let z = (altura - hres).max(0.0);
        Some(na.map_or(z, |n| n.min(z)))
    }
}

/// Geometria da drenagem na seção transversal.
#[derive(Clone, Debug, Default)]
pub struct GeoDrenagem {
    /// Barbacãs: (ponto no tardoz, ponto na face), já com a declividade.
    pub barbacas: Vec<([f64; 2], [f64; 2])>,
    /// Faixa drenante junto ao tardoz: ponto inferior e superior (espessura em +x).
    pub filtro: Option<([f64; 2], [f64; 2])>,
    /// Canaletas: centro e cota do topo.
    pub canaletas: Vec<[f64; 2]>,
    /// Drenos horizontais profundos: início e fim.
    pub dhp: Vec<([f64; 2], [f64; 2])>,
    /// Comprimento de cada barbacã (tubo inteiro, com entrada e saída).
    pub larguras: Vec<f64>,
    pub altura_filtro: f64,
    /// Trecho do barbacã dentro da parede (tardoz, face).
    pub paredes: Vec<([f64; 2], [f64; 2])>,
    /// Camada drenante contínua (areia ou geocomposto) em toda a altura.
    pub filtro_continuo: bool,
    /// Bolsas de brita envoltas em geotêxtil na entrada de cada barbacã (seção).
    pub bolsas: Vec<[[f64; 2]; 4]>,
    /// Envelope de brita + geotêxtil do dreno de pé (seção).
    pub envelope_pe: Option<[[f64; 2]; 4]>,
    /// Dreno de pé: centro do tubo (seção) e saída através do paramento (início, fim).
    pub dreno_pe: Option<[f64; 2]>,
    pub saida_pe: Option<([f64; 2], [f64; 2])>,
    /// Cota do terreno acabado à frente (reaterro frontal).
    pub y_frente: Option<f64>,
}

#[cfg(test)]
mod testes_predim {
    use super::*;

    #[test]
    fn gravidade_proporcoes_do_marchetti() {
        // Exemplo 4.1 do livro: corpo 5,80 m + base 0,90 m (Ht = 6,70 m), Df = 1,20 m → altura livre 5,50 m
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Gravidade;
        p.geometria.h = 5.5;
        assert!(p.pre_dimensionar().is_some());
        let g = &p.gravidade;
        let ht = p.geometria.h + p.geometria.engaste;
        assert!((ht - 6.70).abs() < 1e-9, "Ht = {ht}");
        assert!((g.f - 0.90).abs() < 1e-9, "F = {}", g.f);
        assert!((g.proj_frente - 0.50).abs() < 1e-9, "paramento 1:12 ≈ 0,48 → {}", g.proj_frente);
        assert!((g.ressalto_frente - 0.45).abs() < 1e-9 && (g.ressalto_tras - 0.15).abs() < 1e-9);
        let base = g.ressalto_frente + g.proj_frente + g.crista + g.proj_tardoz + g.ressalto_tras;
        assert!(base >= 0.40 * ht - 1e-9 && base <= 1.00 * ht + 0.05, "base {base} fora de 40–100 % de {ht}");
        assert!((p.geometria.engaste - 1.20).abs() < 1e-9, "embutimento = base + 30 cm");
    }

    #[test]
    fn migra_altura_de_projeto_antigo() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Gravidade;
        p.geometria.h = 5.8; // corpo, no formato antigo
        p.gravidade.f = 0.9;
        p.geometria.engaste = 1.2;
        p.geometria.h_livre = false;
        p.migrar();
        assert!((p.geometria.h - 5.5).abs() < 1e-9);
        assert!((p.preparar_gravidade().h - 5.8).abs() < 1e-9, "estrutura inalterada");
        let json = serde_json::to_string(&p).unwrap().replace("\"h_livre\":true,", "");
        let q: Projeto = serde_json::from_str(&json).unwrap();
        assert!(!q.geometria.h_livre, "arquivo sem o campo é do formato antigo");
    }

    #[test]
    fn flexao_proporcoes_do_marchetti() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 5.5;
        p.pre_dimensionar();
        let m = &p.flexao;
        let ht = p.geometria.h + p.geometria.engaste;
        let b = m.b_ponta + m.t_base + m.b_talao;
        assert!((m.t_topo - 0.20).abs() < 1e-9);
        assert!(m.t_base >= 0.08 * ht && m.t_base <= 0.10 * ht + 0.05);
        assert!(m.b_ponta >= 0.10 * ht && m.b_ponta <= 0.12 * ht + 0.05);
        assert!(b >= 0.40 * ht - 1e-9 && b <= 1.00 * ht + 0.10, "B = {b}");
    }

    #[test]
    fn embutimento_minimo_normas() {
        // muro baixo: governa o mínimo absoluto de 0,40 m (NBR 16920-1 D.6.1 c)
        assert!((super::embutimento_minimo(1.0, 0.05) - 0.40).abs() < 1e-9);
        // base fina: 30 cm sobre a base (Marchetti)
        assert!((super::embutimento_minimo(3.0, 0.25) - 0.55).abs() < 1e-9);
        // muro alto com base fina: governa 0,10·H
        assert!((super::embutimento_minimo(9.0, 0.30) - 0.90).abs() < 1e-9);
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 8.0;
        p.pre_dimensionar();
        assert!(p.geometria.engaste >= 0.80 - 1e-9 && p.geometria.engaste >= p.flexao.h_sapata + 0.30 - 1e-9);
    }

    #[test]
    fn atirantada_ficha_pre_dimensionada() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Atirantada;
        p.geometria.h = 6.0;
        let msg = p.pre_dimensionar().expect("atirantada tem regra de ficha");
        eprintln!("{msg}");
        let d = p.geometria.engaste;
        assert!(d >= 0.50 - 1e-9 && d <= 3.60 + 1e-9, "ficha {d}");
        if !msg.contains("Ainda não atende") {
            // a ficha anterior (0,25 m menor) não atendia
            let ok = |p: &Projeto| p.calcular().verificacoes.iter().find(|v| v.nome.starts_with("Passivo reduzido")).map(|v| v.ok());
            assert_eq!(ok(&p), Some(true));
            if d > 0.50 + 1e-9 {
                p.geometria.engaste = d - 0.25;
                assert_eq!(ok(&p), Some(false));
            }
        }
    }

    /// Razão de utilização invertida de cada verificação (≥ 1 atende), por nome.
    fn margens(p: &Projeto) -> std::collections::HashMap<String, f64> {
        use crate::resultado::Criterio;
        p.calcular()
            .verificacoes
            .iter()
            .filter(|v| !v.nome.starts_with("Drenagem:") && !v.nome.starts_with("Barbacãs") && !v.nome.starts_with("Colchão"))
            .filter_map(|v| {
                let m = match v.criterio {
                    Criterio::Minimo if v.limite > 0.0 => v.valor / v.limite,
                    Criterio::Maximo if v.valor > 0.0 => v.limite / v.valor,
                    // demanda informativa (quanto maior, menor a margem)
                    Criterio::Info if (v.nome.starts_with("Ficha de projeto") || v.nome.starts_with("Momento máximo")) && v.valor > 0.0 => {
                        1.0 / v.valor
                    }
                    _ => return None,
                };
                m.is_finite().then(|| (v.nome.clone(), m))
            })
            .collect()
    }

    /// Menor margem do cálculo.
    fn fs_minimo(p: &Projeto) -> f64 {
        margens(p).values().cloned().fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn condicao_de_drenagem_altera_o_calculo() {
        use crate::drenagem::Condicao;
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 5.0;
            // geometria fixa: o dimensionamento automático se adaptaria à água
            p.grampeado.auto_dim = false;
            p.atirantada.auto_dim = false;
            p.pre_dimensionar();
            // NA da sondagem abaixo do pé (6 m): antes, 'sem drenagem' = 'drenada'
            let mut m = Vec::new();
            for c in [Condicao::Drenada, Condicao::Parcial, Condicao::SemDrenagem] {
                p.drenagem.condicao = c;
                m.push(margens(&p));
            }
            let piora = |a: &std::collections::HashMap<String, f64>, b: &std::collections::HashMap<String, f64>| {
                // nenhuma verificação melhora e ao menos uma piora
                let comuns: Vec<_> = a.keys().filter(|k| b.contains_key(*k)).collect();
                // (a carga dos tirantes se redistribui entre os níveis e a reação da ficha:
                // pode cair num apoio quando a água aumenta a reação noutro)
                let por_tirante = |k: &str| {
                    k.starts_with("Tirante")
                        || k.starts_with("Carga máx")
                        || k.starts_with("Punção")
                        // regras de detalhamento (espaçamentos, bitolas): dependem da escolha das barras
                        || ["NBR 6118 20.1", "NBR 6118 18.3", "NBR 6118 18.4", "NBR 6118 22.7"].iter().any(|n| k.contains(n))
                        // ficha calculada cresce com a água, o que pode melhorar a estabilidade global
                        || (k.contains("estabilidade") && matches!(tipo, TipoContencao::Prancha | TipoContencao::Estacas))
                };
                // a busca do círculo crítico é heurística: admite-se 8 % de ruído na estabilidade global
                let folga = |k: &str| if k.contains("estabilidade") { 0.08 * a[k].abs() } else { 1e-6 };
                let nenhuma_melhora = comuns.iter().all(|k| por_tirante(k) || b[*k] <= a[*k] + folga(k));
                let alguma_piora = comuns.iter().any(|k| b[*k] < a[*k] - 1e-6);
                (nenhuma_melhora, alguma_piora)
            };
            let (ok1, p1) = piora(&m[0], &m[1]);
            let (ok2, p2) = piora(&m[0], &m[2]);
            for (idx, nome) in [(1usize, "parcial"), (2, "sem")] {
                for k in m[0].keys().filter(|k| m[idx].contains_key(*k)) {
                    if m[idx][k] > m[0][k] + 1e-6 {
                        eprintln!("    MELHORA ({nome}) {k}: {:.3} -> {:.3}", m[0][k], m[idx][k]);
                    }
                }
            }
            eprintln!("{}: parcial piora {p1} (sem melhora {ok1}) | sem drenagem piora {p2} (sem melhora {ok2})", tipo.nome());
            assert!(ok1 && ok2, "{}: a água não pode melhorar nenhuma verificação", tipo.nome());
            assert!(p1 && p2, "{}: a condição de drenagem deve alterar o cálculo", tipo.nome());
        }
    }

    #[test]
    fn drenada_exige_sistema_eficaz() {
        use crate::drenagem::Condicao;
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Gravidade;
        p.geometria.h = 4.0;
        p.drenagem.condicao = Condicao::Drenada;
        let ok = fs_minimo(&p);
        p.drenagem.filtro = false;
        let sem_filtro = fs_minimo(&p);
        assert!(sem_filtro < ok - 1e-6, "sem camada drenante a condição 'drenada' não se sustenta");
        let r = p.calcular();
        assert!(r.verificacoes.iter().any(|v| v.nome.starts_with("Drenagem:") && !v.ok()));
        // DHP sustenta a drenagem da cortina atirantada mesmo sem barbacãs
        p.tipo = TipoContencao::Atirantada;
        p.drenagem.barbacas = false;
        p.drenagem.dhp = true;
        assert!(p.drenagem.eficaz(p.tipo));
    }

    #[test]
    fn grampeado_talude_e_agua() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Grampeado;
        p.geometria.h = 6.0;
        p.pre_dimensionar();
        let plano = fs_minimo(&p);
        p.geometria.beta = 15.0;
        let talude = fs_minimo(&p);
        assert!(talude < plano, "talude a montante deve reduzir o FS: {talude} x {plano}");
        let r = p.calcular();
        assert!(!r.avisos.iter().any(|a| a.contains("β ignorado")));
    }

    /// Varredura de segurança: nenhum cenário razoável (ou extremo) pode gerar pânico, NaN
    /// ou memorial vazio, e as tendências físicas básicas precisam valer.
    #[test]
    fn varredura_de_seguranca() {
        use crate::drenagem::Condicao;
        use crate::resultado::Criterio;
        let mut cenarios = 0;
        for tipo in TIPOS_CONTENCAO {
            // em modo debug (CI) a varredura é reduzida: sem otimização o Bishop é ~20x mais lento
            let (alturas, taludes, cargas): (&[f64], &[f64], &[f64]) =
                if cfg!(debug_assertions) { (&[2.0, 6.0], &[0.0, 30.0], &[20.0]) } else { (&[0.5, 2.0, 5.0, 8.0, 12.0], &[0.0, 20.0, 45.0], &[0.0, 20.0, 60.0]) };
            for &h in alturas {
                for &beta in taludes {
                    for &q in cargas {
                        for cond in [Condicao::Drenada, Condicao::Parcial, Condicao::SemDrenagem] {
                            let mut p = Projeto::default();
                            p.tipo = tipo;
                            p.geometria.h = h;
                            p.geometria.beta = beta;
                            p.geometria.q = q;
                            p.drenagem.condicao = cond;
                            p.pre_dimensionar();
                            let r = p.calcular();
                            cenarios += 1;
                            assert!(!r.blocos.is_empty() && !r.desenho.poligonos.is_empty(), "{} H={h}", tipo.nome());
                            for v in &r.verificacoes {
                                assert!(!v.valor.is_nan(), "{} H={h} β={beta} q={q} {cond:?}: NaN em '{}'", tipo.nome(), v.nome);
                                if v.criterio == Criterio::Minimo {
                                    assert!(v.valor > f64::NEG_INFINITY, "{}: −∞ em '{}'", tipo.nome(), v.nome);
                                }
                            }
                            // o desenho nunca pode ter coordenadas inválidas
                            let ruim = r.desenho.linhas.iter().flat_map(|l| l.pontos.iter()).chain(r.desenho.poligonos.iter().flat_map(|q| q.pontos.iter())).any(|c| !c[0].is_finite() || !c[1].is_finite());
                            assert!(!ruim, "{} H={h} β={beta}: coordenada não finita no desenho", tipo.nome());
                            // o memorial em Word sempre pode ser gerado
                            if (h - 5.0).abs() < 1e-9 && q == 20.0 && beta == 20.0 {
                                let bytes = crate::docx::gerar(&r, "teste", "0", &[("Seção".to_string(), r.desenho.clone())]);
                                assert!(bytes.len() > 2000);
                            }
                        }
                    }
                }
            }
        }
        assert!(cenarios > 50);
    }

    /// Tendências físicas: mais empuxo (talude, sobrecarga, água) nunca melhora a estabilidade
    /// de um muro de geometria fixa.
    #[test]
    fn tendencias_fisicas_do_muro() {
        use crate::drenagem::Condicao;
        for tipo in [TipoContencao::Gravidade, TipoContencao::Flexao] {
            let base = {
                let mut p = Projeto::default();
                p.tipo = tipo;
                p.geometria.h = 4.0;
                p.pre_dimensionar();
                p
            };
            let fs = |p: &Projeto, nome: &str| {
                p.calcular().verificacoes.iter().find(|v| v.nome.starts_with(nome) && v.nome.contains("(estático)")).map(|v| v.valor).unwrap()
            };
            for nome in ["FS tombamento", "FS deslizamento"] {
                let f0 = fs(&base, nome);
                let mut p = base.clone();
                p.geometria.beta = 15.0;
                assert!(fs(&p, nome) < f0, "{}: talude deve reduzir {nome}", tipo.nome());
                let mut p = base.clone();
                p.geometria.q = base.geometria.q + 30.0;
                assert!(fs(&p, nome) < f0, "{}: sobrecarga deve reduzir {nome}", tipo.nome());
                let mut p = base.clone();
                p.drenagem.condicao = Condicao::SemDrenagem;
                assert!(fs(&p, nome) < f0, "{}: água deve reduzir {nome}", tipo.nome());
                // base mais larga (mesmo H) nunca piora o tombamento
            }
        }
    }

    #[test]
    fn grampos_dimensionados_pela_tracao() {
        for h in [3.0, 5.0, 8.0] {
            let mut p = Projeto::default();
            p.tipo = TipoContencao::Grampeado;
            p.geometria.h = h;
            let r = p.calcular();
            let g = p.preparar_grampeado();
            assert!(((g.comprimento * 2.0).round() / 2.0 - g.comprimento).abs() < 1e-9, "L múltiplo de 0,5 m: {}", g.comprimento);
            for v in r.verificacoes.iter().filter(|v| v.nome.starts_with("Grampo ") && v.nome.contains("FS")) {
                assert!(v.ok(), "H={h}: {}", v.texto());
            }
        }
        // bitola cresce com a tração (altura maior)
        let mut a = Projeto::default();
        a.tipo = TipoContencao::Grampeado;
        a.geometria.h = 3.0;
        let mut b = a.clone();
        b.geometria.h = 12.0;
        assert!(b.preparar_grampeado().diam_barra >= a.preparar_grampeado().diam_barra);
    }

    #[test]
    fn tirantes_dimensionados_pela_tracao() {
        use crate::tipos::atirantada::AcoTirante;
        for tipo_aco in [AcoTirante::Barra, AcoTirante::Cordoalha] {
            let mut p = Projeto::default();
            p.tipo = TipoContencao::Atirantada;
            p.geometria.h = 6.0;
            p.atirantada.tipo_aco = tipo_aco;
            let r = p.calcular();
            let tir: Vec<_> = r.verificacoes.iter().filter(|v| v.nome.starts_with("Tirante ")).collect();
            assert!(tir.len() >= 4, "uma verificação de aço e uma de ensaio por nível");
            assert!(tir.iter().all(|v| v.ok()), "{:?}", tir.iter().filter(|v| !v.ok()).map(|v| v.texto()).collect::<Vec<_>>());
        }
    }

    #[test]
    fn memorial_traz_spt_do_apoio_do_tardoz_e_materiais() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 5.0;
            let t = p.calcular().texto_completo("teste");
            assert!(t.contains("Solos de projeto") && t.contains("NSPT"), "{}: sem SPT de apoio/tardoz", tipo.nome());
            assert!(t.contains("Solo do tardoz"), "{}", tipo.nome());
            if matches!(tipo, TipoContencao::Gravidade | TipoContencao::Flexao | TipoContencao::Atirantada | TipoContencao::Grampeado) {
                assert!(t.contains("Material drenante") && t.contains("Brita nº 2"), "{}: sem materiais da drenagem", tipo.nome());
            }
            if matches!(tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
                assert!(t.contains("camadas de 20 cm") && t.contains("reaterro compactado"), "{}", tipo.nome());
            }
        }
        // o reaterro especificado altera o empuxo (areia compactada x argila mole)
        let fs = |p: &Projeto| p.calcular().verificacoes.iter().find(|v| v.nome.starts_with("FS tombamento")).unwrap().valor;
        let mut a = Projeto::default();
        a.tipo = TipoContencao::Gravidade;
        a.geometria.h = 4.0;
        a.pre_dimensionar();
        let mut b = a.clone();
        b.reaterro.tipo = crate::spt::TipoSolo::Areia;
        b.reaterro.nspt = 25.0;
        assert!(fs(&b) > fs(&a), "areia compacta no tardoz deve melhorar o tombamento");
    }

    #[test]
    fn estabilidade_global_em_todos_os_tipos() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 5.0;
            let r = p.calcular();
            let glob: Vec<_> = r.verificacoes.iter().filter(|v| v.nome.contains("estabilidade global") || v.nome.contains("estabilidade externa")).collect();
            assert!(!glob.is_empty(), "{}: sem verificação de estabilidade global", tipo.nome());
            assert!(glob.iter().all(|v| v.valor.is_finite() && v.valor > 0.0), "{}: {:?}", tipo.nome(), glob.iter().map(|v| v.texto()).collect::<Vec<_>>());
            assert!(r.desenho.textos.iter().any(|t| t.texto.starts_with("FS global")), "{}: sem o arco crítico no desenho", tipo.nome());
            let t = r.texto_completo("x");
            assert!(t.contains("Estabilidade global") && t.contains("Bishop"), "{}", tipo.nome());
            // o fator de segurança mínimo vem da Tabela 3 da NBR 11682
            assert!(glob.iter().all(|v| (v.limite - p.fatores.fs_global()).abs() < 1e-9));
        }
    }

    /// Auxiliar de desenvolvimento: grava o DXF de cada tipo
    /// (`DXF_AMOSTRA=<pasta> DXF_BETA=<graus> DXF_H=<m> cargo test amostras_dxf`).
    #[test]
    fn amostras_dxf() {
        let Ok(pasta) = std::env::var("DXF_AMOSTRA") else { return };
        let beta: f64 = std::env::var("DXF_BETA").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let h: f64 = std::env::var("DXF_H").ok().and_then(|v| v.parse().ok()).unwrap_or(5.0);
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = h;
            p.geometria.beta = beta;
            p.pre_dimensionar();
            let r = p.calcular();
            let mut d = r.desenho.clone();
            crate::detalhes::anexar(&mut d, &p, &r);
            crate::barras::anexar(&mut d, &r.armaduras);
            let dxf = crate::dxf::gerar(&d, &r.armaduras, tipo.arquivo(), &[], "teste");
            std::fs::write(format!("{pasta}/{}.dxf", tipo.arquivo()), dxf).unwrap();
        }
    }

    /// Informativo: as seções pré-dimensionadas atendem às verificações?
    #[test]
    fn predim_atende_estabilidade() {
        for tipo in [TipoContencao::Gravidade, TipoContencao::Flexao, TipoContencao::Grampeado] {
            for h in [1.0, 2.0, 3.0, 4.0, 6.0] {
                let mut p = Projeto::default();
                p.tipo = tipo;
                p.geometria.h = h;
                if std::env::var("PREDIM_AREIA").is_ok() {
                    p.sondagem.modo = crate::spt::ModoSolo::Simplificado;
                    p.sondagem.simples_tipo = crate::spt::TipoSolo::AreiaArgilosa;
                    p.sondagem.simples_nspt = 15.0;
                    p.sondagem.usar_na = false;
                    p.sondagem.recalcular();
                }
                let msg = p.pre_dimensionar().unwrap_or_default();
                eprintln!("  {msg}");
                let r = p.calcular();
                let falhas: Vec<String> = r.verificacoes.iter().filter(|v| !v.ok()).map(|v| format!("{} {}", v.nome.split(" — ").next().unwrap_or(""), v.expressao())).collect();
                eprintln!("{} H = {h} m: {}", tipo.nome(), if falhas.is_empty() { "ATENDE".into() } else { falhas.join(" | ") });
            }
        }
    }
}

#[cfg(test)]
mod testes_varredura {
    use super::*;
    use crate::drenagem::Condicao;

    /// Varredura de robustez: todo tipo, várias alturas, talude, sobrecarga,
    /// água e sismo. Nenhuma verificação pode sair NaN, nenhum FS negativo,
    /// e as dimensões da estrutura têm de ser coerentes com a altura livre.
    #[test]
    fn sem_nan_nem_valores_incoerentes() {
        let mut casos = 0;
        for tipo in TIPOS_CONTENCAO {
            for h in [1.0, 2.5, 6.0, 10.0] {
                for (beta, q, na, kh) in [(0.0, 0.0, false, 0.0), (15.0, 30.0, true, 0.0), (0.0, 20.0, false, 0.15)] {
                    let mut p = Projeto::default();
                    p.tipo = tipo;
                    p.geometria.h = h;
                    p.geometria.beta = beta;
                    p.geometria.q = q;
                    p.cargas.kh = kh;
                    p.sondagem.usar_na = na;
                    p.sondagem.prof_na = 0.5 * h;
                    if na {
                        p.drenagem.condicao = Condicao::Parcial;
                    }
                    p.pre_dimensionar();
                    let r = p.calcular();
                    casos += 1;
                    let id = format!("{} H={h} β={beta} q={q} NA={na} kh={kh}", tipo.nome());
                    for v in &r.verificacoes {
                        assert!(!v.valor.is_nan() && !v.limite.is_nan(), "{id}: NaN em {}", v.nome);
                        if v.nome.starts_with("FS") {
                            assert!(v.valor >= 0.0, "{id}: {} = {}", v.nome, v.valor);
                        }
                    }
                    if matches!(tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
                        let ht = p.altura_contida();
                        assert!((ht - h - p.geometria.engaste).abs() < 1e-9, "{id}");
                        assert!(p.geometria.engaste >= 0.40 - 1e-9, "{id}: Df");
                    }
                    if tipo == TipoContencao::Gravidade {
                        let g = p.preparar_gravidade();
                        assert!((g.h + g.f - h - p.geometria.engaste).abs() < 1e-9, "{id}: corpo + base = H + Df");
                    }
                }
            }
        }
        assert_eq!(casos, 6 * 4 * 3);
    }
}

#[cfg(test)]
mod testes_drenagem_geo {
    use super::*;
    use crate::drenagem::{MaterialFiltro, BARBACA_ENTRADA, BARBACA_SAIDA};

    #[test]
    fn barbacas_saem_15_cm_e_entram_20_cm_em_todos_os_tipos() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let gd = p.geo_drenagem();
            for ((t, f), (pt, pf)) in gd.barbacas.iter().zip(&gd.paredes) {
                assert!((t[0] - pt[0] - BARBACA_ENTRADA).abs() < 1e-9, "{}: entrada", tipo.nome());
                assert!((pf[0] - f[0] - BARBACA_SAIDA).abs() < 1e-9, "{}: saída", tipo.nome());
                assert!(f[1] < t[1], "{}: caimento para fora", tipo.nome());
            }
            if matches!(tipo, TipoContencao::Prancha | TipoContencao::Estacas) {
                continue;
            }
            assert!(!gd.barbacas.is_empty(), "{}", tipo.nome());
            // brita + geotêxtil (padrão): só bolsas nos barbacãs e envelope do dreno de pé
            assert_eq!(p.drenagem.material_filtro, MaterialFiltro::BritaGeotextil);
            assert!(!gd.filtro_continuo && gd.bolsas.len() == gd.barbacas.len() && gd.envelope_pe.is_some(), "{}", tipo.nome());
            // areia: camada contínua
            p.drenagem.material_filtro = MaterialFiltro::AreiaGrossa;
            let gd = p.geo_drenagem();
            assert!(gd.filtro_continuo && gd.bolsas.is_empty(), "{}", tipo.nome());
        }
    }
}

#[cfg(test)]
mod testes_resumos {
    use super::*;

    #[test]
    fn resumos_de_aco_e_materiais_coerentes() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 3.0;
            p.pre_dimensionar();
            let r = p.calcular();
            let tem = |t: &str| r.blocos.iter().any(|b| matches!(b, crate::resultado::Bloco::Titulo(x) if x.starts_with(t)));
            assert!(tem("Resumo dos demais materiais"), "{}", tipo.nome());
            if !r.armaduras.is_empty() {
                assert!(tem("Resumo de aço"), "{}", tipo.nome());
                let kg = r.verificacoes.iter().find(|v| v.nome.starts_with("Aço total")).unwrap().valor;
                assert!(kg > 0.0 && kg < 600.0, "{}: {kg} kg/m", tipo.nome());
            }
            // nenhum comprimento de corte maior que a maior dimensão plausível da peça
            let limite = 3.0 * (p.altura_contida() + 10.0);
            for a in r.armaduras.iter().filter(|a| !matches!(a.forma, crate::armadura::Forma::Helice { .. })) {
                assert!(a.comprimento_m < limite, "{}: {} com {} m", tipo.nome(), a.pos, a.comprimento_m);
            }
        }
    }
}

#[cfg(test)]
mod testes_drenagem_reaterro {
    use super::*;
    use crate::drenagem::FOLGA_SAIDA;

    #[test]
    fn saidas_acima_do_terreno_frontal_e_lamina_residual() {
        for tipo in [TipoContencao::Gravidade, TipoContencao::Flexao, TipoContencao::Atirantada, TipoContencao::Grampeado] {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 3.0;
            p.pre_dimensionar();
            p.drenagem.altura_1a_fila = 0.05; // valor baixo de propósito: o programa deve elevar
            let gd = p.geo_drenagem();
            let yf = gd.y_frente.expect("terreno à frente");
            let d_b = p.drenagem.diam_barbaca_mm / 2000.0;
            for (_, f) in &gd.barbacas {
                assert!(f[1] - d_b - yf >= FOLGA_SAIDA - 1e-9, "{}: barbacã a {} m (terreno {})", tipo.nome(), f[1], yf);
            }
            let (c, (_, b)) = (gd.dreno_pe.expect("dreno"), gd.saida_pe.expect("saída"));
            assert!(b[1] - d_b - yf >= FOLGA_SAIDA - 1e-9, "{}: saída do dreno", tipo.nome());
            assert!(c[1] > yf, "{}: dreno acima do terreno", tipo.nome());
            // condição drenada: a lâmina residual vira nível d'água no cálculo
            let hres = p.lamina_residual();
            assert!(hres > 0.0);
            let na = p.na_calculo().expect("lâmina residual no cálculo");
            assert!((p.altura_contida() - na - hres).abs() < 1e-9, "{}", tipo.nome());
            // ligado à rede: dreno na base, sem saída pela face nem lâmina
            p.drenagem.pe_em_rede = true;
            assert!(p.geo_drenagem().saida_pe.is_none());
        }
    }
}
