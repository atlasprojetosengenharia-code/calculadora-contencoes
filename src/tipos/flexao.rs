//! Muro de flexão em concreto armado (perfil L / T invertido).

use super::gravidade::{empuxo_no_plano, passivo_frontal, subpressao};
use crate::concreto::{flexao_retangular, vrd1, Concreto};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::estabilidade::{analisar, centroide, Forca, LimitesFs};
use crate::fatores::Fatores;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil, SoloFundacao};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MuroFlexao {
    /// Altura total, do fundo da sapata ao topo do muro (m).
    pub h: f64,
    pub t_topo: f64,
    pub t_base: f64,
    pub h_sapata: f64,
    pub b_ponta: f64,
    pub b_talao: f64,
    pub df: f64,
    pub considerar_passivo: bool,
    pub frac_passivo: f64,
    pub subpressao: bool,
    pub concreto: Concreto,
    pub aterro: Perfil,
    pub fundacao: SoloFundacao,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
}

impl Default for MuroFlexao {
    fn default() -> Self {
        Self {
            h: 5.0,
            t_topo: 0.25,
            t_base: 0.40,
            h_sapata: 0.45,
            b_ponta: 0.8,
            b_talao: 2.2,
            df: 0.8,
            considerar_passivo: false,
            frac_passivo: 0.5,
            subpressao: true,
            concreto: Concreto::default(),
            aterro: Perfil::simples(18.0, 30.0, 0.0),
            fundacao: SoloFundacao::default(),
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
        }
    }
}

impl MuroFlexao {
    fn largura(&self) -> f64 {
        self.b_ponta + self.t_base + self.b_talao
    }

    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let (h, hs, bp, tb, tt) = (self.h, self.h_sapata, self.b_ponta, self.t_base, self.t_topo);
        let b = self.largura();
        let beta = rad(self.empuxo.beta);
        let x_bt = bp + tt; // tardoz no topo
        let x_bb = bp + tb; // tardoz na base
        let hv = h + (b - x_bt) * beta.tan();
        let gs = self.aterro.gamma_medio(hv);
        let pc = self.concreto.peso;

        r.titulo("Dados");
        r.linha(format!(
            "H = {} m ; B = {} m (ponta {} + parede {} + talão {}) ; sapata h = {} m ; parede {}→{} m",
            fmt(h), fmt(b), fmt(bp), fmt(tb), fmt(self.b_talao), fmt(hs), fmt(tt), fmt(tb)
        ));
        r.linha(format!(
            "Teoria: {} (plano virtual vertical no fim do talão) ; δ = {}° ; β = {}° ; q = {} kPa",
            self.empuxo.teoria.nome(), fmt(self.empuxo.delta), fmt(self.empuxo.beta), fmt(self.cargas.q_uniforme)
        ));
        if self.empuxo.teoria == Teoria::Coulomb && (self.empuxo.delta - self.empuxo.beta).abs() > 1e-6 {
            r.aviso("Coulomb em plano virtual solo-solo: recomenda-se δ = β.");
        }

        // Pesos
        let mut base = Vec::new();
        let w_sap = b * hs * pc;
        base.push(Forca::nova("Peso da sapata", 0.0, w_sap, b / 2.0, hs / 2.0));
        let p_par = [[bp, hs], [x_bb, hs], [x_bt, h], [bp, h]];
        let (a, cx, cy) = centroide(&p_par);
        let w_par = a * pc;
        base.push(Forca::nova("Peso da parede", 0.0, w_par, cx, cy));
        let p_solo = [[x_bb, hs], [b, hs], [b, hv], [x_bt, h]];
        let (a, sx, sy) = centroide(&p_solo);
        let w_solo = a * gs;
        base.push(Forca::nova("Solo sobre o talão", 0.0, w_solo, sx, sy));
        if self.df > hs {
            let w = (self.df - hs) * bp * self.fundacao.gamma;
            base.push(Forca::nova("Solo sobre a ponta", 0.0, w, bp / 2.0, (self.df + hs) / 2.0));
        }
        if self.considerar_passivo {
            if let Some(p) = passivo_frontal(&self.fundacao, self.df, self.frac_passivo.min(1.0 / fat.reducao_passivo_muros.max(1.0))) {
                base.push(p);
            }
        }
        if self.subpressao {
            if let Some(u) = subpressao(&self.aterro, hv, b) {
                base.push(u);
            }
        }

        let plano = move |_y: f64| b;
        let est = empuxo_no_plano(&self.aterro, &self.cargas, &self.empuxo, 0.0, hv, &plano, false);
        r.titulo("Empuxo");
        r.linha(format!(
            "K ativo (camada superior) = {} ; Pa = {} kN/m ; inclinação = {}° ; altura do plano = {} m",
            fmt(est.k), fmt(est.pa), fmt(est.incl.to_degrees()), fmt(hv)
        ));
        let mut f = base.clone();
        f.extend(est.forcas.iter().cloned());
        let lim = LimitesFs::estatico(fat);
        let dist = analisar(&mut r, "estático", &f, b, &self.fundacao, self.df, &lim);

        if self.cargas.usar_sismo {
            let sis = empuxo_no_plano(&self.aterro, &self.cargas, &self.empuxo, 0.0, hv, &plano, true);
            let mut f = base.clone();
            f.extend(sis.forcas.iter().cloned());
            let (kh, kv) = (self.cargas.kh, self.cargas.kv);
            f.push(Forca::nova("Inércia da sapata", kh * w_sap, -kv * w_sap, b / 2.0, hs / 2.0));
            f.push(Forca::nova("Inércia da parede", kh * w_par, -kv * w_par, cx, cy));
            f.push(Forca::nova("Inércia do solo s/ talão", kh * w_solo, -kv * w_solo, sx, sy));
            let lim_s = LimitesFs::sismico(fat);
            analisar(&mut r, "sísmico", &f, b, &self.fundacao, self.df, &lim_s);
        }

        self.estrutural(&mut r, fat, &dist, hv);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        r.aviso("Estabilidade global (NBR 11682) ainda não implementada — verificar em software de equilíbrio limite.");
        r.desenho = self.desenho(&est.diagrama, hv);
        r
    }

    fn estrutural(&self, r: &mut Resultado, fat: &Fatores, dist: &crate::estabilidade::DistribuicaoBase, hv: f64) {
        let c = &self.concreto;
        let (h, hs, bp, tb, tt) = (self.h, self.h_sapata, self.b_ponta, self.t_base, self.t_topo);
        let b = self.largura();
        let beta = rad(self.empuxo.beta);
        let gs = self.aterro.gamma_medio(hv);
        let gf = fat.gamma_f;

        r.titulo("Dimensionamento estrutural (NBR 6118) — por metro");
        r.linha(format!(
            "fck = {} MPa ; fyk = {} MPa ; cobrimento = {} cm ; γf = {} ; γc = {} ; γs = {}",
            fmt(c.fck), fmt(c.fyk), fmt(c.cobrimento * 100.0), fmt(gf), fmt(fat.gamma_c), fmt(fat.gamma_s)
        ));

        // Parede
        let hst = h - hs;
        let z0 = hv - h;
        let ph = |s: f64| self.empuxo.pressao_horizontal_total(&self.aterro, &self.cargas, 0.0, z0 + s);
        let (v, sc) = integrar(ph, 0.0, hst, 400);
        let m = v * (hst - sc);
        self.secao(r, "Parede (engaste)", m * gf, v * gf, tb, fat);

        // Talão
        let x_bb = bp + tb;
        let sup = |x: f64| h + (x - (bp + tt)).max(0.0) * beta.tan();
        let carga_talao = |x: f64| {
            gs * (sup(x) - hs) + c.peso * hs + self.cargas.q_uniforme - dist.pressao(x)
        };
        let (vt, xc) = integrar(carga_talao, x_bb, b, 200);
        let mt = vt * (xc - x_bb);
        self.secao(r, "Talão (face da parede)", mt * gf, vt * gf, hs, fat);

        // Ponta
        let carga_ponta = |x: f64| dist.pressao(x) - c.peso * hs;
        let (vp, xp) = integrar(carga_ponta, 0.0, bp, 200);
        let mp = vp * (bp - xp);
        self.secao(r, "Ponta (face da parede)", mp * gf, vp * gf, hs, fat);
        r.aviso("Dimensionamento estrutural feito para o caso estático (sem acréscimo sísmico na parede).");
    }

    fn secao(&self, r: &mut Resultado, nome: &str, md: f64, vd: f64, esp: f64, fat: &Fatores) {
        let c = &self.concreto;
        let d = esp - c.cobrimento - 0.01;
        let fl = flexao_retangular(md, 1.0, esp, d, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
        let face = if md >= 0.0 { "" } else { " (momento invertido)" };
        r.linha(format!(
            "{nome}: Md = {} kN·m/m{face} ; Vd = {} kN/m ; d = {} cm",
            fmt(md.abs()), fmt(vd.abs()), fmt(d * 100.0)
        ));
        if !fl.viavel {
            r.aviso(format!("{nome}: seção insuficiente (x/d > 0,45 ou Md excessivo) — aumentar espessura."));
        }
        r.verif(Verificacao::max(format!("{nome}: x/d"), fl.x_d, 0.45, ""));
        r.verif(Verificacao::info(
            format!("{nome}: As calc. / As mín."),
            fl.as_calc,
            &format!("cm²/m (mín. {})", fmt(fl.as_min)),
        ));
        let vr = vrd1(1.0, d, fl.as_adotar(), c.fck, fat.gamma_c);
        r.verif(Verificacao::max(format!("{nome}: Vd ≤ VRd1"), vd.abs(), vr, "kN/m"));
    }

    fn desenho(&self, diag: &[(f64, f64)], hv: f64) -> Desenho {
        let mut d = Desenho::default();
        let (h, hs, bp, tb, tt) = (self.h, self.h_sapata, self.b_ponta, self.t_base, self.t_topo);
        let b = self.largura();
        let beta = rad(self.empuxo.beta);
        let xe = b + 1.2 * h;
        let xi = -0.5 * h;
        let sup = |x: f64| h + (x - (bp + tt)).max(0.0) * beta.tan();

        d.retangulo(Estilo::SoloFundacao, xi, -0.35 * h, xe, 0.0);
        if self.df > 0.0 {
            d.retangulo(Estilo::Solo, xi, 0.0, 0.0, self.df);
            if self.df > hs {
                d.retangulo(Estilo::Solo, 0.0, hs, bp, self.df);
            }
        }
        d.poli(Estilo::Solo, vec![[bp + tb, hs], [b, hs], [b, hv], [bp + tt, h]]);
        d.poli(Estilo::Solo, vec![[b, 0.0], [xe, 0.0], [xe, sup(xe)], [b, hv]]);
        d.retangulo(Estilo::Concreto, 0.0, 0.0, b, hs);
        d.poli(Estilo::Concreto, vec![[bp, hs], [bp + tb, hs], [bp + tt, h], [bp, h]]);
        d.tracejada(Estilo::Cota, vec![[b, 0.0], [b, hv]]);

        if let Some(na) = self.aterro.na() {
            let y = hv - na;
            if y > 0.0 {
                d.tracejada(Estilo::Agua, vec![[bp + tb, y.max(hs)], [xe, y]]);
                d.texto([xe - 0.3 * h, y + 0.07 * h], "NA");
            }
        }
        if self.cargas.q_uniforme > 0.0 {
            let a = 0.1 * h;
            for i in 0..=8 {
                let x = (bp + tt) + (xe - bp - tt) * i as f64 / 8.0;
                d.seta([x, sup(x) + a], [x, sup(x)]);
            }
            d.texto([(b + xe) / 2.0, sup((b + xe) / 2.0) + 1.6 * a], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        if self.cargas.usar_faixa {
            let x0 = b + self.cargas.faixa_dist;
            d.retangulo(Estilo::Carga, x0, sup(x0), x0 + self.cargas.faixa_larg, sup(x0) + 0.07 * h);
        }
        let pmax = diag.iter().map(|p| p.1.abs()).fold(1e-6, f64::max);
        d.diagrama(b, 0.3 * h / pmax, diag);
        d.texto([b + 0.2 * h, -0.08 * h], format!("p máx = {} kPa", fmt(pmax)));
        d.texto([xi * 0.6, h * 0.5], format!("H = {} m", fmt(h)));
        d.texto([b / 2.0, -0.18 * h], format!("B = {} m", fmt(b)));
        d
    }
}
