//! Estabilidade global (NBR 11682:2009, 7.3.7): equilíbrio limite pelo método de
//! Bishop simplificado, com superfícies de ruptura circulares. A contenção entra
//! como corpo rígido (muros), parede (cortinas e embutidas) ou com reforços
//! (tirantes e grampos). O fator de segurança mínimo é o da Tabela 3, conforme o
//! nível de segurança do projeto; superfícies que não interceptam os reforços
//! caracterizam a estabilidade externa (NBR 11682, 7.3.7.2).

use crate::fatores::Fatores;
use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::{fmt, Estilo, Resultado, Verificacao};
use crate::solo::{Perfil, GAMMA_W};

/// Propriedades de resistência e peso de um ponto do solo.
#[derive(Clone, Copy, Debug)]
pub struct Solo {
    pub gamma: f64,
    pub gamma_sat: f64,
    pub c: f64,
    pub phi: f64,
}

/// Reforço (tirante ou grampo): contribui com tração limitada pelo arrancamento
/// do trecho ancorado além da superfície e pela capacidade do aço.
#[derive(Clone, Debug)]
pub struct Reforco {
    /// Ponto de partida (cabeça), na face.
    pub p0: [f64; 2],
    /// Direção unitária, da face para dentro do maciço.
    pub dir: [f64; 2],
    pub comprimento: f64,
    /// Trecho livre (tirante); 0 para grampos.
    pub livre: f64,
    /// Capacidade de arrancamento por metro de trecho ancorado, por metro de contenção (kN/m por m).
    pub arr_por_m: f64,
    /// Capacidade do aço, por metro de contenção (kN/m).
    pub aco: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Circulo {
    pub xc: f64,
    pub yc: f64,
    pub r: f64,
    pub fs: f64,
    pub x_a: f64,
    pub x_b: f64,
    /// Número de reforços interceptados.
    pub interceptados: usize,
}

pub struct Modelo {
    /// Superfície superior do terreno e da contenção (polilinha em x crescente).
    pub sup: Vec<[f64; 2]>,
    /// Corpos rígidos (polígonos), com peso `gamma_muro`.
    pub muros: Vec<Vec<[f64; 2]>>,
    pub gamma_muro: f64,
    /// Zonas de reaterro compactado (polígonos).
    pub reaterro: Vec<Vec<[f64; 2]>>,
    pub solo_reaterro: Option<Solo>,
    /// Perfil natural; `y_ref` é a cota onde a profundidade z = 0.
    pub natural: Perfil,
    pub y_ref: f64,
    /// Sobrecarga (kPa) a partir de `x_q`.
    pub q: f64,
    pub x_q: f64,
    /// Nível d'água (cota) no lado retido (acima de `y_corte`) e no terreno natural.
    pub yw_ret: Option<f64>,
    pub yw_nat: Option<f64>,
    pub x_corte: f64,
    pub y_corte: f64,
    pub reforcos: Vec<Reforco>,
    /// Domínio horizontal e escala de busca.
    pub x_dom: (f64, f64),
    pub centros_x: (f64, f64),
    pub y_topo: f64,
    pub y_fundo: f64,
    pub esc: f64,
}

fn dentro(poly: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut d = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (pi, pj) = (poly[i], poly[j]);
        if (pi[1] > y) != (pj[1] > y) && x < (pj[0] - pi[0]) * (y - pi[1]) / (pj[1] - pi[1]) + pi[0] {
            d = !d;
        }
        j = i;
    }
    d
}

impl Modelo {
    fn superficie(&self, x: f64) -> Option<f64> {
        let mut y: Option<f64> = None;
        for w in self.sup.windows(2) {
            let (a, b) = (w[0], w[1]);
            if x < a[0].min(b[0]) - 1e-9 || x > a[0].max(b[0]) + 1e-9 {
                continue;
            }
            let v = if (b[0] - a[0]).abs() < 1e-9 { a[1].max(b[1]) } else { a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]) };
            y = Some(y.map_or(v, |u| u.max(v)));
        }
        y
    }

    fn solo_em(&self, x: f64, y: f64) -> Solo {
        if let Some(s) = self.solo_reaterro {
            if self.reaterro.iter().any(|p| dentro(p, x, y)) {
                return s;
            }
        }
        let c = self.natural.camada_em((self.y_ref - y).max(0.0));
        Solo { gamma: c.gamma, gamma_sat: c.gamma_sat, c: c.c, phi: c.phi }
    }

    fn yw(&self, x: f64, y: f64) -> Option<f64> {
        if x >= self.x_corte && y >= self.y_corte { self.yw_ret } else { self.yw_nat }
    }

    /// FS de Bishop simplificado para o círculo (`reforcos` = considerar a tração dos reforços).
    fn fs(&self, xc: f64, yc: f64, r: f64, reforcos: bool) -> Option<Circulo> {
        let ybot = |x: f64| -> Option<f64> {
            let d = r * r - (x - xc).powi(2);
            (d > 0.0).then(|| yc - d.sqrt())
        };
        let (x0, x1) = self.x_dom;
        let n = 160;
        let (mut xa, mut xb) = (None, None);
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            if let (Some(yb), Some(s)) = (ybot(x), self.superficie(x)) {
                if yb < s - 1e-6 {
                    if xa.is_none() {
                        xa = Some(x);
                    }
                    xb = Some(x);
                }
            }
        }
        let (xa, xb) = (xa?, xb?);
        // o arco precisa ter desenvolvimento e profundidade compatíveis com uma ruptura real:
        // superfícies rasas quase paralelas à face (corda curta, fatias finas) são artefatos
        if xa <= x0 + 1e-9 || xb >= x1 - 1e-9 || xb - xa < 0.5 * self.esc {
            return None;
        }
        let prof_max = (0..=n)
            .filter_map(|i| {
                let x = xa + (xb - xa) * i as f64 / n as f64;
                Some(self.superficie(x)? - ybot(x)?)
            })
            .fold(0.0, f64::max);
        if prof_max < 0.15 * self.esc {
            return None;
        }
        // a superfície não pode cortar os corpos rígidos
        for p in &self.muros {
            let (mn, mx) = p.iter().fold((f64::MAX, f64::MIN), |a, q| (a.0.min(q[0]), a.1.max(q[0])));
            for k in 0..=8 {
                let x = mn + (mx - mn) * k as f64 / 8.0;
                if let Some(yb) = ybot(x) {
                    if dentro(p, x, yb) {
                        return None;
                    }
                }
            }
        }
        let nf = 24;
        let b = (xb - xa) / nf as f64;
        let mut fat: Vec<(f64, f64, f64, f64, f64, f64)> = Vec::with_capacity(nf); // (sen, cos, w, c·b, tanφ, u·b)
        for k in 0..nf {
            let xm = xa + (k as f64 + 0.5) * b;
            let (Some(yb), Some(ytop)) = (ybot(xm), self.superficie(xm)) else { return None };
            let h = ytop - yb;
            if h <= 1e-6 {
                continue;
            }
            let sen = ((xm - xc) / r).clamp(-1.0, 1.0);
            let cos = (1.0 - sen * sen).sqrt();
            if cos < 0.35 {
                return None;
            }
            let base = self.solo_em(xm, yb + 1e-3);
            let nsub = ((h / 0.25).ceil() as usize).clamp(4, 40);
            let dy = h / nsub as f64;
            let mut w = 0.0;
            for j in 0..nsub {
                let ym = yb + (j as f64 + 0.5) * dy;
                let g = if self.muros.iter().any(|p| dentro(p, xm, ym)) {
                    self.gamma_muro
                } else {
                    let s = self.solo_em(xm, ym);
                    match self.yw(xm, ym) {
                        Some(yw) if ym < yw => s.gamma_sat,
                        _ => s.gamma,
                    }
                };
                w += g * dy * b;
            }
            if xm >= self.x_q {
                w += self.q * b;
            }
            let u = self.yw(xm, yb).map_or(0.0, |yw| GAMMA_W * (yw - yb).max(0.0));
            fat.push((sen, cos, w, base.c * b, base.phi.to_radians().tan(), u * b));
        }
        let den: f64 = fat.iter().map(|f| f.2 * f.0).sum();
        if den <= 1e-6 {
            return None;
        }
        // reforços: tração limitada pelo arrancamento além da superfície e pelo aço
        let (mut extra, mut interceptados) = (0.0, 0usize);
        if reforcos {
            for rf in &self.reforcos {
                let (px, py) = (rf.p0[0] - xc, rf.p0[1] - yc);
                let bq = px * rf.dir[0] + py * rf.dir[1];
                let cq = px * px + py * py - r * r;
                let disc = bq * bq - cq;
                if disc <= 0.0 {
                    continue;
                }
                let t_saida = -bq + disc.sqrt();
                let t_entrada = -bq - disc.sqrt();
                if t_saida <= 0.0 || t_entrada >= rf.comprimento {
                    continue;
                }
                let t = t_saida.min(rf.comprimento);
                interceptados += 1;
                let ancorado = (rf.comprimento - t.max(rf.livre)).max(0.0);
                let tr = (rf.arr_por_m * ancorado).min(rf.aco);
                let braco = (px * rf.dir[1] - py * rf.dir[0]).abs();
                extra += tr * braco / r;
            }
        }
        let mut fs = 1.5;
        for _ in 0..40 {
            let mut num = 0.0;
            for &(sen, cos, w, cb, tg, ub) in &fat {
                let m = cos + sen * tg / fs;
                if m < 0.2 {
                    return None;
                }
                num += (cb + (w - ub).max(0.0) * tg) / m;
            }
            let novo = (num + extra) / den;
            if !novo.is_finite() {
                return None;
            }
            if (novo - fs).abs() < 1e-4 {
                fs = novo;
                break;
            }
            fs = novo;
        }
        Some(Circulo { xc, yc, r, fs, x_a: xa, x_b: xb, interceptados })
    }

    /// Busca do círculo crítico (grade de centros e fundos, refinada em torno do mínimo).
    /// `externa`: só vale se a superfície não interceptar nenhum reforço.
    pub fn buscar(&self, externa: bool) -> Option<Circulo> {
        let melhor = |a: Option<Circulo>, b: Option<Circulo>| match (a, b) {
            (Some(x), Some(y)) => Some(if y.fs < x.fs { y } else { x }),
            (None, y) => y,
            (x, None) => x,
        };
        let avaliar = |xc: f64, yc: f64, ymin: f64| -> Option<Circulo> {
            let r = yc - ymin;
            if r < 0.5 {
                return None;
            }
            let c = self.fs(xc, yc, r, !self.reforcos.is_empty())?;
            if externa && c.interceptados > 0 { None } else { Some(c) }
        };
        let (cx0, cx1) = self.centros_x;
        let (ya, yb) = (self.y_topo + 0.1 * self.esc, self.y_topo + 2.5 * self.esc);
        let (nx, ny) = (12, 9);
        // fase 1: grade de centros e cotas de fundo; guarda os melhores candidatos
        let mut candidatos: Vec<Circulo> = Vec::new();
        for i in 0..nx {
            let xc = cx0 + (cx1 - cx0) * i as f64 / (nx - 1) as f64;
            for j in 0..ny {
                let yc = ya + (yb - ya) * j as f64 / (ny - 1) as f64;
                for d in [0.05, 0.2, 0.4, 0.7, 1.1, 1.7] {
                    if let Some(c) = avaliar(xc, yc, self.y_fundo - d * self.esc) {
                        candidatos.push(c);
                    }
                }
            }
        }
        // famílias clássicas semeadas explicitamente, para serem cobertas sempre (independe da
        // grade): círculos que entram no pé ou na face e saem no terrapleno atrás da crista
        let entradas: Vec<[f64; 2]> = {
            let mut e = vec![[self.x_corte.min(0.0), self.y_fundo]];
            for fr in [0.25, 0.5, 0.75] {
                let y = self.y_fundo + fr * (self.y_topo - self.y_fundo);
                // ponto da superfície (face) nessa cota: varre x até achar a face
                let mut xf = None;
                let mut x = self.x_dom.0;
                while x < self.x_dom.1 {
                    if let Some(sy) = self.superficie(x) {
                        if sy >= y {
                            xf = Some(x);
                            break;
                        }
                    }
                    x += 0.02 * self.esc;
                }
                if let Some(xf) = xf {
                    e.push([xf, y]);
                }
            }
            e
        };
        for ent in &entradas {
            for k in 1..=8 {
                let xs = ent[0] + (0.4 + 0.45 * k as f64) * self.esc;
                let Some(ys) = self.superficie(xs) else { continue };
                // círculo pelos dois pontos, centro na mediatriz, a várias distâncias
                let (mx, my) = ((ent[0] + xs) / 2.0, (ent[1] + ys) / 2.0);
                let (dx, dy) = (xs - ent[0], ys - ent[1]);
                let meia = 0.5 * (dx * dx + dy * dy).sqrt();
                let (nx, ny) = (-dy / (2.0 * meia), dx / (2.0 * meia));
                for t in [0.1, 0.4, 0.8, 1.3, 2.0, 3.0] {
                    let (xc, yc) = (mx + nx * t * meia, my + ny * t * meia);
                    let r = (meia * meia + (t * meia).powi(2)).sqrt();
                    if yc < ent[1] {
                        continue;
                    }
                    if let Some(c) = self.fs(xc, yc, r, !self.reforcos.is_empty()) {
                        if !(externa && c.interceptados > 0) {
                            candidatos.push(c);
                        }
                    }
                }
            }
        }
        candidatos.sort_by(|a, b| a.fs.partial_cmp(&b.fs).unwrap_or(std::cmp::Ordering::Equal));
        // fase 2: refina os 4 melhores candidatos de regiões distintas (a tração dos reforços
        // torna a superfície de FS irregular e o mínimo da grade pode não ser o global)
        let (dx0, dy0) = ((cx1 - cx0) / (nx - 1) as f64, (yb - ya) / (ny - 1) as f64);
        let mut escolhidos: Vec<Circulo> = Vec::new();
        for c in candidatos {
            if escolhidos.len() >= 6 {
                break;
            }
            if escolhidos.iter().all(|e| (e.xc - c.xc).abs() > 1.5 * dx0 || (e.yc - c.yc).abs() > 1.5 * dy0) {
                escolhidos.push(c);
            }
        }
        let mut b: Option<Circulo> = None;
        for inicio in escolhidos {
            let (mut dx, mut dy) = (dx0, dy0);
            let mut atual = inicio;
            for _ in 0..5 {
                dx *= 0.6;
                dy *= 0.6;
                let (ymin0, centro) = (atual.yc - atual.r, atual);
                for i in -2..=2 {
                    for j in -2..=2 {
                        for k in -2..=2 {
                            let c = avaliar(centro.xc + i as f64 * dx, centro.yc + j as f64 * dy, ymin0 + k as f64 * dy);
                            atual = melhor(Some(atual), c).unwrap();
                        }
                    }
                }
            }
            b = melhor(b, Some(atual));
        }
        b
    }
}

impl Projeto {
    /// Modelo geométrico da estabilidade global do tipo de contenção atual.
    fn modelo_global(&self, reforcos: &[Reforco], ficha_calc: Option<f64>) -> Modelo {
        let beta = self.geometria.beta.to_radians().tan();
        let q = self.geometria.q;
        let na_nat = self.sondagem.na();
        let na_ret = self.drenagem.na_efetivo(na_nat, self.altura_contida(), self.tipo);
        let natural = self.sondagem.perfil(None);
        let mut m = Modelo {
            sup: vec![],
            muros: vec![],
            gamma_muro: crate::materiais::CONCRETO_ARMADO,
            reaterro: vec![],
            solo_reaterro: None,
            natural,
            y_ref: 0.0,
            q,
            x_q: 0.0,
            yw_ret: None,
            yw_nat: None,
            x_corte: 0.0,
            y_corte: 0.0,
            reforcos: reforcos.to_vec(),
            x_dom: (0.0, 1.0),
            centros_x: (0.0, 1.0),
            y_topo: 0.0,
            y_fundo: 0.0,
            esc: 1.0,
        };
        match self.tipo {
            TipoContencao::Gravidade => {
                let g = self.preparar_gravidade();
                let gg = g.geo();
                let (ht, b, f, df) = (gg.htot, gg.base, g.f, g.df.max(0.0));
                let alc = 2.2 * ht + 2.0;
                let sup_y = |x: f64| ht + (x - gg.x_cr).max(0.0) * beta;
                m.gamma_muro = g.material.peso();
                m.muros = vec![vec![[0.0, 0.0], [b, 0.0], [b, f], [0.0, f]], vec![[gg.x_pf, f], [gg.x_pt, f], [gg.x_cr, ht], [gg.x_cl, ht]]];
                m.sup = vec![[-alc, df], [0.0, df], [0.0, f], [gg.x_pf, f], [gg.x_cl, ht], [gg.x_cr, ht], [b + alc, sup_y(b + alc)]];
                if df > f {
                    m.sup = vec![[-alc, df], [gg.x_pf - 0.0, df], [gg.x_pf, f.max(df)], [gg.x_cl, ht], [gg.x_cr, ht], [b + alc, sup_y(b + alc)]];
                }
                m.reaterro = vec![
                    vec![[b, 0.0], [b + alc, 0.0], [b + alc, sup_y(b + alc)], [b, sup_y(b)]],
                    vec![[gg.x_pt, f], [b, f], [b, sup_y(b)], [gg.x_cr, ht]],
                ];
                m.y_ref = ht;
                m.x_q = gg.x_cr;
                m.x_corte = b;
                m.y_corte = 0.0;
                m.x_dom = (-alc, b + alc);
                m.centros_x = (-0.3 * ht, b + 1.5 * ht);
                m.y_topo = ht;
                m.y_fundo = 0.0;
                m.esc = ht;
            }
            TipoContencao::Flexao => {
                let fx = self.preparar_flexao();
                let (h, hs, bp, tb, tt) = (fx.h, fx.h_sapata, fx.b_ponta, fx.t_base, fx.t_topo);
                let b = bp + tb + fx.b_talao;
                let df = fx.df.max(0.0);
                let alc = 2.2 * h + 2.0;
                let sup_y = |x: f64| h + (x - bp - tt).max(0.0) * beta;
                m.muros = vec![vec![[0.0, 0.0], [b, 0.0], [b, hs], [0.0, hs]], vec![[bp, hs], [bp + tb, hs], [bp + tt, h], [bp, h]]];
                m.sup = if df > hs {
                    vec![[-alc, df], [bp, df], [bp, h], [bp + tt, h], [b + alc, sup_y(b + alc)]]
                } else {
                    vec![[-alc, df], [0.0, df], [0.0, hs], [bp, hs], [bp, h], [bp + tt, h], [b + alc, sup_y(b + alc)]]
                };
                m.reaterro = vec![
                    vec![[b, 0.0], [b + alc, 0.0], [b + alc, sup_y(b + alc)], [b, sup_y(b)]],
                    vec![[bp + tb, hs], [b, hs], [b, sup_y(b)], [bp + tt, h]],
                ];
                m.y_ref = h;
                m.x_q = bp + tt;
                m.x_corte = b;
                m.y_corte = 0.0;
                m.x_dom = (-alc, b + alc);
                m.centros_x = (-0.3 * h, b + 1.5 * h);
                m.y_topo = h;
                m.y_fundo = 0.0;
                m.esc = h;
            }
            TipoContencao::Atirantada | TipoContencao::Prancha | TipoContencao::Estacas => {
                let h = self.geometria.h;
                let (esp, ficha) = match self.tipo {
                    TipoContencao::Atirantada => (self.atirantada.espessura, self.geometria.engaste),
                    TipoContencao::Prancha => (0.02_f64.max(0.012 * h.max(4.0)), ficha_calc.unwrap_or(0.5 * h)),
                    _ => (self.estacas.diametro, ficha_calc.unwrap_or(0.5 * h)),
                };
                let lt = h + ficha;
                let alc = 2.5 * lt + 2.0;
                let sup_y = |x: f64| x.max(0.0) * beta;
                let (xa, xb) = if self.tipo == TipoContencao::Atirantada { (-esp, 0.0) } else { (-esp / 2.0, esp / 2.0) };
                m.muros = vec![vec![[xa, -lt], [xb, -lt], [xb, 0.0], [xa, 0.0]]];
                m.sup = vec![[-alc, -h], [xa, -h], [xa, 0.0], [xb, 0.0], [alc, sup_y(alc)]];
                m.y_ref = 0.0;
                m.x_q = xb;
                m.x_corte = xb;
                m.y_corte = -h;
                m.x_dom = (-alc, alc);
                m.centros_x = (-0.5 * h, 1.6 * lt);
                m.y_topo = 0.0;
                m.y_fundo = -lt;
                m.esc = lt;
            }
            TipoContencao::Grampeado => {
                let g = self.preparar_grampeado();
                let h = g.h;
                let x_ft = h * g.incl_face.to_radians().tan();
                let alc = 2.5 * h + 2.0;
                let sup_y = |x: f64| h + (x - x_ft).max(0.0) * beta;
                m.sup = vec![[-alc, 0.0], [0.0, 0.0], [x_ft, h], [alc + x_ft, sup_y(alc + x_ft)]];
                m.y_ref = h;
                m.x_q = x_ft;
                m.x_corte = 0.0;
                m.y_corte = 0.0;
                m.x_dom = (-alc, alc + x_ft);
                m.centros_x = (-0.3 * h, 1.6 * h + g.comprimento);
                m.y_topo = h;
                m.y_fundo = 0.0;
                m.esc = h;
            }
        }
        // água: lado retido (condição de drenagem) e terreno natural (sondagem)
        m.yw_ret = na_ret.map(|d| m.y_ref - d);
        m.yw_nat = na_nat.map(|d| m.y_ref - d);
        if self.tipo == TipoContencao::Grampeado {
            let hw = self.preparar_grampeado().hw;
            m.yw_ret = if hw > 0.0 { Some(hw) } else { None };
        }
        if self.reaterro.usar && matches!(self.tipo, TipoContencao::Gravidade | TipoContencao::Flexao) {
            let (g, gs, c, phi) = self.reaterro.parametros(&self.sondagem);
            m.solo_reaterro = Some(Solo { gamma: g, gamma_sat: gs, c, phi });
        } else {
            m.reaterro.clear();
        }
        m
    }

    /// Verifica a estabilidade global e a registra no memorial, no desenho e nas verificações.
    pub fn estabilidade_global(&self, r: &mut Resultado) {
        let f: &Fatores = &self.fatores;
        let ficha = r.verificacoes.iter().find(|v| v.nome.starts_with("Ficha de projeto")).map(|v| v.valor);
        let reforcos = r.reforcos.clone();
        let m = self.modelo_global(&reforcos, ficha);
        let fs_min = f.fs_global();
        r.titulo("Estabilidade global — equilíbrio limite (NBR 11682)");
        r.norma(crate::normas::NBR11682_TAB3);
        let com_reforco = !reforcos.is_empty();
        let Some(glob) = m.buscar(false) else {
            r.aviso("Estabilidade global: nenhuma superfície circular válida encontrada para a geometria informada.");
            return;
        };
        r.linha(format!(
            "Método de Bishop simplificado, superfícies circulares, 24 fatias; contenção como {} ; sobrecarga q = {} kPa ; água: {}.",
            match self.tipo {
                TipoContencao::Gravidade | TipoContencao::Flexao => "corpo rígido (superfícies passam abaixo da base)",
                TipoContencao::Grampeado => "maciço reforçado por grampos (tração limitada ao arrancamento e ao aço)",
                TipoContencao::Atirantada => "parede com tirantes (tração limitada ao bulbo e ao aço)",
                _ => "parede embutida (superfícies passam abaixo da ponta)",
            },
            fmt(self.geometria.q),
            match (m.yw_ret, m.yw_nat) {
                (None, None) => "sem nível d'água".to_string(),
                (a, b) => format!("tardoz {} ; terreno natural {}", a.map_or("seco".to_string(), |y| format!("cota {} m", fmt(y))), b.map_or("seco".to_string(), |y| format!("cota {} m", fmt(y)))),
            }
        ));
        r.formula(
            "Bishop simplificado",
            "FS = Σ[(c'·b + (W − u·b)·tg φ') / mα] / ΣW·sen α ;  mα = cos α + sen α·tg φ'/FS  (+ ΣT·d/R com reforços)",
            format!("centro = ({} ; {}) m ;  R = {} m ;  arco de x = {} a {} m", fmt(glob.xc), fmt(glob.yc), fmt(glob.r), fmt(glob.x_a), fmt(glob.x_b)),
            format!("FS crítico = {}  (mínimo {} — NBR 11682 Tab. 3)", fmt(glob.fs), fmt(fs_min)),
        );
        // desenho: arco crítico
        let pts: Vec<[f64; 2]> = (0..=40)
            .filter_map(|i| {
                let x = glob.x_a + (glob.x_b - glob.x_a) * i as f64 / 40.0;
                let d = glob.r * glob.r - (x - glob.xc).powi(2);
                (d > 0.0).then(|| [x, glob.yc - d.sqrt()])
            })
            .collect();
        if pts.len() > 2 {
            let meio = pts[pts.len() / 2];
            r.desenho.tracejada(Estilo::Ruptura, pts);
            r.desenho.texto([meio[0], meio[1] - 0.06 * m.esc], format!("FS global = {}", fmt(glob.fs)));
        }
        if com_reforco {
            let ext = m.buscar(true);
            match ext {
                Some(e) => {
                    r.linha(format!(
                        "Estabilidade externa (superfícies que não interceptam os reforços): FS = {} ; interna/mista (com reforços): FS = {}.",
                        fmt(e.fs), fmt(glob.fs)
                    ));
                    r.verif(Verificacao::min("FS estabilidade externa — Bishop (NBR 11682 Tab. 3)", e.fs, fs_min, ""));
                }
                None => r.aviso("Estabilidade externa: nenhuma superfície circular que evite os reforços foi encontrada no domínio de busca."),
            }
            r.verif(Verificacao::min("FS estabilidade global com reforços — Bishop (NBR 11682 Tab. 3)", glob.fs, fs_min, ""));
        } else {
            r.verif(Verificacao::min("FS estabilidade global — Bishop (NBR 11682 Tab. 3)", glob.fs, fs_min, ""));
        }
        r.aviso("Estabilidade global por Bishop simplificado (superfícies circulares): casos com estratigrafia complexa, sismo ou carga em faixa devem ser conferidos com métodos rigorosos (Morgenstern-Price/Spencer).");
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::solo::Camada;

    fn talude_simples(phi: f64, c: f64, beta_graus: f64) -> Modelo {
        // talude infinito aproximado: terreno inclinado sem contenção, solo homogêneo
        let tb = beta_graus.to_radians().tan();
        let mut natural = Perfil::simples(18.0, phi, c);
        natural.camadas = vec![Camada::nova("solo", 100.0, 18.0, 20.0, phi, c)];
        let h = 6.0;
        Modelo {
            sup: vec![[-30.0, 0.0], [0.0, 0.0], [h / tb, h], [h / tb + 30.0, h]],
            muros: vec![],
            gamma_muro: 25.0,
            reaterro: vec![],
            solo_reaterro: None,
            natural,
            y_ref: h,
            q: 0.0,
            x_q: 1e9,
            yw_ret: None,
            yw_nat: None,
            x_corte: 0.0,
            y_corte: 0.0,
            reforcos: vec![],
            x_dom: (-30.0, h / tb + 30.0),
            centros_x: (-2.0, h / tb + 6.0),
            y_topo: h,
            y_fundo: 0.0,
            esc: h,
        }
    }

    /// Talude 1:1,5 (V:H) em solo c' = 10 kPa, φ' = 30°, H = 6 m: o FS de Bishop deve ficar na
    /// faixa dos ábacos de Taylor/Janbu (≈ 1,3 a 1,9) e crescer com a coesão e o atrito.
    #[test]
    fn bishop_em_talude_homogeneo() {
        let base = talude_simples(30.0, 10.0, 33.7).buscar(false).expect("círculo crítico").fs;
        assert!((1.2..2.2).contains(&base), "FS = {base}");
        let mais_c = talude_simples(30.0, 20.0, 33.7).buscar(false).unwrap().fs;
        let mais_phi = talude_simples(35.0, 10.0, 33.7).buscar(false).unwrap().fs;
        assert!(mais_c > base && mais_phi > base);
        // talude mais íngreme é menos seguro
        assert!(talude_simples(30.0, 10.0, 45.0).buscar(false).unwrap().fs < base);
        // com a água na superfície, cai
        let mut seco = talude_simples(30.0, 5.0, 33.7);
        let fs_seco = seco.buscar(false).unwrap().fs;
        seco.yw_nat = Some(6.0);
        seco.yw_ret = Some(6.0);
        assert!(seco.buscar(false).unwrap().fs < fs_seco);
    }

    /// Talude de areia seca (c' = 0): FS ≈ tg φ' / tg β (talude infinito), com Bishop próximo.
    #[test]
    fn areia_seca_confere_com_talude_infinito() {
        let (phi, beta): (f64, f64) = (35.0, 25.0);
        let esperado = phi.to_radians().tan() / beta.to_radians().tan();
        let fs = talude_simples(phi, 0.0, beta).buscar(false).unwrap().fs;
        assert!((fs - esperado).abs() / esperado < 0.12, "FS = {fs} ; esperado ≈ {esperado}");
    }
}
