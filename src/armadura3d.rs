//! Armaduras no modelo 3D: as barras da tabela de armação de cada tipo,
//! posicionadas com o cobrimento, o espaçamento e as dobras do detalhamento
//! (NBR 6118). Cada posição vira uma peça (malha de tubos) para o visualizador
//! e para o IFC (IfcReinforcingBar).
//!
//! Coordenadas como em `modelo3d`: x na seção (para o maciço), y vertical, z ao
//! longo da contenção.

use crate::armadura::Armadura;
use crate::modelo3d::{Classe, Forma, Material, Peca, Tri};
use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::Resultado;

type P3 = [f64; 3];

/// Tubo ao longo de uma polilinha (dobras como vértices).
fn tubo(pts: &[P3], phi_mm: f64, saida: &mut Vec<Tri>) {
    let d = (phi_mm / 1000.0).max(0.004);
    for s in pts.windows(2) {
        saida.extend(Forma::Barra { p0: s[0], p1: s[1], diam: d }.triangulos(6));
    }
}

/// Posições ao longo de [a, b] com espaçamento s (primeira e última no cobrimento).
fn ao_longo(a: f64, b: f64, s: f64) -> Vec<f64> {
    if b <= a {
        return vec![];
    }
    let n = ((b - a) / s.max(0.03)).floor() as usize;
    let passo = if n > 0 { (b - a) / n as f64 } else { 0.0 };
    (0..=n).map(|k| a + k as f64 * passo).collect()
}

/// Primeiro número inteiro do texto de quantidade ("4 por pilar" → 4).
fn qtd(a: &Armadura) -> usize {
    a.quantidade.split(|c: char| !c.is_ascii_digit()).find(|t| !t.is_empty()).and_then(|t| t.parse().ok()).unwrap_or(2).max(1)
}

struct Gerador<'a> {
    tab: &'a [Armadura],
    pecas: Vec<Peca>,
}

impl<'a> Gerador<'a> {
    fn pos(&self, p: &str) -> Option<&'a Armadura> {
        self.tab.iter().find(|a| a.pos == p)
    }
    fn add(&mut self, a: &Armadura, tris: Vec<Tri>) {
        if tris.is_empty() {
            return;
        }
        self.pecas.push(Peca { nome: format!("{} — {}", a.descricao(), a.elemento), classe: Classe::Armadura, material: Material::Armadura, forma: Forma::Malha(tris) });
    }
    /// Barras repetidas ao longo de z: `perfil` dá a polilinha (x, y) da barra.
    fn repetir_z(&mut self, a: &Armadura, zs: &[f64], perfil: &[[f64; 2]]) {
        let mut t = Vec::new();
        for &z in zs {
            let pts: Vec<P3> = perfil.iter().map(|q| [q[0], q[1], z]).collect();
            tubo(&pts, a.bitola_mm, &mut t);
        }
        self.add(a, t);
    }
    /// Barras longitudinais (ao longo de z) nas posições (x, y) dadas.
    fn longitudinais(&mut self, a: &Armadura, xy: &[[f64; 2]], z0: f64, z1: f64) {
        let mut t = Vec::new();
        for q in xy {
            tubo(&[[q[0], q[1], z0], [q[0], q[1], z1]], a.bitola_mm, &mut t);
        }
        self.add(a, t);
    }
}

/// Estribo retangular fechado no plano dado (cantos a, b, c, d) com ganchos a 135°.
fn estribo(cantos: [P3; 4], phi: f64, t: &mut Vec<Tri>) {
    let g = (crate::armadura::gancho_estribo_cm(phi) / 100.0) * 0.7;
    let [a, b, c, d] = cantos;
    // ganchos dobrados para dentro a partir do canto a
    let dentro = |p: P3, q: P3| [p[0] + (q[0] - p[0]) * 0.0 + (c[0] - a[0]).signum() * g * 0.5, p[1] + (c[1] - a[1]).signum() * g * 0.5, p[2] + (c[2] - a[2]).signum() * g * 0.5];
    tubo(&[dentro(a, b), a, b, c, d, a, dentro(a, d)], phi, t);
}

/// Hélice/estribos circulares de estaca: aros a cada `passo` (polígono de 10 lados).
fn aros(xc: f64, zc: f64, r: f64, y0: f64, y1: f64, passo: f64, phi: f64, t: &mut Vec<Tri>) {
    for y in ao_longo(y0, y1, passo) {
        let pts: Vec<P3> = (0..=10).map(|k| {
            let a = std::f64::consts::TAU * k as f64 / 10.0;
            [xc + r * a.cos(), y, zc + r * a.sin()]
        }).collect();
        tubo(&pts, phi, t);
    }
}

/// Gera as armaduras do projeto num trecho de comprimento `l`.
pub fn gerar(p: &Projeto, r: &Resultado, l: f64) -> Vec<Peca> {
    let mut g = Gerador { tab: &r.armaduras, pecas: Vec::new() };
    match p.tipo {
        TipoContencao::Flexao => flexao(p, r, l, &mut g),
        TipoContencao::Atirantada => atirantada(p, l, &mut g),
        TipoContencao::Estacas => estacas_parede(p, r, l, &mut g),
        TipoContencao::Grampeado => grampeado(p, l, &mut g),
        _ => {}
    }
    g.pecas
}

fn flexao(p: &Projeto, _r: &Resultado, l: f64, g: &mut Gerador) {
    let m = p.preparar_flexao();
    let c = m.concreto.cobrimento;
    let (bp, tb, tt, hs, h) = (m.b_ponta, m.t_base, m.t_topo, m.h_sapata, m.h);
    let b = bp + tb + m.b_talao;
    let misto = m.sistema == crate::tipos::misto::Sistema::Misto;
    if !misto {
        // parede (face do aterro inclinada) e sapata — posições N1 a N7
        let xr = |y: f64| bp + tb - (tb - tt) * ((y - hs) / (h - hs)).clamp(0.0, 1.0);
        let phi = |a: Option<&Armadura>| a.map(|a| a.bitola_mm / 1000.0).unwrap_or(0.01);
        let (f1, f4, f5) = (phi(g.pos("N1")), phi(g.pos("N4")), phi(g.pos("N5")));
        if let Some(a) = g.pos("N1") {
            let zs = ao_longo(c, l - c, a.espac_cm.unwrap_or(15.0) / 100.0);
            let x = |y: f64| xr(y) - c - f1 / 2.0;
            g.repetir_z(a, &zs, &[[x(hs), hs], [x(h - c), h - c]]);
        }
        if let Some(a) = g.pos("N3") {
            let zs = ao_longo(c, l - c, a.espac_cm.unwrap_or(20.0) / 100.0);
            let x = bp + c + a.bitola_mm / 2000.0;
            g.repetir_z(a, &zs, &[[x, c + f5], [x, h - c]]);
        }
        if let Some(a) = g.pos("N2") {
            // horizontais por dentro de N1 (face do aterro) e de N3 (face externa)
            let s = a.espac_cm.unwrap_or(20.0) / 100.0;
            let f2 = a.bitola_mm / 1000.0;
            let mut xy = Vec::new();
            for y in ao_longo(hs + 0.05, h - c, s) {
                xy.push([xr(y) - c - f1 - f2 / 2.0, y]);
                xy.push([bp + c + f2 / 2.0 + 0.012, y]);
            }
            g.longitudinais(a, &xy, c, l - c);
        }
        if let Some(a) = g.pos("N4") {
            let zs = ao_longo(c, l - c, a.espac_cm.unwrap_or(15.0) / 100.0);
            let y = hs - c - f4 / 2.0;
            g.repetir_z(a, &zs, &[[bp + c, y], [b - c, y], [b - c, c + f5 + 0.05]]);
        }
        if let Some(a) = g.pos("N5") {
            let zs = ao_longo(c, l - c, a.espac_cm.unwrap_or(15.0) / 100.0);
            let y = c + f5 / 2.0;
            let perna = hs - c - 0.05;
            g.repetir_z(a, &zs, &[[c, perna], [c, y], [b - c, y], [b - c, perna]]);
        }
        if let Some(a) = g.pos("N6") {
            let s = a.espac_cm.unwrap_or(20.0) / 100.0;
            let f6 = a.bitola_mm / 1000.0;
            let mut xy = Vec::new();
            for x in ao_longo(c + 0.05, b - c - 0.05, s) {
                xy.push([x, c + f5 + f6 / 2.0]);
                xy.push([x, hs - c - f4 - f6 / 2.0]);
            }
            g.longitudinais(a, &xy, c, l - c);
        }
        if let (Some(a), Some(n1)) = (g.pos("N7"), g.pos("N1")) {
            // esperas: dobra no fundo da sapata e traspasse acima da junta, ao lado de N1
            let zs: Vec<f64> = ao_longo(c, l - c, n1.espac_cm.unwrap_or(15.0) / 100.0).into_iter().map(|z| (z + 0.03).min(l - c)).collect();
            let (longa, curta) = match a.forma {
                crate::armadura::Forma::EmL { longa, curta } => (longa, curta),
                _ => (hs + 0.6, 0.3),
            };
            let x = xr(hs) - c - f1 / 2.0;
            let y0 = c + f5;
            g.repetir_z(a, &zs, &[[x - curta, y0], [x, y0], [x, (y0 + longa).min(h - c)]]);
        }
        return;
    }

    // ---------- muro misto ----------
    let mi = &m.misto;
    let est = mi.fundacao_estacas();
    let (e_est, lb_bl, bb_bl, hb_bl) = mi.geometria_bloco();
    let pilares = crate::modelo3d::posicoes_pub(l, mi.espac_pilar);
    let y_base_pilar = if est { hs - hb_bl + c + 0.05 } else { c + 0.05 };
    // pilares: N1 na face do aterro, 2 porta-estribos na face externa, estribos N3
    if let (Some(n1), Some(n3)) = (g.pos("N1"), g.pos("N3")) {
        let n = qtd(n1);
        let (f, ft) = (n1.bitola_mm / 1000.0, n3.bitola_mm / 1000.0);
        let mut t_l = Vec::new();
        let mut t_e = Vec::new();
        for &zc in &pilares {
            let zmin = zc - mi.pilar_b / 2.0 + c + ft + f / 2.0;
            let zmax = zc + mi.pilar_b / 2.0 - c - ft - f / 2.0;
            let x_at = bp + mi.pilar_h - c - ft - f / 2.0;
            let x_ex = bp + c + ft + 0.005;
            for k in 0..n {
                let z = if n > 1 { zmin + (zmax - zmin) * k as f64 / (n - 1) as f64 } else { zc };
                // ancoragem no bloco/sapata com dobra de 90° (12Ø) para dentro
                tubo(&[[x_at - 12.0 * f, y_base_pilar, z], [x_at, y_base_pilar, z], [x_at, h - c, z]], n1.bitola_mm, &mut t_l);
            }
            for z in [zmin, zmax] {
                tubo(&[[x_ex, hs + 0.02, z], [x_ex, h - c, z]], 10.0, &mut t_l);
            }
            for y in ao_longo(hs + 0.05, h - c - 0.03, n3.espac_cm.unwrap_or(15.0) / 100.0) {
                let (xa, xb) = (bp + c, bp + mi.pilar_h - c);
                let (za, zb) = (zc - mi.pilar_b / 2.0 + c, zc + mi.pilar_b / 2.0 - c);
                estribo([[xa, y, za], [xb, y, za], [xb, y, zb], [xa, y, zb]], n3.bitola_mm, &mut t_e);
            }
        }
        g.add(n1, t_l);
        g.add(n3, t_e);
    }
    // painel: barras verticais nos furos grauteados (no centro do bloco), entre pilares
    if let Some(n2) = g.pos("N2") {
        let s = n2.espac_cm.unwrap_or(40.0) / 100.0;
        let mut t = Vec::new();
        let mut faces = vec![0.0];
        for &z in &pilares {
            faces.push(z - mi.pilar_b / 2.0);
            faces.push(z + mi.pilar_b / 2.0);
        }
        faces.push(l);
        for par in faces.chunks(2) {
            if par.len() == 2 && par[1] - par[0] > 0.3 {
                for z in ao_longo(par[0] + 0.10, par[1] - 0.10, s) {
                    tubo(&[[bp + mi.bloco / 2.0, hs + 0.02, z], [bp + mi.bloco / 2.0, h - c, z]], n2.bitola_mm, &mut t);
                }
            }
        }
        g.add(n2, t);
    }
    // vigas intermediárias: n barras por face ao longo do muro + estribos N12
    let vigas = p.niveis_vigas_misto();
    if let Some(n8) = g.pos("N8") {
        let n = qtd(n8);
        let f = n8.bitola_mm / 1000.0;
        let mut xy = Vec::new();
        for &yv in &vigas {
            let (y0, y1) = (yv - mi.viga_h / 2.0 + c + 0.008 + f / 2.0, yv + mi.viga_h / 2.0 - c - 0.008 - f / 2.0);
            for k in 0..n {
                let y = if n > 1 { y0 + (y1 - y0) * k as f64 / (n - 1) as f64 } else { yv };
                xy.push([bp + c + 0.008 + f / 2.0, y]);
                xy.push([bp + mi.viga_b - c - 0.008 - f / 2.0, y]);
            }
        }
        g.longitudinais(n8, &xy, c, l - c);
    }
    if let Some(n12) = g.pos("N12") {
        let mut t = Vec::new();
        for &yv in &vigas {
            for z in ao_longo(c, l - c, n12.espac_cm.unwrap_or(15.0) / 100.0) {
                if pilares.iter().any(|zp| (z - zp).abs() < mi.pilar_b / 2.0) {
                    continue; // dentro do pilar valem os estribos do pilar
                }
                let (xa, xb) = (bp + c, bp + mi.viga_b - c);
                let (ya, yb) = (yv - mi.viga_h / 2.0 + c, yv + mi.viga_h / 2.0 - c);
                estribo([[xa, ya, z], [xb, ya, z], [xb, yb, z], [xa, yb, z]], n12.bitola_mm, &mut t);
            }
        }
        g.add(n12, t);
    }
    if !est {
        return;
    }
    // baldrame: barras por face + estribos N13
    if let Some(n9) = g.pos("N9") {
        let n = qtd(n9);
        let f = n9.bitola_mm / 1000.0;
        let mut xy = Vec::new();
        for k in 0..n {
            let x = if n > 1 { bp + c + 0.008 + f / 2.0 + (mi.baldrame_b - 2.0 * c - 0.016 - f) * k as f64 / (n - 1) as f64 } else { bp + mi.baldrame_b / 2.0 };
            xy.push([x, c + 0.008 + f / 2.0]);
            xy.push([x, hs - c - 0.008 - f / 2.0]);
        }
        g.longitudinais(n9, &xy, c, l - c);
    }
    if let Some(n13) = g.pos("N13") {
        let mut t = Vec::new();
        for z in ao_longo(c, l - c, n13.espac_cm.unwrap_or(15.0) / 100.0) {
            if pilares.iter().any(|zp| (z - zp).abs() < bb_bl / 2.0) {
                continue; // dentro do bloco
            }
            let (xa, xb) = (bp + c, bp + mi.baldrame_b - c);
            estribo([[xa, c, z], [xb, c, z], [xb, hs - c, z], [xa, hs - c, z]], n13.bitola_mm, &mut t);
        }
        g.add(n13, t);
    }
    let xc = bp + mi.pilar_h / 2.0;
    let y_bl = hs - hb_bl;
    // bloco: tirante inferior sobre as estacas (em U), concentrado na faixa das estacas
    if let Some(n11) = g.pos("N11") {
        let n = qtd(n11);
        let f = n11.bitola_mm / 1000.0;
        let mut t = Vec::new();
        for &zc in &pilares {
            let faixa = (1.2 * mi.diam_estaca).min(bb_bl - 2.0 * c) - f;
            for k in 0..n {
                let z = if n > 1 { zc - faixa / 2.0 + faixa * k as f64 / (n - 1) as f64 } else { zc };
                let (xa, xb) = (xc - lb_bl / 2.0 + c, xc + lb_bl / 2.0 - c);
                let y = y_bl + c + 0.05 + f / 2.0; // acima do topo das estacas (embutidas 5 cm)
                let perna = (hb_bl - 2.0 * c).min(12.0 * f + 0.1);
                tubo(&[[xa, y + perna, z], [xa, y, z], [xb, y, z], [xb, y + perna, z]], n11.bitola_mm, &mut t);
            }
        }
        g.add(n11, t);
    }
    // estacas: 5 longitudinais no trecho armado (ancoradas no bloco) + hélice N14
    if let Some(n10) = g.pos("N10") {
        let f = n10.bitola_mm / 1000.0;
        let n = qtd(n10);
        let l_arm = (n10.comprimento_m - n10.lb_cm / 100.0).max(1.0);
        let rr = mi.diam_estaca / 2.0 - c - 0.0063 - f / 2.0;
        let mut t = Vec::new();
        let mut th = Vec::new();
        for &zc in &pilares {
            for s in [-1.0, 1.0] {
                let xe = xc + s * e_est / 2.0;
                for k in 0..n {
                    let a = std::f64::consts::TAU * k as f64 / n as f64;
                    let (x, z) = (xe + rr * a.cos(), zc + rr * a.sin());
                    tubo(&[[x, y_bl - l_arm, z], [x, y_bl + hb_bl - c - 0.05, z]], n10.bitola_mm, &mut t);
                }
                if let Some(n14) = g.pos("N14") {
                    aros(xe, zc, rr + f / 2.0 + 0.003, y_bl - l_arm, y_bl - 0.05, n14.espac_cm.unwrap_or(20.0) / 100.0, n14.bitola_mm, &mut th);
                }
            }
        }
        g.add(n10, t);
        if let Some(n14) = g.pos("N14") {
            g.add(n14, th);
        }
    }
}

fn atirantada(p: &Projeto, l: f64, g: &mut Gerador) {
    let c = p.atirantada.concreto.cobrimento;
    let cc = p.preparar_atirantada();
    let (esp, h, ficha) = (cc.espessura, cc.h, cc.ficha);
    let (y0, y1) = (-(h + ficha) + c, -c);
    if let Some(n1) = g.pos("N1") {
        let f = n1.bitola_mm / 1000.0;
        let zs = ao_longo(c, l - c, n1.espac_cm.unwrap_or(15.0) / 100.0);
        let mut t = Vec::new();
        for x in [-esp + c + f / 2.0, -c - f / 2.0] {
            for &z in &zs {
                tubo(&[[x, y0, z], [x, y1, z]], n1.bitola_mm, &mut t);
            }
        }
        g.add(n1, t);
        if let Some(n2) = g.pos("N2") {
            let f2 = n2.bitola_mm / 1000.0;
            let mut xy = Vec::new();
            for y in ao_longo(y0 + 0.02, y1 - 0.02, n2.espac_cm.unwrap_or(15.0) / 100.0) {
                xy.push([-esp + c + f + f2 / 2.0, y]);
                xy.push([-c - f - f2 / 2.0, y]);
            }
            g.longitudinais(n2, &xy, c, l - c);
        }
    }
}

fn estacas_parede(p: &Projeto, r: &Resultado, l: f64, g: &mut Gerador) {
    let e = p.preparar_estacas();
    let c = e.concreto.cobrimento;
    let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(1.5 * p.geometria.h);
    if let Some(n1) = g.pos("N1") {
        let n = qtd(n1);
        let f = n1.bitola_mm / 1000.0;
        let rr = e.diametro / 2.0 - c - 0.0063 - f / 2.0;
        // barras adicionais do corte (N1b): trecho "de z = a a b m" (z a partir do topo)
        let adicional = g.pos("N1b").map(|a| {
            let nums: Vec<f64> = a
                .elemento
                .split(|ch: char| !(ch.is_ascii_digit() || ch == ','))
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.replace(',', ".").parse().ok())
                .collect();
            (a, qtd(a), nums.first().copied().unwrap_or(0.0), nums.get(1).copied().unwrap_or(lt))
        });
        let (mut t, mut tb, mut th) = (Vec::new(), Vec::new(), Vec::new());
        for zc in crate::modelo3d::posicoes_pub(l, e.espacamento) {
            for k in 0..n {
                let a = std::f64::consts::TAU * k as f64 / n as f64;
                tubo(&[[rr * a.cos(), -lt + c, zc + rr * a.sin()], [rr * a.cos(), -c, zc + rr * a.sin()]], n1.bitola_mm, &mut t);
            }
            if let Some((_, nb, z1, z2)) = adicional {
                for k in 0..nb {
                    // entre as contínuas, na face mais tracionada (lado do aterro, +x)
                    let a = std::f64::consts::TAU * (k as f64 + 0.5) / n as f64 - std::f64::consts::PI * (nb as f64 - 1.0) / n as f64;
                    tubo(&[[rr * a.cos(), -z2, zc + rr * a.sin()], [rr * a.cos(), -z1, zc + rr * a.sin()]], n1.bitola_mm, &mut tb);
                }
            }
            if let Some(n2) = g.pos("N2") {
                aros(0.0, zc, rr + f / 2.0 + 0.003, -lt + c, -c, n2.espac_cm.unwrap_or(15.0) / 100.0, n2.bitola_mm, &mut th);
            }
        }
        g.add(n1, t);
        if let Some((a, ..)) = adicional {
            g.add(a, tb);
        }
        if let Some(n2) = g.pos("N2") {
            g.add(n2, th);
        }
    }
    // tela do fechamento entre estacas, no meio da espessura do concreto projetado
    if let (Some(n5), Some((esp, ..))) = (g.pos("N5"), e.dimensionar_fechamento(p.fatores.gamma_f, p.fatores.gamma_c, p.fatores.gamma_s)) {
        let x = -e.diametro / 2.0 - esp / 2.0;
        let s = n5.espac_cm.map(|s| s / 100.0).unwrap_or(if n5.as_ef < 1.13 { 0.15 } else { 0.10 });
        let phi = if n5.bitola_mm > 0.0 { n5.bitola_mm } else { (n5.as_ef * s * 4.0 / std::f64::consts::PI).sqrt() * 10.0 };
        let (y0, y1) = (-p.geometria.h + 0.03, -e.viga_h - 0.03);
        let mut t = Vec::new();
        for y in ao_longo(y0, y1, s) {
            tubo(&[[x, y, 0.03], [x, y, l - 0.03]], phi, &mut t);
        }
        for z in ao_longo(0.03, l - 0.03, s) {
            tubo(&[[x, y0, z], [x, y1, z]], phi, &mut t);
        }
        g.add(n5, t);
    }
}

fn grampeado(p: &Projeto, l: f64, g: &mut Gerador) {
    let gr = p.preparar_grampeado();
    let Some(n2) = g.pos("N2") else { return };
    // tela/malha no meio da espessura do paramento (plano inclinado da face)
    let eta = gr.incl_face.to_radians();
    let e = gr.espessura_face.max(0.05);
    let s = n2.espac_cm.map(|s| s / 100.0).unwrap_or(if n2.as_ef < 1.13 { 0.15 } else { 0.10 });
    let phi = if n2.bitola_mm > 0.0 { n2.bitola_mm } else { (n2.as_ef * s * 100.0 / 100.0 * 4.0 / std::f64::consts::PI).sqrt() * 10.0 };
    let face = |y: f64| y * eta.tan() - e / 2.0;
    let mut t = Vec::new();
    for y in ao_longo(0.05, gr.h - 0.05, s) {
        tubo(&[[face(y), y, 0.03], [face(y), y, l - 0.03]], phi, &mut t);
    }
    for z in ao_longo(0.03, l - 0.03, s) {
        tubo(&[[face(0.05), 0.05, z], [face(gr.h - 0.05), gr.h - 0.05, z]], phi, &mut t);
    }
    g.add(n2, t);
}
