//! Muro de flexão MISTO: pilares de concreto armado engastados na sapata,
//! painéis de blocos estruturais de concreto vencendo o vão horizontal entre
//! pilares (armadura horizontal em canaletas grauteadas), viga de respaldo
//! no topo. A sapata e a estabilidade externa são as do muro de flexão.
//!
//! Hipóteses (registradas no memorial):
//! - o pilar recebe o empuxo da faixa de largura igual ao espaçamento entre eixos;
//! - o painel apoia-se nos pilares e na sapata (áreas de influência a 45°): a faixa
//!   horizontal crítica fica a L/2 da base (M = p·e·L²/8) e, abaixo dela, arranques
//!   verticais grauteados trabalham em balanço até a sapata;
//! - alvenaria: NBR 16868-1 — fk = 0,7·fpk, fpk ≈ 0,8·fbk (bloco de concreto,
//!   a confirmar com ensaio de prisma), γm = 2,0, γs = 1,15.

use crate::concreto::{flexao_retangular, Concreto};
use crate::fatores::Fatores;
use crate::resultado::{fmt, Resultado, Verificacao};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum Sistema {
    #[default]
    ConcretoArmado,
    Misto,
}

impl Sistema {
    pub fn nome(&self) -> &'static str {
        match self {
            Sistema::ConcretoArmado => "Concreto armado (parede maciça)",
            Sistema::Misto => "Misto: pilares e vigas em concreto armado + blocos estruturais",
        }
    }
}

/// Blocos estruturais de concreto (largura nominal, m).
pub const BLOCOS: [(f64, &str); 2] = [(0.14, "Bloco de concreto 14 × 19 × 39 cm"), (0.19, "Bloco de concreto 19 × 19 × 39 cm")];
/// Resistências características usuais (fbk, MPa) — NBR 6136.
pub const FBK: [f64; 6] = [4.0, 6.0, 8.0, 10.0, 12.0, 14.0];
/// Peso específico da alvenaria de blocos de concreto com canaletas grauteadas (kN/m³) —
/// valor baixo a favor da segurança na estabilidade.
pub const GAMMA_ALVENARIA: f64 = 16.0;
const GAMMA_M: f64 = 2.0;
/// Bitolas possíveis na canaleta (mm) e número máximo de barras por canaleta.
const BITOLAS_CANALETA: [f64; 4] = [8.0, 10.0, 12.5, 16.0];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Misto {
    /// Largura do bloco (m): 0,14 ou 0,19.
    pub bloco: f64,
    /// Resistência característica do bloco (MPa).
    pub fbk: f64,
    /// Espaçamento entre eixos dos pilares (m).
    pub espac_pilar: f64,
    /// Pilar: largura ao longo do muro e altura da seção (perpendicular, para o aterro), m.
    pub pilar_b: f64,
    pub pilar_h: f64,
    /// Espaçamento vertical das canaletas armadas (m).
    pub esp_canaleta: f64,
}

impl Default for Misto {
    fn default() -> Self {
        Self { bloco: 0.19, fbk: 8.0, espac_pilar: 3.0, pilar_b: 0.30, pilar_h: 0.40, esp_canaleta: 0.40 }
    }
}

/// Resultado do dimensionamento do sistema misto (para o detalhamento).
#[derive(Clone, Debug, Default)]
pub struct DimMisto {
    /// Pilar: área de aço longitudinal (cm²), barras (n, Ø mm) e estribos (Ø, s cm).
    pub as_pilar: f64,
    pub n_barras: usize,
    pub phi_pilar: f64,
    pub phi_estribo: f64,
    pub s_estribo: f64,
    /// Canaleta: barras (n, Ø mm) por canaleta.
    pub n_canaleta: usize,
    pub phi_canaleta: f64,
    /// Arranques verticais grauteados na zona junto à sapata: Ø (mm), espaçamento (cm), altura (m).
    pub phi_vertical: f64,
    pub s_vertical: f64,
    pub y_arranque: f64,
}

impl Misto {
    /// Espessura equivalente da parede (alvenaria + pilares distribuídos), m.
    pub fn t_equivalente(&self) -> f64 {
        let s = self.espac_pilar.max(self.pilar_b + 0.1);
        self.bloco + (self.pilar_h - self.bloco).max(0.0) * self.pilar_b / s
    }

    /// Peso específico equivalente da parede (kN/m³).
    pub fn gamma_equivalente(&self, gamma_concreto: f64) -> f64 {
        let s = self.espac_pilar.max(self.pilar_b + 0.1);
        let alv = self.bloco * (s - self.pilar_b) * GAMMA_ALVENARIA;
        let pil = self.pilar_h * self.pilar_b * gamma_concreto;
        (alv + pil) / (self.t_equivalente() * s)
    }

    pub fn fk(&self) -> f64 {
        0.7 * 0.8 * self.fbk
    }

    /// Dimensiona pilar e painel. `m_k`, `v_k`: momento e cortante característicos
    /// da parede por metro na base; `p_base`: pressão horizontal na base do painel (kPa).
    /// `pressao(y)`: pressão horizontal (kPa) a `y` m acima do topo da sapata; `h_painel`: altura do painel.
    pub fn dimensionar(&self, r: &mut Resultado, fat: &Fatores, c: &Concreto, m_k: f64, v_k: f64, pressao: &dyn Fn(f64) -> f64, h_painel: f64) -> DimMisto {
        let gf = fat.gamma_f;
        let s = self.espac_pilar.max(self.pilar_b + 0.1);
        let mut dm = DimMisto::default();
        r.titulo("Sistema misto — pilares de concreto armado e painéis de alvenaria estrutural");
        r.linha(format!(
            "Pilares {} × {} cm a cada {} m (eixos) ; painel de {} ; fbk = {} MPa ; canaletas armadas a cada {} m ; viga de respaldo no topo",
            fmt(100.0 * self.pilar_b),
            fmt(100.0 * self.pilar_h),
            fmt(s),
            BLOCOS.iter().find(|b| (b.0 - self.bloco).abs() < 1e-6).map(|b| b.1).unwrap_or("bloco de concreto"),
            fmt(self.fbk),
            fmt(self.esp_canaleta)
        ));
        r.linha(format!(
            "Estabilidade com parede equivalente: e = {} m e γ = {} kN/m³ (alvenaria com canaletas grauteadas a {} kN/m³, a favor da segurança).",
            fmt(self.t_equivalente()),
            fmt(self.gamma_equivalente(c.peso)),
            fmt(GAMMA_ALVENARIA)
        ));

        // ---------- pilar ----------
        let md = m_k * s * gf;
        let vd = v_k * s * gf;
        let cob = c.cobrimento;
        let (bw, hp) = (self.pilar_b, self.pilar_h);
        let d = hp - cob - 0.0125;
        let fl = flexao_retangular(md, bw, hp, d, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        let as_min = (0.0015 * bw * hp * 1e4).max(fl.as_min * 0.0);
        let as_nec = fl.as_calc.max(as_min);
        r.formula(
            "Pilar — flexão na base (faixa de influência = espaçamento)",
            "Md = γf·M·s ;  y = d·(1 − √(1 − 2·Md/(0,85·fcd·b·d²))) ;  As = Md/(fyd·(d − y/2))",
            format!("Md = {}·{}·{} ;  b × h = {} × {} cm ;  d = {} cm", fmt(gf), fmt(m_k), fmt(s), fmt(100.0 * bw), fmt(100.0 * hp), fmt(100.0 * d)),
            format!("Md = {} kN·m ;  x/d = {} ;  As = {} cm² (mín. 0,15 % = {} cm²)", fmt(md), fmt(fl.x_d), fmt(fl.as_calc), fmt(as_min)),
        );
        r.verif(Verificacao::max("Pilar: x/d (NBR 6118)", fl.x_d, 0.45, ""));
        // barras: menor bitola com n entre 2 e o que cabe na largura
        let cabe = |phi: f64| (((bw - 2.0 * cob - 0.01) * 1000.0 + 20.0) / (phi + 20.0)).floor().max(2.0) as usize;
        let mut escolha = None;
        for phi in [10.0, 12.5, 16.0, 20.0, 25.0] {
            let a1 = std::f64::consts::PI * phi * phi / 400.0;
            let n = ((as_nec / a1).ceil() as usize).max(2);
            if n <= cabe(phi) {
                escolha = Some((n, phi));
                break;
            }
        }
        let (n, phi) = escolha.unwrap_or((cabe(25.0), 25.0));
        let as_ef = n as f64 * std::f64::consts::PI * phi * phi / 400.0;
        r.verif(Verificacao::min(format!("Pilar: As efetiva ({n} Ø{}) ≥ As necessária", crate::armadura::fmt_bitola(phi)), as_ef, as_nec, "cm²"));
        r.verif(Verificacao::max("Pilar: taxa de armadura ≤ 4 % (NBR 6118 17.3.5.2.4)", 100.0 * as_ef / (bw * hp * 1e4), 4.0, "%"));
        // cortante — modelo I (NBR 6118 17.4.2.2)
        let fcd = c.fck / fat.gamma_c;
        let fctd = 0.21 * c.fck.powf(2.0 / 3.0) / fat.gamma_c;
        let vrd2 = 0.27 * (1.0 - c.fck / 250.0) * fcd * bw * d * 1000.0;
        let vc = 0.6 * fctd * bw * d * 1000.0;
        let fywd = (c.fyk / fat.gamma_s).min(435.0);
        let asw_s = ((vd - vc).max(0.0) / (0.9 * d * fywd * 1000.0) * 1e4).max(0.2 * 0.3 * c.fck.powf(2.0 / 3.0) / c.fyk * bw * 1e4); // cm²/m
        let phi_e = if asw_s > 4.0 { 8.0 } else { 6.3 };
        let a_e = 2.0 * std::f64::consts::PI * phi_e * phi_e / 400.0; // 2 ramos
        let s_e = ((a_e / asw_s * 100.0).floor()).clamp(5.0, (0.6 * d * 100.0).min(30.0));
        r.formula(
            "Pilar — cortante (modelo I, NBR 6118 17.4.2.2)",
            "VRd2 = 0,27·αv2·fcd·bw·d ;  Vc = 0,6·fctd·bw·d ;  Asw/s = (Vd − Vc)/(0,9·d·fywd) ≥ ρw,mín·bw",
            format!("Vd = {}·{}·{} = {} kN ;  bw = {} cm ;  d = {} cm", fmt(gf), fmt(v_k), fmt(s), fmt(vd), fmt(100.0 * bw), fmt(100.0 * d)),
            format!("VRd2 = {} kN ;  Vc = {} kN ;  estribos Ø{} c/{} cm (2 ramos)", fmt(vrd2), fmt(vc), crate::armadura::fmt_bitola(phi_e), fmt(s_e)),
        );
        r.verif(Verificacao::max("Pilar: Vd ≤ VRd2 (bielas)", vd, vrd2, "kN"));
        dm.as_pilar = as_nec;
        dm.n_barras = n;
        dm.phi_pilar = phi;
        dm.phi_estribo = phi_e;
        dm.s_estribo = s_e;

        // ---------- painel de alvenaria (apoiado nos dois pilares e na sapata) ----------
        // Áreas de influência a 45° a partir dos cantos inferiores: abaixo de y* = L/2 a carga
        // desce para a sapata (arranques verticais em balanço); acima, as faixas horizontais
        // vencem o vão entre pilares — a mais carregada é a de y*.
        let l = (s - bw).max(0.3);
        let esp = self.esp_canaleta.max(0.2);
        let t = self.bloco;
        let dp = t / 2.0; // barra no centro da canaleta / do furo grauteado
        let fd = self.fk() / GAMMA_M;
        let fyd = c.fyk / 1.15;
        let y_est = (l / 2.0).min(h_painel);
        let p_h = pressao(y_est);
        let p_0 = pressao(0.0);
        let md_p = gf * p_h * esp * l * l / 8.0; // kN·m por canaleta
        let vd_p = gf * p_h * esp * l / 2.0;
        // MRd = As·fyd·z ; z = d·(1 − 0,5·As·fyd/(fd·b·d)) ≤ 0,95·d
        // aço efetivo limitado ao balanceado (As·fyd ≤ fd·b·d): acima disso a alvenaria comprimida governa
        let mrd_de = |a_cm2: f64, b: f64| {
            let a = (a_cm2 * 1e-4).min(fd * b * dp / fyd);
            let z = (dp * (1.0 - 0.5 * a * fyd / (fd * b * dp))).min(0.95 * dp);
            a * fyd * 1000.0 * z
        };
        let a_min = 0.0005 * esp * t * 1e4; // 0,05 % da seção bruta (critério construtivo)
        let mut canaleta = None;
        'busca: for phi in BITOLAS_CANALETA {
            for n in 1..=2usize {
                let a = n as f64 * std::f64::consts::PI * phi * phi / 400.0;
                let fvd_a = (0.35 + 17.5 * (a * 1e-4 / (esp * dp)).min(0.02)).min(0.7) / GAMMA_M;
                if a >= a_min && mrd_de(a, esp) >= md_p && vd_p / (esp * dp) / 1000.0 <= fvd_a && (phi <= 12.5 || t >= 0.19) {
                    canaleta = Some((n, phi, a));
                    break 'busca;
                }
            }
        }
        let (nc, phic, a_ef) = canaleta.unwrap_or((2, 16.0, 2.0 * std::f64::consts::PI * 256.0 / 400.0));
        let mrd = mrd_de(a_ef, esp);
        r.norma(crate::normas::NBR16868_JUNTAS);
        r.formula(
            "Painel de alvenaria — faixa horizontal entre pilares (NBR 16868-1, alvenaria armada)",
            "y* = mín(L/2 ; Hp) ;  Md = γf·p(y*)·e·L²/8 ;  MRd = As·fyd·z ;  z = d·(1 − 0,5·As·fyd/(fd·b·d)) ≤ 0,95·d ;  fd = fk/γm ;  fk = 0,7·fpk ;  fpk ≈ 0,8·fbk",
            format!(
                "L = {} m ;  y* = {} m ;  p(y*) = {} kPa ;  e = {} m ;  d = {} cm (barra no centro da canaleta) ;  fk = {} MPa ;  γm = {}",
                fmt(l), fmt(y_est), fmt(p_h), fmt(esp), fmt(100.0 * dp), fmt(self.fk()), fmt(GAMMA_M)
            ),
            format!("Md = {} kN·m ;  {} Ø{} por canaleta (As = {} cm²) → MRd = {} kN·m", fmt(md_p), nc, crate::armadura::fmt_bitola(phic), fmt(a_ef), fmt(mrd)),
        );
        r.verif(Verificacao::max("Painel: Md ≤ MRd (faixa horizontal entre pilares)", md_p, mrd, "kN·m"));
        let rho = (a_ef * 1e-4 / (esp * dp)).min(0.02);
        let fvk = (0.35 + 17.5 * rho).min(0.7);
        let tau = vd_p / (esp * dp) / 1000.0;
        r.formula(
            "Painel — cisalhamento da faixa junto ao pilar (NBR 16868-1)",
            "τd = Vd/(b·d) ;  fvk = 0,35 + 17,5·ρ ≤ 0,7 MPa ;  fvd = fvk/γm",
            format!("Vd = {} kN ;  b = {} m ;  d = {} m ;  ρ = {}", fmt(vd_p), fmt(esp), fmt(dp), format!("{:.4}", rho).replace('.', ",")),
            format!("τd = {} MPa ;  fvd = {} MPa", fmt(tau), fmt(fvk / GAMMA_M)),
        );
        r.verif(Verificacao::max("Painel: τd ≤ fvd (cisalhamento junto ao pilar)", tau, fvk / GAMMA_M, "MPa"));

        // zona junto à sapata: balanço vertical de altura y* com a pressão da base
        // arranques no furo junto à face do aterro (tração): d = t − 5 cm
        let dv = (t - 0.05).max(dp);
        let mrd_v_de = |a_cm2: f64| {
            let a = (a_cm2 * 1e-4).min(fd * dv / fyd);
            let z = (dv * (1.0 - 0.5 * a * fyd / (fd * dv))).min(0.95 * dv);
            a * fyd * 1000.0 * z
        };
        let md_v = gf * p_0 * y_est * y_est / 2.0; // kN·m/m
        let vd_v = gf * p_0 * y_est;
        let mut vertical = None;
        'v: for sv in [0.40, 0.20] {
            for phi in [10.0, 12.5, 16.0] {
                let a = std::f64::consts::PI * phi * phi / 400.0 / sv; // cm²/m
                let fvd_a = (0.35 + 17.5 * (a * 1e-4 / dv).min(0.02)).min(0.7) / GAMMA_M;
                if mrd_v_de(a) >= md_v && vd_v / dv / 1000.0 <= fvd_a && (phi <= 12.5 || t >= 0.19) {
                    vertical = Some((phi, sv, a));
                    break 'v;
                }
            }
        }
        let (phiv, sv, av) = vertical.unwrap_or((16.0, 0.20, std::f64::consts::PI * 256.0 / 400.0 / 0.20));
        let mrd_v = mrd_v_de(av);
        r.formula(
            "Painel — zona junto à sapata: arranques verticais grauteados em balanço",
            "Md = γf·p(0)·y*²/2 ;  Vd = γf·p(0)·y* ;  MRd = As·fyd·z (b = 1 m ; d = t − 5 cm, barra no furo junto ao aterro)",
            format!("p(0) = {} kPa ;  y* = {} m ;  arranques Ø{} c/{} cm nos furos grauteados", fmt(p_0), fmt(y_est), crate::armadura::fmt_bitola(phiv), fmt(100.0 * sv)),
            format!("Md = {} kN·m/m ;  As = {} cm²/m → MRd = {} kN·m/m ;  Vd = {} kN/m", fmt(md_v), fmt(av), fmt(mrd_v), fmt(vd_v)),
        );
        r.verif(Verificacao::max("Painel: arranques verticais junto à sapata (Md ≤ MRd)", md_v, mrd_v, "kN·m/m"));
        let rho_v = (av * 1e-4 / dv).min(0.02);
        let fvk_v = (0.35 + 17.5 * rho_v).min(0.7);
        r.verif(Verificacao::max("Painel: cisalhamento junto à sapata (τd ≤ fvd)", vd_v / dv / 1000.0, fvk_v / GAMMA_M, "MPa"));
        r.verif(Verificacao::min("Painel: fbk ≥ 4 MPa (bloco estrutural, NBR 6136)", self.fbk, 4.0, "MPa"));
        dm.n_canaleta = nc;
        dm.phi_canaleta = phic;
        dm.phi_vertical = phiv;
        dm.s_vertical = 100.0 * sv;
        dm.y_arranque = y_est;
        r.aviso(format!(
            "Sistema misto: fpk ≈ 0,8·fbk adotado para o bloco de concreto grauteado — confirmar com ensaio de prisma (NBR 16868-2/3). Graute fgk ≥ 2·fbk nas canaletas e nos furos junto aos pilares; ligar o painel aos pilares com ferros-cabelo ou tela a cada {} m.",
            fmt(esp)
        ));
        dm
    }
}

#[cfg(test)]
mod testes {
    use crate::projeto::{Projeto, TipoContencao};
    use super::Sistema;

    #[test]
    fn misto_dimensiona_pilar_e_painel() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 3.0;
        p.pre_dimensionar();
        p.flexao.sistema = Sistema::Misto;
        let r = p.calcular();
        for n in ["Pilar: x/d", "Pilar: Vd ≤ VRd2", "Painel: Md ≤ MRd", "Painel: τd ≤ fvd"] {
            assert!(r.verificacoes.iter().any(|v| v.nome.starts_with(n)), "falta {n}");
        }
        assert!(r.armaduras.iter().any(|a| a.elemento.starts_with("Pilar — longitudinal")));
        assert!(r.armaduras.iter().any(|a| a.elemento.starts_with("Painel — horizontal")));
        assert!(!r.armaduras.iter().any(|a| a.elemento.starts_with("Parede —")), "sem armadura de parede maciça");
        // espaçamento maior → mais momento no pilar e no painel
        let md = |s: f64| {
            let mut q = p.clone();
            q.flexao.misto.espac_pilar = s;
            let r = q.calcular();
            r.verificacoes.iter().find(|v| v.nome.starts_with("Painel: Md ≤ MRd")).unwrap().valor
        };
        assert!(md(4.0) > md(2.0));
        // 3D e IFC: pilares como IfcColumn e painel de blocos
        let m = crate::modelo3d::Modelo3d::montar(&p, &r, 9.0, false);
        assert!(m.pecas.iter().any(|x| x.classe == crate::modelo3d::Classe::Pilar));
        assert!(crate::ifc::gerar(&m, &p).contains("IFCCOLUMN("));
    }

    #[test]
    fn otimizador_acha_misto_que_atende() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 2.0; // faixa usual do muro misto de blocos
        p.pre_dimensionar();
        p.flexao.sistema = Sistema::Misto;
        let o = crate::otimizar::otimizar(&p);
        let m = o.melhor.clone().unwrap_or_else(|| panic!("{}", o.mensagem(crate::otimizar::custo(&p))));
        assert!(m.projeto.calcular().atende());
        assert_eq!(m.projeto.flexao.sistema, Sistema::Misto);
    }
}
