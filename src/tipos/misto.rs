//! Muro de flexão MISTO: pilares de concreto armado, vigas (baldrame,
//! intermediárias e respaldo) e painéis de blocos estruturais de concreto.
//! Fundação em blocos sobre estacas ligados pela viga baldrame (usual) ou em
//! sapata corrida.
//!
//! Caminho das cargas (registrado no memorial):
//! - o painel de blocos vence o vão VERTICAL entre vigas (faixa de 1 m biapoiada,
//!   barras verticais nos furos grauteados);
//! - as vigas recebem a faixa de influência (metade do vão acima e abaixo) e a
//!   levam aos pilares (vão = espaçamento entre eixos, biapoiada);
//! - como a pressão cresce com a profundidade, o espaçamento das vigas é o maior
//!   que o painel resiste em cada trecho — as vigas ficam mais próximas embaixo,
//!   onde está a maior parte do empuxo (resultante a ≈ H/3 da base);
//! - o pilar, engastado no bloco (ou na sapata), recebe as reações das vigas: o
//!   momento na base é o do empuxo da faixa entre eixos;
//! - bloco sobre 2 estacas alinhadas perpendicularmente ao muro: binário
//!   compressão/tração; capacidade por Décourt-Quaresma (SPT), NBR 6122.
//!
//! Alvenaria: NBR 16868-1 — fk = 0,7·fpk, fpk ≈ 0,8·fbk (bloco de concreto, a
//! confirmar com ensaio de prisma), γm = 2,0, γs = 1,15.

use crate::concreto::{flexao_retangular, vrd1, Concreto};
use crate::fatores::Fatores;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::spt::TipoSolo;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

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

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum FundacaoMisto {
    /// Blocos sobre estacas sob os pilares, ligados pela viga baldrame.
    #[default]
    Estacas,
    /// Sapata corrida (como o muro de flexão em concreto armado).
    Sapata,
}

impl FundacaoMisto {
    pub fn nome(&self) -> &'static str {
        match self {
            FundacaoMisto::Estacas => "Blocos sobre estacas + viga baldrame",
            FundacaoMisto::Sapata => "Sapata corrida",
        }
    }
}

/// Blocos estruturais de concreto (largura nominal, m).
pub const BLOCOS: [(f64, &str); 2] = [(0.14, "Bloco de concreto 14 × 19 × 39 cm"), (0.19, "Bloco de concreto 19 × 19 × 39 cm")];
/// Resistências características usuais (fbk, MPa) — NBR 6136.
pub const FBK: [f64; 6] = [4.0, 6.0, 8.0, 10.0, 12.0, 14.0];
/// Diâmetros usuais de estaca escavada (m).
pub const DIAM_ESTACA: [f64; 4] = [0.25, 0.30, 0.40, 0.50];
/// Peso específico da alvenaria de blocos de concreto com furos/canaletas grauteados
/// (kN/m³) — valor baixo a favor da segurança na estabilidade.
pub const GAMMA_ALVENARIA: f64 = 16.0;
const GAMMA_M: f64 = 2.0;
/// Altura da viga de respaldo (uma fiada de canaleta, m).
pub const H_RESPALDO: f64 = 0.19;

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
    /// Legado (modelo anterior de canaletas horizontais); não usado.
    pub esp_canaleta: f64,
    pub fundacao: FundacaoMisto,
    /// Vigas intermediárias: largura (perpendicular ao muro) e altura (vertical), m.
    pub viga_b: f64,
    pub viga_h: f64,
    /// Viga baldrame: largura e altura (m).
    pub baldrame_b: f64,
    pub baldrame_h: f64,
    /// Diâmetro das estacas escavadas (m).
    pub diam_estaca: f64,
    /// Perfil SPT para as estacas: (profundidade final, NSPT, tipo). Preenchido pelo projeto.
    #[serde(skip)]
    pub spt: Vec<(f64, f64, TipoSolo)>,
    /// Profundidade (na sondagem) do topo do baldrame — referência das estacas. Preenchido pelo projeto.
    #[serde(skip)]
    pub z_topo: f64,
}

impl Default for Misto {
    fn default() -> Self {
        Self {
            bloco: 0.19,
            fbk: 8.0,
            espac_pilar: 3.0,
            pilar_b: 0.30,
            pilar_h: 0.40,
            esp_canaleta: 0.40,
            fundacao: FundacaoMisto::Estacas,
            viga_b: 0.30,
            viga_h: 0.30,
            baldrame_b: 0.30,
            baldrame_h: 0.40,
            diam_estaca: 0.30,
            spt: Vec::new(),
            z_topo: 0.0,
        }
    }
}

/// Viga intermediária dimensionada.
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct VigaDim {
    /// Cota do eixo acima do topo da fundação (m).
    pub y: f64,
    /// Carga horizontal característica (kN/m) e barras por face (n, Ø mm).
    pub w: f64,
    pub n: usize,
    pub phi: f64,
}

/// Estacas dimensionadas (por bloco: 2 estacas).
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct EstacaDim {
    pub comprimento: f64,
    pub n_comp: f64,
    pub n_trac: f64,
    pub h: f64,
    pub q_adm: f64,
    pub qt_adm: f64,
    /// Distância entre eixos das estacas (m) e dimensões do bloco (comprimento ⟂ muro, largura, altura).
    pub e: f64,
    pub bloco: [f64; 3],
    /// Armadura principal do bloco (tirante), cm².
    pub as_bloco: f64,
    /// Estaca: barras longitudinais (n, Ø mm) e comprimento armado (m) pela carga horizontal.
    pub n_barras: usize,
    pub phi: f64,
    pub l_armada: f64,
    /// Hélice pelo cortante (Ø mm, passo cm).
    pub phi_helice: f64,
    pub s_helice: f64,
}

/// Resultado do dimensionamento do sistema misto (para o detalhamento e o desenho).
#[derive(Clone, Debug, Default)]
pub struct DimMisto {
    pub as_pilar: f64,
    pub n_barras: usize,
    pub phi_pilar: f64,
    pub phi_estribo: f64,
    pub s_estribo: f64,
    /// Painel: barras verticais nos furos grauteados (Ø mm, espaçamento cm).
    pub phi_vertical: f64,
    pub s_vertical: f64,
    pub vigas: Vec<VigaDim>,
    /// Viga baldrame: barras por face (n, Ø mm).
    pub baldrame_n: usize,
    pub baldrame_phi: f64,
    pub estacas: Option<EstacaDim>,
}

fn area(phi_mm: f64) -> f64 {
    PI * phi_mm * phi_mm / 400.0
}

/// Escolhe n barras de Ø (2 a o que cabe em `largura`) com área ≥ `as_nec` (cm²).
pub(crate) fn barras(as_nec: f64, largura: f64, cob: f64) -> (usize, f64) {
    for phi in [10.0, 12.5, 16.0, 20.0, 25.0] {
        // largura útil: cobrimento e estribo (Ø8) dos dois lados ; espaçamento livre mínimo da NBR 6118 18.3.2.2
        let livre = crate::armadura::livre_minimo_mm(phi);
        let util = (largura - 2.0 * cob - 0.016) * 1000.0;
        let cabe = (((util + livre) / (phi + livre)).floor() as usize).max(2);
        let n = ((as_nec / area(phi)).ceil() as usize).max(2);
        if n <= cabe {
            return (n, phi);
        }
    }
    let cabe = ((((largura - 2.0 * cob - 0.016) * 1000.0 + 25.0) / 50.0).floor() as usize).max(2);
    (cabe, 25.0)
}

impl Misto {
    pub fn fundacao_estacas(&self) -> bool {
        self.fundacao == FundacaoMisto::Estacas
    }

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

    /// Momento resistente (kN·m/m) do painel com As (cm²/m) na profundidade útil d.
    fn mrd_painel(&self, a_cm2: f64, d: f64, fyd: f64) -> f64 {
        let fd = self.fk() / GAMMA_M;
        let a = (a_cm2 * 1e-4).min(fd * d / fyd); // limitado ao balanceado
        let z = (d * (1.0 - 0.5 * a * fyd / (fd * d))).min(0.95 * d);
        a * fyd * 1000.0 * z
    }

    /// Cotas das vigas intermediárias (acima do topo da fundação), de cima para baixo,
    /// com FAIXAS DE INFLUÊNCIA IGUAIS: o diagrama de pressões é dividido em n + 1 faixas
    /// de mesma força e as vigas ficam nas divisas — cada viga recebe a mesma parcela do
    /// empuxo (com pressão triangular e 1 viga, ela fica a ≈ 0,29·H da base, perto de H/3).
    /// n é o menor número em que todos os vãos do painel (≤ 1,60 m) resistem com barras
    /// verticais Ø12,5 c/20 cm grauteadas.
    pub fn niveis_vigas(&self, pressao: &dyn Fn(f64) -> f64, h_painel: f64, gf: f64, fyd: f64) -> Vec<f64> {
        let d = self.bloco / 2.0;
        let cap = self.mrd_painel(area(12.5) / 0.20, d, fyd);
        let fvd = (0.35 + 17.5 * (area(12.5) / 0.20 * 1e-4 / d).min(0.02)).min(0.7) / GAMMA_M;
        // força acumulada de cima para baixo: F(y) = ∫ de y a Hp de p
        let passos = 400;
        let dy = h_painel / passos as f64;
        let mut acum = vec![0.0; passos + 1]; // acum[k]: força acima da cota h_painel − k·dy
        for k in 1..=passos {
            let y = h_painel - (k as f64 - 0.5) * dy;
            acum[k] = acum[k - 1] + pressao(y.max(0.0)) * dy;
        }
        let total = acum[passos];
        let cota_para = |f: f64| -> f64 {
            let k = acum.iter().position(|&a| a >= f).unwrap_or(passos);
            let (a0, a1) = (acum[k.saturating_sub(1)], acum[k]);
            let frac = if a1 > a0 { (f - a0) / (a1 - a0) } else { 0.0 };
            h_painel - ((k as f64 - 1.0).max(0.0) + frac) * dy
        };
        let atende = |cotas: &[f64]| -> bool {
            let mut apoios = vec![h_painel];
            apoios.extend_from_slice(cotas);
            apoios.push(0.0);
            apoios.windows(2).all(|w| {
                let a = w[0] - w[1];
                let p = pressao(w[1]);
                a <= 1.60 + 1e-9 && gf * p * a * a / 8.0 <= cap && gf * p * a / 2.0 / d / 1000.0 <= fvd
            })
        };
        for n in 0..=20usize {
            // arredonda as cotas a 5 cm (modulação das formas/fiadas)
            let cotas: Vec<f64> = (1..=n).map(|k| ((cota_para(total * k as f64 / (n + 1) as f64) / 0.05).round() * 0.05).clamp(0.20, h_painel - 0.20)).collect();
            if atende(&cotas) || n == 20 {
                return cotas;
            }
        }
        Vec::new()
    }

    /// Dimensiona painel, vigas intermediárias e pilar. `m_k`, `v_k`: momento e cortante
    /// característicos por metro no topo da fundação; `pressao(y)`: pressão horizontal
    /// (kPa) a `y` m acima do topo da fundação; `h_painel`: altura do painel.
    pub fn dimensionar(&self, r: &mut Resultado, fat: &Fatores, c: &Concreto, m_k: f64, v_k: f64, pressao: &dyn Fn(f64) -> f64, h_painel: f64) -> DimMisto {
        let gf = fat.gamma_f;
        let s = self.espac_pilar.max(self.pilar_b + 0.1);
        let cob = c.cobrimento;
        let fyd = c.fyk / 1.15;
        let mut dm = DimMisto::default();
        r.titulo("Sistema misto — pilares, vigas e painéis de alvenaria estrutural");
        r.linha(format!(
            "Pilares {} × {} cm a cada {} m (eixos) ; painel de {} ; fbk = {} MPa ; fundação: {}",
            fmt(100.0 * self.pilar_b),
            fmt(100.0 * self.pilar_h),
            fmt(s),
            BLOCOS.iter().find(|b| (b.0 - self.bloco).abs() < 1e-6).map(|b| b.1).unwrap_or("bloco de concreto"),
            fmt(self.fbk),
            self.fundacao.nome()
        ));
        r.linha(format!(
            "Parede equivalente (estabilidade e desenho): e = {} m e γ = {} kN/m³ (alvenaria com furos grauteados a {} kN/m³, a favor da segurança).",
            fmt(self.t_equivalente()),
            fmt(self.gamma_equivalente(c.peso)),
            fmt(GAMMA_ALVENARIA)
        ));

        // ---------- vigas intermediárias e painel ----------
        let niveis = self.niveis_vigas(pressao, h_painel, gf, fyd);
        let mut apoios = vec![0.0];
        apoios.extend(niveis.iter().rev());
        apoios.push(h_painel);
        r.formula(
            "Distribuição das vigas intermediárias (pela carga)",
            "faixas de influência iguais: ∫ p dy dividido em n + 1 partes de mesma força, vigas nas divisas ; n mínimo com todos os vãos ≤ 1,60 m e γf·p·a²/8 ≤ MRd do painel",
            format!("altura do painel = {} m ; pressão na base = {} kPa ; no topo = {} kPa", fmt(h_painel), fmt(pressao(0.0)), fmt(pressao(h_painel))),
            format!(
                "{} viga(s) intermediária(s) a {} m acima da fundação — cada uma com a mesma parcela do empuxo (mais próximas embaixo)",
                niveis.len(),
                if niveis.is_empty() { "—".to_string() } else { niveis.iter().rev().map(|y| fmt(*y)).collect::<Vec<_>>().join(" ; ") }
            ),
        );
        let d = self.bloco / 2.0;
        let (mut md_p, mut vd_p, mut a_crit) = (0.0f64, 0.0f64, 0.0);
        for w in apoios.windows(2) {
            let a = w[1] - w[0];
            let p = pressao(w[0]);
            let md = gf * p * a * a / 8.0;
            if md > md_p {
                md_p = md;
                a_crit = a;
            }
            vd_p = vd_p.max(gf * p * a / 2.0);
        }
        let mut esc = (10.0, 0.40);
        'b: for sv in [0.40, 0.20] {
            for phi in [10.0, 12.5] {
                let a = area(phi) / sv;
                let fvd = (0.35 + 17.5 * (a * 1e-4 / d).min(0.02)).min(0.7) / GAMMA_M;
                esc = (phi, sv);
                if self.mrd_painel(a, d, fyd) >= md_p && vd_p / d / 1000.0 <= fvd {
                    break 'b;
                }
            }
        }
        let (phiv, sv) = esc;
        let av = area(phiv) / sv;
        let mrd = self.mrd_painel(av, d, fyd);
        r.norma(crate::normas::NBR16868_JUNTAS);
        r.formula(
            "Painel de alvenaria — vão vertical entre vigas (NBR 16868-1, alvenaria armada)",
            "Md = γf·p·a²/8 ;  MRd = As·fyd·z ;  z = d·(1 − 0,5·As·fyd/(fd·b·d)) ≤ 0,95·d ;  fd = fk/γm ;  fk = 0,7·fpk ;  fpk ≈ 0,8·fbk",
            format!(
                "trecho crítico a = {} m ;  d = {} cm (barra no centro do furo) ;  fk = {} MPa ;  barras verticais Ø{} c/{} cm grauteadas",
                fmt(a_crit), fmt(100.0 * d), fmt(self.fk()), crate::armadura::fmt_bitola(phiv), fmt(100.0 * sv)
            ),
            format!("Md = {} kN·m/m ;  MRd = {} kN·m/m", fmt(md_p), fmt(mrd)),
        );
        r.verif(Verificacao::max("Painel: Md ≤ MRd (vão vertical entre vigas)", md_p, mrd, "kN·m/m"));
        let fvk = (0.35 + 17.5 * (av * 1e-4 / d).min(0.02)).min(0.7);
        r.verif(Verificacao::max("Painel: τd ≤ fvd (cisalhamento junto às vigas)", vd_p / d / 1000.0, fvk / GAMMA_M, "MPa"));
        r.verif(Verificacao::min("Painel: fbk ≥ 4 MPa (bloco estrutural, NBR 6136)", self.fbk, 4.0, "MPa"));
        dm.phi_vertical = phiv;
        dm.s_vertical = 100.0 * sv;

        // vigas: faixa de influência entre os pontos médios dos vãos vizinhos
        let integrar = |y0: f64, y1: f64| crate::empuxo::integrar(pressao, y0, y1, 60).0;
        let (vb, vh) = (self.viga_b, self.viga_h);
        let dv = vb - cob - 0.0125;
        let (mut pior_xd, mut pior_v, mut pior_vr) = (0.0f64, 0.0f64, 1.0f64);
        let mut linhas = Vec::new();
        for (i, &y) in niveis.iter().enumerate() {
            let acima = if i == 0 { h_painel } else { niveis[i - 1] };
            let abaixo = niveis.get(i + 1).copied().unwrap_or(0.0);
            let w = integrar(0.5 * (y + abaixo), 0.5 * (y + acima));
            // viga contínua sobre os pilares: momento de apoio ≈ w·s²/10 (vãos iguais)
            let md = gf * w * s * s / 10.0;
            let vd = gf * w * s / 2.0;
            // flexão horizontal: braço na largura perpendicular ao muro
            let fl = flexao_retangular(md, vh, vb, dv, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
            let as_nec = fl.as_calc.max(crate::concreto::rho_min(c.fck) * vb * vh * 1e4);
            let (n, phi) = barras(as_nec, vh, cob);
            let vr = vrd1(vh, dv, n as f64 * area(phi), c.fck, fat.gamma_c);
            pior_xd = pior_xd.max(fl.x_d);
            if vd / vr.max(1e-9) > pior_v / pior_vr {
                pior_v = vd;
                pior_vr = vr.max(1e-9);
            }
            linhas.push(format!("y = {} m: w = {} kN/m ; Md = {} kN·m ; {} Ø{} por face", fmt(y), fmt(w), fmt(md), n, crate::armadura::fmt_bitola(phi)));
            dm.vigas.push(VigaDim { y, w, n, phi });
        }
        if !niveis.is_empty() {
            r.formula(
                "Vigas intermediárias — flexão horizontal entre pilares (NBR 6118)",
                "w = ∫ p dy na faixa de influência ;  Md = γf·w·s²/10 (contínua) ;  Vd = γf·w·s/2 ;  seção h (vertical) × b (para o aterro)",
                format!("viga {} × {} cm ;  s = {} m", fmt(100.0 * vh), fmt(100.0 * vb), fmt(s)),
                linhas.join(" | "),
            );
            r.verif(Verificacao::max("Vigas intermediárias: x/d (NBR 6118)", pior_xd, 0.45, ""));
            r.verif(Verificacao::max("Vigas intermediárias: Vd ≤ VRd1", pior_v, pior_vr, "kN"));
        }

        // ---------- pilar ----------
        let md = m_k * s * gf;
        let vd = v_k * s * gf;
        let (bw, hp) = (self.pilar_b, self.pilar_h);
        let d = hp - cob - 0.0125;
        let fl = flexao_retangular(md, bw, hp, d, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        // NBR 6118 17.3.5.3.1: As,mín de pilar = máx(0,15·Nd/fyd ; 0,4 % Ac) — total, descontados os 2 Ø10 porta-estribos
        let as_min = (0.004 * bw * hp * 1e4 - 2.0 * area(10.0)).max(crate::concreto::rho_min(c.fck) * bw * hp * 1e4);
        let as_nec = fl.as_calc.max(as_min);
        r.formula(
            "Pilar — flexão na base (reações das vigas na faixa entre eixos)",
            "Md = γf·M·s ;  y = d·(1 − √(1 − 2·Md/(0,85·fcd·b·d²))) ;  As = Md/(fyd·(d − y/2))",
            format!("Md = {}·{}·{} ;  b × h = {} × {} cm ;  d = {} cm", fmt(gf), fmt(m_k), fmt(s), fmt(100.0 * bw), fmt(100.0 * hp), fmt(100.0 * d)),
            format!("Md = {} kN·m ;  x/d = {} ;  As = {} cm² (mín. de pilar 0,4 % Ac − 2 Ø10 = {} cm²)", fmt(md), fmt(fl.x_d), fmt(fl.as_calc), fmt(as_min)),
        );
        r.verif(Verificacao::max("Pilar: x/d (NBR 6118)", fl.x_d, 0.45, ""));
        let (n, phi) = barras(as_nec, bw, cob);
        let as_ef = n as f64 * area(phi);
        r.verif(Verificacao::min(format!("Pilar: As efetiva ({n} Ø{}) ≥ As necessária", crate::armadura::fmt_bitola(phi)), as_ef, as_nec, "cm²"));
        r.verif(Verificacao::max("Pilar: taxa de armadura ≤ 4 % (NBR 6118 17.3.5.2.4)", 100.0 * as_ef / (bw * hp * 1e4), 4.0, "%"));
        let fcd = c.fck / fat.gamma_c;
        let fctd = 0.21 * c.fck.powf(2.0 / 3.0) / fat.gamma_c;
        let vrd2 = 0.27 * (1.0 - c.fck / 250.0) * fcd * bw * d * 1000.0;
        let vc = 0.6 * fctd * bw * d * 1000.0;
        let fywd = (c.fyk / fat.gamma_s).min(435.0);
        let asw_s = ((vd - vc).max(0.0) / (0.9 * d * fywd * 1000.0) * 1e4).max(0.2 * 0.3 * c.fck.powf(2.0 / 3.0) / c.fyk * bw * 1e4);
        let phi_e = if asw_s > 4.0 { 8.0 } else { 6.3 };
        // espaçamento: cortante e NBR 6118 18.4.3 (≤ 20 cm, ≤ menor dimensão, ≤ 12·Øl para CA-50)
        let s_e = ((2.0 * area(phi_e) / asw_s * 100.0).floor()).min(20.0).min(100.0 * bw.min(hp)).min(1.2 * phi).min((0.6 * d * 100.0).min(30.0)).max(5.0).floor();
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
        r.aviso(
            "Sistema misto: fpk ≈ 0,8·fbk adotado para o bloco de concreto grauteado — confirmar com ensaio de prisma (NBR 16868-2/3). Graute fgk ≥ 2·fbk nos furos armados; ligar o painel aos pilares com ferros-cabelo a cada 2 fiadas.",
        );
        dm
    }

    /// Dimensões do bloco sobre 2 estacas: (distância entre eixos e, comprimento ⟂ muro, largura, altura).
    pub fn geometria_bloco(&self) -> (f64, f64, f64, f64) {
        let d = self.diam_estaca;
        let e = 3.0 * d;
        let l = e + d + 0.30;
        let b = (d + 0.30).max(self.pilar_b + 0.10);
        let h = crate::projeto::arred((0.5 * (e - self.pilar_h / 2.0)).max(0.50).max(self.baldrame_h));
        (e, l, b, h)
    }

    /// Fundação em blocos sobre 2 estacas + viga baldrame. `m_k`, `v_k`: momento e
    /// cortante por metro no topo da fundação; `p_base`: pressão na altura do baldrame;
    /// `h_painel`: altura acima do baldrame (painel + respaldo).
    #[allow(clippy::too_many_arguments)]
    pub fn fundacao(&self, r: &mut Resultado, fat: &Fatores, c: &Concreto, m_k: f64, v_k: f64, p_base: f64, h_painel: f64, dm: &mut DimMisto) {
        let gf = fat.gamma_f;
        let s = self.espac_pilar.max(self.pilar_b + 0.1);
        let cob = c.cobrimento;
        let (bb, hb) = (self.baldrame_b, self.baldrame_h);
        r.titulo("Fundação — blocos sobre estacas e viga baldrame (NBR 6122 / NBR 6118)");

        // viga baldrame: peso da parede entre blocos (vertical) e faixa do empuxo (horizontal)
        let q_v = GAMMA_ALVENARIA * self.bloco * h_painel + c.peso * bb * hb;
        let md_v = gf * q_v * s * s / 10.0;
        let db = hb - cob - 0.0125;
        let flv = flexao_retangular(md_v, bb, hb, db, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        let w_h = p_base * hb;
        // baldrame contínuo sobre os blocos: momento de apoio ≈ w·s²/10
        let md_h = gf * w_h * s * s / 10.0;
        let flh = flexao_retangular(md_h, hb, bb, bb - cob - 0.0125, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        let as_nec = flv.as_calc.max(flh.as_calc).max(0.0015 * bb * hb * 1e4);
        let (n, phi) = barras(as_nec, bb, cob);
        r.formula(
            "Viga baldrame — entre blocos (vão = espaçamento dos pilares)",
            "vertical: q = γalv·t·Hp + peso próprio ;  Md = γf·q·s²/10 ;  horizontal: w = p·hb ;  Md = γf·w·s²/10 (contínua sobre os blocos)",
            format!("q = {} kN/m ;  w = {} kN/m ;  seção {} × {} cm ;  s = {} m", fmt(q_v), fmt(w_h), fmt(100.0 * bb), fmt(100.0 * hb), fmt(s)),
            format!("Md,v = {} kN·m (x/d = {}) ;  Md,h = {} kN·m (x/d = {}) ;  {} Ø{} por face", fmt(md_v), fmt(flv.x_d), fmt(md_h), fmt(flh.x_d), n, crate::armadura::fmt_bitola(phi)),
        );
        r.verif(Verificacao::max("Viga baldrame: x/d (NBR 6118)", flv.x_d.max(flh.x_d), 0.45, ""));
        dm.baldrame_n = n;
        dm.baldrame_phi = phi;

        // bloco sobre 2 estacas (alinhadas perpendicularmente ao muro)
        let d = self.diam_estaca;
        let (e, l_bloco, b_bloco, h_bloco) = self.geometria_bloco();
        let v_tot = v_k * s + p_base * hb * s;
        let m_tot = m_k * s + v_tot * h_bloco;
        let n_tot = s * (GAMMA_ALVENARIA * self.bloco * h_painel + c.peso * bb * hb)
            + c.peso * (self.pilar_b * self.pilar_h * h_painel + l_bloco * b_bloco * h_bloco)
            + c.peso * self.viga_b * self.viga_h * s * dm.vigas.len() as f64;
        let n_comp = n_tot / 2.0 + m_tot / e;
        let n_trac = n_tot / 2.0 - m_tot / e; // < 0: tração
        let h_est = v_tot / 2.0;
        r.formula(
            "Bloco sobre 2 estacas — esforços (binário perpendicular ao muro)",
            "M = M·s + V·hbloco ;  N = pesos da faixa entre eixos ;  Ni = N/2 ± M/e ;  e = 3·Ø",
            format!("N = {} kN ;  V = {} kN ;  M = {} kN·m ;  e = {} m ;  bloco {} × {} × {} m", fmt(n_tot), fmt(v_tot), fmt(m_tot), fmt(e), fmt(l_bloco), fmt(b_bloco), fmt(h_bloco)),
            format!(
                "estaca comprimida: {} kN ;  estaca {}: {} kN ;  horizontal: {} kN por estaca",
                fmt(n_comp),
                if n_trac < 0.0 { "tracionada" } else { "menos comprimida" },
                fmt(n_trac.abs()),
                fmt(h_est)
            ),
        );

        // capacidade (Décourt-Quaresma) — o menor comprimento que atende
        let ap = PI * d * d / 4.0;
        let u = PI * d;
        let z0 = self.z_topo + h_bloco;
        let n_em = |z: f64| -> (f64, TipoSolo) {
            let c = self.spt.iter().find(|c| z < c.0).or(self.spt.last());
            c.map(|c| (c.1.clamp(3.0, 50.0), c.2)).unwrap_or((5.0, TipoSolo::Argila))
        };
        // (C ponta kPa, α, β) — Décourt-Quaresma, estaca escavada
        let categoria = |t: TipoSolo| -> (f64, f64, f64) {
            let n = format!("{t:?}");
            if n.starts_with("Areia") {
                (400.0, 0.50, 0.50)
            } else if n.starts_with("SilteArenoso") {
                (250.0, 0.60, 0.65)
            } else if n.starts_with("Silte") {
                (200.0, 0.60, 0.65)
            } else {
                (120.0, 0.85, 0.80)
            }
        };
        let capacidade = |l: f64| -> (f64, f64, f64, f64) {
            let mut ql_b = 0.0;
            let dz = 0.25;
            let mut z = z0 + dz / 2.0;
            while z < z0 + l {
                let (n, t) = n_em(z);
                ql_b += categoria(t).2 * 10.0 * (n / 3.0 + 1.0) * u * dz;
                z += dz;
            }
            let ponta = z0 + l;
            let np = (n_em(ponta - 1.0).0 + n_em(ponta).0 + n_em(ponta + 1.0).0) / 3.0;
            let (cp, alfa, _) = categoria(n_em(ponta).1);
            let qp = alfa * cp * np * ap;
            // NBR 6122 8.2.1.2: em estacas escavadas a ponta não pode passar de 20 % da carga admissível
            let ql_adm = (ql_b / 1.3).min(ql_b / 2.0 + qp / 2.0);
            let qp_adm = (qp / 4.0).min(0.25 * ql_adm);
            let q_adm = (ql_adm + qp_adm).min((ql_b + qp) / 2.0);
            let qt_adm = ql_b / 2.0 + 0.9 * c.peso * ap * l;
            (q_adm, qt_adm, ql_b, qp)
        };
        let mut l = 4.0;
        while l < 25.0 {
            let (qa, qt, ..) = capacidade(l);
            if qa >= n_comp && (n_trac >= 0.0 || qt >= -n_trac) {
                break;
            }
            l += 0.5;
        }
        let (q_adm, qt_adm, ql, qp) = capacidade(l);
        r.formula(
            "Estacas escavadas — capacidade (Décourt-Quaresma, SPT ; NBR 6122)",
            "Ql = Σ β·10·(N/3 + 1)·U·Δz ;  Qp = α·C·Np·Ap ;  Qadm = mín(Ql/1,3 + Qp/4 ; (Ql + Qp)/2), ponta ≤ 20 % de Qadm (NBR 6122, escavada) ;  tração: Ql/2 + 0,9·Pestaca",
            format!("Ø = {} cm ;  topo a {} m de profundidade ;  L = {} m", fmt(100.0 * d), fmt(z0), fmt(l)),
            format!("Ql = {} kN ;  Qp = {} kN ;  Qadm = {} kN ;  Qt,adm = {} kN", fmt(ql), fmt(qp), fmt(q_adm), fmt(qt_adm)),
        );
        r.verif(Verificacao::info("Estaca: comprimento adotado", l, "m"));
        r.verif(Verificacao::max("Estaca: compressão ≤ Qadm (Décourt-Quaresma)", n_comp, q_adm, "kN"));
        if n_trac < 0.0 {
            r.verif(Verificacao::max("Estaca: tração ≤ Qt,adm (atrito lateral)", -n_trac, qt_adm, "kN"));
        }
        r.verif(Verificacao::max("Estaca: tensão média no fuste ≤ 5 MPa (NBR 6122, escavada)", n_comp / ap / 1000.0, 5.0, "MPa"));
        r.verif(Verificacao::info("Estaca: carga horizontal (por estaca)", h_est, "kN"));
        // ---- carga horizontal: estaca longa em meio elástico (Matlock-Reese / Miche) ----
        // kh = nh·z (solo com módulo de reação crescente com a profundidade) ; cabeça
        // impedida de girar pelo bloco + baldrame: M_cabeça = 0,93·H·T ; y0 = 0,93·H·T³/EI
        let ecs = 0.85 * 5600.0 * c.fck.sqrt() * 1000.0; // kPa (NBR 6118 8.2.8)
        let inercia = PI * d.powi(4) / 64.0;
        let ei = ecs * inercia;
        // nh pelo solo nos 3 m abaixo do bloco (Terzaghi para areias; estimativa para argilas/siltes)
        let (n_med, t_solo) = {
            let pts: Vec<(f64, TipoSolo)> = (0..6).map(|k| n_em(z0 + 0.25 + 0.5 * k as f64)).collect();
            (pts.iter().map(|p| p.0).sum::<f64>() / pts.len() as f64, pts[0].1)
        };
        let areia = format!("{t_solo:?}").starts_with("Areia");
        let nh = 1000.0
            * if areia {
                if n_med < 10.0 { 2.2 } else if n_med < 30.0 { 6.6 } else { 17.6 }
            } else if n_med < 4.0 {
                0.35
            } else if n_med < 8.0 {
                0.7
            } else if n_med < 15.0 {
                1.5
            } else {
                3.0
            }; // kN/m³
        let t_rel = (ei / nh).powf(0.2);
        let m_cab = 0.93 * h_est * t_rel;
        let y0 = 0.93 * h_est * t_rel.powi(3) / ei * 1000.0; // mm
        let c_eixo = cob + 0.0063 + 0.008;
        let ac = ap * 1e4;
        let as_min_est = 0.004 * ac; // pilar/estaca fletida: 0,4 % Ac (NBR 6118 17.3.5.3.1)
        let as_flex = crate::concreto::as_circular(gf * m_cab, d, c_eixo, 6, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        let as_est = as_flex.unwrap_or(0.04 * ac).max(as_min_est);
        let (mut n_est, mut phi_est) = (6usize, 12.5);
        'esc: for phi in [10.0, 12.5, 16.0, 20.0, 25.0] {
            for n in 6..=12usize {
                if n as f64 * area(phi) >= as_est {
                    let r_s = d / 2.0 - c_eixo;
                    let livre = 2.0 * PI * r_s * 1000.0 / n as f64 - phi;
                    if livre >= crate::armadura::livre_minimo_mm(phi) && phi <= d * 1000.0 / 8.0 {
                        n_est = n;
                        phi_est = phi;
                        break 'esc;
                    }
                }
            }
        }
        // comprimento armado: até onde o momento se anula na prática (≈ 4·T), mín. 3 m
        let l_arm = crate::projeto::arred((4.0 * t_rel).max(3.0)).min(l);
        let vrd = crate::concreto::vrd1(0.9 * d, 0.75 * d, n_est as f64 * area(phi_est) / 2.0, c.fck, fat.gamma_c);
        r.formula(
            "Estaca sob carga horizontal — Matlock-Reese (kh = nh·z), cabeça engastada no bloco",
            "T = (EI/nh)^(1/5) ;  M = 0,93·H·T ;  y0 = 0,93·H·T³/EI ;  Md = γf·M → As (seção circular) ;  L armado ≥ 4·T",
            format!(
                "H = {} kN ;  Ecs = {} MPa ;  I = {} m⁴ ;  nh = {} MN/m³ ({}, N médio {})",
                fmt(h_est), fmt(ecs / 1000.0), format!("{:.5}", inercia).replace('.', ","), fmt(nh / 1000.0),
                if areia { "areia, Terzaghi" } else { "argila/silte, estimativa" }, fmt(n_med)
            ),
            format!(
                "T = {} m ;  M = {} kN·m ;  y0 = {} mm ;  As = {} cm² → {} Ø{} ;  L armado = {} m",
                fmt(t_rel), fmt(m_cab), fmt(y0), fmt(as_est), n_est, crate::armadura::fmt_bitola(phi_est), fmt(l_arm)
            ),
        );
        r.verif(Verificacao::min("Estaca: As (flexão por carga horizontal) ≤ As efetiva", n_est as f64 * area(phi_est), as_est, "cm²"));
        // cortante: hélice dimensionada (modelo I, NBR 6118 17.4.2.2) com bw = 0,9·D e d = 0,75·D
        let (bw_e, d_e) = (0.9 * d, 0.75 * d);
        let fcd_e = c.fck / fat.gamma_c;
        let vrd2_e = 0.27 * (1.0 - c.fck / 250.0) * fcd_e * bw_e * d_e * 1000.0;
        let vc_e = vrd.min(0.6 * 0.21 * c.fck.powf(2.0 / 3.0) / fat.gamma_c * bw_e * d_e * 1000.0);
        let asw_s_e = ((gf * h_est - vc_e).max(0.0) / (0.9 * d_e * (c.fyk / fat.gamma_s).min(435.0) * 1000.0) * 1e4).max(0.2 * 0.3 * c.fck.powf(2.0 / 3.0) / c.fyk * bw_e * 1e4);
        let phi_h = if asw_s_e > 5.0 { 8.0 } else { 6.3 };
        let s_h = ((2.0 * area(phi_h) / asw_s_e * 100.0).floor()).min(20.0).min(1.2 * phi_est).min(100.0 * d).max(5.0);
        r.formula(
            "Estaca — cortante da carga horizontal: hélice (modelo I, NBR 6118 17.4.2.2)",
            "bw = 0,9·D ; d = 0,75·D ;  VRd2 = 0,27·αv2·fcd·bw·d ;  Asw/s = (Vd − Vc)/(0,9·d·fywd) ;  s ≤ mín(20 cm ; 12·Øl ; D)",
            format!("Vd = {} kN ;  Vc = {} kN ;  VRd2 = {} kN", fmt(gf * h_est), fmt(vc_e), fmt(vrd2_e)),
            format!("hélice Ø{} c/{} cm (2 ramos por seção)", crate::armadura::fmt_bitola(phi_h), fmt(s_h)),
        );
        r.verif(Verificacao::max("Estaca: Vd ≤ VRd2 (carga horizontal)", gf * h_est, vrd2_e, "kN"));
        r.verif(Verificacao::info("Estaca: deslocamento horizontal do topo (Matlock-Reese)", y0, "mm"));
        if as_flex.is_none() {
            r.aviso("Estaca: seção insuficiente para o momento da carga horizontal mesmo com 4 % de armadura — aumentar o diâmetro.");
        }
        // bloco rígido sobre 2 estacas — método das bielas (Blévot)
        let db_b = h_bloco - 0.06;
        let rs = n_comp.max(0.0) * gf * (e / 2.0 - self.pilar_h / 4.0) / db_b;
        let as_bloco = (rs / (c.fyk / fat.gamma_s * 1000.0) * 1e4).max(0.00075 * b_bloco * h_bloco * 1e4);
        r.formula(
            "Bloco — tirante (método das bielas, Blévot)",
            "Rs = γf·Nest·(e/2 − ap/4)/d ;  As = Rs/fyd",
            format!("Nest = {} kN ;  e = {} m ;  ap = {} m ;  d = {} m", fmt(n_comp), fmt(e), fmt(self.pilar_h), fmt(db_b)),
            format!("Rs = {} kN ;  As = {} cm²", fmt(rs), fmt(as_bloco)),
        );
        dm.estacas = Some(EstacaDim { comprimento: l, n_comp, n_trac, h: h_est, q_adm, qt_adm, e, bloco: [l_bloco, b_bloco, h_bloco], as_bloco, n_barras: n_est, phi: phi_est, l_armada: l_arm, phi_helice: phi_h, s_helice: s_h });
    }

    /// Vista superior (planta) de 2 vãos: pilares, painel, vigas, blocos, estacas e cotas.
    pub fn planta(&self, d: &mut Desenho, cx: f64, cy: f64, k: f64) {
        let s = self.espac_pilar;
        let larg = 2.0 * s + 1.4;
        let esc = k * 3.8 / larg;
        let t = |x: f64, y: f64| [cx + esc * (x - s), cy + esc * y];
        let (bp, hp, tb) = (self.pilar_b, self.pilar_h, self.bloco);
        let (x0, x1) = (-0.6, 2.0 * s + 0.6);
        // painel de blocos (face externa em y = 0, aterro para +y)
        d.poli(Estilo::Hachura, vec![t(x0, 0.0), t(x1, 0.0), t(x1, tb), t(x0, tb)]);
        d.linha(Estilo::Concreto, vec![t(x0, 0.0), t(x1, 0.0)]);
        d.linha(Estilo::Concreto, vec![t(x0, tb), t(x1, tb)]);
        // vigas intermediárias / baldrame (projeção)
        d.tracejada(Estilo::Concreto, vec![t(x0, self.viga_b), t(x1, self.viga_b)]);
        let estacas = self.fundacao_estacas();
        let (e, lb, bb, _) = self.geometria_bloco();
        for i in 0..3 {
            let xc = i as f64 * s;
            d.poli(Estilo::Concreto, vec![t(xc - bp / 2.0, 0.0), t(xc + bp / 2.0, 0.0), t(xc + bp / 2.0, hp), t(xc - bp / 2.0, hp)]);
            if estacas {
                let yc = hp / 2.0;
                d.tracejada(
                    Estilo::Concreto,
                    vec![t(xc - bb / 2.0, yc - lb / 2.0), t(xc + bb / 2.0, yc - lb / 2.0), t(xc + bb / 2.0, yc + lb / 2.0), t(xc - bb / 2.0, yc + lb / 2.0), t(xc - bb / 2.0, yc - lb / 2.0)],
                );
                for sy in [-1.0, 1.0] {
                    d.circulo(Estilo::Concreto, t(xc, yc + sy * e / 2.0), esc * self.diam_estaca / 2.0);
                }
            }
            d.texto(t(xc, (hp / 2.0 + lb / 2.0).max(hp) + 0.30), format!("P{}", i + 1));
        }
        let yc = if estacas { hp / 2.0 - lb / 2.0 - 0.45 } else { -0.45 };
        for i in 0..2 {
            let (a, b) = (i as f64 * s, (i + 1) as f64 * s);
            d.cota_h(t(a, 0.0)[0], t(b, 0.0)[0], cy + esc * yc, cy + esc * (yc + 0.3), 0.07 * k, format!("{} m (eixos)", fmt(s)));
        }
        d.texto(t(s, (hp / 2.0 + lb / 2.0).max(hp) + 0.75), "VISTA SUPERIOR — pilares, blocos e estacas");
        d.texto(t(x1 - 0.1, tb + 0.25), "aterro");
    }
}

#[cfg(test)]
mod testes {
    use super::{FundacaoMisto, Sistema};
    use crate::projeto::{Projeto, TipoContencao};

    fn misto(h: f64, fund: FundacaoMisto) -> Projeto {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = h;
        p.pre_dimensionar();
        p.flexao.sistema = Sistema::Misto;
        p.flexao.misto.fundacao = fund;
        p
    }

    #[test]
    fn vigas_mais_proximas_embaixo() {
        let p = misto(3.0, FundacaoMisto::Estacas);
        let pr = |y: f64| 30.0 * (1.0 - y / 3.0) + 6.0; // pressão maior embaixo
        let n = p.flexao.misto.niveis_vigas(&pr, 3.0, 1.4, 435.0);
        assert!(!n.is_empty(), "{n:?}");
        let mut apoios = vec![3.0];
        apoios.extend(n.iter());
        apoios.push(0.0);
        let vaos: Vec<f64> = apoios.windows(2).map(|w| w[0] - w[1]).collect();
        // vãos não crescem para baixo (exceto o trecho final, que é o resto)
        for w in vaos[..vaos.len() - 1].windows(2) {
            assert!(w[1] <= w[0] + 1e-9, "{vaos:?}");
        }
    }

    #[test]
    fn misto_com_estacas_dimensiona_tudo() {
        let p = misto(2.5, FundacaoMisto::Estacas);
        let r = p.calcular();
        for n in ["Pilar: x/d", "Painel: Md ≤ MRd", "Viga baldrame", "Estaca: compressão", "Estaca: comprimento"] {
            assert!(r.verificacoes.iter().any(|v| v.nome.starts_with(n)), "falta {n}");
        }
        assert!(!r.verificacoes.iter().any(|v| v.nome.starts_with("FS tombamento")), "sem sapata: sem tombamento da sapata");
        assert!(r.armaduras.iter().any(|a| a.elemento.starts_with("Estaca")));
        let m = crate::modelo3d::Modelo3d::montar(&p, &r, 9.0, false);
        assert!(m.pecas.iter().any(|x| x.classe == crate::modelo3d::Classe::Estaca));
        assert!(m.pecas.iter().any(|x| x.classe == crate::modelo3d::Classe::Pilar));
        let ifc = crate::ifc::gerar(&m, &p);
        assert!(ifc.contains("IFCCOLUMN(") && ifc.contains("IFCPILE(") && ifc.contains("PILE_CAP"));
        assert!(crate::detalhes::quantidade(&p) >= 1, "vista superior");
    }

    #[test]
    fn misto_com_sapata_ainda_funciona() {
        let p = misto(2.5, FundacaoMisto::Sapata);
        let r = p.calcular();
        assert!(r.verificacoes.iter().any(|v| v.nome.starts_with("FS tombamento")));
        assert!(r.verificacoes.iter().any(|v| v.nome.starts_with("Painel: Md ≤ MRd")));
    }

    #[test]
    fn otimizador_acha_misto_que_atende() {
        for fund in [FundacaoMisto::Estacas, FundacaoMisto::Sapata] {
            let p = misto(2.0, fund);
            let o = crate::otimizar::otimizar(&p);
            let m = o.melhor.clone().unwrap_or_else(|| panic!("{fund:?}: {}", o.mensagem(crate::otimizar::custo(&p))));
            assert!(m.projeto.calcular().atende());
        }
    }
}

#[cfg(test)]
mod testes_vigas_barbacas {
    use super::{FundacaoMisto, Sistema};
    use crate::projeto::{Projeto, TipoContencao};

    #[test]
    fn vigas_com_faixas_de_influencia_iguais() {
        let p = Projeto::default();
        let pr = |y: f64| 10.0 * (3.0 - y); // triangular, zero no topo
        let n = p.flexao.misto.niveis_vigas(&pr, 3.0, 1.4, 435.0);
        assert!(!n.is_empty());
        // forças entre apoios consecutivos (topo, vigas, base) iguais a menos do arredondamento de 5 cm
        let f = |a: f64, b: f64| 5.0 * ((3.0 - b).powi(2) - (3.0 - a).powi(2)); // ∫ de b a a
        let mut apoios = vec![3.0];
        apoios.extend(n.iter());
        apoios.push(0.0);
        let faixas: Vec<f64> = apoios.windows(2).map(|w| f(w[0], w[1])).collect();
        let media = faixas.iter().sum::<f64>() / faixas.len() as f64;
        for x in &faixas {
            assert!((x - media).abs() < 0.15 * media, "{faixas:?}");
        }
        if n.len() == 1 {
            assert!((n[0] - 0.29 * 3.0).abs() < 0.1, "1 viga perto de H/3: {}", n[0]);
        }
    }

    #[test]
    fn barbacas_so_nos_panos_de_alvenaria() {
        let mut p = Projeto::default();
        p.tipo = TipoContencao::Flexao;
        p.geometria.h = 3.0;
        p.pre_dimensionar();
        p.flexao.sistema = Sistema::Misto;
        p.flexao.misto.fundacao = FundacaoMisto::Estacas;
        let r = p.calcular();
        let m = crate::modelo3d::Modelo3d::montar(&p, &r, 9.0, false);
        let pilares: Vec<(f64, f64)> = m.pecas.iter().filter(|x| x.classe == crate::modelo3d::Classe::Pilar).filter_map(|x| match &x.forma {
            crate::modelo3d::Forma::Extrusao { z0, z1, .. } => Some((*z0, *z1)),
            _ => None,
        }).collect();
        let vigas = p.niveis_vigas_misto();
        let vh = p.flexao.misto.viga_h;
        let mut n = 0;
        for pc in m.pecas.iter().filter(|x| x.nome.starts_with("Barbacã")) {
            if let crate::modelo3d::Forma::Barra { p0, .. } = pc.forma {
                n += 1;
                assert!(pilares.iter().all(|(a, b)| p0[2] < a - 0.05 || p0[2] > b + 0.05), "barbacã no pilar em z = {}", p0[2]);
                assert!(vigas.iter().all(|y| (p0[1] - y).abs() > vh / 2.0), "barbacã na viga em y = {}", p0[1]);
            }
        }
        assert!(n > 0);
    }
}
