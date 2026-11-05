//! Dimensionamento de concreto armado — NBR 6118 (fck ≤ 50 MPa).
//! Unidades: momentos em kN·m, dimensões em m, tensões em MPa, áreas em cm².

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Concreto {
    pub fck: f64,
    pub fyk: f64,
    /// Cobrimento nominal (m).
    pub cobrimento: f64,
    /// Peso específico do concreto armado (kN/m³).
    pub peso: f64,
}

impl Default for Concreto {
    fn default() -> Self {
        Self { fck: 30.0, fyk: 500.0, cobrimento: 0.04, peso: 25.0 }
    }
}

/// Taxa mínima de armadura de flexão (NBR 6118, Tab. 17.3), em fração.
pub fn rho_min(fck: f64) -> f64 {
    let tab = [(20.0, 0.150), (30.0, 0.150), (35.0, 0.164), (40.0, 0.179), (45.0, 0.194), (50.0, 0.208)];
    let f = fck.clamp(20.0, 50.0);
    for w in tab.windows(2) {
        let (a, ra) = w[0];
        let (b, rb) = w[1];
        if f <= b {
            return (ra + (rb - ra) * (f - a) / (b - a)) / 100.0;
        }
    }
    0.00208
}

#[derive(Clone, Debug)]
pub struct Flexao {
    pub as_calc: f64,
    pub as_min: f64,
    pub x_d: f64,
    pub viavel: bool,
}

impl Flexao {
    pub fn as_adotar(&self) -> f64 {
        self.as_calc.max(self.as_min)
    }
}

/// Fator ηc (NBR 6118:2023, 8.2.10.1).
pub fn eta_c(fck: f64) -> f64 {
    if fck <= 40.0 { 1.0 } else { (40.0 / fck).cbrt() }
}

/// Tensão resistente à punção sem armadura de punção — τRd1 (MPa),
/// NBR 6118:2023 19.5.3.2 (σcp desprezado). `d` em m.
pub fn tau_rd1_puncao(d: f64, rho: f64, fck: f64) -> f64 {
    let ke = (1.0 + (20.0 / (d * 100.0)).sqrt()).min(2.0);
    0.13 * ke * (100.0 * rho.min(0.02) * fck).cbrt()
}

/// Armadura simples para seção retangular b×h com altura útil d
/// (diagrama retangular: y = 0,8x ; σc = 0,85·ηc·fcd — NBR 6118:2023 17.2.2).
pub fn flexao_retangular(md: f64, b: f64, h: f64, d: f64, fck: f64, fyk: f64, gc: f64, gs: f64) -> Flexao {
    let fcd = eta_c(fck) * fck / gc;
    let fyd = fyk / gs;
    let (bm, dm) = (b * 1000.0, d * 1000.0);
    let m = md.abs() * 1e6;
    let as_min = rho_min(fck) * b * h * 1e4;
    let k = 2.0 * m / (0.85 * fcd * bm * dm * dm);
    if k >= 1.0 || dm <= 0.0 {
        return Flexao { as_calc: f64::INFINITY, as_min, x_d: f64::INFINITY, viavel: false };
    }
    let y = dm * (1.0 - (1.0 - k).sqrt());
    let x_d = y / 0.8 / dm;
    let as_mm2 = m / (fyd * (dm - y / 2.0));
    // Limite de ductilidade NBR 6118 14.6.4.3 (x/d ≤ 0,45 para fck ≤ 50)
    Flexao { as_calc: as_mm2 / 100.0, as_min, x_d, viavel: x_d <= 0.45 }
}

/// Força cortante resistente sem armadura transversal — VRd1 (kN), NBR 6118 19.4.1.
pub fn vrd1(b: f64, d: f64, as_cm2: f64, fck: f64, gc: f64) -> f64 {
    let fctd = 0.7 * 0.3 * fck.powf(2.0 / 3.0) / gc;
    let trd = 0.25 * fctd;
    let k = (1.6 - d).max(1.0);
    let rho = (as_cm2 / 1e4 / (b * d)).min(0.02);
    trd * k * (1.2 + 40.0 * rho) * (b * 1000.0) * (d * 1000.0) / 1000.0
}

/// Momento resistente (kN·m) de seção circular com `n` barras uniformemente
/// distribuídas — diagrama retangular com 0,9·αc·ηc·fcd (largura decrescente
/// para a borda comprimida, NBR 6118:2023 17.2.2 e), εcu = 3,5‰, flexão simples.
pub fn mrd_circular(diam: f64, c_eixo: f64, as_cm2: f64, n: usize, fck: f64, fyk: f64, gc: f64, gs: f64) -> f64 {
    let r = diam * 500.0; // mm
    let rs = r - c_eixo * 1000.0;
    let fcd = eta_c(fck) * fck / gc;
    let fyd = fyk / gs;
    let es = 210_000.0;
    let ab = as_cm2 * 100.0 / n as f64;
    let barras: Vec<f64> = (0..n).map(|i| r - rs * (2.0 * PI * i as f64 / n as f64).cos()).collect();

    let segmento = |a: f64| -> (f64, f64) {
        // área e distância do centróide ao centro do círculo
        let a = a.clamp(0.0, 2.0 * r);
        if a <= 0.0 {
            return (0.0, 0.0);
        }
        let h = r - a;
        let s = (2.0 * r * a - a * a).max(0.0);
        let area = r * r * (h / r).clamp(-1.0, 1.0).acos() - h * s.sqrt();
        let yb = if area > 1e-9 { (2.0 / 3.0) * s.powf(1.5) / area } else { 0.0 };
        (area, yb)
    };
    let esforcos = |x: f64| -> (f64, f64) {
        let (ac, yc) = segmento(0.8 * x);
        let fc = 0.9 * 0.85 * fcd * ac;
        let (mut n_tot, mut m_tot) = (fc, fc * yc);
        for &yi in &barras {
            let eps = 0.0035 * (x - yi) / x;
            let sig = (es * eps).clamp(-fyd, fyd);
            n_tot += sig * ab;
            m_tot += sig * ab * (r - yi);
        }
        (n_tot, m_tot)
    };
    let (mut lo, mut hi) = (1e-3, 4.0 * r);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if esforcos(mid).0 > 0.0 { hi = mid } else { lo = mid }
    }
    esforcos(0.5 * (lo + hi)).1 / 1e6
}

/// Armadura longitudinal necessária (cm²) em seção circular para `md`.
/// Retorna `None` se nem 4% da seção for suficiente.
pub fn as_circular(md: f64, diam: f64, c_eixo: f64, n: usize, fck: f64, fyk: f64, gc: f64, gs: f64) -> Option<f64> {
    let ac = PI * diam * diam / 4.0 * 1e4;
    let max = 0.04 * ac;
    if mrd_circular(diam, c_eixo, max, n, fck, fyk, gc, gs) < md.abs() {
        return None;
    }
    let (mut lo, mut hi) = (0.0, max);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if mrd_circular(diam, c_eixo, mid, n, fck, fyk, gc, gs) >= md.abs() { hi = mid } else { lo = mid }
    }
    Some(hi)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn flexao_exemplo() {
        // b=1 m, h=0,30, d=0,25, Md=100 kN·m, C30, CA-50 → As ≈ 9,7 cm²/m
        let f = flexao_retangular(100.0, 1.0, 0.30, 0.25, 30.0, 500.0, 1.4, 1.15);
        assert!((f.as_calc - 9.7).abs() < 0.3, "As = {}", f.as_calc);
        assert!(f.viavel);
    }

    #[test]
    fn circular_monotono() {
        let m1 = mrd_circular(0.6, 0.06, 10.0, 10, 30.0, 500.0, 1.4, 1.15);
        let m2 = mrd_circular(0.6, 0.06, 20.0, 10, 30.0, 500.0, 1.4, 1.15);
        assert!(m2 > m1 && m1 > 0.0);
        let a = as_circular(m1, 0.6, 0.06, 10, 30.0, 500.0, 1.4, 1.15).unwrap();
        assert!((a - 10.0).abs() < 0.1);
    }
}

#[cfg(test)]
mod testes_2023 {
    use super::*;

    #[test]
    fn eta_c_e_puncao() {
        assert_eq!(eta_c(30.0), 1.0);
        assert!((eta_c(50.0) - (0.8f64).cbrt()).abs() < 1e-12);
        // d = 5 cm -> 1 + sqrt(4) = 3 -> limitado a 2
        let t = tau_rd1_puncao(0.05, 0.01, 30.0);
        assert!((t - 0.13 * 2.0 * 30f64.cbrt()).abs() < 1e-9);
    }
}
