//! Projeto: dados gerais, sondagem SPT, geometria essencial, cargas,
//! drenagem, fatores e parâmetros específicos de cada tipo de contenção.
//! Salvo em arquivo `.ctc` (JSON).

use crate::drenagem::Drenagem;
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
            TipoContencao::Gravidade => "Altura do corpo H",
            TipoContencao::Flexao => "Altura total H",
            TipoContencao::Grampeado => "Altura do talude H",
            _ => "Altura escavada H",
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
                "Prática corrente (não normativa): L de 0,6 a 1,2·H (o menor que atende; mín. 3 m), Sh = Sv = 1,5 m, 1º grampo a 0,75 m.",
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
    pub h: f64,
    /// Inclinação do talude de montante (crista), graus.
    pub beta: f64,
    /// Sobrecarga na crista (kPa).
    pub q: f64,
    pub engaste: f64,
}

impl Default for Geometria {
    fn default() -> Self {
        Self { h: 4.0, beta: 0.0, q: 20.0, engaste: 0.6 }
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
}

fn verdadeiro() -> bool {
    true
}

/// Arredonda para cima em múltiplos de 5 cm.
fn arred(x: f64) -> f64 {
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
                let m = &mut self.gravidade;
                m.f = arred(0.13 * h / 0.87);
                let ht = h + m.f;
                let base = arred(k * ht);
                m.crista = arred((0.10 * ht).max(0.30));
                m.proj_frente = arred(h / 12.0);
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
                let m = &mut self.flexao;
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
                g.sh = 1.5;
                g.sv = 1.5;
                g.z1 = 0.75;
            }
            _ => {}
        }
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
        let (faixa, nome): (&[f64], &str) = match self.tipo {
            TipoContencao::Gravidade | TipoContencao::Flexao => {
                (&[0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70], "base")
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
                    _ => v.nome.contains("(estático)"),
                })
                .collect();
            if estab.is_empty() {
                return None;
            }
            Some(estab.iter().filter(|v| !v.ok()).map(|v| v.nome.split(" — ").next().unwrap_or(&v.nome).to_string()).collect())
        };
        for &k in faixa {
            self.aplicar_proporcoes(k);
            if pendentes(self).is_some_and(|f| f.is_empty()) {
                return Some(format!(
                    "Pré-dimensionamento aplicado: {nome} = {} % de H, a menor da faixa recomendada que atende à estabilidade.",
                    (k * 100.0).round()
                ));
            }
        }
        let k = *faixa.last().unwrap();
        self.aplicar_proporcoes(k);
        let falta = pendentes(self).unwrap_or_default().join(", ");
        let dica = match self.tipo {
            TipoContencao::Grampeado => "aumente o qs, o diâmetro do furo ou reduza o espaçamento",
            _ if falta.contains("deslizamento") => {
                "para o deslizamento, considere dente de base, atrito k·tg φ (Marchetti) ou reveja a sobrecarga"
            }
            _ => "ajuste a geometria manualmente",
        };
        Some(format!(
            "Pré-dimensionamento aplicado com {nome} = {} % de H (limite da faixa recomendada). Ainda não atende: {falta} — {dica}.",
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
        serde_json::from_str(&txt).map_err(|e| format!("Arquivo inválido: {e}"))
    }

    /// Altura total de solo contido pela estrutura (para drenagem / NA).
    pub fn altura_contida(&self) -> f64 {
        match self.tipo {
            TipoContencao::Gravidade => self.geometria.h + self.gravidade.f,
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
        let na = self.drenagem.na_efetivo(self.sondagem.na(), self.altura_contida(), self.tipo);
        self.sondagem.perfil(na)
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
        m.h = self.geometria.h;
        m.df = self.geometria.engaste;
        m.aterro = self.perfil_calc();
        m.fundacao = self.fundacao_em(m.h + m.f, &m.fundacao);
        m.cargas = self.cargas_calc();
        m.empuxo = self.empuxo_calc();
        m
    }

    pub fn preparar_flexao(&self) -> MuroFlexao {
        let mut m = self.flexao.clone();
        m.concreto.peso = crate::materiais::CONCRETO_ARMADO;
        m.h = self.geometria.h;
        m.df = self.geometria.engaste;
        m.aterro = self.perfil_calc();
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
            .drenagem
            .na_efetivo(self.sondagem.na(), g.h, self.tipo)
            .map(|n| (g.h - n).max(0.0))
            .unwrap_or(0.0);
        g
    }

    pub fn calcular(&self) -> Resultado {
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

        // Cabeçalho do memorial
        let t = |x: String| Bloco::Texto(x);
        let mut cab = vec![
            Bloco::Titulo("Identificação".into()),
            t(format!("Projeto: {}", self.nome)),
            t(format!("Local: {}", if self.local.is_empty() { "—" } else { &self.local })),
            t(format!("Responsável técnico: {}", if self.responsavel.is_empty() { "—" } else { &self.responsavel })),
            t(format!("Tipo de contenção: {}", self.tipo.nome())),
            Bloco::Titulo(match self.sondagem.modo {
                crate::spt::ModoSolo::Completo => format!("Sondagem {} (SPT)", self.sondagem.identificacao),
                crate::spt::ModoSolo::Simplificado => "Solo — entrada simplificada na base (SPT)".into(),
            }),
        ];
        let s = &self.sondagem;
        cab.push(t(format!(
            "NA da sondagem: {} ; NA adotado no cálculo: {}",
            s.na().map(|n| format!("{} m", fmt(n))).unwrap_or("não encontrado".into()),
            self.drenagem
                .na_efetivo(s.na(), self.altura_contida(), self.tipo)
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
        cab.append(&mut r.blocos);
        r.blocos = cab;
        r
    }

    fn drenagem_no_resultado(&self, r: &mut Resultado) {
        let dr = &self.drenagem;
        let d = &mut r.desenho;
        let mut larguras = Vec::new();
        let mut altura_filtro = 0.0;
        match self.tipo {
            TipoContencao::Gravidade => {
                let m = self.preparar_gravidade();
                let g = m.geo();
                let y0 = m.df.max(m.f);
                for v in dr.filas(g.htot - y0) {
                    let y = y0 + v;
                    let (xl, xr) = m.faces(y);
                    dr.barbaca(d, xr, xl, y);
                    larguras.push(xr - xl);
                }
                let yt = g.htot - 0.3;
                dr.faixa_filtro(d, [g.x_pt, m.f], [m.faces(yt).1, yt]);
                altura_filtro = yt - m.f;
                if dr.canaleta_crista {
                    let x0 = g.x_cr + dr.esp_filtro + 0.2;
                    let y = g.htot + (x0 - g.x_cr) * rad(self.geometria.beta).tan();
                    d.retangulo(crate::resultado::Estilo::Concreto, x0, y - 0.15, x0 + 0.35, y);
                }
            }
            TipoContencao::Flexao => {
                let m = self.preparar_flexao();
                let (bp, tb, tt, hs, h) = (m.b_ponta, m.t_base, m.t_topo, m.h_sapata, m.h);
                let xr = |y: f64| bp + tb - (tb - tt) * ((y - hs) / (h - hs)).clamp(0.0, 1.0);
                let y0 = m.df.max(hs);
                for v in dr.filas(h - y0) {
                    let y = y0 + v;
                    dr.barbaca(d, xr(y), bp, y);
                    larguras.push(xr(y) - bp);
                }
                let yt = h - 0.3;
                dr.faixa_filtro(d, [bp + tb, hs], [xr(yt), yt]);
                altura_filtro = yt - hs;
            }
            TipoContencao::Atirantada => {
                let c = self.preparar_atirantada();
                for v in dr.filas(c.h) {
                    let y = -c.h + v;
                    dr.barbaca(d, 0.0, -c.espessura, y);
                    larguras.push(c.espessura);
                }
                dr.faixa_filtro(d, [0.0, -c.h], [0.0, -0.3]);
                altura_filtro = c.h - 0.3;
                if dr.dhp {
                    let mut y = -c.h + 0.5;
                    while y < -0.5 {
                        d.tracejada(crate::resultado::Estilo::Barbaca, vec![[0.0, y], [dr.dhp_comprimento, y + 0.05 * dr.dhp_comprimento]]);
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
                    dr.barbaca(d, xf, xf - e, v);
                    larguras.push(e);
                }
                let yt = g.h - 0.3;
                dr.faixa_filtro(d, [0.0, 0.0], [yt * eta.tan(), yt]);
                altura_filtro = yt;
                if dr.dhp {
                    let mut y = 0.5;
                    while y < g.h - 0.5 {
                        let xf = y * eta.tan();
                        d.tracejada(crate::resultado::Estilo::Barbaca, vec![[xf, y], [xf + dr.dhp_comprimento, y + 0.05 * dr.dhp_comprimento]]);
                        y += 4.0;
                    }
                }
            }
            TipoContencao::Prancha | TipoContencao::Estacas => {}
        }
        dr.relatar(r, self.altura_contida(), &larguras, altura_filtro, self.tipo);
    }
}

#[cfg(test)]
mod testes_predim {
    use super::*;

    #[test]
    fn gravidade_proporcoes_do_marchetti() {
        // Exemplo 4.1 do livro: corpo 5,80 m + base 0,90 m (Ht = 6,70 m)
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Gravidade;
        p.geometria.h = 5.8;
        assert!(p.pre_dimensionar().is_some());
        let g = &p.gravidade;
        let ht = p.geometria.h + g.f;
        assert!((g.f - 0.90).abs() < 1e-9, "F = {}", g.f);
        assert!((g.proj_frente - 0.50).abs() < 1e-9, "paramento 1:12 ≈ 0,48 → {}", g.proj_frente);
        assert!((g.ressalto_frente - 0.45).abs() < 1e-9 && (g.ressalto_tras - 0.15).abs() < 1e-9);
        let base = g.ressalto_frente + g.proj_frente + g.crista + g.proj_tardoz + g.ressalto_tras;
        assert!(base >= 0.40 * ht - 1e-9 && base <= 0.70 * ht + 0.05, "base {base} fora de 40–70 % de {ht}");
        assert!((p.geometria.engaste - 1.20).abs() < 1e-9, "embutimento = base + 30 cm");
    }

    #[test]
    fn flexao_proporcoes_do_marchetti() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 5.5;
        p.pre_dimensionar();
        let m = &p.flexao;
        let b = m.b_ponta + m.t_base + m.b_talao;
        assert!((m.t_topo - 0.20).abs() < 1e-9);
        assert!(m.t_base >= 0.08 * 5.5 && m.t_base <= 0.10 * 5.5 + 0.05);
        assert!(m.b_ponta >= 0.10 * 5.5 && m.b_ponta <= 0.12 * 5.5 + 0.05);
        assert!(b >= 0.40 * 5.5 - 1e-9 && b <= 0.70 * 5.5 + 0.10, "B = {b}");
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
                let por_tirante = |k: &str| k.starts_with("Tirante") || k.starts_with("Carga máx") || k.starts_with("Punção");
                let nenhuma_melhora = comuns.iter().all(|k| por_tirante(k) || b[*k] <= a[*k] + 1e-6);
                let alguma_piora = comuns.iter().any(|k| b[*k] < a[*k] - 1e-6);
                (nenhuma_melhora, alguma_piora)
            };
            let (ok1, p1) = piora(&m[0], &m[1]);
            let (ok2, p2) = piora(&m[0], &m[2]);
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
            let dxf = crate::dxf::gerar(&r.desenho, &r.armaduras, tipo.arquivo(), &[], "teste");
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
