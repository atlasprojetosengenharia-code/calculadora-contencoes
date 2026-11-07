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
    /// Tensão de compressão admissível da alvenaria/concreto (MPa).
    pub sigma_adm: f64,
    /// Tração admissível nas juntas (MPa) — 0 = resultante no núcleo central.
    pub tracao_adm: f64,
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
            sigma_adm: 1.0,
            tracao_adm: 0.0,
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

        if self.considerar_passivo {
            r.norma(normas::NBR6122_PASSIVO);
            if let Some(p) =
                passivo_frontal(&self.fundacao, self.df, self.frac_passivo.min(1.0 / fat.reducao_passivo_muros.max(1.0)))
            {
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
        analisar(&mut r, "estático", &f_est, b, &fundacao, self.df, &LimitesFs::estatico(fat));
        let ph_total: f64 = est.forcas.iter().map(|f| f.h.max(0.0)).sum();
        diagnostico_deslizamento(&mut r, ph_q, ph_total, self.considerar_passivo);

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
            analisar(&mut r, "sísmico", &f, b, &fundacao, self.df, &LimitesFs::sismico(fat));
        }

        self.secoes_internas(&mut r, hv, theta);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        if self.material == Material::Gabiao || self.material == Material::PedraSeca {
            r.aviso("Gabião/pedra seca: verificar também o deslizamento entre camadas e a especificação das telas/pedras.");
        }
        r.aviso("Estabilidade global (NBR 11682) ainda não implementada — verificar em software de equilíbrio limite.");
        r.desenho = self.desenho(&est.diagrama, hv);
        r
    }

    /// Tensões em seções horizontais do corpo (alvenaria/concreto não armado).
    fn secoes_internas(&self, r: &mut Resultado, hv: f64, theta: f64) {
        let g = self.geo();
        let ht = g.htot;
        r.titulo("Seções internas do corpo (tensões na alvenaria)");
        r.linha(format!(
            "σ admissível = {} MPa ; tração admissível = {} MPa (critério do projetista)",
            fmt(self.sigma_adm), fmt(self.tracao_adm)
        ));
        let incl = self.empuxo.inclinacao_ativa(theta);
        let mut pior_comp = 0.0f64;
        let mut pior_tracao = f64::INFINITY;
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
            let n = wgt + pv;
            let m_ponta = wgt * (cx - xl) + pv * w - ph * braco;
            let x_r = m_ponta / n;
            let e = w / 2.0 - x_r;
            let (smax, smin) = if e.abs() <= w / 6.0 {
                (n / w * (1.0 + 6.0 * e.abs() / w), n / w * (1.0 - 6.0 * e.abs() / w))
            } else {
                let xr = x_r.min(w - x_r).max(1e-3);
                (2.0 * n / (3.0 * xr), -n / w * (6.0 * e.abs() / w - 1.0))
            };
            r.linha(format!(
                "y = {} m (largura {} m): N = {} kN/m ; H = {} kN/m ; e = {} m ; σmáx = {} kPa ; σmín = {} kPa",
                fmt(ys), fmt(w), fmt(n), fmt(ph), fmt(e), fmt(smax), fmt(smin)
            ));
            pior_comp = pior_comp.max(smax);
            pior_tracao = pior_tracao.min(smin);
        }
        r.formula(
            "Tensões nas seções horizontais do corpo",
            "σ = (N/b)·(1 ± 6e/b)  (|e| ≤ b/6) ;  σmáx = 2N / (3·x_R)  (|e| > b/6)",
            "N = peso do corpo acima da seção + Pv ;  e = b/2 − ΣM/N",
            format!("σmáx = {} MPa ;  σmín = {} MPa", fmt(pior_comp / 1000.0), fmt(pior_tracao / 1000.0)),
        );
        r.verif(Verificacao::max("Compressão máx. no corpo", pior_comp / 1000.0, self.sigma_adm, "MPa"));
        r.verif(Verificacao::min("Tensão mín. no corpo (tração)", pior_tracao / 1000.0, -self.tracao_adm, "MPa"));
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
        d.poli(Estilo::Concreto, vec![[g.x_pf, f], [g.x_pt, f], [g.x_cr, ht], [g.x_cl, ht]]);

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
