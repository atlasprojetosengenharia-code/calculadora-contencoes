//! Projeto: dados gerais, sondagem SPT, geometria essencial, cargas,
//! drenagem, fatores e parâmetros específicos de cada tipo de contenção.
//! Salvo em arquivo `.ctc` (JSON).

use crate::drenagem::Drenagem;
use crate::empuxo::{rad, ParamEmpuxo};
use crate::fatores::Fatores;
use crate::resultado::{fmt, Resultado};
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
        }
    }
}

impl Projeto {
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
        let na = self.drenagem.na_efetivo(self.sondagem.na(), self.altura_contida());
        self.sondagem.perfil(na)
    }

    fn fundacao_em(&self, z: f64, base: &SoloFundacao) -> SoloFundacao {
        let mut f = base.clone();
        if let Some(c) = self.sondagem.camada_em(z + 0.01) {
            f.gamma = c.gamma;
            f.phi = c.phi;
            f.c = c.c;
        }
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
        let mut cab = vec![
            format!("Projeto: {}", self.nome),
            format!("Local: {}", if self.local.is_empty() { "—" } else { &self.local }),
            format!("Responsável técnico: {}", if self.responsavel.is_empty() { "—" } else { &self.responsavel }),
            format!("Tipo de contenção: {}", self.tipo.nome()),
            String::new(),
            format!("== Sondagem {} (SPT) ==", self.sondagem.identificacao),
        ];
        let s = &self.sondagem;
        cab.push(format!(
            "NA da sondagem: {} ; NA adotado no cálculo: {}",
            s.na().map(|n| format!("{} m", fmt(n))).unwrap_or("não encontrado".into()),
            self.drenagem
                .na_efetivo(s.na(), self.altura_contida())
                .map(|n| format!("{} m (condição: {})", fmt(n), self.drenagem.condicao.nome()))
                .unwrap_or(format!("sem água (condição: {})", self.drenagem.condicao.nome()))
        ));
        cab.push(format!(
            "Correlações: γ — Godoy (1972) ; φ' granulares — {} ; coesivos — {}",
            s.metodo_phi.nome(),
            match s.coesivos {
                CondicaoCoesivos::NaoDrenada => "não drenado, Su = 10·N (Teixeira e Godoy, 1996)".to_string(),
                CondicaoCoesivos::Drenada => format!("drenado, c' = {} kPa e φ' = {}° (valores adotados)", fmt(s.c_drenado), fmt(s.phi_drenado)),
            }
        ));
        cab.push(format!("{:<13} {:>5}  {:<34} {:>6} {:>6} {:>6} {:>6}  {}", "Prof. (m)", "NSPT", "Descrição", "γ", "γsat", "c'", "φ'", "origem"));
        for (i, c) in s.camadas.iter().enumerate() {
            let desc: String = c.descricao.chars().take(34).collect();
            cab.push(format!(
                "{:<13} {:>5}  {:<34} {:>6} {:>6} {:>6} {:>6}  {}",
                format!("{}–{}", fmt(s.de(i)), fmt(c.ate)),
                fmt(c.nspt),
                desc,
                fmt(c.gamma),
                fmt(c.gamma_sat),
                fmt(c.c),
                fmt(c.phi),
                if c.manual { "manual" } else { "correlação" }
            ));
        }
        if self.tipo == TipoContencao::Grampeado {
            let (g, p, c) = s.medias(self.geometria.h);
            cab.push(format!(
                "Solo grampeado: parâmetros médios até H — γ = {} ; φ' = {}° ; c' = {} kPa",
                fmt(g), fmt(p), fmt(c)
            ));
        }
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
        cab.append(&mut r.memorial);
        r.memorial = cab;
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
        dr.relatar(r, self.altura_contida(), &larguras, altura_filtro);
    }
}
