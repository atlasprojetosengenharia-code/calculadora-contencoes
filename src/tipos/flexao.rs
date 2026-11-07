//! Muro de flexão em concreto armado (perfil L / T invertido).

use super::gravidade::{empuxo_no_plano, formula_empuxo, passivo_com_dente, passivo_frontal, subpressao};
use crate::armadura::{self, Armadura};
use crate::concreto::{flexao_retangular, vrd1, Concreto, Flexao};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::estabilidade::{analisar, centroide, Forca, LimitesFs};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil, SoloFundacao};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
    /// Dente de base (chave de cisalhamento) sob a parede.
    pub dente: bool,
    pub dente_h: f64,
    pub dente_b: f64,
    pub concreto: Concreto,
    pub aterro: Perfil,
    pub fundacao: SoloFundacao,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
}

/// Dados do detalhamento usados no esquema das armaduras.
struct DetalheFlexao {
    cob: f64,
    /// Traspasse da espera acima da junta (m).
    l0t: f64,
    /// Ancoragem da espera na sapata (m).
    lb_n7: f64,
    rotulos: Vec<String>,
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
            dente: false,
            dente_h: 0.4,
            dente_b: 0.4,
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
        let frac_p = self.frac_passivo.min(1.0 / fat.reducao_passivo_muros.max(1.0));
        if self.dente && self.dente_h > 0.0 {
            let xd = bp + tb / 2.0;
            base.push(Forca::nova("Peso do dente de base", 0.0, self.dente_b * self.dente_h * pc, xd, -self.dente_h / 2.0));
            r.norma(normas::NBR6122_PASSIVO);
            if let Some((p, kp)) = passivo_com_dente(&self.fundacao, self.df, self.dente_h, frac_p) {
                r.formula(
                    "Dente de base — passivo mobilizado até a base do dente",
                    "D = Df + hd ;  Ep = f·(½·Kp·γ·D² + 2c·√Kp·D) ;  Kp = tan²(45° + φ/2)",
                    format!(
                        "D = {} + {} ;  Kp = {} ;  γ = {} kN/m³ ;  c = {} kPa ;  f = {}",
                        fmt(self.df), fmt(self.dente_h), fmt(kp), fmt(self.fundacao.gamma), fmt(self.fundacao.c), fmt(frac_p)
                    ),
                    format!("D = {} m ;  Ep = {} kN/m (somado à resistência ao deslizamento)", fmt(self.df + self.dente_h), fmt(-p.h)),
                );
                base.push(p);
            }
            r.aviso("Dente de base: o passivo à frente só pode ser considerado se o solo for mantido permanentemente (sem escavações futuras).");
        } else if self.considerar_passivo {
            r.norma(normas::NBR6122_PASSIVO);
            if let Some(p) = passivo_frontal(&self.fundacao, self.df, frac_p) {
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
        let ph_q = formula_empuxo(&mut r, &self.empuxo, &self.aterro, &self.cargas, 0.0, hv, &est);
        // NA no solo de fundação: o mais alto entre o informado e o do aterro.
        let fundacao = self.fundacao.com_na(self.aterro.na().map(|n| n - hv));
        let mut f = base.clone();
        f.extend(est.forcas.iter().cloned());
        let lim = LimitesFs::estatico(fat);
        let mut cargas_perm = self.cargas.clone();
        cargas_perm.q_uniforme = 0.0;
        cargas_perm.usar_faixa = false;
        let perm = empuxo_no_plano(&self.aterro, &cargas_perm, &self.empuxo, 0.0, hv, &plano, false);
        let mut f_perm = base.clone();
        f_perm.extend(perm.forcas.iter().cloned());
        let dist = analisar(&mut r, "estático", &f, Some(&f_perm), b, &fundacao, self.df, &lim);
        let ph_total: f64 = est.forcas.iter().map(|f| f.h.max(0.0)).sum();
        super::gravidade::diagnostico_deslizamento(&mut r, ph_q, ph_total, self.considerar_passivo || self.dente);

        if self.cargas.usar_sismo {
            let sis = empuxo_no_plano(&self.aterro, &self.cargas, &self.empuxo, 0.0, hv, &plano, true);
            let mut f = base.clone();
            f.extend(sis.forcas.iter().cloned());
            let (kh, kv) = (self.cargas.kh, self.cargas.kv);
            f.push(Forca::nova("Inércia da sapata", kh * w_sap, -kv * w_sap, b / 2.0, hs / 2.0));
            f.push(Forca::nova("Inércia da parede", kh * w_par, -kv * w_par, cx, cy));
            f.push(Forca::nova("Inércia do solo s/ talão", kh * w_solo, -kv * w_solo, sx, sy));
            let lim_s = LimitesFs::sismico(fat);
            analisar(&mut r, "sísmico", &f, None, b, &fundacao, self.df, &lim_s);
        }

        let (fp, ft, fpo) = self.estrutural(&mut r, fat, &dist, hv);
        let det = self.detalhamento(&mut r, &fp, &ft, &fpo);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        r.aviso("Estabilidade global (NBR 11682) ainda não implementada — verificar em software de equilíbrio limite.");
        r.desenho = self.desenho(&est.diagrama, hv);
        self.desenhar_armaduras(&mut r.desenho, &det);
        r
    }

    fn estrutural(
        &self,
        r: &mut Resultado,
        fat: &Fatores,
        dist: &crate::estabilidade::DistribuicaoBase,
        hv: f64,
    ) -> (Flexao, Flexao, Flexao) {
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
        let fp = self.secao(r, "Parede (engaste)", m * gf, v * gf, tb, fat);

        // Talão
        let x_bb = bp + tb;
        let sup = |x: f64| h + (x - (bp + tt)).max(0.0) * beta.tan();
        let carga_talao = |x: f64| {
            gs * (sup(x) - hs) + c.peso * hs + self.cargas.q_uniforme - dist.pressao(x)
        };
        let (vt, xc) = integrar(carga_talao, x_bb, b, 200);
        let mt = vt * (xc - x_bb);
        let ft = self.secao(r, "Talão (face da parede)", mt * gf, vt * gf, hs, fat);

        // Ponta
        let carga_ponta = |x: f64| dist.pressao(x) - c.peso * hs;
        let (vp, xp) = integrar(carga_ponta, 0.0, bp, 200);
        let mp = vp * (bp - xp);
        let fpo = self.secao(r, "Ponta (face da parede)", mp * gf, vp * gf, hs, fat);
        r.aviso("Dimensionamento estrutural feito para o caso estático (sem acréscimo sísmico na parede).");
        (fp, ft, fpo)
    }

    fn secao(&self, r: &mut Resultado, nome: &str, md: f64, vd: f64, esp: f64, fat: &Fatores) -> Flexao {
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
        let fcd = crate::concreto::eta_c(c.fck) * c.fck / fat.gamma_c;
        let fyd = c.fyk / fat.gamma_s;
        r.norma(normas::NBR6118_FLEXAO);
        r.norma(normas::NBR6118_AS_MIN);
        r.formula(
            format!("{nome} — flexão (b = 1 m)"),
            "y = d·(1 − √(1 − 2·Md / (0,85·fcd·b·d²))) ;  x = y/0,8 ;  As = Md / (fyd·(d − y/2))",
            format!(
                "Md = {} kN·m ;  d = {} cm ;  fcd = {} MPa ;  fyd = {} MPa",
                fmt(md.abs()), fmt(d * 100.0), fmt(fcd), fmt(fyd)
            ),
            format!(
                "x/d = {} ;  As = {} cm²/m ;  As,mín = {} cm²/m → adotar {} cm²/m",
                fmt(fl.x_d), fmt(fl.as_calc), fmt(fl.as_min), fmt(fl.as_adotar())
            ),
        );
        r.verif(Verificacao::max(format!("{nome}: x/d"), fl.x_d, 0.45, ""));
        r.verif(Verificacao::info(
            format!("{nome}: As calc. / As mín."),
            fl.as_calc,
            &format!("cm²/m (mín. {})", fmt(fl.as_min)),
        ));
        let vr = vrd1(1.0, d, fl.as_adotar(), c.fck, fat.gamma_c);
        let rho = (fl.as_adotar() / 1e4 / d).min(0.02);
        r.norma(normas::NBR6118_CORTANTE);
        r.formula(
            format!("{nome} — cortante sem armadura transversal"),
            "VRd1 = τRd·k·(1,2 + 40·ρ1)·bw·d ;  τRd = 0,25·fctd ;  fctd = 0,21·fck^(2/3)/γc",
            format!(
                "fck = {} MPa ;  k = {} ;  ρ1 = {} ;  d = {} m",
                fmt(c.fck), fmt((1.6 - d).max(1.0)), format!("{:.4}", rho).replace('.', ","), fmt(d)
            ),
            format!("VRd1 = {} kN/m ;  Vd = {} kN/m", fmt(vr), fmt(vd.abs())),
        );
        r.verif(Verificacao::max(format!("{nome}: Vd ≤ VRd1"), vd.abs(), vr, "kN/m"));
        fl
    }

    /// Detalhamento do aço (tabela de armação) e itens construtivos.
    fn detalhamento(&self, r: &mut Resultado, fp: &Flexao, ft: &Flexao, fpo: &Flexao) -> DetalheFlexao {
        let c = &self.concreto;
        let (h, hs, bp, tb, tt) = (self.h, self.h_sapata, self.b_ponta, self.t_base, self.t_topo);
        let b = self.largura();
        let cob = c.cobrimento;
        let lb = |phi: f64, boa: bool| armadura::lb_basico(phi, c.fck, c.fyk, 1.4, 1.15, boa);

        r.titulo("Detalhamento construtivo — muro de flexão (concreto armado)");
        r.linha(format!(
            "Paramento: espessura {} m no topo e {} m na base ; altura livre {} m",
            fmt(tt), fmt(tb), fmt(h - hs)
        ));
        r.linha(format!(
            "Sapata: espessura {} m ; ponta {} m ; talão (calcanhar) {} m ; largura total {} m",
            fmt(hs), fmt(bp), fmt(self.b_talao), fmt(b)
        ));
        r.linha(format!("Concreto C{} ; aço CA-{} ; cobrimento {} cm", fmt(c.fck), fmt(c.fyk / 10.0), fmt(cob * 100.0)));
        r.norma(normas::NBR6118_ANCORAGEM);
        r.norma(normas::NBR6118_DISTRIBUICAO);

        let mut tab = Vec::new();
        let mut linha = |pos: &str, elem: &str, as_nec: f64, esp: f64, principal: bool, comp: f64, boa: bool| -> Armadura {
            let (phi, s, as_ef) = if principal {
                armadura::malha_principal(as_nec, esp)
            } else {
                armadura::malha_distribuicao(as_nec, esp)
            };
            let lbb = lb(phi, boa);
            let a = Armadura {
                pos: pos.into(),
                elemento: elem.into(),
                bitola_mm: phi,
                espac_cm: Some(s),
                quantidade: format!("{}/m", (100.0 / s).ceil() as i64),
                comprimento_m: comp,
                as_nec,
                as_ef,
                unidade: "cm²/m",
                lb_cm: armadura::lb_nec(lbb, phi, as_nec, as_ef),
            };
            tab.push(a.clone());
            a
        };

        // Parede
        let as_par = fp.as_adotar();
        let n1 = linha("N1", "Parede — vertical principal (face do aterro)", as_par, tb, true, h - hs - cob, true);
        let lb1 = lb(n1.bitola_mm, true);
        let l0t = armadura::traspasse(lb1, n1.lb_cm, n1.bitola_mm);
        let as_dist_par = armadura::as_distribuicao(as_par, c.fck, tb);
        linha("N2", "Parede — horizontal de distribuição", as_dist_par, tb, false, 0.0, true);
        linha("N3", "Parede — vertical construtiva (face externa)", as_dist_par, tb, false, h - 2.0 * cob, true);
        // Sapata
        let as_tal = ft.as_adotar();
        let lb_sup = lb(armadura::malha_principal(as_tal, hs).0, false);
        linha(
            "N4",
            "Talão — superior principal (tração)",
            as_tal,
            hs,
            true,
            self.b_talao + tb + lb_sup - cob,
            false,
        );
        let as_pon = fpo.as_adotar();
        linha("N5", "Ponta / sapata — inferior principal", as_pon, hs, true, b - 2.0 * cob, true);
        let as_dist_sap = armadura::as_distribuicao(as_tal.max(as_pon), c.fck, hs);
        linha("N6", "Sapata — distribuição (longitudinal)", as_dist_sap, hs, false, 0.0, true);
        // Arranque: esperas com a mesma bitola/espaçamento de N1
        let comp_n7 = (hs - cob) + n1.lb_cm / 100.0 + l0t / 100.0;
        drop(linha);
        tab.push(Armadura {
            pos: "N7".into(),
            elemento: "Arranque — esperas da parede na sapata".into(),
            comprimento_m: comp_n7,
            ..n1.clone()
        });

        r.formula(
            "Ancoragem e traspasse (CA-50 nervurado)",
            "lb = (Ø/4)·(fyd/fbd) ≥ 25Ø ;  fbd = 2,25·η2·fctd ;  lb,nec = lb·As,calc/As,ef ;  l0t = 2,0·lb,nec",
            format!(
                "N1: Ø{} ;  fck = {} MPa ;  boa aderência (η2 = 1,0) ;  As,calc/As,ef = {}/{}",
                armadura::fmt_bitola(n1.bitola_mm), fmt(c.fck), fmt(n1.as_nec), fmt(n1.as_ef)
            ),
            format!("lb = {} cm ;  lb,nec = {} cm ;  traspasse da espera l0t = {} cm", fmt(lb1), fmt(n1.lb_cm), fmt(l0t)),
        );
        r.formula(
            "Armadura de distribuição",
            "As,dist ≥ máx(0,2·As,princ ; 0,9 cm²/m ; 0,5·ρmín·b·h) ;  s ≤ 33 cm",
            format!("Parede: máx(0,2·{} ; 0,9 ; 0,5·ρmín·100·{})", fmt(as_par), fmt(tb * 100.0)),
            format!("Parede: {} cm²/m ;  sapata: {} cm²/m", fmt(as_dist_par), fmt(as_dist_sap)),
        );
        r.linha(format!(
            "Arranque: esperas N7 ancoradas na sapata com dobra de {} cm e projetadas {} cm acima da junta de concretagem; recomenda-se alternar o comprimento das esperas para emendar no máximo 50 % das barras na mesma seção.",
            fmt(n1.lb_cm), fmt(l0t)
        ));
        if self.dente && self.dente_h > 0.0 {
            r.linha(format!(
                "Dente de base {} × {} m sob a parede: prolongar as esperas N7 até o fundo do dente.",
                fmt(self.dente_b), fmt(self.dente_h)
            ));
        }
        r.linha("Juntas de dilatação a cada 15 a 20 m (critério usual), com mata-junta no tardoz; drenagem conforme a seção “Drenagem”.");
        r.armaduras.extend(tab);
        DetalheFlexao { cob, l0t: l0t / 100.0, lb_n7: n1.lb_cm / 100.0, rotulos: r.armaduras.iter().map(|a| a.descricao()).collect() }
    }

    /// Esquema das armaduras na seção (para tela, relatório e DXF). Os rótulos
    /// ficam numa coluna à direita do talão, com linha de chamada até a barra.
    fn desenhar_armaduras(&self, d: &mut Desenho, det: &DetalheFlexao) {
        let (h, hs, bp, tb, tt) = (self.h, self.h_sapata, self.b_ponta, self.t_base, self.t_topo);
        let b = self.largura();
        let c = det.cob;
        let xr = |y: f64| bp + tb - (tb - tt) * ((y - hs) / (h - hs)).clamp(0.0, 1.0);
        let y4 = hs - c;
        let x7 = xr(hs) - c - 0.025;
        // N1 — vertical principal, face do aterro
        d.linha(Estilo::Armadura, vec![[xr(hs) - c, hs], [xr(h - c) - c, h - c]]);
        // N7 — espera: dobra na sapata e traspasse acima da junta
        d.linha(Estilo::Armadura, vec![[x7 + det.lb_n7, c], [x7, c], [x7, hs + det.l0t]]);
        // N3 — construtiva na face externa
        d.linha(Estilo::Armadura, vec![[bp + c, c], [bp + c, h - c]]);
        // N4 — superior do talão
        d.linha(Estilo::Armadura, vec![[bp + c, y4], [b - c, y4], [b - c, c + 0.05]]);
        // N5 — inferior da sapata
        d.linha(Estilo::Armadura, vec![[c, hs - c - 0.05], [c, c], [b - c, c], [b - c, hs - c - 0.05]]);
        // N2 / N6 — distribuição (pontos)
        let mut y = hs + 0.2;
        while y < h - 0.1 {
            d.circulo(Estilo::ArmaduraSecao, [xr(y) - c - 0.02, y], 0.012);
            y += 0.2;
        }
        let mut x = c + 0.1;
        while x < b - c {
            d.circulo(Estilo::ArmaduraSecao, [x, y4 - 0.02], 0.012);
            d.circulo(Estilo::ArmaduraSecao, [x, c + 0.02], 0.012);
            x += 0.2;
        }
        // rótulos: (índice na tabela, ponto da barra)
        let alvos = [
            (0, [xr(0.75 * h) - c, 0.75 * h]),
            (1, [xr(0.55 * h) - c - 0.02, 0.55 * h]),
            (2, [bp + c, 0.40 * h]),
            (6, [x7, hs + 0.6 * det.l0t]),
            (3, [0.5 * (bp + tb + b), y4]),
            (5, [b - 0.3, y4 - 0.02]),
            (4, [0.7 * b, c]),
        ];
        let x_lab = b + 0.75 * h;
        for (k, (i, p)) in alvos.iter().enumerate() {
            let Some(rot) = det.rotulos.get(*i) else { continue };
            let y_lab = h * (0.95 - 0.09 * k as f64);
            let x_ini = x_lab - 0.06 * h * (rot.chars().count() as f64 / 14.0).max(1.0) * 1.6;
            d.linha(Estilo::Cota, vec![*p, [x_ini, y_lab]]);
            d.texto([x_lab, y_lab], rot.clone());
        }
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
        if self.dente && self.dente_h > 0.0 {
            let x0 = bp + tb / 2.0 - self.dente_b / 2.0;
            d.retangulo(Estilo::Concreto, x0, -self.dente_h, x0 + self.dente_b, 0.0);
        }
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tabela_de_armacao_do_muro_de_flexao() {
        let m = MuroFlexao::default();
        let r = m.calcular(&Fatores::default());
        let pos: Vec<&str> = r.armaduras.iter().map(|a| a.pos.as_str()).collect();
        assert_eq!(pos, ["N1", "N2", "N3", "N4", "N5", "N6", "N7"]);
        for a in &r.armaduras {
            assert!(a.as_ef + 1e-9 >= a.as_nec, "{}: As,ef {} < As,nec {}", a.pos, a.as_ef, a.as_nec);
            if let Some(s) = a.espac_cm {
                assert!((10.0..=33.0).contains(&s), "{} s = {s}", a.pos);
            }
        }
        // a espera (N7) tem a mesma bitola e espaçamento da armadura principal da parede (N1)
        let (n1, n7) = (&r.armaduras[0], &r.armaduras[6]);
        assert_eq!((n1.bitola_mm, n1.espac_cm), (n7.bitola_mm, n7.espac_cm));
        assert!(r.desenho.linhas.iter().any(|l| l.estilo == Estilo::Armadura));
    }

    #[test]
    fn dente_no_muro_de_flexao() {
        let mut m = MuroFlexao::default();
        let fs = |m: &MuroFlexao| {
            m.calcular(&Fatores::default()).verificacoes.iter().find(|v| v.nome.starts_with("FS deslizamento")).unwrap().valor
        };
        let sem = fs(&m);
        m.dente = true;
        assert!(fs(&m) > sem);
    }
}
