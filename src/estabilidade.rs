//! Verificações de estabilidade externa de muros (gravidade e flexão):
//! deslizamento, tombamento, excentricidade e capacidade de carga.
//!
//! Origem do sistema: ponta (bico) da base, x → para dentro do aterro, y ↑.

use crate::empuxo::rad;
use crate::normas;
use crate::resultado::{fmt, Resultado, Verificacao};
use crate::solo::SoloFundacao;
use std::f64::consts::PI;

/// Força aplicada ao muro (kN/m).
/// `h` > 0 empurra o muro para a frente (instabilizante);
/// `v` > 0 atua para baixo.
#[derive(Clone, Debug)]
pub struct Forca {
    pub nome: String,
    pub h: f64,
    pub v: f64,
    pub x: f64,
    pub y: f64,
}

impl Forca {
    pub fn nova(nome: &str, h: f64, v: f64, x: f64, y: f64) -> Self {
        Self { nome: nome.into(), h, v, x, y }
    }
}

pub struct LimitesFs {
    pub deslizamento: f64,
    pub tombamento: f64,
    pub capacidade: f64,
    pub excentricidade: f64,
    pub area_comprimida: f64,
}

impl LimitesFs {
    pub fn estatico(f: &crate::fatores::Fatores) -> Self {
        Self {
            deslizamento: f.fs_deslizamento,
            tombamento: f.fs_tombamento,
            capacidade: f.fs_capacidade_carga,
            excentricidade: f.excentricidade_max,
            area_comprimida: f.area_comprimida_min,
        }
    }
    pub fn sismico(f: &crate::fatores::Fatores) -> Self {
        Self {
            deslizamento: f.fs_deslizamento_sismo,
            tombamento: f.fs_tombamento_sismo,
            capacidade: f.fs_capacidade_carga_sismo,
            excentricidade: f.excentricidade_max,
            area_comprimida: f.area_comprimida_min,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DistribuicaoBase {
    pub b: f64,
    pub n: f64,
    pub x_r: f64,
    pub e: f64,
}

impl DistribuicaoBase {
    /// Tensão de contato (kPa) na abscissa `x` (0 = ponta).
    pub fn pressao(&self, x: f64) -> f64 {
        let (b, n, e) = (self.b, self.n, self.e);
        if n <= 0.0 {
            return 0.0;
        }
        if e.abs() <= b / 6.0 {
            let q0 = n / b * (1.0 + 6.0 * e / b);
            let qb = n / b * (1.0 - 6.0 * e / b);
            q0 + (qb - q0) * x / b
        } else if e > 0.0 {
            let l = 3.0 * self.x_r;
            if x < l { 2.0 * n / l * (1.0 - x / l) } else { 0.0 }
        } else {
            let l = 3.0 * (b - self.x_r);
            let d = b - x;
            if d < l { 2.0 * n / l * (1.0 - d / l) } else { 0.0 }
        }
    }
    /// Comprimento da base em compressão (m).
    pub fn comprimento_comprimido(&self) -> f64 {
        if self.n <= 0.0 {
            0.0
        } else if self.e.abs() <= self.b / 6.0 {
            self.b
        } else if self.e > 0.0 {
            (3.0 * self.x_r).clamp(0.0, self.b)
        } else {
            (3.0 * (self.b - self.x_r)).clamp(0.0, self.b)
        }
    }
    pub fn qmax(&self) -> f64 {
        self.pressao(0.0).max(self.pressao(self.b))
    }
    pub fn qmin(&self) -> f64 {
        self.pressao(0.0).min(self.pressao(self.b))
    }
}

/// Área e centróide (área, x, y) de um polígono simples.
pub fn centroide(p: &[[f64; 2]]) -> (f64, f64, f64) {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..p.len() {
        let [x0, y0] = p[i];
        let [x1, y1] = p[(i + 1) % p.len()];
        let cr = x0 * y1 - x1 * y0;
        a += cr;
        cx += (x0 + x1) * cr;
        cy += (y0 + y1) * cr;
    }
    a *= 0.5;
    if a.abs() < 1e-12 {
        return (0.0, p[0][0], p[0][1]);
    }
    (a.abs(), cx / (6.0 * a), cy / (6.0 * a))
}

/// Capacidade de carga (kPa) de sapata corrida — Terzaghi/Meyerhof/Vesic, com
/// fatores de inclinação de Meyerhof e largura efetiva B' (fatores de
/// profundidade desprezados, a favor da segurança).
pub fn capacidade_carga(phi_g: f64, c: f64, gamma: f64, q: f64, b_ef: f64, alfa: f64) -> (f64, [f64; 3]) {
    let phi = rad(phi_g);
    let (nq, nc, ng) = if phi < 1e-6 {
        (1.0, 5.14, 0.0)
    } else {
        let nq = (PI * phi.tan()).exp() * (PI / 4.0 + phi / 2.0).tan().powi(2);
        (nq, (nq - 1.0) / phi.tan(), 2.0 * (nq + 1.0) * phi.tan())
    };
    let ic = (1.0 - alfa / (PI / 2.0)).max(0.0).powi(2);
    let ig = if phi > 1e-6 { (1.0 - alfa / phi).max(0.0).powi(2) } else { 0.0 };
    let qult = c * nc * ic + q * nq * ic + 0.5 * gamma * b_ef * ng * ig;
    (qult, [nc, nq, ng])
}

/// Executa as verificações externas e registra no `Resultado`.
pub fn analisar(
    res: &mut Resultado,
    caso: &str,
    forcas: &[Forca],
    b: f64,
    fund: &SoloFundacao,
    df: f64,
    lim: &LimitesFs,
) -> DistribuicaoBase {
    res.titulo(&format!("Estabilidade externa — {caso}"));
    res.linha(format!("{:<34} {:>9} {:>9} {:>7} {:>7}", "Força", "H (kN/m)", "V (kN/m)", "x (m)", "y (m)"));
    for f in forcas {
        res.linha(format!("{:<34} {:>9} {:>9} {:>7} {:>7}", f.nome, fmt(f.h), fmt(f.v), fmt(f.x), fmt(f.y)));
    }

    let (mut m_est, mut m_inst, mut n, mut t, mut r_pas, mut h_liq, mut m_liq) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for f in forcas {
        if f.v > 0.0 { m_est += f.v * f.x } else { m_inst += -f.v * f.x }
        if f.h > 0.0 { m_inst += f.h * f.y; t += f.h } else { m_est += -f.h * f.y; r_pas += -f.h }
        n += f.v;
        h_liq += f.h;
        m_liq += f.v * f.x - f.h * f.y;
    }

    // Tombamento
    let fs_t = if m_inst > 0.0 { m_est / m_inst } else { f64::INFINITY };
    res.linha(format!("Momento estabilizante = {} kN·m/m ; instabilizante = {} kN·m/m", fmt(m_est), fmt(m_inst)));
    res.norma(normas::NBR11682_TOMBAMENTO);
    res.formula(
        format!("Tombamento em torno da ponta ({caso})"),
        "FS = ΣM_est / ΣM_inst",
        format!("FS = {} / {}", fmt(m_est), fmt(m_inst)),
        format!("FS = {}", fmt(fs_t)),
    );
    res.verif(Verificacao::min(format!("FS tombamento — NBR 11682 Tab. 4 ({caso})"), fs_t, lim.tombamento, ""));

    // Deslizamento
    let tan_db = (rad(fund.phi) * fund.frac_delta_base).tan();
    let ca = fund.c * fund.frac_adesao;
    let r = n.max(0.0) * tan_db + ca * b + r_pas;
    let fs_d = if t > 0.0 { r / t } else { f64::INFINITY };
    res.linha(format!(
        "N = {} kN/m ; tan δb = {} ; adesão = {} kPa ; passivo = {} kN/m ; R = {} kN/m ; T = {} kN/m",
        fmt(n), fmt(tan_db), fmt(ca), fmt(r_pas), fmt(r), fmt(t)
    ));
    res.norma(normas::NBR11682_DESLIZAMENTO);
    res.formula(
        format!("Deslizamento na base ({caso})"),
        "FS = (N·tan δb + ca·B + Ep) / ΣH ;  δb = k·φ ;  ca = k·c",
        format!(
            "FS = ({}·{} + {}·{} + {}) / {}",
            fmt(n.max(0.0)), fmt(tan_db), fmt(ca), fmt(b), fmt(r_pas), fmt(t)
        ),
        format!("R = {} kN/m ;  FS = {}", fmt(r), fmt(fs_d)),
    );
    res.verif(Verificacao::min(format!("FS deslizamento na base — NBR 11682 Tab. 4 ({caso})"), fs_d, lim.deslizamento, ""));

    // Excentricidade
    let x_r = if n > 0.0 { m_liq / n } else { 0.0 };
    let e = b / 2.0 - x_r;
    let dist = DistribuicaoBase { b, n, x_r, e };
    res.linha(format!("Resultante a x = {} m da ponta ; e = {} m ; B/6 = {} m", fmt(x_r), fmt(e), fmt(b / 6.0)));
    res.norma(normas::NBR11682_TERCO_CENTRAL);
    res.formula(
        format!("Excentricidade da resultante ({caso})"),
        "x_R = ΣM_ponta / N ;  e = B/2 − x_R  ≤  B/6",
        format!("x_R = {} / {} = {} m ;  e = {}/2 − {}", fmt(m_liq), fmt(n), fmt(x_r), fmt(b), fmt(x_r)),
        format!("e = {} m  (B/6 = {} m)", fmt(e), fmt(b / 6.0)),
    );
    res.verif(Verificacao::max(
        format!("Excentricidade |e| — terço central, NBR 11682 ({caso})"),
        e.abs(),
        lim.excentricidade * b,
        "m",
    ));
    res.norma(normas::NBR6122_AREA_COMPRIMIDA);
    res.formula(
        format!("Tensões na base ({caso})"),
        "σ = (N/B)·(1 ± 6e/B)  se |e| ≤ B/6 ;  σmáx = 2N / (3·x_R)  se |e| > B/6",
        format!("σ = ({}/{})·(1 ± 6·{}/{})", fmt(n), fmt(b), fmt(e.abs()), fmt(b)),
        format!(
            "σmáx = {} kPa ;  σmín = {} kPa ;  comprimido = {} m de {} m",
            fmt(dist.qmax()), fmt(dist.qmin()), fmt(dist.comprimento_comprimido()), fmt(b)
        ),
    );
    res.verif(Verificacao::min(
        format!("Área comprimida / B — NBR 6122 7.6.2 ({caso})"),
        dist.comprimento_comprimido() / b,
        lim.area_comprimida,
        "",
    ));
    if n <= 0.0 || x_r <= 0.0 || x_r >= b {
        res.aviso(format!("{caso}: resultante fora da base — muro instável."));
    }
    res.linha(format!("σmáx = {} kPa ; σmín = {} kPa", fmt(dist.qmax()), fmt(dist.qmin())));

    // Capacidade de carga
    let b_ef = (b - 2.0 * e.abs()).max(0.0);
    let alfa = if n > 0.0 { (h_liq.max(0.0) / n).atan() } else { PI / 2.0 };
    let gamma_ng = fund.gamma_nq_gama(b_ef);
    let q_sob = fund.sobrecarga_efetiva(df);
    let (qult, [nc, nq, ng]) = capacidade_carga(fund.phi, fund.c, gamma_ng, q_sob, b_ef, alfa);
    let q_ef = if b_ef > 0.0 { n / b_ef } else { f64::INFINITY };
    let fs_c = if q_ef > 0.0 { qult / q_ef } else { f64::INFINITY };
    if let Some(dw) = fund.na_abaixo_base {
        res.linha(format!(
            "NA a {} m {} da base → γ (parcela Nγ) = {} kN/m³ ; sobrecarga efetiva q = {} kPa",
            fmt(dw.abs()),
            if dw >= 0.0 { "abaixo" } else { "acima" },
            fmt(gamma_ng),
            fmt(q_sob)
        ));
    }
    res.linha(format!(
        "B' = {} m ; inclinação da carga = {}° ; Nc = {} ; Nq = {} ; Nγ = {} ; qult = {} kPa ; q' = N/B' = {} kPa",
        fmt(b_ef), fmt(alfa.to_degrees()), fmt(nc), fmt(nq), fmt(ng), fmt(qult), fmt(q_ef)
    ));
    let phi_r = rad(fund.phi);
    let ic = (1.0 - alfa / (PI / 2.0)).max(0.0).powi(2);
    let ig = if phi_r > 1e-6 { (1.0 - alfa / phi_r).max(0.0).powi(2) } else { 0.0 };
    res.norma(normas::NBR11682_CAPACIDADE);
    res.norma(normas::NBR6122_FS_GLOBAL);
    res.formula(
        format!("Capacidade de carga — Meyerhof/Vesic com B' ({caso})"),
        "q_ult = c·Nc·ic + q·Nq·iq + ½·γ·B'·Nγ·iγ ;  B' = B − 2|e| ;  FS = q_ult / (N/B')",
        format!(
            "q_ult = {}·{}·{} + {}·{}·{} + ½·{}·{}·{}·{}",
            fmt(fund.c), fmt(nc), fmt(ic), fmt(q_sob), fmt(nq), fmt(ic), fmt(gamma_ng), fmt(b_ef), fmt(ng), fmt(ig)
        ),
        format!("q_ult = {} kPa ;  q' = {} kPa ;  FS = {}", fmt(qult), fmt(q_ef), fmt(fs_c)),
    );
    res.verif(Verificacao::min(format!("FS capacidade de carga — NBR 11682 Tab. 4 / NBR 6122 ({caso})"), fs_c, lim.capacidade, ""));
    dist
}
