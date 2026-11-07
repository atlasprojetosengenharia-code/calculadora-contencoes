//! Banco de dados de alvenarias: tensões admissíveis à compressão, à tração
//! e ao cisalhamento nas juntas, por material do muro de gravidade.
//!
//! Os valores são ADMISSÍVEIS (comparados com tensões das ações
//! características), obtidos como resistência de cálculo ÷ γf = 1,4:
//! - concreto simples / ciclópico: ABNT NBR 6118:2023, 24.5.2 (γc = 1,2 × 1,4 = 1,68;
//!   σcRd = 0,85·fcd; σctRd = 0,85·fctd; τwRd = 0,30·fctd·(1 + 3·σcmd/fck), fator ≤ 2);
//! - pedra argamassada: compressão de referência usual (1,0 MPa, sem tração) e
//!   cisalhamento nas juntas por ANALOGIA com a ABNT NBR 16868-1:2020, Tabela 4
//!   (fvk = f0 + 0,5·σ, σ = 0,9 × compressão permanente; γm = 2,0 — Tabela 2);
//! - gabião e pedra seca: Manual técnico Maccaferri — σadm = 50·γg − 30 (tf/m²)
//!   e atrito entre camadas φ* = 25·γg − 10 (°), γg em tf/m³; sem tração.

use serde::{Deserialize, Serialize};

const GAMMA_F: f64 = 1.4;
/// NBR 6118 24.4: γc do concreto simples = 1,2 × 1,4.
const GAMMA_C_SIMPLES: f64 = 1.68;
/// NBR 16868-1 Tabela 2 (combinações normais).
const GAMMA_M_ALVENARIA: f64 = 2.0;

/// Faixa de resistência média à compressão da argamassa (NBR 16868-1, Tabelas 3 e 4).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClasseArgamassa {
    /// 1,5 a 3,4 MPa
    Fraca,
    /// 3,5 a 7,0 MPa
    Media,
    /// acima de 7,0 MPa
    Forte,
}

impl ClasseArgamassa {
    pub fn nome(&self) -> &'static str {
        match self {
            ClasseArgamassa::Fraca => "1,5 a 3,4 MPa",
            ClasseArgamassa::Media => "3,5 a 7,0 MPa",
            ClasseArgamassa::Forte => "acima de 7,0 MPa",
        }
    }
    /// fvk = f0 + 0,5·σ ≤ fmáx (MPa) — NBR 16868-1 Tabela 4.
    pub fn fvk(&self) -> (f64, f64) {
        match self {
            ClasseArgamassa::Fraca => (0.10, 1.0),
            ClasseArgamassa::Media => (0.15, 1.4),
            ClasseArgamassa::Forte => (0.35, 1.7),
        }
    }
}

pub const CLASSES_ARGAMASSA: [ClasseArgamassa; 3] = [ClasseArgamassa::Fraca, ClasseArgamassa::Media, ClasseArgamassa::Forte];

/// Tensões admissíveis da alvenaria (MPa).
#[derive(Clone, Debug)]
pub struct Propriedades {
    pub sigma_adm: f64,
    pub tracao_adm: f64,
    /// Cisalhamento admissível: τadm = c0 + μ·σ ≤ τmáx (σ = compressão média na junta, MPa).
    pub c0: f64,
    pub mu: f64,
    pub tau_max: f64,
    pub origem: String,
}

impl Propriedades {
    pub fn tau_adm(&self, sigma_mpa: f64) -> f64 {
        (self.c0 + self.mu * sigma_mpa.max(0.0)).min(self.tau_max)
    }
}

/// Concreto simples (NBR 6118 Seção 24), tensões admissíveis.
pub fn concreto_simples(fck: f64) -> Propriedades {
    let fcd = fck / GAMMA_C_SIMPLES;
    let fctd = 0.21 * fck.powf(2.0 / 3.0) / GAMMA_C_SIMPLES;
    Propriedades {
        sigma_adm: 0.85 * fcd / GAMMA_F,
        tracao_adm: 0.85 * fctd / GAMMA_F,
        // τwRd = 0,30·fctd·(1 + 3·σcmd/fck) ≤ 2·0,30·fctd ; σcmd = 1,4·σ (característica → cálculo)
        c0: 0.30 * fctd / GAMMA_F,
        mu: 0.30 * fctd / GAMMA_F * 3.0 * GAMMA_F / fck,
        tau_max: 2.0 * 0.30 * fctd / GAMMA_F,
        origem: format!(
            "NBR 6118:2023 24.5.2 — fck = {} MPa, γc = 1,68: σ = 0,85·fcd/1,4 ; σt = 0,85·fctd/1,4 ; τ = 0,30·fctd·(1 + 3·1,4·σ/fck)/1,4 ≤ 2×",
            crate::resultado::fmt(fck)
        ),
    }
}

/// Pedra argamassada: compressão de referência, sem tração, cisalhamento por
/// analogia com a NBR 16868-1 (Tabela 4).
pub fn pedra_argamassada(arg: ClasseArgamassa) -> Propriedades {
    let (f0, fmax) = arg.fvk();
    let k = GAMMA_M_ALVENARIA * GAMMA_F;
    Propriedades {
        sigma_adm: 1.0,
        tracao_adm: 0.0,
        c0: f0 / k,
        mu: 0.5 * 0.9 / k,
        tau_max: fmax / k,
        origem: format!(
            "compressão 1,0 MPa (referência usual) e tração nula ; cisalhamento por analogia NBR 16868-1 Tab. 4 (argamassa {}): τ = ({} + 0,5·0,9·σ)/(2,0·1,4)",
            arg.nome(),
            crate::resultado::fmt(f0)
        ),
    }
}

/// Gabião / pedra seca (Manual técnico Maccaferri).
pub fn gabiao(gamma_kn: f64) -> Propriedades {
    let g = gamma_kn / 9.80665; // tf/m³
    let sigma_tf = (50.0 * g - 30.0).max(0.0); // tf/m²
    let phi = (25.0 * g - 10.0).clamp(0.0, 45.0);
    Propriedades {
        sigma_adm: sigma_tf * 9.80665 / 1000.0,
        tracao_adm: 0.0,
        c0: 0.0,
        mu: phi.to_radians().tan(),
        tau_max: f64::INFINITY,
        origem: format!(
            "Manual Maccaferri: σadm = 50·γg − 30 = {} tf/m² ; atrito entre camadas φ* = 25·γg − 10 = {}° (γg = {} tf/m³) ; sem tração",
            crate::resultado::fmt(sigma_tf),
            crate::resultado::fmt(phi),
            crate::resultado::fmt(g)
        ),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn valores_do_banco() {
        // C15: fcd = 8,93 ; σadm = 0,85·8,93/1,4 = 5,42 MPa ; fctd = 0,21·15^(2/3)/1,68 = 0,761 ; σt = 0,462
        let c = concreto_simples(15.0);
        assert!((c.sigma_adm - 5.42).abs() < 0.01, "{}", c.sigma_adm);
        assert!((c.tracao_adm - 0.462).abs() < 0.002, "{}", c.tracao_adm);
        // τ = 0,163·(1 + 3·1,4·1,0/15) = 0,163·1,28 = 0,209 ; limite 2 × 0,163
        assert!((c.tau_adm(0.0) - 0.163).abs() < 0.002);
        assert!((c.tau_adm(1.0) - 0.209).abs() < 0.002, "{}", c.tau_adm(1.0));
        assert!((c.tau_adm(50.0) - 0.326).abs() < 0.003);
        // pedra argamassada, argamassa 3,5–7: τ = (0,15 + 0,45·σ)/2,8
        let p = pedra_argamassada(ClasseArgamassa::Media);
        assert_eq!((p.sigma_adm, p.tracao_adm), (1.0, 0.0));
        assert!((p.tau_adm(0.1) - (0.15 + 0.045) / 2.8).abs() < 1e-9);
        assert!((p.tau_adm(10.0) - 1.4 / 2.8).abs() < 1e-9);
        // gabião 17,1 kN/m³ = 1,744 tf/m³ → σ = 57,2 tf/m² = 0,561 MPa ; φ* = 33,6°
        let g = gabiao(17.1);
        assert!((g.sigma_adm - 0.561).abs() < 0.002, "{}", g.sigma_adm);
        assert!((g.mu - 33.6f64.to_radians().tan()).abs() < 0.01);
    }
}
