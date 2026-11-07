//! Detalhamento do aço: escolha de bitola e espaçamento, ancoragem,
//! traspasse, armadura de distribuição e telas soldadas — ABNT NBR 6118:2023.

use crate::resultado::fmt;

/// Bitolas comerciais CA-50/CA-60 (mm).
pub const BITOLAS: [f64; 8] = [6.3, 8.0, 10.0, 12.5, 16.0, 20.0, 25.0, 32.0];

pub fn area_barra_cm2(phi_mm: f64) -> f64 {
    std::f64::consts::PI * phi_mm * phi_mm / 400.0
}

/// Linha da tabela de armação.
#[derive(Clone, Debug)]
pub struct Armadura {
    /// Posição (N1, N2 …).
    pub pos: String,
    pub elemento: String,
    pub bitola_mm: f64,
    /// Espaçamento (cm) — `None` para barras contadas (ex.: estacas).
    pub espac_cm: Option<f64>,
    /// Quantidade (por metro ou por peça), texto livre.
    pub quantidade: String,
    /// Comprimento unitário aproximado da barra (m).
    pub comprimento_m: f64,
    /// As necessária e efetiva (cm²/m ou cm², ver `unidade`).
    pub as_nec: f64,
    pub as_ef: f64,
    pub unidade: &'static str,
    /// Comprimento de ancoragem necessário (cm).
    pub lb_cm: f64,
}

impl Armadura {
    pub fn descricao(&self) -> String {
        if self.bitola_mm <= 0.0 {
            return format!("{} {}", self.pos, self.quantidade);
        }
        match self.espac_cm {
            Some(s) => format!("{} Ø{} c/{}", self.pos, fmt_bitola(self.bitola_mm), fmt_bitola(s)),
            None => format!("{} {} Ø{}", self.pos, self.quantidade, fmt_bitola(self.bitola_mm)),
        }
    }
}

/// Cabeçalho da tabela de armação.
pub const CABECALHO: [&str; 9] = ["Pos.", "Elemento", "Bitola", "s (cm)", "Quant.", "C (m)", "As,nec", "As,ef", "lb (cm)"];

impl Armadura {
    /// Células da tabela de armação (tela, .txt, HTML e DXF).
    pub fn celulas(&self) -> [String; 9] {
        [
            self.pos.clone(),
            self.elemento.clone(),
            if self.bitola_mm > 0.0 { format!("Ø{}", fmt_bitola(self.bitola_mm)) } else { "—".into() },
            self.espac_cm.map(fmt_bitola).unwrap_or_else(|| "—".into()),
            self.quantidade.clone(),
            if self.comprimento_m > 0.0 { format!("≈ {}", fmt(self.comprimento_m)) } else { "corrida".into() },
            if self.as_nec > 0.0 { format!("{} {}", fmt(self.as_nec), self.unidade) } else { "—".into() },
            format!("{} {}", fmt(self.as_ef), self.unidade),
            if self.lb_cm > 0.0 { fmt(self.lb_cm) } else { "—".into() },
        ]
    }
}

pub fn fmt_bitola(phi: f64) -> String {
    if (phi - phi.round()).abs() < 1e-9 { format!("{phi:.0}") } else { format!("{phi:.1}").replace('.', ",") }
}

/// Escolhe bitola e espaçamento para `as_nec` (cm²/m): menor bitola em
/// `bitolas` com espaçamento ≥ `s_min`, limitado a `s_max` (cm, múltiplo de 2,5).
/// Retorna (Ø, s, As efetiva).
pub fn malha(as_nec: f64, bitolas: &[f64], s_min: f64, s_max: f64) -> (f64, f64, f64) {
    let as_nec = as_nec.max(1e-6);
    for &phi in bitolas {
        let s = ((area_barra_cm2(phi) / as_nec * 100.0) / 2.5).floor() * 2.5;
        if s >= s_min {
            let s = s.min(s_max);
            return (phi, s, area_barra_cm2(phi) * 100.0 / s);
        }
    }
    let phi = *bitolas.last().unwrap_or(&32.0);
    (phi, s_min, area_barra_cm2(phi) * 100.0 / s_min)
}

/// Bitolas permitidas numa placa de espessura h: Ø ≤ h/8 (NBR 6118 20.1).
fn bitolas_ate(h_m: f64, de: usize) -> Vec<f64> {
    let v: Vec<f64> = BITOLAS[de..].iter().copied().filter(|&p| p <= h_m * 1000.0 / 8.0).collect();
    if v.is_empty() { vec![BITOLAS[de]] } else { v }
}

/// Armadura principal de laje/parede: Ø ≥ 8 mm e ≤ h/8 ; 10 ≤ s ≤ mín(20 cm, 2h) — NBR 6118 20.1.
pub fn malha_principal(as_nec: f64, h_m: f64) -> (f64, f64, f64) {
    malha(as_nec, &bitolas_ate(h_m, 1), 10.0, (2.0 * h_m * 100.0).min(20.0))
}

/// Armadura de distribuição (NBR 6118 Tabela 19.1): ≥ 20 % da principal,
/// ≥ 0,9 cm²/m e ≥ 0,5·ρmin·b·h ; s ≤ 33 cm.
pub fn as_distribuicao(as_principal: f64, fck: f64, h_m: f64) -> f64 {
    (0.2 * as_principal).max(0.9).max(0.5 * crate::concreto::rho_min(fck) * h_m * 1e4)
}

pub fn malha_distribuicao(as_nec: f64, h_m: f64) -> (f64, f64, f64) {
    malha(as_nec, &bitolas_ate(h_m, 0), 10.0, 30.0)
}

/// Comprimento de ancoragem básico lb (cm) — NBR 6118 9.4.2.4:
/// lb = (Ø/4)·(fyd/fbd) ≥ 25Ø ; fbd = η1·η2·η3·fctd ; η1 = 2,25 (nervurada).
pub fn lb_basico(phi_mm: f64, fck: f64, fyk: f64, gc: f64, gs: f64, boa_aderencia: bool) -> f64 {
    let fctd = 0.21 * fck.powf(2.0 / 3.0) / gc;
    let eta2 = if boa_aderencia { 1.0 } else { 0.7 };
    let eta3 = if phi_mm < 32.0 { 1.0 } else { (132.0 - phi_mm) / 100.0 };
    let fbd = 2.25 * eta2 * eta3 * fctd;
    let lb_mm = phi_mm / 4.0 * (fyk / gs) / fbd;
    (lb_mm.max(25.0 * phi_mm)) / 10.0
}

/// lb,nec = lb·As,calc/As,ef ≥ lb,mín = máx(0,3·lb ; 10Ø ; 10 cm) — NBR 6118 9.4.2.5 (α = 1).
pub fn lb_nec(lb: f64, phi_mm: f64, as_calc: f64, as_ef: f64) -> f64 {
    let r = if as_ef > 0.0 { (as_calc / as_ef).min(1.0) } else { 1.0 };
    (lb * r).max(0.3 * lb).max(phi_mm).max(10.0)
}

/// Traspasse de barras tracionadas: l0t = α0t·lb,nec ≥ máx(0,3·α0t·lb ; 15Ø ; 20 cm),
/// α0t = 2,0 (100 % das barras emendadas na mesma seção — arranque) — NBR 6118 9.5.2.2.
pub fn traspasse(lb: f64, lbnec: f64, phi_mm: f64) -> f64 {
    let a = 2.0;
    (a * lbnec).max(0.3 * a * lb).max(1.5 * phi_mm).max(20.0)
}

/// Viga de coroamento/travamento com armadura construtiva mínima:
/// As,mín = ρmín·b·h (NBR 6118 Tabela 17.3) em barras Ø12,5 (mínimo 4) e
/// estribos Ø6,3 c/15. Retorna as duas linhas da tabela (posições `p1`, `p2`).
pub fn viga_construtiva(p1: &str, p2: &str, elemento: &str, b: f64, h: f64, fck: f64, fyk: f64, cob: f64) -> Vec<Armadura> {
    let as_min = crate::concreto::rho_min(fck) * b * h * 1e4;
    let ab = area_barra_cm2(12.5);
    let n = ((as_min / ab).ceil() as usize).max(4);
    let n = n + n % 2;
    let lb = lb_basico(12.5, fck, fyk, 1.4, 1.15, false);
    let est = 2.0 * (b - 2.0 * cob) + 2.0 * (h - 2.0 * cob) + 2.0 * 0.08;
    vec![
        Armadura {
            pos: p1.into(),
            elemento: format!("{elemento} — longitudinal ({}×{} m)", fmt(b), fmt(h)),
            bitola_mm: 12.5,
            espac_cm: None,
            quantidade: n.to_string(),
            comprimento_m: 0.0,
            as_nec: as_min,
            as_ef: n as f64 * ab,
            unidade: "cm²",
            lb_cm: lb,
        },
        Armadura {
            pos: p2.into(),
            elemento: format!("{elemento} — estribos"),
            bitola_mm: 6.3,
            espac_cm: Some(15.0),
            quantidade: "7/m".into(),
            comprimento_m: est,
            as_nec: 0.0,
            as_ef: 2.0 * area_barra_cm2(6.3) * 100.0 / 15.0,
            unidade: "cm²/m",
            lb_cm: 0.0,
        },
    ]
}

/// Telas soldadas série Q (malha quadrada) — designação e As (cm²/m) por direção.
pub const TELAS_Q: [(&str, f64); 13] = [
    ("Q61", 0.61),
    ("Q75", 0.75),
    ("Q92", 0.92),
    ("Q113", 1.13),
    ("Q138", 1.38),
    ("Q159", 1.59),
    ("Q196", 1.96),
    ("Q246", 2.46),
    ("Q283", 2.83),
    ("Q335", 3.35),
    ("Q396", 3.96),
    ("Q503", 5.03),
    ("Q636", 6.36),
];

/// Menor tela Q com As ≥ `as_nec` (cm²/m); `None` se nenhuma bastar.
pub fn tela(as_nec: f64) -> Option<(&'static str, f64)> {
    TELAS_Q.iter().copied().find(|t| t.1 >= as_nec)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn escolha_de_malha() {
        // 5,0 cm²/m, h = 0,40: Ø8 → s = 10,0 (≥10) → 5,03 cm²/m
        let (phi, s, a) = malha_principal(5.0, 0.40);
        assert_eq!((phi, s), (8.0, 10.0));
        assert!((a - 5.027).abs() < 0.01);
        // 9,7 cm²/m: Ø8 s=5 (<10) ; Ø10 s=7,5 ; Ø12,5 → s = 12,5
        let (phi, s, _) = malha_principal(9.7, 0.30);
        assert_eq!((phi, s), (12.5, 12.5));
        // espaçamento limitado a 2h para placas finas: h = 0,08 → s ≤ 16 cm
        let (_, s, _) = malha_principal(0.5, 0.08);
        assert!(s <= 16.0);
        // Ø ≤ h/8: placa de 10 cm admite até Ø 12,5
        let (phi, _, _) = malha_principal(12.0, 0.10);
        assert!(phi <= 12.5);
        assert_eq!(fmt_bitola(12.5), "12,5");
        assert_eq!(fmt_bitola(10.0), "10");
    }

    #[test]
    fn ancoragem_nbr6118() {
        // Ø12,5, C30, CA-50, boa aderência: fctd = 0,21·30^(2/3)/1,4 = 1,448 ; fbd = 3,26 ;
        // lb = 12,5/4·434,8/3,26 = 417 mm → 41,7 cm (≥ 25Ø = 31,25)
        let lb = lb_basico(12.5, 30.0, 500.0, 1.4, 1.15, true);
        assert!((lb - 41.7).abs() < 0.3, "{lb}");
        assert!((lb_nec(lb, 12.5, 5.0, 10.0) - (0.5 * lb)).abs() < 1e-9);
        assert!(lb_nec(lb, 12.5, 0.1, 10.0) >= 0.3 * lb);
        assert!((traspasse(lb, lb, 12.5) - 2.0 * lb).abs() < 1e-9);
        // distribuição: máx(0,2·5 ; 0,9 ; 0,5·0,0015·0,40·1e4 = 3,0) = 3,0
        assert!((as_distribuicao(5.0, 30.0, 0.40) - 3.0).abs() < 1e-9);
        assert_eq!(tela(1.2).unwrap().0, "Q138");
    }
}
