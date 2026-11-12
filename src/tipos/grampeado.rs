//! Solo grampeado (NBR 16920-2): estabilidade por cunhas planas passando
//! pelo pé da escavação, arrancamento e ruptura das barras, e face.

use crate::armadura::{self, Armadura, Forma};
use crate::concreto::{flexao_retangular, Concreto};
use crate::empuxo::{rad, ParamEmpuxo};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil, GAMMA_W};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

fn verdadeiro() -> bool {
    true
}

/// Bitolas de barra de grampo (CA-50), em mm.
pub const BARRAS_GRAMPO: [f64; 4] = [16.0, 20.0, 25.0, 32.0];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoloGrampeado {
    /// Dimensionar a barra e o comprimento dos grampos pela tração (NBR 16920-2).
    #[serde(default = "verdadeiro")]
    pub auto_dim: bool,
    pub h: f64,
    /// Inclinação da face com a vertical (graus), positiva para dentro do maciço.
    pub incl_face: f64,
    pub gamma: f64,
    pub phi: f64,
    pub c: f64,
    pub sh: f64,
    pub sv: f64,
    /// Profundidade do primeiro grampo (m).
    pub z1: f64,
    pub comprimento: f64,
    /// Inclinação dos grampos abaixo da horizontal (graus).
    pub incl_grampo: f64,
    pub diam_furo: f64,
    /// Resistência ao arrancamento qs (kPa).
    pub qs: f64,
    /// Diâmetro da barra (mm).
    pub diam_barra: f64,
    pub fyk_barra: f64,
    pub espessura_face: f64,
    /// Lado da placa de cabeça do grampo (m).
    pub placa: f64,
    pub concreto: Concreto,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
    /// Altura d'água acima do pé da escavação (m), conforme a condição de
    /// drenagem. Definida no cálculo, não é salva.
    #[serde(skip)]
    pub hw: f64,
}

impl Default for SoloGrampeado {
    fn default() -> Self {
        let mut c = Concreto::default();
        c.fck = 25.0;
        c.cobrimento = 0.03;
        c.fyk = 600.0;
        Self {
            auto_dim: true,
            h: 6.0,
            incl_face: 10.0,
            gamma: 18.0,
            phi: 28.0,
            c: 10.0,
            sh: 1.5,
            sv: 1.5,
            z1: 0.75,
            comprimento: 6.0,
            incl_grampo: 15.0,
            diam_furo: 0.10,
            qs: 100.0,
            diam_barra: 20.0,
            fyk_barra: 500.0,
            espessura_face: 0.10,
            placa: 0.20,
            concreto: c,
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
            hw: 0.0,
        }
    }
}

struct Cunha {
    fs: f64,
    omega: f64,
}

impl SoloGrampeado {
    fn profundidades(&self) -> Vec<f64> {
        let mut v = Vec::new();
        let mut z = self.z1.max(0.1);
        while z < self.h - 0.3 && v.len() < 100 {
            v.push(z);
            z += self.sv.max(0.3);
        }
        v
    }

    /// Tração estimada em cada grampo (kN): (Ka·σ'v − 2c·√Ka)·Sh·Sv, com σ'v incluindo a sobrecarga.
    fn tracoes(&self, zs: &[f64]) -> Vec<f64> {
        let perfil = Perfil::simples(self.gamma, self.phi, self.c);
        let ka = self.empuxo.coef_ativo(self.phi, -rad(self.incl_face));
        zs.iter()
            .map(|&z| {
                let sigma = perfil.sigma_v_ef(z) + self.cargas.q_uniforme;
                (ka * sigma - 2.0 * self.c * ka.sqrt()).max(0.0) * self.sh * self.sv
            })
            .collect()
    }

    /// Distância ao longo do grampo, a partir da face, até o plano de ruptura `omega` (m).
    fn dist_ate_plano(&self, z: f64, omega: f64) -> f64 {
        let (eta, i) = (rad(self.incl_face), rad(self.incl_grampo));
        let y = self.h - z;
        let xf = y * eta.tan();
        ((y - xf * omega.tan()) / (i.sin() + i.cos() * omega.tan())).max(0.0)
    }

    /// Dimensiona a bitola pela tração máxima (Rtd ≥ FS·T) e o comprimento pelo arrancamento
    /// (L = distância até o plano crítico + FS·T/(π·D·qs)), em múltiplos de 0,5 m, até que a
    /// cunha atenda ao FS mínimo.
    pub fn dimensionar(&mut self, fat: &Fatores) {
        let zs = self.profundidades();
        if zs.is_empty() {
            return;
        }
        let trac = self.tracoes(&zs);
        let t_max = trac.iter().cloned().fold(0.0, f64::max);
        let (gm, fs) = (fat.gamma_m_grampo, fat.fs_grampo_interna);
        for &phi in &BARRAS_GRAMPO {
            self.diam_barra = phi;
            if self.area_barra_mm2() * self.fyk_barra / 1000.0 / gm >= fs * t_max {
                break;
            }
        }
        let mut l = 3.0f64;
        for _ in 0..8 {
            self.comprimento = l;
            let crit = self.busca(&zs, false, gm);
            let mut nec = 3.0f64;
            for (k, &z) in zs.iter().enumerate() {
                nec = nec.max(self.dist_ate_plano(z, crit.omega) + fs * trac[k] / (PI * self.diam_furo * self.qs));
            }
            let novo = (nec * 2.0).ceil() / 2.0;
            if (novo - l).abs() < 1e-9 {
                break;
            }
            l = novo;
        }
        self.comprimento = l;
        while l < 1.5 * self.h && self.busca(&zs, false, gm).fs < fat.fs_global() {
            l += 0.5;
            self.comprimento = l;
        }
    }

    fn area_barra_mm2(&self) -> f64 {
        PI * self.diam_barra.powi(2) / 4.0
    }

    /// Comprimento de grampo além do plano de ruptura de ângulo `omega`.
    fn comprimento_ancorado(&self, z: f64, omega: f64) -> f64 {
        let eta = rad(self.incl_face);
        let i = rad(self.incl_grampo);
        let y = self.h - z;
        let xf = y * eta.tan();
        let t = (y - xf * omega.tan()) / (i.sin() + i.cos() * omega.tan());
        if t <= 0.0 { self.comprimento } else { (self.comprimento - t).max(0.0) }
    }

    /// Abscissa em que o plano de ruptura de ângulo `omega` (pelo pé) corta o
    /// terrapleno, que sobe com o talude β a partir da crista da face.
    /// `None` se o plano não emerge no terrapleno.
    fn x_topo(&self, omega: f64) -> Option<f64> {
        let x_ft = self.h * rad(self.incl_face).tan();
        let b = rad(self.empuxo.beta).tan();
        let t = omega.tan();
        if t <= b + 1e-9 {
            return None;
        }
        let x = (self.h - x_ft * b) / (t - b);
        if x <= x_ft + 1e-6 { None } else { Some(x) }
    }

    fn cunha(&self, omega: f64, zs: &[f64], sismo: bool, gm: f64) -> f64 {
        let h = self.h;
        let eta = rad(self.incl_face);
        let i = rad(self.incl_grampo);
        let phi = rad(self.phi);
        let x_ft = h * eta.tan();
        let Some(x_top) = self.x_topo(omega) else {
            return f64::INFINITY;
        };
        // cunha: pé (0,0), crista da face (x_ft, h) e ponto de emergência no terrapleno
        let w = 0.5 * x_top * (h - x_ft * omega.tan()) * self.gamma;
        let q = self.cargas.q_uniforme * (x_top - x_ft);
        let lp = x_top / omega.cos();
        // poropressão no plano de ruptura: nível d'água horizontal a hw acima do pé
        let u = if self.hw > 0.0 {
            let lw = (self.hw / omega.sin()).min(lp);
            let p0 = GAMMA_W * self.hw;
            let p1 = GAMMA_W * (self.hw - lw * omega.sin()).max(0.0);
            0.5 * (p0 + p1) * lw
        } else {
            0.0
        };
        // NBR 16920-2 9.1: Rtd = A·fyk/γm
        let rt = self.area_barra_mm2() * self.fyk_barra / 1000.0 / gm;
        let mut res_g = 0.0;
        for &z in zs {
            let le = self.comprimento_ancorado(z, omega);
            if le <= 0.0 || le >= self.comprimento {
                continue;
            }
            let tn = (PI * self.diam_furo * le * self.qs).min(rt) / self.sh;
            res_g += tn * ((omega + i).cos() + (omega + i).sin() * phi.tan());
        }
        let kh = if sismo { self.cargas.kh } else { 0.0 };
        let kv = if sismo { self.cargas.kv } else { 0.0 };
        let wv = w * (1.0 - kv) + q;
        let solic = wv * omega.sin() + kh * w * omega.cos();
        let normal = (wv * omega.cos() - kh * w * omega.sin() - u).max(0.0);
        (normal * phi.tan() + self.c * lp + res_g) / solic
    }

    fn busca(&self, zs: &[f64], sismo: bool, gm: f64) -> Cunha {
        let face = 90.0 - self.incl_face;
        let mut melhor = Cunha { fs: f64::INFINITY, omega: rad(45.0) };
        let mut g = 10.0f64.max(self.empuxo.beta + 0.5);
        while g < face - 0.5 {
            let om = rad(g);
            let fs = self.cunha(om, zs, sismo, gm);
            if fs < melhor.fs {
                melhor = Cunha { fs, omega: om };
            }
            g += 0.25;
        }
        melhor
    }

    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let zs = self.profundidades();
        let gm = fat.gamma_m_grampo;
        let rt = self.area_barra_mm2() * self.fyk_barra / 1000.0 / gm;
        let perfil = Perfil::simples(self.gamma, self.phi, self.c);
        let theta = -rad(self.incl_face);
        let ka = self.empuxo.coef_ativo(self.phi, theta);

        r.titulo("Dados");
        r.linha(format!(
            "H = {} m ; face a {}° da vertical ; solo: γ = {} kN/m³, φ = {}°, c = {} kPa ; q = {} kPa",
            fmt(self.h), fmt(self.incl_face), fmt(self.gamma), fmt(self.phi), fmt(self.c), fmt(self.cargas.q_uniforme)
        ));
        r.linha(format!(
            "Grampos: {} linhas ; Sh = {} m ; Sv = {} m ; L = {} m ; inclinação = {}° ; furo Ø {} cm ; barra Ø {} mm ; qs = {} kPa",
            zs.len(), fmt(self.sh), fmt(self.sv), fmt(self.comprimento), fmt(self.incl_grampo),
            fmt(self.diam_furo * 100.0), fmt(self.diam_barra), fmt(self.qs)
        ));
        r.linha(format!("K ativo ({}) = {} ; Rtd = A·fyk/{} = {} kN (NBR 16920-2 9.1)", self.empuxo.teoria.nome(), fmt(ka), fmt(gm), fmt(rt)));
        if self.empuxo.beta.abs() > 1e-6 {
            r.linha(format!(
                "Terrapleno em talude β = {}° a partir da crista da face: entra no peso da cunha (plano pelo pé, ω > β) e em Ka.",
                fmt(self.empuxo.beta)
            ));
            if self.empuxo.beta >= self.phi {
                r.aviso("β ≥ φ: o próprio talude do terrapleno é instável sem a contribuição da coesão; verifique a estabilidade global.");
            }
        }
        if self.hw > 0.0 {
            r.linha(format!(
                "Água no maciço: nível a {} m acima do pé (condição de drenagem) — poropressão U no plano de ruptura reduz a normal efetiva.",
                fmt(self.hw)
            ));
        }
        if zs.is_empty() {
            r.aviso("Nenhuma linha de grampos cabe na altura informada.");
        }
        if self.auto_dim && !zs.is_empty() {
            let trac = self.tracoes(&zs);
            let t_max = trac.iter().cloned().fold(0.0, f64::max);
            r.formula(
                "Dimensionamento dos grampos pela tração",
                "T = (Ka·σ'v − 2c·√Ka)·Sh·Sv ;  Rtd = As·fyk/γm ≥ FS·T,máx ;  L = t_plano + FS·T/(π·D·qs)",
                format!(
                    "T,máx = {} kN ;  FS = {} ;  γm = {} ;  D = {} cm ;  qs = {} kPa",
                    fmt(t_max), fmt(fat.fs_grampo_interna), fmt(gm), fmt(self.diam_furo * 100.0), fmt(self.qs)
                ),
                format!(
                    "barra Ø {} mm (Rtd = {} kN) ;  L = {} m (múltiplo de 0,5 m, mínimo 3 m)",
                    fmt(self.diam_barra), fmt(rt), fmt(self.comprimento)
                ),
            );
        }

        // reforços para a estabilidade global: resistências não minoradas (o FS global da
        // NBR 11682 Tab. 3 é aplicado uma única vez, como na análise por cunhas)
        {
            let (eta, inc) = (rad(self.incl_face), rad(self.incl_grampo));
            r.reforcos = zs
                .iter()
                .map(|&z| {
                    let y = self.h - z;
                    crate::global::Reforco {
                        p0: [y * eta.tan(), y],
                        dir: [inc.cos(), -inc.sin()],
                        comprimento: self.comprimento,
                        livre: 0.0,
                        arr_por_m: PI * self.diam_furo * self.qs / self.sh,
                        aco: rt / self.sh,
                    }
                })
                .collect();
        }
        let sem = self.busca(&[], false, gm);
        // Estabilidade por cunhas
        r.titulo("Estabilidade — cunhas planas pelo pé");
        let crit = self.busca(&zs, false, gm);
        r.linha(format!(
            "Cunha crítica: ω = {}° com a horizontal ; FS = {}",
            fmt(crit.omega.to_degrees()), fmt(crit.fs)
        ));
        r.norma(normas::NBR16920_ESTABILIDADE);
        r.norma(normas::NBR11682_TAB3);
        r.formula(
            "Cunha plana pelo pé — busca do mínimo em ω",
            "FS = [(N − U)·tan φ + c·L + Σ Tn·(cos(ω+i) + sen(ω+i)·tan φ)] / [(W + Q)·sen ω] ;  Tn = mín(π·D·Le·qs ; Rtd) / Sh ;  U = poropressão no plano (0 se drenado)",
            format!(
                "γ = {} kN/m³ ;  φ = {}° ;  c = {} kPa ;  q = {} kPa ;  {} linhas de grampos",
                fmt(self.gamma), fmt(self.phi), fmt(self.c), fmt(self.cargas.q_uniforme), zs.len()
            ),
            format!("ω crítico = {}° ;  FS = {} (sem grampos: {})", fmt(crit.omega.to_degrees()), fmt(crit.fs), fmt(sem.fs)),
        );
        r.verif(Verificacao::min("FS cunha crítica — estático (NBR 16920-2 Tab. B.3)", crit.fs, fat.fs_global(), ""));
        if self.cargas.usar_sismo {
            let cs = self.busca(&zs, true, gm);
            r.linha(format!("Sísmico: ω = {}° ; FS = {}", fmt(cs.omega.to_degrees()), fmt(cs.fs)));
            r.verif(Verificacao::min("FS cunha crítica — sísmico (NBR 11682 Tab. 3)", cs.fs, fat.fs_global(), ""));
        }
        r.linha(format!("Referência sem grampos: FS = {} (ω = {}°)", fmt(sem.fs), fmt(sem.omega.to_degrees())));

        // Grampos individuais
        r.titulo("Grampos — arrancamento e tração");
        r.norma(normas::NBR16920_BARRA);
        r.norma(normas::NBR16920_INTERNA);
        r.formula(
            "Resistências de um grampo",
            "T = (Ka·σv − 2c·√Ka)·Sh·Sv ;  R_arr = π·D·Le·qs ;  Rtd = As·fyk / γm",
            format!(
                "Ka = {} ;  Sh·Sv = {}·{} ;  π·{}·Le·{} ;  Rtd = {} mm²·{} MPa / {}",
                fmt(ka), fmt(self.sh), fmt(self.sv), fmt(self.diam_furo), fmt(self.qs),
                fmt(self.area_barra_mm2()), fmt(self.fyk_barra), fmt(gm)
            ),
            format!("Rtd = {} kN ;  FS = R/T ≥ {}", fmt(rt), fmt(fat.fs_grampo_interna)),
        );
        let mut t_max: f64 = 0.0;
        for (k, &z) in zs.iter().enumerate() {
            let sigma = perfil.sigma_v_ef(z) + self.cargas.q_uniforme;
            let t = (ka * sigma - 2.0 * self.c * ka.sqrt()).max(0.0) * self.sh * self.sv;
            t_max = t_max.max(t);
            let le = self.comprimento_ancorado(z, crit.omega);
            let rp = PI * self.diam_furo * le * self.qs;
            r.linha(format!(
                "Grampo {} (z = {} m): T estimada = {} kN ; L ancorado = {} m ; R arranc. = {} kN",
                k + 1, fmt(z), fmt(t), fmt(le), fmt(rp)
            ));
            let fs_a = if t > 0.0 { rp / t } else { f64::INFINITY };
            let fs_t = if t > 0.0 { rt / t } else { f64::INFINITY };
            r.verif(Verificacao::min(format!("Grampo {}: FS arrancamento", k + 1), fs_a, fat.fs_grampo_interna, ""));
            r.verif(Verificacao::min(format!("Grampo {}: FS tração (Rtd/T)", k + 1), fs_t, fat.fs_grampo_interna, ""));
        }

        // Face
        let mut as_face = 0.0;
        if t_max > 0.0 {
            let c = &self.concreto;
            let smax = self.sh.max(self.sv);
            let t0 = t_max * (0.5 + (smax - 0.5) / 5.0);
            let p = t0 / (self.sh * self.sv);
            let m = p * smax * smax / 10.0;
            let md = fat.gamma_f * m;
            let e = self.espessura_face;
            let d = e / 2.0;
            let f = flexao_retangular(md, 1.0, e, d, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
            r.titulo("Face (concreto projetado)");
            r.linha(format!(
                "T0 na cabeça (Clouterre) = {} kN ; pressão na face = {} kPa ; M ≈ pS²/10 = {} kN·m/m ; As = {} cm²/m (mín. {})",
                fmt(t0), fmt(p), fmt(m), fmt(f.as_calc), fmt(f.as_min)
            ));
            r.formula(
                "Face — carga na cabeça do grampo (Clouterre) e flexão",
                "T0 = T_máx·(0,5 + (S − 0,5)/5) ;  p = T0/(Sh·Sv) ;  M = p·S²/10 ;  As = Md / (fyd·(d − y/2))",
                format!("T0 = {}·(0,5 + ({} − 0,5)/5) ;  d = e/2 = {} cm", fmt(t_max), fmt(smax), fmt(d * 100.0)),
                format!("T0 = {} kN ;  p = {} kPa ;  M = {} kN·m/m ;  As = {} cm²/m", fmt(t0), fmt(p), fmt(m), fmt(f.as_calc)),
            );
            r.norma(normas::NBR6118_FLEXAO);
            as_face = f.as_adotar();
            r.verif(Verificacao::max("Face: x/d", f.x_d, 0.45, ""));
            let dm = d * 1000.0;
            let u = 4.0 * self.placa * 1000.0 + 4.0 * PI * dm;
            let rho = (f.as_adotar() / 1e4 / d).min(0.02);
            let trd1 = crate::concreto::tau_rd1_puncao(d, rho, c.fck);
            let tsd = fat.gamma_f * t0 * 1000.0 / (u * dm);
            r.norma(normas::NBR6118_PUNCAO);
            r.formula(
                "Face — punção da placa no contorno C'",
                "u = 4·a + 4·π·d ;  τSd = γf·T0 / (u·d) ;  τRd1 = 0,13·(1 + √(20/d))·(100·ρ·fck)^(1/3)",
                format!("γf·T0 = {}·{} kN ;  a = {} m ;  d = {} cm", fmt(fat.gamma_f), fmt(t0), fmt(self.placa), fmt(d * 100.0)),
                format!("τSd = {} MPa ;  τRd1 = {} MPa", fmt(tsd), fmt(trd1)),
            );
            r.verif(Verificacao::max("Face: punção na placa τSd ≤ τRd1", tsd, trd1, "MPa"));
        }

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        r.norma(normas::NBR11682_SOBRECARGA);
        if perfil.usar_na {
            r.aviso("Água não considerada no solo grampeado.");
        }
        r.aviso("Análise por cunhas planas é simplificada; verificar superfícies circulares/poligonais (NBR 11682) em software específico.");
        r.aviso("qs deve ser confirmado por ensaios de arrancamento (NBR 16920-2).");
        self.detalhamento(&mut r, &zs, as_face);
        r.desenho = self.desenho(&zs, crit.omega);
        self.desenhar_tela(&mut r.desenho, &r.armaduras);
        r
    }

    /// Detalhamento: grampos, proteção, tela soldada e concreto projetado.
    fn detalhamento(&self, r: &mut Resultado, zs: &[f64], as_face: f64) {
        let c = &self.concreto;
        r.titulo("Detalhamento construtivo — solo grampeado");
        r.norma(normas::NBR16920_PROTECAO);
        r.norma(normas::NBR16920_PARAMENTO);
        r.linha(format!(
            "Grampos: {} linhas ; malha Sh × Sv = {} × {} m ; 1ª linha a {} m do topo ; L = {} m ; furo Ø {} cm ; barra Ø {} mm (fyk {} MPa) ; inclinação {}° abaixo da horizontal",
            zs.len(),
            fmt(self.sh),
            fmt(self.sv),
            fmt(self.z1),
            fmt(self.comprimento),
            fmt(self.diam_furo * 100.0),
            fmt(self.diam_barra),
            fmt(self.fyk_barra),
            fmt(self.incl_grampo)
        ));
        r.verif(Verificacao::min("Grampos: inclinação ≥ 10° (critério de projeto)", self.incl_grampo, 10.0, "°"));
        r.verif(Verificacao::max("Grampos: inclinação ≤ 15° (critério de projeto)", self.incl_grampo, 15.0, "°"));
        let cob_calda = (self.diam_furo * 1000.0 - self.diam_barra) / 2.0;
        r.formula(
            "Cobrimento de calda da barra",
            "c = (Ø_furo − Ø_barra) / 2 ≥ 20 mm",
            format!("c = ({} − {}) / 2", fmt(self.diam_furo * 1000.0), fmt(self.diam_barra)),
            format!("c = {} mm (com espaçadores centralizadores)", fmt(cob_calda)),
        );
        r.verif(Verificacao::min("Grampos: cobrimento de calda ≥ 20 mm (NBR 16920-2 10.2.3)", cob_calda, 20.0, "mm"));
        let n_esp = (self.comprimento / 2.0).ceil().max(1.0);
        r.linha(format!(
            "Espaçadores (centralizadores) a cada 2 m no máximo: {} por grampo ; calda com a/c ≤ 0,5 e fck ≥ 25 MPa ; cabeça protegida até 30 cm dentro do terreno com cobrimento ≥ 30 mm.",
            n_esp as i64
        ));

        let mut tab = vec![Armadura {
            pos: "N1".into(),
            elemento: "Grampo — barra de aço".into(),
            bitola_mm: self.diam_barra,
            espac_cm: None,
            quantidade: format!("1 a cada {} × {} m", fmt(self.sh), fmt(self.sv)),
            comprimento_m: self.comprimento + 0.15,
            as_nec: 0.0,
            as_ef: self.area_barra_mm2() / 100.0,
            unidade: "cm²",
            lb_cm: 0.0,
            forma: Forma::Reta,
        }];
        let e = self.espessura_face;
        if as_face > 0.0 {
            match armadura::tela(as_face) {
                Some((nome, as_t)) => {
                    tab.push(Armadura {
                        pos: "N2".into(),
                        elemento: format!("Paramento — tela soldada {nome} (duas direções)"),
                        bitola_mm: 0.0,
                        espac_cm: None,
                        quantidade: nome.into(),
                        comprimento_m: 0.0,
                        as_nec: as_face,
                        as_ef: as_t,
                        unidade: "cm²/m",
                        lb_cm: 0.0,
                        forma: Forma::Tela,
                    });
                    r.formula(
                        "Tela soldada do paramento",
                        "Tela série Q com As ≥ As,nec (cada direção)",
                        format!("As,nec = {} cm²/m", fmt(as_face)),
                        format!("Tela {nome} ({} cm²/m), posicionada no meio da espessura, com traspasse de 2 malhas", fmt(as_t)),
                    );
                }
                None => {
                    let (phi, s, as_ef) = armadura::malha_principal(as_face, e);
                    tab.push(Armadura {
                        pos: "N2".into(),
                        elemento: "Paramento — malha de barras (duas direções)".into(),
                        bitola_mm: phi,
                        espac_cm: Some(s),
                        quantidade: format!("{}/m", (100.0 / s).ceil() as i64),
                        comprimento_m: 0.0,
                        as_nec: as_face,
                        as_ef,
                        unidade: "cm²/m",
                        lb_cm: armadura::lb_basico(phi, c.fck, c.fyk, 1.4, 1.15, true),
                        forma: Forma::Tela,
                    });
                    r.aviso("Paramento: nenhuma tela Q padronizada atende — adotada malha de barras.");
                    if let Some(a) = tab.last() {
                        armadura::verificar_laje(r, a, e, true);
                    }
                }
            }
        }
        r.linha(format!(
            "Paramento em concreto projetado C{} com espessura de {} cm, aplicado em camadas, placa de {} × {} m na cabeça de cada grampo.",
            fmt(c.fck),
            fmt(e * 100.0),
            fmt(self.placa),
            fmt(self.placa)
        ));
        r.armaduras.extend(tab);
    }

    /// Tela do paramento no desenho.
    fn desenhar_tela(&self, d: &mut Desenho, arm: &[Armadura]) {
        let x_ft = self.h * rad(self.incl_face).tan();
        let e = self.espessura_face.max(0.02 * self.h);
        d.linha(Estilo::Armadura, vec![[-e / 2.0, 0.05], [x_ft - e / 2.0, self.h - 0.05]]);
        if let Some(a) = arm.iter().find(|a| a.pos == "N2") {
            d.texto([-0.25 * self.h, 0.75 * self.h], a.descricao());
        }
    }

    fn desenho(&self, zs: &[f64], omega: f64) -> Desenho {
        let mut d = Desenho::default();
        let h = self.h;
        let eta = rad(self.incl_face);
        let i = rad(self.incl_grampo);
        let x_ft = h * eta.tan();
        let x_top = self.x_topo(omega).unwrap_or(h / omega.tan());
        let xe = (x_ft + self.comprimento * 1.4 + 0.3 * h).max(1.1 * x_top);
        let xi = -0.5 * h;
        let e = self.espessura_face.max(0.02 * h);
        d.retangulo(Estilo::SoloFundacao, xi, -0.3 * h, xe, 0.0);
        // terrapleno em talude β a partir da crista da face
        let tb = rad(self.empuxo.beta).tan();
        let sup = move |x: f64| h + (x - x_ft).max(0.0) * tb;
        d.poli(Estilo::Solo, vec![[0.0, 0.0], [xe, 0.0], [xe, sup(xe)], [x_ft, h]]);
        d.poli(Estilo::Concreto, vec![[-e, 0.0], [0.0, 0.0], [x_ft, h], [x_ft - e, h]]);
        for &z in zs {
            let y = h - z;
            let xf = y * eta.tan();
            let l = self.comprimento;
            // o grampo atravessa o concreto projetado até a placa de cabeça (face externa)
            let ext = [xf - e, y + e * i.tan()];
            d.linha(Estilo::Tirante, vec![ext, [xf + l * i.cos(), y - l * i.sin()]]);
            let meia = 0.5 * self.placa;
            d.retangulo(Estilo::Aco, ext[0] - 0.04, ext[1] - meia, ext[0], ext[1] + meia);
        }
        d.tracejada(Estilo::Ruptura, vec![[0.0, 0.0], [x_top, x_top * omega.tan()]]);
        d.texto([x_top * 0.6 + 0.15 * h, x_top * 0.6 * omega.tan() * 0.9], format!("ω = {}°", fmt(omega.to_degrees())));
        if self.hw > 0.0 {
            d.tracejada(Estilo::Agua, vec![[0.0, self.hw], [xe, self.hw]]);
            d.texto([xe * 0.9, self.hw + 0.04 * h], "NA");
        }
        if self.cargas.q_uniforme > 0.0 {
            d.sobrecarga_sup(x_ft, xe, 0.08 * h, &sup);
            d.texto([xe * 0.75, sup(xe * 0.75) + 0.2 * h], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        d.texto([xi * 0.6, h / 2.0], format!("H = {} m", fmt(h)));
        d
    }
}

#[cfg(test)]
mod testes_cunha {
    use super::*;

    /// Cunha plana pelo pé, talude β = 0, sem grampos, calculada à mão.
    #[test]
    fn cunha_sem_grampos_confere_com_calculo_manual() {
        let mut g = SoloGrampeado::default();
        g.h = 1.0;
        g.incl_face = 0.0;
        g.gamma = 18.0;
        g.phi = 28.0;
        g.c = 10.0;
        g.cargas.q_uniforme = 20.0;
        // ω = 45°: W = ½·1·1·18 = 9 ; Q = 20·1 = 20 ; L = 1/sen45° = 1,4142
        // N = 29·cos45° = 20,506 ; T = 29·sen45° = 20,506
        // FS = (20,506·tan28° + 10·1,4142) / 20,506 = (10,903 + 14,142)/20,506 = 1,2214
        let fs = g.cunha(45f64.to_radians(), &[], false, 1.15);
        assert!((fs - 1.2214).abs() < 0.002, "FS = {fs}");
        // talude β = 10° eleva o peso da cunha → FS menor
        g.empuxo.beta = 10.0;
        assert!(g.cunha(45f64.to_radians(), &[], false, 1.15) < fs);
        // água a 0,5 m acima do pé: U reduz a normal efetiva → FS menor
        g.empuxo.beta = 0.0;
        g.hw = 0.5;
        assert!(g.cunha(45f64.to_radians(), &[], false, 1.15) < fs);
        // com o talude β = ω a cunha não emerge: FS infinito
        g.hw = 0.0;
        g.empuxo.beta = 45.0;
        assert!(g.cunha(45f64.to_radians(), &[], false, 1.15).is_infinite());
    }
}
