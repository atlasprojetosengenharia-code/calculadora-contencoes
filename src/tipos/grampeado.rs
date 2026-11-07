//! Solo grampeado (NBR 16920-2): estabilidade por cunhas planas passando
//! pelo pé da escavação, arrancamento e ruptura das barras, e face.

use crate::concreto::{flexao_retangular, Concreto};
use crate::empuxo::{rad, ParamEmpuxo};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoloGrampeado {
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
}

impl Default for SoloGrampeado {
    fn default() -> Self {
        let mut c = Concreto::default();
        c.fck = 25.0;
        c.cobrimento = 0.03;
        c.fyk = 600.0;
        Self {
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

    fn cunha(&self, omega: f64, zs: &[f64], sismo: bool, gm: f64) -> f64 {
        let h = self.h;
        let eta = rad(self.incl_face);
        let i = rad(self.incl_grampo);
        let phi = rad(self.phi);
        let x_ft = h * eta.tan();
        let x_top = h / omega.tan();
        if x_top <= x_ft + 1e-6 {
            return f64::INFINITY;
        }
        let w = 0.5 * h * (x_top - x_ft) * self.gamma;
        let q = self.cargas.q_uniforme * (x_top - x_ft);
        let lp = h / omega.sin();
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
        let normal = (wv * omega.cos() - kh * w * omega.sin()).max(0.0);
        (normal * phi.tan() + self.c * lp + res_g) / solic
    }

    fn busca(&self, zs: &[f64], sismo: bool, gm: f64) -> Cunha {
        let face = 90.0 - self.incl_face;
        let mut melhor = Cunha { fs: f64::INFINITY, omega: rad(45.0) };
        let mut g = 10.0;
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
            r.aviso("Solo grampeado: terrapleno considerado horizontal na busca de cunhas (β ignorado).");
        }
        if zs.is_empty() {
            r.aviso("Nenhuma linha de grampos cabe na altura informada.");
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
            "FS = [N·tan φ + c·L + Σ Tn·(cos(ω+i) + sen(ω+i)·tan φ)] / [(W + Q)·sen ω] ;  Tn = mín(π·D·Le·qs ; Rtd) / Sh",
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
        r.desenho = self.desenho(&zs, crit.omega);
        r
    }

    fn desenho(&self, zs: &[f64], omega: f64) -> Desenho {
        let mut d = Desenho::default();
        let h = self.h;
        let eta = rad(self.incl_face);
        let i = rad(self.incl_grampo);
        let x_ft = h * eta.tan();
        let xe = x_ft + self.comprimento * 1.4 + 0.3 * h;
        let xi = -0.5 * h;
        let e = self.espessura_face.max(0.02 * h);
        d.retangulo(Estilo::SoloFundacao, xi, -0.3 * h, xe, 0.0);
        d.poli(Estilo::Solo, vec![[0.0, 0.0], [xe, 0.0], [xe, h], [x_ft, h]]);
        d.poli(Estilo::Concreto, vec![[-e, 0.0], [0.0, 0.0], [x_ft, h], [x_ft - e, h]]);
        for &z in zs {
            let y = h - z;
            let xf = y * eta.tan();
            let l = self.comprimento;
            d.linha(Estilo::Tirante, vec![[xf, y], [xf + l * i.cos(), y - l * i.sin()]]);
        }
        let x_top = h / omega.tan();
        d.tracejada(Estilo::Ruptura, vec![[0.0, 0.0], [x_top, h]]);
        d.texto([x_top * 0.6 + 0.15 * h, h * 0.55], format!("ω = {}°", fmt(omega.to_degrees())));
        if self.cargas.q_uniforme > 0.0 {
            d.sobrecarga(x_ft, xe, h, 0.08 * h);
            d.texto([xe * 0.75, h + 0.14 * h], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        d.texto([xi * 0.6, h / 2.0], format!("H = {} m", fmt(h)));
        d
    }
}
