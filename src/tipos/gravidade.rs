//! Muro de gravidade — pedra argamassada, concreto ciclópico, gabião etc.
//!
//! Geometria (croqui do usuário), origem no canto inferior frontal da base:
//!
//! ```text
//!                 C
//!              ┌─────┐
//!         B   /       \  D
//!            /         \
//!       A   /           \ E
//!   ┌──────┘             └──┐
//!   │            F          │
//!   └───────────────────────┘
//!            base
//! ```
//! A = ressalto frontal, B = projeção do paramento frontal, C = crista,
//! D = projeção do tardoz, E = ressalto posterior, F = altura da base,
//! H = altura do corpo (acima da base). Base = A + B + C + D + E.

use crate::alvenaria::{self, ClasseArgamassa, Propriedades};
use crate::armadura::{self, Armadura};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::estabilidade::{analisar, centroide, Forca, LimitesFs};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil, SoloFundacao, GAMMA_W};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Material {
    PedraArgamassada,
    ConcretoCiclopico,
    ConcretoSimples,
    Gabiao,
    PedraSeca,
}

pub const MATERIAIS: [Material; 5] = [
    Material::PedraArgamassada,
    Material::ConcretoCiclopico,
    Material::ConcretoSimples,
    Material::Gabiao,
    Material::PedraSeca,
];

impl Material {
    pub fn nome(&self) -> &'static str {
        match self {
            Material::PedraArgamassada => "Pedra argamassada",
            Material::ConcretoCiclopico => "Concreto ciclópico",
            Material::ConcretoSimples => "Concreto simples",
            Material::Gabiao => "Gabião",
            Material::PedraSeca => "Pedra seca (arrumada)",
        }
    }
    /// Peso específico FIXO (kN/m³) — ver `crate::materiais`.
    pub fn peso(&self) -> f64 {
        use crate::materiais as m;
        match self {
            Material::PedraArgamassada => m::PEDRA_ARGAMASSADA,
            Material::ConcretoCiclopico => m::CONCRETO_CICLOPICO,
            Material::ConcretoSimples => m::CONCRETO_SIMPLES,
            Material::Gabiao => m::GABIAO,
            Material::PedraSeca => m::PEDRA_SECA,
        }
    }
    /// Origem normativa do valor.
    pub fn origem(&self) -> &'static str {
        match self {
            Material::ConcretoSimples => "NBR 6120:2019, Tab. 1 — concreto simples",
            Material::ConcretoCiclopico => {
                "concreto simples (NBR 6120:2019, Tab. 1) + pedra de mão; adotado o limite inferior dos constituintes"
            }
            Material::PedraArgamassada => {
                "pedra (granito 28,5) + argamassa de cimento e areia (21) — NBR 6120:2019, Tab. 1; adotado o limite inferior"
            }
            Material::Gabiao => {
                "γp·(1 − n) = 28,5 × (1 − 0,40): granito NBR 6120:2019 Tab. 1; n máx. do Manual Maccaferri (0,30 a 0,40)"
            }
            Material::PedraSeca => "γp·(1 − n) = 28,5 × (1 − 0,40): granito NBR 6120:2019 Tab. 1; mesma porosidade do gabião",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MuroGravidade {
    pub material: Material,
    /// H — altura do corpo, acima da base (m).
    pub h: f64,
    /// F — altura da base (m).
    pub f: f64,
    /// C — largura da crista (m).
    pub crista: f64,
    /// B — projeção horizontal do paramento frontal (m).
    pub proj_frente: f64,
    /// D — projeção horizontal do tardoz (m).
    pub proj_tardoz: f64,
    /// A — ressalto frontal da base (m).
    pub ressalto_frente: f64,
    /// E — ressalto posterior da base (m).
    pub ressalto_tras: f64,
    /// Altura de solo à frente, acima do fundo da base (m).
    pub df: f64,
    pub considerar_passivo: bool,
    pub frac_passivo: f64,
    pub subpressao: bool,
    /// Dente de base (chave de cisalhamento) sob o corpo do muro.
    pub dente: bool,
    /// Profundidade do dente abaixo da base (m).
    pub dente_h: f64,
    /// Largura do dente (m).
    pub dente_b: f64,
    /// Banco de alvenarias: fck do concreto simples/ciclópico (MPa).
    pub fck_alvenaria: f64,
    /// Banco de alvenarias: classe da argamassa da pedra argamassada.
    pub argamassa: ClasseArgamassa,
    /// Usar valores do projetista em vez do banco de alvenarias.
    pub personalizar_alvenaria: bool,
    /// Tensão de compressão admissível (MPa) — só com `personalizar_alvenaria`.
    pub sigma_adm: f64,
    /// Tração admissível nas juntas (MPa) — só com `personalizar_alvenaria`.
    pub tracao_adm: f64,
    /// Cisalhamento admissível nas juntas (MPa) — só com `personalizar_alvenaria`.
    pub tau_adm: f64,
    /// Espaçamento das juntas de dilatação (m).
    pub junta_espac: f64,
    /// Viga de coroamento em concreto armado.
    pub coroamento: bool,
    pub coroamento_h: f64,
    pub aterro: Perfil,
    pub fundacao: SoloFundacao,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
}

impl Default for MuroGravidade {
    fn default() -> Self {
        Self {
            material: Material::PedraArgamassada,
            h: 4.0,
            f: 0.5,
            crista: 0.6,
            proj_frente: 1.4,
            proj_tardoz: 0.2,
            ressalto_frente: 0.2,
            ressalto_tras: 0.2,
            df: 0.6,
            considerar_passivo: false,
            frac_passivo: 0.5,
            subpressao: true,
            dente: false,
            dente_h: 0.4,
            dente_b: 0.4,
            fck_alvenaria: 15.0,
            argamassa: ClasseArgamassa::Media,
            personalizar_alvenaria: false,
            sigma_adm: 1.0,
            tracao_adm: 0.0,
            tau_adm: 0.05,
            junta_espac: 10.0,
            coroamento: true,
            coroamento_h: 0.20,
            aterro: Perfil::simples(18.0, 30.0, 0.0),
            fundacao: SoloFundacao::default(),
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
        }
    }
}

/// Forças do aterro sobre um plano de cálculo — usado também pelo muro de flexão.
pub struct EmpuxoNoPlano {
    pub forcas: Vec<Forca>,
    pub pa: f64,
    pub incl: f64,
    pub k: f64,
    /// Diagrama (y, p_h) para desenho.
    pub diagrama: Vec<(f64, f64)>,
}

/// Calcula as forças horizontais do aterro sobre um plano de altura `hv`;
/// `x_plano(y)` dá a abscissa do plano na cota y (y = 0 na base).
pub fn empuxo_no_plano(
    aterro: &Perfil,
    cargas: &Cargas,
    emp: &ParamEmpuxo,
    theta: f64,
    hv: f64,
    x_plano: &dyn Fn(f64) -> f64,
    sismo: bool,
) -> EmpuxoNoPlano {
    let n = 400;
    let incl = emp.inclinacao_ativa(theta);
    let mut forcas = Vec::new();

    let (pa, zc) = integrar(|z| emp.pressao_ativa(aterro, cargas, theta, z), 0.0, hv, n);
    let ya = hv - zc;
    forcas.push(Forca::nova("Empuxo ativo (solo + sobrecarga)", pa * incl.cos(), pa * incl.sin(), x_plano(ya), ya));

    let (uw, zw) = integrar(|z| aterro.u(z), 0.0, hv, n);
    if uw > 1e-6 {
        forcas.push(Forca::nova("Empuxo hidrostático", uw, 0.0, x_plano(hv - zw), hv - zw));
    }
    let (pf, zf) = integrar(|z| cargas.pressao_faixa(z), 0.0, hv, n);
    if pf > 1e-6 {
        forcas.push(Forca::nova("Carga em faixa (Boussinesq)", pf, 0.0, x_plano(hv - zf), hv - zf));
    }
    if sismo && cargas.usar_sismo {
        let (dp, _, _) = emp.incremento_sismico(aterro, cargas, theta, hv);
        let y = 0.6 * hv;
        forcas.push(Forca::nova("Acréscimo sísmico ΔPAE (M-O)", dp * incl.cos(), dp * incl.sin(), x_plano(y), y));
    }

    let k = emp.coef_ativo(aterro.camada_em(0.0).phi, theta);
    let diagrama = (0..=40)
        .map(|i| {
            let z = hv * i as f64 / 40.0;
            (hv - z, emp.pressao_horizontal_total(aterro, cargas, theta, z))
        })
        .collect();
    EmpuxoNoPlano { forcas, pa, incl, k, diagrama }
}

/// Fórmulas do coeficiente de empuxo ativo e da resultante Pa (memorial).
pub fn formula_empuxo(
    r: &mut Resultado,
    emp: &ParamEmpuxo,
    aterro: &Perfil,
    cargas: &Cargas,
    theta: f64,
    hv: f64,
    est: &EmpuxoNoPlano,
) -> f64 {
    let cam = aterro.camada_em(0.0);
    let (eq, sub) = match emp.teoria {
        Teoria::Rankine if emp.beta.abs() < 1e-9 => (
            "Ka = tan²(45° − φ/2)".to_string(),
            format!("Ka = tan²(45° − {}°/2)", fmt(cam.phi)),
        ),
        Teoria::Rankine => (
            "Ka = cos β·(cos β − √(cos²β − cos²φ)) / (cos β + √(cos²β − cos²φ))".to_string(),
            format!("φ = {}° ;  β = {}°", fmt(cam.phi), fmt(emp.beta)),
        ),
        Teoria::Coulomb => (
            "Ka = cos²(φ−θ) / [cos²θ·cos(δ+θ)·(1 + √(sin(φ+δ)·sin(φ−β) / (cos(δ+θ)·cos(θ−β))))²]".to_string(),
            format!(
                "φ = {}° ;  δ = {}° ;  θ = {}° ;  β = {}°",
                fmt(cam.phi), fmt(emp.delta), fmt(theta.to_degrees()), fmt(emp.beta)
            ),
        ),
    };
    r.formula(
        format!("Coeficiente de empuxo ativo — {} (camada superior)", emp.teoria.nome()),
        eq,
        sub,
        format!("Ka = {}", fmt(est.k)),
    );
    r.norma(normas::NBR11682_SOBRECARGA);
    r.formula(
        "Empuxo ativo resultante (solo + sobrecarga)",
        "Pa = ∫₀ᴴ (Ka·σ'v + Ka·q − 2c'·√Ka) dz ;  Ph = Pa·cos i ;  Pv = Pa·sen i",
        format!(
            "H = {} m ;  γ = {} kN/m³ ;  q = {} kPa ;  c' = {} kPa ;  i = {}°",
            fmt(hv), fmt(aterro.gamma_medio(hv)), fmt(cargas.q_uniforme), fmt(cam.c), fmt(est.incl.to_degrees())
        ),
        format!(
            "Pa = {} kN/m ;  Ph = {} kN/m ;  Pv = {} kN/m",
            fmt(est.pa), fmt(est.pa * est.incl.cos()), fmt(est.pa * est.incl.sin())
        ),
    );
    // Parcela da sobrecarga no empuxo (Ka·q ao longo da altura)
    let pq = integrar(|z| emp.coef_ativo(aterro.camada_em(z).phi, theta) * cargas.q_uniforme, 0.0, hv, 400).0
        * est.incl.cos();
    let ph = est.pa * est.incl.cos();
    if ph > 1e-9 && cargas.q_uniforme > 0.0 {
        r.formula(
            "Composição do empuxo horizontal",
            "Ph,q = ∫₀ᴴ Ka·q dz ·cos i ;  Ph,solo = Ph − Ph,q",
            format!("Ph,q = Ka·{}·{} (≈)", fmt(cargas.q_uniforme), fmt(hv)),
            format!(
                "Ph,q = {} kN/m ({} %) ;  Ph,solo = {} kN/m ({} %)",
                fmt(pq.min(ph)),
                format!("{:.0}", 100.0 * pq.min(ph) / ph),
                fmt((ph - pq).max(0.0)),
                format!("{:.0}", 100.0 * (ph - pq).max(0.0) / ph)
            ),
        );
    }
    pq
}

/// Aviso didático quando o deslizamento estático não atende: mostra o peso
/// da sobrecarga no empuxo e as alternativas usuais de projeto.
pub fn diagnostico_deslizamento(r: &mut Resultado, ph_q: f64, ph_total: f64, passivo: bool) {
    let falhou = r
        .verificacoes
        .iter()
        .any(|v| v.nome.starts_with("FS deslizamento") && v.nome.contains("(estático)") && !v.ok());
    if !falhou {
        return;
    }
    let pct = if ph_total > 1e-9 { 100.0 * ph_q.min(ph_total) / ph_total } else { 0.0 };
    r.aviso(format!(
        "Deslizamento não atende: a sobrecarga responde por {} % do empuxo horizontal. Alternativas: {}Coulomb com δ no tardoz, \
         revisar δb/φ' e adesão do contato base-solo, parâmetros do solo de fundação por ensaio (ou entrada simplificada \
         com o NSPT real), base inclinada ou dente de ancoragem, ou ampliar a base.",
        format!("{pct:.0}"),
        if passivo { "" } else { "considerar o passivo frontal (NBR 6122 7.6.3, reduzido), " }
    ));
}

/// Subpressão triangular sob a base (NA no aterro, sem água à frente).
pub fn subpressao(aterro: &Perfil, hv: f64, b: f64) -> Option<Forca> {
    let hw = hv - aterro.na()?;
    if hw <= 0.0 {
        return None;
    }
    Some(Forca::nova("Subpressão na base", 0.0, -0.5 * GAMMA_W * hw * b, 2.0 * b / 3.0, 0.0))
}

/// Empuxo passivo frontal (Rankine, solo de fundação), reduzido por `frac`.
pub fn passivo_frontal(f: &SoloFundacao, df: f64, frac: f64) -> Option<Forca> {
    if df <= 0.0 || frac <= 0.0 {
        return None;
    }
    let kp = crate::empuxo::kp_rankine(rad(f.phi));
    let p1 = 0.5 * kp * f.gamma * df * df;
    let p2 = 2.0 * f.c * kp.sqrt() * df;
    let y = (p1 * df / 3.0 + p2 * df / 2.0) / (p1 + p2);
    Some(Forca::nova("Empuxo passivo frontal (reduzido)", -frac * (p1 + p2), 0.0, 0.0, y))
}

/// Empuxo passivo frontal até a base do dente (profundidade df + hd), reduzido
/// por `frac`; `y` medido a partir da face inferior da base (negativo no dente).
pub fn passivo_com_dente(f: &SoloFundacao, df: f64, hd: f64, frac: f64) -> Option<(Forca, f64)> {
    let d = df.max(0.0) + hd.max(0.0);
    if d <= 0.0 || frac <= 0.0 {
        return None;
    }
    let kp = crate::empuxo::kp_rankine(rad(f.phi));
    let p1 = 0.5 * kp * f.gamma * d * d;
    let p2 = 2.0 * f.c * kp.sqrt() * d;
    let y = (p1 * d / 3.0 + p2 * d / 2.0) / (p1 + p2) - hd.max(0.0);
    Some((Forca::nova("Empuxo passivo frontal + dente (reduzido)", -frac * (p1 + p2), 0.0, 0.0, y), kp))
}

/// Espaçamento máximo das juntas de dilatação: exigência da NBR 6118 24.4 para
/// concreto simples/ciclópico; adotado como critério para as demais alvenarias.
pub const JUNTA_ESPAC_MAX: f64 = 15.0;

/// Coordenadas principais da seção.
pub struct Geo {
    pub htot: f64,
    pub base: f64,
    /// x do pé do paramento frontal (= A)
    pub x_pf: f64,
    /// x da crista (esquerda, direita)
    pub x_cl: f64,
    pub x_cr: f64,
    /// x do pé do tardoz (= base − E)
    pub x_pt: f64,
}

impl MuroGravidade {
    pub fn geo(&self) -> Geo {
        let a = self.ressalto_frente.max(0.0);
        let x_cl = a + self.proj_frente;
        let x_cr = x_cl + self.crista.max(0.05);
        let x_pt = x_cr + self.proj_tardoz;
        Geo { htot: self.h + self.f, base: x_pt + self.ressalto_tras.max(0.0), x_pf: a, x_cl, x_cr, x_pt }
    }

    pub fn poligono(&self) -> Vec<[f64; 2]> {
        let g = self.geo();
        let (f, ht) = (self.f, g.htot);
        vec![[0.0, 0.0], [g.base, 0.0], [g.base, f], [g.x_pt, f], [g.x_cr, ht], [g.x_cl, ht], [g.x_pf, f], [0.0, f]]
    }

    /// Abscissas da face frontal e do tardoz do corpo na cota y (y ≥ F).
    pub fn faces(&self, y: f64) -> (f64, f64) {
        let g = self.geo();
        let t = ((y - self.f) / self.h.max(1e-6)).clamp(0.0, 1.0);
        (g.x_pf + self.proj_frente * t, g.x_pt - self.proj_tardoz * t)
    }

    /// Tensões admissíveis da alvenaria (banco de dados ou valores do projetista).
    pub fn alvenaria(&self) -> Propriedades {
        if self.personalizar_alvenaria {
            return Propriedades {
                sigma_adm: self.sigma_adm,
                tracao_adm: self.tracao_adm,
                c0: self.tau_adm,
                mu: 0.0,
                tau_max: f64::INFINITY,
                origem: "valores informados pelo projetista".into(),
            };
        }
        match self.material {
            Material::ConcretoSimples | Material::ConcretoCiclopico => alvenaria::concreto_simples(self.fck_alvenaria),
            Material::PedraArgamassada => alvenaria::pedra_argamassada(self.argamassa),
            Material::Gabiao | Material::PedraSeca => alvenaria::gabiao(self.material.peso()),
        }
    }

    /// Centro do dente (x, a partir da ponta): sob o pé do paramento frontal.
    fn x_dente(&self) -> f64 {
        self.geo().x_pf + self.dente_b / 2.0
    }

    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let g = self.geo();
        let (b, ht) = (g.base, g.htot);
        let beta = rad(self.empuxo.beta);
        if self.h <= 0.0 || self.f < 0.0 || self.proj_frente < 0.0 {
            r.aviso("Geometria inválida.");
        }
        if self.empuxo.beta > self.aterro.camadas[0].phi {
            r.aviso("Inclinação do talude β maior que φ do solo contido — talude instável.");
        }

        r.titulo("Dados");
        r.linha(format!(
            "Material: {} (γ = {} kN/m³) ; H = {} m ; F = {} m ; altura total = {} m",
            self.material.nome(), fmt(self.material.peso()), fmt(self.h), fmt(self.f), fmt(ht)
        ));
        r.linha(format!("γ do muro (valor fixo): {}", self.material.origem()));
        r.linha(format!(
            "A = {} ; B = {} ; C = {} ; D = {} ; E = {} m → base = {} m (base/altura = {})",
            fmt(self.ressalto_frente), fmt(self.proj_frente), fmt(self.crista), fmt(self.proj_tardoz),
            fmt(self.ressalto_tras), fmt(b), fmt(b / ht)
        ));
        if self.proj_frente > 0.0 {
            r.linha(format!("Paramento frontal 1 : {} (H:V) ; tardoz 1 : {}", fmt(self.proj_frente / self.h), fmt(self.proj_tardoz / self.h)));
        }
        r.linha(format!(
            "Teoria: {} ; δ = {}° ; β = {}° ; q = {} kPa",
            self.empuxo.teoria.nome(), fmt(self.empuxo.delta), fmt(self.empuxo.beta), fmt(self.cargas.q_uniforme)
        ));

        // Peso próprio
        let pw = self.poligono();
        let (area, cx, cy) = centroide(&pw);
        let w = area * self.material.peso();
        r.norma(normas::NBR6120_PESOS);
        r.formula(
            "Peso próprio do muro",
            "W = A·γ_muro",
            format!("W = {} m²·{} kN/m³", fmt(area), fmt(self.material.peso())),
            format!("W = {} kN/m  (centróide x = {} m ; y = {} m)", fmt(w), fmt(cx), fmt(cy)),
        );
        let mut base = vec![Forca::nova("Peso próprio do muro", 0.0, w, cx, cy)];

        // Plano de cálculo: reta do canto posterior da base (b, 0) ao canto
        // posterior da crista (Coulomb) ou vertical pelo calcanhar (Rankine).
        let dt = b - g.x_cr;
        let gs = self.aterro.gamma_medio(ht);
        let (theta, hv, plano_x): (f64, f64, Box<dyn Fn(f64) -> f64>) = match self.empuxo.teoria {
            Teoria::Coulomb => ((dt / ht).atan(), ht, Box::new(move |y| b - dt * y / ht)),
            Teoria::Rankine => (0.0, ht + dt.max(0.0) * beta.tan(), Box::new(move |_| b)),
        };
        // Solo entre o muro e o plano de cálculo (envoltória − muro)
        let mut env = vec![[0.0, 0.0], [b, 0.0]];
        if self.empuxo.teoria == Teoria::Rankine {
            env.push([b, hv]);
        }
        env.extend([[g.x_cr, ht], [g.x_cl, ht], [g.x_pf, self.f], [0.0, self.f]]);
        let (ae, xe, ye) = centroide(&env);
        let a_solo = ae - area;
        let mut cunha = None;
        if a_solo.abs() > 1e-6 {
            let xs = (ae * xe - area * cx) / a_solo;
            let ys = (ae * ye - area * cy) / a_solo;
            let peso = if a_solo > 0.0 { gs } else { self.material.peso() };
            base.push(Forca::nova("Solo sobre o tardoz / ressalto", 0.0, a_solo * peso, xs, ys));
            cunha = Some((a_solo * peso, xs, ys));
        }
        if self.empuxo.teoria == Teoria::Coulomb
            && a_solo > 1e-6
            && (self.empuxo.delta - self.empuxo.beta).abs() > 1e-6
        {
            r.aviso("Coulomb: o plano de cálculo (calcanhar → crista) passa pelo solo sobre o tardoz/ressalto — nesse trecho solo-solo recomenda-se δ = β.");
        }

        // NA no solo de fundação: o mais alto entre o informado e o do aterro
        // (profundidade do NA no plano de cálculo − altura do plano).
        let fundacao = self.fundacao.com_na(self.aterro.na().map(|n| n - hv));

        let frac_p = self.frac_passivo.min(1.0 / fat.reducao_passivo_muros.max(1.0));
        if self.dente && self.dente_h > 0.0 {
            let wd = self.dente_b * self.dente_h * self.material.peso();
            base.push(Forca::nova("Peso do dente de base", 0.0, wd, self.x_dente(), -self.dente_h / 2.0));
            r.norma(normas::NBR6122_PASSIVO);
            if let Some((p, kp)) = passivo_com_dente(&self.fundacao, self.df, self.dente_h, frac_p) {
                let d = self.df + self.dente_h;
                r.formula(
                    "Dente de base — passivo mobilizado até a base do dente",
                    "D = Df + hd ;  Ep = f·(½·Kp·γ·D² + 2c·√Kp·D) ;  Kp = tan²(45° + φ/2)",
                    format!(
                        "D = {} + {} ;  Kp = {} ;  γ = {} kN/m³ ;  c = {} kPa ;  f = {}",
                        fmt(self.df), fmt(self.dente_h), fmt(kp), fmt(self.fundacao.gamma), fmt(self.fundacao.c), fmt(frac_p)
                    ),
                    format!("D = {} m ;  Ep = {} kN/m (somado à resistência ao deslizamento)", fmt(d), fmt(-p.h)),
                );
                base.push(p);
            }
            r.aviso("Dente de base: o passivo à frente só pode ser considerado se o solo for mantido permanentemente (sem escavações futuras).");
        } else if self.considerar_passivo {
            r.norma(normas::NBR6122_PASSIVO);
            if let Some(p) = passivo_frontal(&self.fundacao, self.df, frac_p) {
                base.push(p);
            }
        }
        if self.subpressao {
            if let Some(u) = subpressao(&self.aterro, hv, b) {
                base.push(u);
            }
        }

        // Caso estático
        let est = empuxo_no_plano(&self.aterro, &self.cargas, &self.empuxo, theta, hv, &*plano_x, false);
        r.titulo("Empuxo");
        r.linha(format!(
            "θ do plano = {}° ; K ativo (camada superior) = {} ; Pa = {} kN/m ; inclinação = {}°",
            fmt(theta.to_degrees()), fmt(est.k), fmt(est.pa), fmt(est.incl.to_degrees())
        ));
        let ph_q = formula_empuxo(&mut r, &self.empuxo, &self.aterro, &self.cargas, theta, hv, &est);
        let mut f_est = base.clone();
        f_est.extend(est.forcas.iter().cloned());
        // ações permanentes (sem sobrecarga nem carga em faixa) — terço central
        let mut cargas_perm = self.cargas.clone();
        cargas_perm.q_uniforme = 0.0;
        cargas_perm.usar_faixa = false;
        let perm = empuxo_no_plano(&self.aterro, &cargas_perm, &self.empuxo, theta, hv, &*plano_x, false);
        let mut f_perm = base.clone();
        f_perm.extend(perm.forcas.iter().cloned());
        analisar(&mut r, "estático", &f_est, Some(&f_perm), b, &fundacao, self.df, &LimitesFs::estatico(fat));
        let ph_total: f64 = est.forcas.iter().map(|f| f.h.max(0.0)).sum();
        diagnostico_deslizamento(&mut r, ph_q, ph_total, self.considerar_passivo || self.dente);

        // Caso sísmico
        if self.cargas.usar_sismo {
            let sis = empuxo_no_plano(&self.aterro, &self.cargas, &self.empuxo, theta, hv, &*plano_x, true);
            let mut f = base.clone();
            f.extend(sis.forcas.iter().cloned());
            let (kh, kv) = (self.cargas.kh, self.cargas.kv);
            f.push(Forca::nova("Inércia do muro (kh·W)", kh * w, -kv * w, cx, cy));
            if let Some((ws, xs, ys)) = cunha {
                f.push(Forca::nova("Inércia do solo s/ tardoz", kh * ws, -kv * ws, xs, ys));
            }
            analisar(&mut r, "sísmico", &f, None, b, &fundacao, self.df, &LimitesFs::sismico(fat));
        }

        self.secoes_internas(&mut r, hv, theta);
        self.detalhamento(&mut r);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        if self.material == Material::Gabiao || self.material == Material::PedraSeca {
            r.aviso("Gabião/pedra seca: verificar também o deslizamento entre camadas e a especificação das telas/pedras.");
        }
        r.aviso("Estabilidade global (NBR 11682) ainda não implementada — verificar em software de equilíbrio limite.");
        r.desenho = self.desenho(&est.diagrama, hv);
        r
    }

    /// Tensões em seções horizontais do corpo (alvenaria/concreto não armado):
    /// compressão, tração e cisalhamento nas juntas.
    fn secoes_internas(&self, r: &mut Resultado, hv: f64, theta: f64) {
        let g = self.geo();
        let ht = g.htot;
        let prop = self.alvenaria();
        r.titulo("Seções internas do corpo (tensões na alvenaria)");
        if !self.personalizar_alvenaria {
            r.norma(match self.material {
                Material::ConcretoSimples | Material::ConcretoCiclopico => normas::NBR6118_CONCRETO_SIMPLES,
                Material::PedraArgamassada => normas::NBR16868_JUNTAS,
                Material::Gabiao | Material::PedraSeca => normas::MACCAFERRI_GABIAO,
            });
        }
        r.formula(
            format!("Tensões admissíveis — {}", self.material.nome()),
            "σadm ;  σt,adm ;  τadm = c0 + μ·σ ≤ τmáx",
            prop.origem.clone(),
            format!(
                "σadm = {} MPa ;  σt,adm = {} MPa ;  τadm = {} + {}·σ{} MPa",
                fmt(prop.sigma_adm),
                fmt(prop.tracao_adm),
                format!("{:.3}", prop.c0).replace('.', ","),
                format!("{:.3}", prop.mu).replace('.', ","),
                if prop.tau_max.is_finite() { format!(" ≤ {}", fmt(prop.tau_max)) } else { String::new() }
            ),
        );
        let concreto = matches!(self.material, Material::ConcretoSimples | Material::ConcretoCiclopico);
        let k_tau = if concreto { 1.5 } else { 1.0 };
        if concreto {
            r.norma(normas::MARCHETTI_CICLOPICO);
        }
        let incl = self.empuxo.inclinacao_ativa(theta);
        let mut pior_comp = 0.0f64;
        let mut pior_tracao = f64::INFINITY;
        // (razão τ/τadm, τ, τadm, cota)
        let mut pior_cis = (0.0f64, 0.0f64, f64::INFINITY, 0.0f64);
        for frac in [0.0, 0.25, 0.5, 0.75] {
            let ys = self.f + frac * self.h;
            let (xl, xr) = self.faces(ys);
            let w = xr - xl;
            if w <= 0.01 {
                continue;
            }
            let corpo = [[xl, ys], [xr, ys], [g.x_cr, ht], [g.x_cl, ht]];
            let (a, cx, _) = centroide(&corpo);
            let wgt = a * self.material.peso();
            // empuxo acima da seção (profundidades no plano de cálculo)
            let z0 = hv - ht;
            let z1 = hv - ys;
            let (ph, zc) = integrar(
                |z| self.empuxo.pressao_horizontal_total(&self.aterro, &self.cargas, theta, z),
                z0,
                z1,
                200,
            );
            let pv = integrar(|z| self.empuxo.pressao_ativa(&self.aterro, &self.cargas, theta, z), z0, z1, 200).0
                * incl.sin();
            let braco = (hv - zc) - ys;
            // solo apoiado sobre o tardoz inclinado acima da seção: o empuxo atua no
            // plano vertical que passa pela face interna da seção, logo a cunha entre
            // esse plano e o tardoz pesa sobre o corpo (Marchetti 4.1, parte 4)
            let (w_solo, m_solo) = if xr > g.x_cr + 1e-6 {
                let cunha = [[g.x_cr, ht], [xr, ys], [xr, ht]];
                let (a_s, sx, _) = centroide(&cunha);
                let ws = a_s * self.aterro.gamma_medio(ht);
                (ws, ws * (sx - xl))
            } else {
                (0.0, 0.0)
            };
            let n = wgt + pv + w_solo;
            let m_ponta = wgt * (cx - xl) + pv * w + m_solo - ph * braco;
            let x_r = m_ponta / n;
            let e = w / 2.0 - x_r;
            let (smax, smin) = if e.abs() <= w / 6.0 {
                (n / w * (1.0 + 6.0 * e.abs() / w), n / w * (1.0 - 6.0 * e.abs() / w))
            } else {
                let xr = x_r.min(w - x_r).max(1e-3);
                (2.0 * n / (3.0 * xr), -n / w * (6.0 * e.abs() / w - 1.0))
            };
            // concreto simples/ciclópico: τwd = 1,5·V/(b·h) — pico da distribuição
            // parabólica (Marchetti 2.1 c); juntas de alvenaria: τ médio.
            let tau = k_tau * ph / w / 1000.0;
            let tadm = prop.tau_adm(n / w / 1000.0);
            r.linha(format!(
                "y = {} m (largura {} m): N = {} kN/m ; H = {} kN/m ; e = {} m ; σmáx = {} kPa ; σmín = {} kPa ; τ = {} kPa",
                fmt(ys), fmt(w), fmt(n), fmt(ph), fmt(e), fmt(smax), fmt(smin), fmt(tau * 1000.0)
            ));
            pior_comp = pior_comp.max(smax);
            pior_tracao = pior_tracao.min(smin);
            let razao = if tadm > 0.0 { tau / tadm } else if tau > 0.0 { f64::INFINITY } else { 0.0 };
            if razao >= pior_cis.0 {
                pior_cis = (razao, tau, tadm, ys);
            }
        }
        r.formula(
            "Tensões nas seções horizontais do corpo",
            "σ = (N/b)·(1 ± 6e/b)  (|e| ≤ b/6) ;  σmáx = 2N / (3·x_R)  (|e| > b/6)",
            "N = peso do corpo acima da seção + solo sobre o tardoz + Pv ;  e = b/2 − ΣM/N",
            format!("σmáx = {} MPa ;  σmín = {} MPa", fmt(pior_comp / 1000.0), fmt(pior_tracao / 1000.0)),
        );
        r.verif(Verificacao::max("Compressão máx. no corpo", pior_comp / 1000.0, prop.sigma_adm, "MPa"));
        r.verif(Verificacao::min("Tensão mín. no corpo (tração)", pior_tracao / 1000.0, -prop.tracao_adm, "MPa"));
        r.formula(
            format!("Cisalhamento na junta horizontal mais solicitada (y = {} m)", fmt(pior_cis.3)),
            if concreto { "τ = 1,5·H / b ;  τadm = c0 + μ·(N/b)" } else { "τ = H / b ;  τadm = c0 + μ·(N/b)" },
            "H = empuxo horizontal acima da junta ;  N/b = compressão média na junta",
            format!(
                "τ = {} MPa ;  τadm = {} MPa",
                format!("{:.3}", pior_cis.1).replace('.', ","),
                format!("{:.3}", pior_cis.2).replace('.', ",")
            ),
        );
        r.verif(Verificacao::max("Cisalhamento nas juntas: τ ≤ τadm", pior_cis.1, pior_cis.2, "MPa"));
    }

    fn coroa_aplicavel(&self) -> bool {
        !matches!(self.material, Material::Gabiao | Material::PedraSeca)
    }

    /// Detalhamento construtivo: juntas, coroamento e drenagem de base.
    fn detalhamento(&self, r: &mut Resultado) {
        let g = self.geo();
        r.titulo("Detalhamento construtivo — muro de gravidade");
        // inclinação como 1 (H) : n (V), na notação do Marchetti (ex.: 1:12)
        let incl = |proj: f64| if proj < 0.005 { "vertical".to_string() } else { format!("1:{}", fmt(self.h / proj)) };
        r.linha(format!(
            "Seção: base {} m ; crista {} m ; altura total {} m (corpo {} m + base {} m) ; paramento {} ; tardoz {} (1 horizontal : n vertical)",
            fmt(g.base),
            fmt(self.crista),
            fmt(g.htot),
            fmt(self.h),
            fmt(self.f),
            incl(self.proj_frente),
            incl(self.proj_tardoz)
        ));
        r.norma(normas::NBR6122_LASTRO);
        r.norma(normas::NBR6122_PROF_MINIMA);
        r.linha("Fundação: lastro de concreto não estrutural (e ≥ 5 cm) sob toda a base; apoio a no mínimo 1,5 m de profundidade nas divisas, salvo em rocha.");
        if self.dente && self.dente_h > 0.0 {
            r.linha(format!(
                "Dente de base: {} m (largura) × {} m (profundidade), sob o pé do paramento, concretado contra o solo sem forma.",
                fmt(self.dente_b),
                fmt(self.dente_h)
            ));
        }
        if !self.coroa_aplicavel() {
            r.linha("Estrutura flexível (gabião/pedra seca): dispensa juntas de dilatação e viga de coroamento; caixas amarradas entre si com arame da mesma especificação da malha.");
            r.linha("Geotêxtil não tecido entre o tardoz e o aterro (filtro) e sob a primeira camada, se o solo de fundação for erodível.");
        } else {
            r.formula(
                "Juntas de dilatação",
                "Juntas verticais a cada s_j, atravessando base e corpo",
                format!("s_j = {} m", fmt(self.junta_espac)),
                format!(
                    "Juntas a cada {} m ; abertura 2 cm ; placa compressível (EPS) + selante ; geotêxtil cobrindo a junta no tardoz",
                    fmt(self.junta_espac)
                ),
            );
            let origem_junta = match self.material {
                Material::ConcretoSimples | Material::ConcretoCiclopico => "NBR 6118 24.4",
                _ => "critério de projeto",
            };
            r.verif(Verificacao::max(
                format!("Juntas de dilatação: espaçamento ≤ {} m ({origem_junta})", fmt(JUNTA_ESPAC_MAX)),
                self.junta_espac,
                JUNTA_ESPAC_MAX,
                "m",
            ));
            if self.coroamento {
                let (bc, hc) = (self.crista, self.coroamento_h.min(0.5 * self.h));
                let (fck, fyk, cob) = (25.0, 500.0, 0.03);
                let lb = armadura::lb_basico(10.0, fck, fyk, 1.4, 1.15, true);
                let as_min = crate::concreto::rho_min(fck) * bc * hc * 1e4;
                let as_ef = 4.0 * armadura::area_barra_cm2(10.0);
                let l_barra = (self.junta_espac - 2.0 * cob).max(1.0);
                let est = 2.0 * (bc - 2.0 * cob) + 2.0 * (hc - 2.0 * cob) + 2.0 * 0.10;
                r.formula(
                    "Viga de coroamento (concreto armado C25)",
                    "As,mín = ρmín·b·h ;  armadura construtiva 4 Ø10 + estribos Ø6,3 c/20",
                    format!("As,mín = {}·{}·{}", format!("{:.4}", crate::concreto::rho_min(fck)).replace('.', ","), fmt(bc), fmt(hc)),
                    format!("As,mín = {} cm² ;  4 Ø10 = {} cm² ;  lb = {} cm", fmt(as_min), fmt(as_ef), fmt(lb)),
                );
                r.armaduras.push(Armadura {
                    pos: "N1".into(),
                    elemento: "Coroamento — longitudinal".into(),
                    bitola_mm: 10.0,
                    espac_cm: None,
                    quantidade: "4".into(),
                    comprimento_m: l_barra,
                    as_nec: as_min,
                    as_ef,
                    unidade: "cm²",
                    lb_cm: lb,
                });
                r.armaduras.push(Armadura {
                    pos: "N2".into(),
                    elemento: "Coroamento — estribos".into(),
                    bitola_mm: 6.3,
                    espac_cm: Some(20.0),
                    quantidade: "5/m".into(),
                    comprimento_m: est,
                    as_nec: 0.0,
                    as_ef: armadura::area_barra_cm2(6.3) * 2.0 * 5.0,
                    unidade: "cm²/m",
                    lb_cm: 0.0,
                });
                r.linha(format!(
                    "Coroamento: viga {} × {} m em concreto armado C25, 4 Ø10 (N1) e estribos Ø6,3 c/20 (N2), interrompida nas juntas.",
                    fmt(bc),
                    fmt(hc)
                ));
            }
        }
        r.linha("Drenagem de base: colchão drenante, dreno de pé e barbacãs conforme a seção “Drenagem”.");
    }

    fn desenho(&self, diag: &[(f64, f64)], _hv: f64) -> Desenho {
        let mut d = Desenho::default();
        let g = self.geo();
        let (ht, b, f) = (g.htot, g.base, self.f);
        let beta = rad(self.empuxo.beta);
        let xe = b + 1.4 * ht;
        let xi = -0.6 * ht;
        let sup = |x: f64| ht + (x - g.x_cr).max(0.0) * beta.tan();

        d.retangulo(Estilo::SoloFundacao, xi, -0.35 * ht, xe, 0.0);
        if self.df > 0.0 {
            d.retangulo(Estilo::Solo, xi, 0.0, 0.0, self.df.min(f));
            if self.df > f {
                let xf = self.faces(self.df).0;
                d.poli(Estilo::Solo, vec![[xi, f], [g.x_pf, f], [xf, self.df], [xi, self.df]]);
            }
        }
        // maciço contido (dois polígonos convexos)
        d.poli(Estilo::Solo, vec![[b, 0.0], [xe, 0.0], [xe, sup(xe)], [b, sup(b)]]);
        d.poli(Estilo::Solo, vec![[g.x_pt, f], [b, f], [b, sup(b)], [g.x_cr, ht]]);
        // muro: base + corpo (polígonos convexos)
        d.retangulo(Estilo::Concreto, 0.0, 0.0, b, f);
        if !(self.dente && self.dente_h > 0.0) {
            // lastro de concreto magro (NBR 6122 7.7.3), e = 5 cm
            d.retangulo(Estilo::Concreto, -0.05, -0.05, b + 0.05, 0.0);
        }
        if self.dente && self.dente_h > 0.0 {
            let x0 = self.x_dente() - self.dente_b / 2.0;
            d.retangulo(Estilo::Concreto, x0, -self.dente_h, x0 + self.dente_b, 0.0);
            d.texto([x0 + self.dente_b / 2.0, -self.dente_h - 0.06 * ht], "dente");
        }
        d.poli(Estilo::Concreto, vec![[g.x_pf, f], [g.x_pt, f], [g.x_cr, ht], [g.x_cl, ht]]);

        if self.coroamento && self.coroa_aplicavel() {
            let hc = self.coroamento_h.min(0.5 * self.h);
            d.retangulo(Estilo::Aco, g.x_cl, ht - hc, g.x_cr, ht);
            let c = 0.04;
            for (x, y) in [(g.x_cl + c, ht - hc + c), (g.x_cr - c, ht - hc + c), (g.x_cl + c, ht - c), (g.x_cr - c, ht - c)] {
                d.circulo(Estilo::ArmaduraSecao, [x, y], 0.012 * ht.max(1.0));
            }
            d.texto([(g.x_cl + g.x_cr) / 2.0, ht + 0.16 * ht], "coroamento");
        }
        if let Some(na) = self.aterro.na() {
            let y = ht - na;
            if y > 0.0 {
                d.tracejada(Estilo::Agua, vec![[self.faces(y.max(f)).1, y], [xe, y]]);
                d.texto([xe - 0.25 * ht, y + 0.07 * ht], "NA");
            }
        }
        if self.cargas.q_uniforme > 0.0 {
            let a = 0.1 * ht;
            for i in 0..=8 {
                let x = g.x_cr + 0.15 + (xe - g.x_cr - 0.15) * i as f64 / 8.0;
                d.seta([x, sup(x) + a], [x, sup(x)]);
            }
            d.texto([(b + xe) / 2.0, sup((b + xe) / 2.0) + 1.7 * a], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        if self.cargas.usar_faixa {
            let x0 = g.x_cr + self.cargas.faixa_dist;
            d.retangulo(Estilo::Carga, x0, sup(x0), x0 + self.cargas.faixa_larg, sup(x0) + 0.07 * ht);
        }
        let pmax = diag.iter().map(|p| p.1.abs()).fold(1e-6, f64::max);
        let pts: Vec<(f64, f64)> = diag.iter().map(|&(y, p)| (y.min(sup(b)), p)).collect();
        d.diagrama(b + 1.4 * ht * 0.55, 0.3 * ht / pmax, &pts);
        d.texto([b + 0.95 * ht, -0.08 * ht], format!("p máx = {} kPa", fmt(pmax)));

        // cotas do croqui (A..F, H)
        let c = ht * 0.05;
        d.texto([g.x_pf / 2.0, f + c], format!("A={}", fmt(self.ressalto_frente)));
        d.texto([(g.x_pf + g.x_cl) / 2.0 - 2.0 * c, (f + ht) / 2.0], format!("B={}", fmt(self.proj_frente)));
        d.texto([(g.x_cl + g.x_cr) / 2.0, ht + 1.2 * c], format!("C={}", fmt(self.crista)));
        d.texto([(g.x_pt + b) / 2.0 + 0.5, f + c], format!("E={}", fmt(self.ressalto_tras)));
        d.texto([b / 2.0, -0.12 * ht], format!("Base = {} m", fmt(b)));
        d.texto([xi * 0.45, f / 2.0], format!("F = {}", fmt(f)));
        d.tracejada(Estilo::Cota, vec![[xi * 0.2, f], [xi * 0.2, ht]]);
        d.texto([xi * 0.55, (f + ht) / 2.0], format!("H = {} m", fmt(self.h)));
        d
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fatores::Fatores;

    #[test]
    fn muro_retangular_manual() {
        // Bloco 2,0 x 3,0 m (F = 0, sem ressaltos), concreto simples γ = 24 -> W = 144 kN/m a x = 1,0
        // Rankine φ=30°, γ=18 -> Pa = 27 kN/m a y = 1,0 -> FS tomb = 144/27 = 5,333
        // FS desliz = 144·tan(20°)/27 = 1,941
        let mut m = MuroGravidade::default();
        m.material = Material::ConcretoSimples;
        m.h = 3.0;
        m.f = 0.0;
        m.crista = 2.0;
        m.proj_frente = 0.0;
        m.proj_tardoz = 0.0;
        m.ressalto_frente = 0.0;
        m.ressalto_tras = 0.0;
        m.subpressao = false;
        m.cargas.q_uniforme = 0.0;
        m.fundacao.c = 0.0;
        let r = m.calcular(&Fatores::default());
        let fs = |nome: &str| r.verificacoes.iter().find(|v| v.nome.starts_with(nome)).unwrap().valor;
        assert!((fs("FS tombamento") - 5.333).abs() < 0.01, "{}", fs("FS tombamento"));
        assert!((fs("FS deslizamento") - 1.941).abs() < 0.01, "{}", fs("FS deslizamento"));
    }

    #[test]
    fn capacidade_de_carga_com_na() {
        let fs_cap = |na: Option<f64>| {
            let mut m = MuroGravidade::default();
            m.subpressao = false;
            m.fundacao.na_abaixo_base = na;
            let r = m.calcular(&Fatores::default());
            r.verificacoes.iter().find(|v| v.nome.starts_with("FS capacidade de carga — NBR 11682 Tab. 4 / NBR 6122 (estático")).unwrap().valor
        };
        let seco = fs_cap(None);
        let na_base = fs_cap(Some(0.0));
        let na_fundo = fs_cap(Some(50.0));
        assert!(na_base < seco, "NA na base deve reduzir o FS: {na_base} x {seco}");
        assert!((na_fundo - seco).abs() < 1e-9);
    }

    #[test]
    fn avisos_sobrecarga_e_coulomb() {
        let mut m = MuroGravidade::default();
        m.cargas.q_uniforme = 10.0;
        let r = m.calcular(&Fatores::default());
        assert!(r.avisos.iter().any(|a| a.contains("Sobrecarga") && a.contains("NBR 11682")));
        assert!(!r.avisos.iter().any(|a| a.contains("δ = β")));

        m.empuxo.teoria = Teoria::Coulomb;
        m.ressalto_tras = 0.4;
        let r = m.calcular(&Fatores::default());
        assert!(r.avisos.iter().any(|a| a.contains("δ = β")));
        m.empuxo.delta = m.empuxo.beta;
        let r = m.calcular(&Fatores::default());
        assert!(!r.avisos.iter().any(|a| a.contains("δ = β")));
    }

    #[test]
    fn muro_1m_calculo_manual() {
        // Bloco 0,60 × 1,00 m (F = 0), concreto simples γ = 24 → W = 14,4 kN/m a x = 0,30 m.
        // Rankine φ = 30°, γ = 18, q = 0 → Pa = ½·⅓·18·1² = 3,0 kN/m a y = 1/3 m.
        // FS tomb = M_res / M_tomb = 14,4·0,30 / (3,0·1/3) = 4,32
        // FS desl = 14,4·tan(⅔·30°) / 3,0 = 1,747 ;  e = 0,30 − (4,32 − 1,0)/14,4 = 0,0694 m ≤ B/6 = 0,10
        let mut m = MuroGravidade::default();
        m.material = Material::ConcretoSimples;
        m.h = 1.0;
        m.f = 0.0;
        m.crista = 0.6;
        m.proj_frente = 0.0;
        m.proj_tardoz = 0.0;
        m.ressalto_frente = 0.0;
        m.ressalto_tras = 0.0;
        m.subpressao = false;
        m.cargas.q_uniforme = 0.0;
        m.fundacao.c = 0.0;
        let r = m.calcular(&Fatores::default());
        let v = |nome: &str| r.verificacoes.iter().find(|v| v.nome.starts_with(nome)).unwrap().valor;
        assert!((v("FS tombamento") - 4.32).abs() < 0.01, "{}", v("FS tombamento"));
        assert!((v("FS deslizamento") - 1.747).abs() < 0.01, "{}", v("FS deslizamento"));
        assert!((v("Excentricidade") - 0.0694).abs() < 0.001, "{}", v("Excentricidade"));
        // base de 0,60 m (B/H = 0,6) atende tombamento, deslizamento e excentricidade
        assert!(r.verificacoes.iter().filter(|v| v.nome.contains("(estático)")).all(|v| v.ok()));

        // Com q = 20 kPa: Ph,q = ⅓·20·1 = 6,67 kN/m → T = 9,67 ; FS desl = 5,24/9,67 = 0,54 e aviso didático
        m.cargas.q_uniforme = 20.0;
        let r = m.calcular(&Fatores::default());
        let fs = r.verificacoes.iter().find(|v| v.nome.starts_with("FS deslizamento")).unwrap().valor;
        assert!((fs - 0.542).abs() < 0.01, "{fs}");
        assert!(r.avisos.iter().any(|a| a.contains("sobrecarga responde por 69")), "{:?}", r.avisos);
    }

    /// Base mínima (bloco retangular) de um muro de 1 m que atende todas as
    /// verificações estáticas de estabilidade.
    fn base_minima_1m(fat: &Fatores, prep: impl Fn(&mut MuroGravidade)) -> f64 {
        let mut m = MuroGravidade::default();
        m.material = Material::ConcretoCiclopico;
        m.h = 1.0;
        m.f = 0.3;
        m.proj_frente = 0.0;
        m.proj_tardoz = 0.0;
        m.ressalto_frente = 0.0;
        m.ressalto_tras = 0.0;
        m.cargas.q_uniforme = 20.0;
        prep(&mut m);
        let mut b = 0.30;
        while b < 3.0 {
            m.crista = b;
            let r = m.calcular(fat);
            if r.verificacoes.iter().filter(|v| v.nome.contains("(estático)")).all(|v| v.ok()) {
                return b;
            }
            b += 0.05;
        }
        f64::INFINITY
    }

    #[test]
    fn criterios_marchetti_reduzem_base_do_muro_baixo() {
        let padrao = base_minima_1m(&Fatores::default(), |_| {});
        let mut fat = Fatores::default();
        fat.excentricidade_redistribuicao = true;
        let tangente = |m: &mut MuroGravidade| {
            m.fundacao.criterio_atrito = crate::solo::CriterioAtrito::TangenteReduzida;
            m.fundacao.frac_delta_base = 0.67;
        };
        let pratico = base_minima_1m(&fat, tangente);
        eprintln!("muro 1 m, q = 20 kPa: base mínima padrão = {padrao:.2} m ; critérios Marchetti = {pratico:.2} m");
        assert!(pratico < padrao, "padrão {padrao} ; prático {pratico}");
    }

    #[test]
    fn dente_aumenta_deslizamento() {
        let mut m = MuroGravidade::default();
        m.cargas.q_uniforme = 0.0;
        let fs = |m: &MuroGravidade| {
            let r = m.calcular(&Fatores::default());
            r.verificacoes.iter().find(|v| v.nome.starts_with("FS deslizamento")).unwrap().valor
        };
        let sem = fs(&m);
        m.dente = true;
        let com = fs(&m);
        assert!(com > sem + 0.05, "sem {sem} ; com {com}");
        // passivo até a base do dente: D = 0,6 + 0,4 = 1,0 m ; Kp(30°) = 3 ; c = 10 ; f = 0,5
        // Ep = 0,5·(½·3·19·1² + 2·10·√3·1) = 0,5·(28,5 + 34,64) = 31,57 kN/m
        let (p, _) = passivo_com_dente(&m.fundacao, 0.6, 0.4, 0.5).unwrap();
        assert!((-p.h - 31.57).abs() < 0.05, "{}", -p.h);
    }

    #[test]
    fn terco_central_com_acoes_permanentes() {
        // muro de 1 m com q = 20 kPa: o terço central usa só as ações permanentes
        let mut m = MuroGravidade::default();
        m.h = 1.0;
        m.f = 0.3;
        m.crista = 0.5;
        m.proj_frente = 0.3;
        m.proj_tardoz = 0.0;
        m.cargas.q_uniforme = 20.0;
        let e = |f: &Fatores| {
            let r = m.calcular(f);
            r.verificacoes.iter().find(|v| v.nome.starts_with("Excentricidade")).unwrap().valor
        };
        // padrão (NBR 11682 7.3.3.1): todas as ações ; exceção justificada: só permanentes
        let mut f = Fatores::default();
        assert!(!f.terco_central_permanente);
        let e_total = e(&f);
        f.terco_central_permanente = true;
        let e_perm = e(&f);
        assert!(e_perm < e_total, "perm {e_perm} ; total {e_total}");
        let r = m.calcular(&f);
        assert!(r.avisos.iter().any(|a| a.contains("exceção à NBR 11682 7.3.3.1")));
    }

    #[test]
    fn banco_de_alvenaria_e_detalhamento() {
        let mut m = MuroGravidade::default();
        m.material = Material::ConcretoCiclopico;
        let p = m.alvenaria();
        assert!((p.sigma_adm - crate::alvenaria::concreto_simples(15.0).sigma_adm).abs() < 1e-12);
        let r = m.calcular(&Fatores::default());
        assert!(r.verificacoes.iter().any(|v| v.nome.starts_with("Cisalhamento nas juntas")));
        assert!(r.armaduras.iter().any(|a| a.elemento.contains("Coroamento")));
        m.personalizar_alvenaria = true;
        m.sigma_adm = 3.0;
        assert_eq!(m.alvenaria().sigma_adm, 3.0);
        m.material = Material::Gabiao;
        m.personalizar_alvenaria = false;
        let r = m.calcular(&Fatores::default());
        assert!(r.armaduras.is_empty(), "gabião sem coroamento armado");
    }

    #[test]
    fn geometria_do_croqui() {
        let m = MuroGravidade::default();
        let g = m.geo();
        let b = m.ressalto_frente + m.proj_frente + m.crista + m.proj_tardoz + m.ressalto_tras;
        assert!((g.base - b).abs() < 1e-12);
        let (a, _, _) = centroide(&m.poligono());
        // base B×F + trapézio do corpo
        let b_corpo = m.proj_frente + m.crista + m.proj_tardoz;
        assert!((a - (b * m.f + (b_corpo + m.crista) / 2.0 * m.h)).abs() < 1e-9);
    }
}

#[cfg(test)]
mod teste_exemplo_marchetti {
    use super::*;
    use crate::fatores::Fatores;

    /// Exemplo 4.1 do Marchetti (2007): H = 5,80 m + base 0,90 m, β = 10°, φ = 32°,
    /// γ = 18 kN/m³, concreto ciclópico γ = 22 kN/m³; Ea = 141,56, Eh = 139,40 e
    /// Ev = 24,58 kN/m; FS tombamento = 2,12 (livro) — aqui γc = 24 (NBR 6120).
    #[test]
    fn exemplo_4_1_marchetti() {
        let mut m = MuroGravidade::default();
        m.material = Material::ConcretoCiclopico;
        m.h = 5.8;
        m.f = 0.9;
        m.crista = 0.45;
        m.proj_frente = 0.48;
        m.ressalto_frente = 0.42;
        m.proj_tardoz = 1.55;
        m.ressalto_tras = 0.15;
        m.df = 1.2;
        m.subpressao = false;
        m.cargas.q_uniforme = 0.0;
        m.empuxo.teoria = Teoria::Rankine;
        m.empuxo.beta = 10.0;
        m.aterro = Perfil::simples(18.0, 32.0, 0.0);
        m.fundacao.phi = 36.0;
        m.fundacao.c = 0.0;
        let r = m.calcular(&Fatores::default());
        let empuxo = r
            .blocos
            .iter()
            .find_map(|b| match b {
                crate::resultado::Bloco::Formula(f) if f.titulo.starts_with("Empuxo ativo resultante") => Some(f.resultado.clone()),
                _ => None,
            })
            .expect("fórmula do empuxo");
        // livro: Ea = 141,56 ; Eh = 139,40 ; Ev = 24,58 kN/m
        assert!(empuxo.contains("Pa = 141,5") && empuxo.contains("Ph = 139,4") && empuxo.contains("Pv = 24,58"), "{empuxo}");
        // livro: FS tombamento = 2,12 com γc = 22; com γc = 24 (NBR 6120) o muro pesa ≈ 9 % mais
        let fs = r.verificacoes.iter().find(|v| v.nome.starts_with("FS tombamento")).unwrap().valor;
        assert!((2.0..2.4).contains(&fs), "FS tombamento = {fs}");
    }
}
