//! Coeficientes e distribuições de empuxo — Rankine, Coulomb e
//! Mononobe-Okabe (sismo).
//!
//! Ângulos de entrada em graus nas structs; funções `k*` recebem radianos.
//! `theta` = inclinação do tardoz com a vertical, positiva quando o solo fica
//! sobre o tardoz (base do muro avança para dentro do aterro).

use crate::solo::{Cargas, Perfil};
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_4;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Teoria {
    Rankine,
    Coulomb,
}

impl Teoria {
    pub fn nome(&self) -> &'static str {
        match self {
            Teoria::Rankine => "Rankine",
            Teoria::Coulomb => "Coulomb",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamEmpuxo {
    pub teoria: Teoria,
    /// Ângulo de atrito solo-estrutura δ (graus) — usado em Coulomb.
    pub delta: f64,
    /// Inclinação do terrapleno β (graus).
    pub beta: f64,
}

impl Default for ParamEmpuxo {
    fn default() -> Self {
        Self { teoria: Teoria::Rankine, delta: 20.0, beta: 0.0 }
    }
}

pub fn rad(g: f64) -> f64 {
    g.to_radians()
}

pub fn ka_rankine(phi: f64, beta: f64) -> f64 {
    if beta.abs() < 1e-9 {
        return (FRAC_PI_4 - phi / 2.0).tan().powi(2);
    }
    let cb = beta.cos();
    let r = (cb * cb - phi.cos().powi(2)).max(0.0).sqrt();
    cb * (cb - r) / (cb + r)
}

pub fn kp_rankine(phi: f64) -> f64 {
    (FRAC_PI_4 + phi / 2.0).tan().powi(2)
}

/// Mononobe-Okabe (reduz-se a Coulomb para kh = kv = 0).
pub fn kae_mo(phi: f64, delta: f64, theta: f64, beta: f64, kh: f64, kv: f64) -> f64 {
    let psi = (kh / (1.0 - kv)).atan();
    let num = (phi - theta - psi).cos().powi(2);
    let s = ((phi + delta).sin() * (phi - beta - psi).sin().max(0.0)
        / ((delta + theta + psi).cos() * (theta - beta).cos()))
    .max(0.0)
    .sqrt();
    num / (psi.cos() * theta.cos().powi(2) * (delta + theta + psi).cos() * (1.0 + s).powi(2))
}

pub fn ka_coulomb(phi: f64, delta: f64, theta: f64, beta: f64) -> f64 {
    kae_mo(phi, delta, theta, beta, 0.0, 0.0)
}

pub fn kp_coulomb(phi: f64, delta: f64, theta: f64, beta: f64) -> f64 {
    let num = (phi + theta).cos().powi(2);
    let s = ((phi + delta).sin() * (phi + beta).sin() / ((delta - theta).cos() * (beta - theta).cos()))
        .max(0.0)
        .sqrt();
    let den = theta.cos().powi(2) * (delta - theta).cos() * (1.0 - s).powi(2);
    if den <= 1e-9 { 1e6 } else { num / den }
}

impl ParamEmpuxo {
    pub fn coef_ativo(&self, phi_graus: f64, theta: f64) -> f64 {
        let phi = rad(phi_graus);
        match self.teoria {
            Teoria::Rankine => ka_rankine(phi, rad(self.beta)),
            Teoria::Coulomb => ka_coulomb(phi, rad(self.delta), theta, rad(self.beta)),
        }
    }

    /// Coeficiente passivo para terreno horizontal e paramento vertical.
    pub fn coef_passivo(&self, phi_graus: f64, delta_graus: f64) -> f64 {
        let phi = rad(phi_graus);
        match self.teoria {
            Teoria::Rankine => kp_rankine(phi),
            Teoria::Coulomb => kp_coulomb(phi, rad(delta_graus), 0.0, 0.0),
        }
    }

    /// Inclinação da resultante ativa com a horizontal (rad).
    pub fn inclinacao_ativa(&self, theta: f64) -> f64 {
        match self.teoria {
            Teoria::Rankine => rad(self.beta),
            Teoria::Coulomb => rad(self.delta) + theta,
        }
    }

    /// Pressão ativa (kPa, por unidade de altura vertical, na direção da
    /// resultante) — solo + sobrecarga uniforme + coesão. Sem água e sem
    /// carga em faixa.
    pub fn pressao_ativa(&self, perfil: &Perfil, cargas: &Cargas, theta: f64, z: f64) -> f64 {
        let cam = perfil.camada_em(z);
        let k = self.coef_ativo(cam.phi, theta);
        (k * (perfil.sigma_v_ef(z) + cargas.q_uniforme) - 2.0 * cam.c * k.sqrt()).max(0.0)
    }

    /// Pressão horizontal total no lado ativo: solo (componente horizontal) +
    /// água + carga em faixa.
    pub fn pressao_horizontal_total(&self, perfil: &Perfil, cargas: &Cargas, theta: f64, z: f64) -> f64 {
        self.pressao_ativa(perfil, cargas, theta, z) * self.inclinacao_ativa(theta).cos()
            + perfil.u(z)
            + cargas.pressao_faixa(z)
    }

    /// Acréscimo sísmico de empuxo ΔPAE (kN/m) para altura `h` — Mononobe-Okabe
    /// com γ e φ médios. Retorna (ΔP, KAE, KA).
    pub fn incremento_sismico(&self, perfil: &Perfil, cargas: &Cargas, theta: f64, h: f64) -> (f64, f64, f64) {
        let g = perfil.gamma_medio(h);
        let phi = rad(perfil.phi_medio(h));
        let (delta, th) = match self.teoria {
            Teoria::Rankine => (0.0, 0.0),
            Teoria::Coulomb => (rad(self.delta), theta),
        };
        let beta = rad(self.beta);
        let kae = kae_mo(phi, delta, th, beta, cargas.kh, cargas.kv);
        let ka = kae_mo(phi, delta, th, beta, 0.0, 0.0);
        let dk = ((1.0 - cargas.kv) * kae - ka).max(0.0);
        ((0.5 * g * h * h + cargas.q_uniforme * h) * dk, kae, ka)
    }
}

/// Integra f(z) entre `a` e `b` (regra do ponto médio).
/// Retorna (resultante, profundidade do centro de pressão).
pub fn integrar<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> (f64, f64) {
    if b <= a {
        return (0.0, a);
    }
    let dz = (b - a) / n as f64;
    let (mut r, mut m) = (0.0, 0.0);
    for i in 0..n {
        let z = a + (i as f64 + 0.5) * dz;
        let p = f(z) * dz;
        r += p;
        m += p * z;
    }
    let zc = if r.abs() > 1e-12 { m / r } else { (a + b) / 2.0 };
    (r, zc)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn rankine_classico() {
        assert!((ka_rankine(rad(30.0), 0.0) - 1.0 / 3.0).abs() < 1e-9);
        assert!((kp_rankine(rad(30.0)) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn coulomb_reduz_a_rankine() {
        // δ = 0, θ = 0, β = 0 → Coulomb = Rankine
        assert!((ka_coulomb(rad(30.0), 0.0, 0.0, 0.0) - 1.0 / 3.0).abs() < 1e-9);
        assert!((kp_coulomb(rad(30.0), 0.0, 0.0, 0.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn coulomb_tabelado() {
        // Das, Tab. 7.x: φ=30°, δ=20°, θ=0, β=0 → Ka ≈ 0.297
        let ka = ka_coulomb(rad(30.0), rad(20.0), 0.0, 0.0);
        assert!((ka - 0.297).abs() < 0.002, "Ka = {ka}");
    }

    #[test]
    fn mononobe_okabe_cresce_com_kh() {
        let k0 = kae_mo(rad(30.0), rad(15.0), 0.0, 0.0, 0.0, 0.0);
        let k1 = kae_mo(rad(30.0), rad(15.0), 0.0, 0.0, 0.2, 0.0);
        assert!(k1 > k0);
        // Valor de referência (Kramer): φ=30, δ=15, kh=0.2 → KAE ≈ 0.452
        assert!((k1 - 0.452).abs() < 0.01, "KAE = {k1}");
    }
}
