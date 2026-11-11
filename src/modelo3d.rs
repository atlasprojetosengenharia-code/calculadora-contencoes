//! Modelo 3D da contenção proposta: a seção transversal do cálculo (a mesma
//! da estabilidade global) extrudada ao longo de um trecho de comprimento `L`,
//! com estacas, perfis de estaca-prancha, tirantes e grampos posicionados pelos
//! espaçamentos do projeto. Serve ao visualizador 3D e à exportação IFC.
//!
//! Coordenadas: x = horizontal na seção (para o maciço contido), y = vertical,
//! z = ao longo da contenção. Unidades em metros.

use crate::perfis_prancha::{Catalogo, Forma as FormaPerfil};
use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::Resultado;
use crate::tipos::embutida::Apoio;
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    Concreto,
    Pedra,
    Gabiao,
    Aco,
    Pvc,
    TiranteLivre,
    Bulbo,
    Grampo,
    Brita,
    Areia,
    Geocomposto,
    Bloco,
    SoloArgila,
    SoloAreia,
    SoloSilte,
    SoloAterro,
    Reaterro,
}

impl Material {
    pub fn nome(&self) -> &'static str {
        match self {
            Material::Concreto => "Concreto",
            Material::Pedra => "Alvenaria de pedra",
            Material::Gabiao => "Gabião",
            Material::Aco => "Aço",
            Material::Pvc => "PVC",
            Material::TiranteLivre => "Tirante — trecho livre",
            Material::Bulbo => "Calda de cimento — bulbo",
            Material::Grampo => "Grampo — barra e calda",
            Material::Brita => "Brita envolta em geotêxtil",
            Material::Areia => "Areia grossa (filtro)",
            Material::Geocomposto => "Geocomposto drenante",
            Material::Bloco => "Alvenaria estrutural de blocos de concreto",
            Material::SoloArgila => "Solo argiloso (corte)",
            Material::SoloAreia => "Solo arenoso (corte)",
            Material::SoloSilte => "Solo siltoso (corte)",
            Material::SoloAterro => "Aterro existente",
            Material::Reaterro => "Reaterro compactado",
        }
    }
    /// Cor RGB (0–1) e transparência (0 = opaco).
    pub fn cor(&self) -> ([f64; 3], f64) {
        let c = match self {
            Material::Concreto => [0.64, 0.64, 0.62],
            Material::Pedra => [0.56, 0.51, 0.44],
            Material::Gabiao => [0.52, 0.50, 0.47],
            Material::Aco => [0.40, 0.44, 0.50],
            Material::Pvc => [0.93, 0.93, 0.90],
            Material::TiranteLivre => [0.20, 0.22, 0.25],
            Material::Bulbo => [0.62, 0.62, 0.58],
            Material::Grampo => [0.55, 0.30, 0.20],
            Material::Brita => [0.56, 0.56, 0.57],
            Material::Areia => [0.83, 0.72, 0.52],
            Material::Geocomposto => [0.12, 0.12, 0.13],
            Material::Bloco => [0.66, 0.66, 0.64],
            Material::SoloArgila => [0.72, 0.43, 0.25],
            Material::SoloAreia => [0.82, 0.70, 0.50],
            Material::SoloSilte => [0.66, 0.53, 0.36],
            Material::SoloAterro => [0.32, 0.24, 0.17],
            Material::Reaterro => [0.42, 0.32, 0.23],
        };
        (c, 0.0)
    }
}

/// Classe IFC do elemento.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Classe {
    Muro,
    Sapata,
    Estaca,
    EstacaPrancha,
    Tirante,
    Grampo,
    Terreno,
    /// Placa de ancoragem (tirante ou grampo).
    Placa,
    /// Cabeça de ancoragem (porca, cunhas, bloco).
    Cabeca,
    /// Tubos de drenagem: barbacãs, dreno de pé, DHP.
    Tubo,
    /// Camada drenante junto ao tardoz.
    Filtro,
    Canaleta,
    /// Viga de travamento (longarina) de parede ancorada.
    Viga,
    /// Pilar de concreto armado (muro misto).
    Pilar,
}

#[derive(Clone, Debug)]
pub enum Forma {
    /// Polígono da seção (x, y) extrudado de z0 a z1.
    Extrusao { secao: Vec<[f64; 2]>, z0: f64, z1: f64 },
    /// Polígono em planta (x, z) extrudado verticalmente de y0 a y1.
    Vertical { planta: Vec<[f64; 2]>, y0: f64, y1: f64 },
    /// Barra cilíndrica de p0 a p1.
    Barra { p0: [f64; 3], p1: [f64; 3], diam: f64 },
    /// Malha já triangulada (peças orientadas, como placas).
    Malha(Vec<Tri>),
}

#[derive(Clone, Debug)]
pub struct Peca {
    pub nome: String,
    pub classe: Classe,
    pub material: Material,
    pub forma: Forma,
}

#[derive(Clone, Debug, Default)]
pub struct Modelo3d {
    pub pecas: Vec<Peca>,
    /// Comprimento do trecho modelado (m).
    pub comprimento: f64,
    /// Avisos (p. ex. comprimento de tirante indicativo).
    pub notas: Vec<String>,
}

pub type Tri = [[f64; 3]; 3];

fn pt(x: f64, y: f64, z: f64) -> [f64; 3] {
    [x, y, z]
}

/// Área com sinal de um polígono 2D.
fn area(p: &[[f64; 2]]) -> f64 {
    let mut a = 0.0;
    for i in 0..p.len() {
        let (u, v) = (p[i], p[(i + 1) % p.len()]);
        a += u[0] * v[1] - v[0] * u[1];
    }
    0.5 * a
}

/// Triangulação por remoção de orelhas (polígonos simples, convexos ou não).
pub fn triangular(poly: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return vec![];
    }
    let mut idx: Vec<usize> = (0..n).collect();
    if area(poly) < 0.0 {
        idx.reverse();
    }
    let cruz = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let mut tris = Vec::new();
    let mut guarda = 0;
    while idx.len() > 3 && guarda < 10 * n {
        guarda += 1;
        let m = idx.len();
        let mut cortou = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cruz(a, b, c) <= 1e-12 {
                continue;
            }
            let contem = idx.iter().any(|&k| {
                k != ia && k != ib && k != ic && {
                    let p = poly[k];
                    cruz(a, b, p) >= -1e-12 && cruz(b, c, p) >= -1e-12 && cruz(c, a, p) >= -1e-12
                }
            });
            if !contem {
                tris.push([ia, ib, ic]);
                idx.remove(i);
                cortou = true;
                break;
            }
        }
        if !cortou {
            break; // polígono degenerado: o restante vai em leque
        }
    }
    for k in 1..idx.len().saturating_sub(1) {
        tris.push([idx[0], idx[k], idx[k + 1]]);
    }
    tris
}

/// Prisma: polígono `base` (pontos 3D por função) e sua cópia deslocada.
fn prisma(poly: &[[f64; 2]], f0: impl Fn([f64; 2]) -> [f64; 3], f1: impl Fn([f64; 2]) -> [f64; 3]) -> Vec<Tri> {
    let mut v = Vec::new();
    for t in triangular(poly) {
        v.push([f0(poly[t[0]]), f0(poly[t[2]]), f0(poly[t[1]])]);
        v.push([f1(poly[t[0]]), f1(poly[t[1]]), f1(poly[t[2]])]);
    }
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        v.push([f0(a), f0(b), f1(b)]);
        v.push([f0(a), f1(b), f1(a)]);
    }
    v
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cruz3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

impl Forma {
    /// Triângulos da peça (cilindros com `lados` faces).
    pub fn triangulos(&self, lados: usize) -> Vec<Tri> {
        match self {
            Forma::Extrusao { secao, z0, z1 } => prisma(secao, |p| pt(p[0], p[1], *z0), |p| pt(p[0], p[1], *z1)),
            Forma::Vertical { planta, y0, y1 } => {
                // planta (x, z): orientação invertida em relação a (x, y)
                prisma(planta, |p| pt(p[0], *y1, p[1]), |p| pt(p[0], *y0, p[1]))
            }
            Forma::Barra { p0, p1, diam } => {
                let eixo = norm(sub(*p1, *p0));
                let aux = if eixo[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
                let u = norm(cruz3(eixo, aux));
                let w = cruz3(eixo, u);
                let r = diam / 2.0;
                let anel = |c: [f64; 3], k: usize| {
                    let a = 2.0 * PI * k as f64 / lados as f64;
                    let (s, co) = a.sin_cos();
                    [c[0] + r * (co * u[0] + s * w[0]), c[1] + r * (co * u[1] + s * w[1]), c[2] + r * (co * u[2] + s * w[2])]
                };
                let mut v = Vec::new();
                for k in 0..lados {
                    let (a0, a1, b0, b1) = (anel(*p0, k), anel(*p0, k + 1), anel(*p1, k), anel(*p1, k + 1));
                    v.push([a0, b0, b1]);
                    v.push([a0, b1, a1]);
                    v.push([*p0, a1, a0]);
                    v.push([*p1, b0, b1]);
                }
                v
            }
            Forma::Malha(t) => t.clone(),
        }
    }
}

fn fmt0(x: f64) -> String {
    format!("{:.0}", x)
}

/// Dois vetores unitários perpendiculares ao eixo (e entre si).
fn base_eixo(a: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let aux = if a[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let u = norm(cruz3(a, aux));
    (u, cruz3(a, u))
}

/// Paralelepípedo de centro `c`, eixos unitários `a`, `u`, `w` e meias-dimensões.
fn caixa_orientada(c: [f64; 3], a: [f64; 3], u: [f64; 3], w: [f64; 3], ha: f64, hu: f64, hw: f64) -> Vec<Tri> {
    let v = |sa: f64, su: f64, sw: f64| {
        [
            c[0] + sa * ha * a[0] + su * hu * u[0] + sw * hw * w[0],
            c[1] + sa * ha * a[1] + su * hu * u[1] + sw * hw * w[1],
            c[2] + sa * ha * a[2] + su * hu * u[2] + sw * hw * w[2],
        ]
    };
    let p = [v(-1.0, -1.0, -1.0), v(1.0, -1.0, -1.0), v(1.0, 1.0, -1.0), v(-1.0, 1.0, -1.0), v(-1.0, -1.0, 1.0), v(1.0, -1.0, 1.0), v(1.0, 1.0, 1.0), v(-1.0, 1.0, 1.0)];
    let mut t = Vec::new();
    for q in [[0, 3, 2, 1], [4, 5, 6, 7], [0, 1, 5, 4], [1, 2, 6, 5], [2, 3, 7, 6], [3, 0, 4, 7]] {
        t.push([p[q[0]], p[q[1]], p[q[2]]]);
        t.push([p[q[0]], p[q[2]], p[q[3]]]);
    }
    t
}

/// Envoltória convexa (cadeia monótona), em sentido anti-horário.
fn envoltoria(p: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut v = p.to_vec();
    v.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    v.dedup();
    if v.len() < 3 {
        return v;
    }
    let cruz = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut h: Vec<[f64; 2]> = Vec::new();
    for passo in 0..2 {
        let ini = h.len();
        let it: Box<dyn Iterator<Item = &[f64; 2]>> = if passo == 0 { Box::new(v.iter()) } else { Box::new(v.iter().rev()) };
        for &q in it {
            while h.len() >= ini + 2 && cruz(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}

/// Polígono circular em planta (x, z).
fn circulo(xc: f64, zc: f64, d: f64, lados: usize) -> Vec<[f64; 2]> {
    (0..lados)
        .map(|k| {
            let a = 2.0 * PI * k as f64 / lados as f64;
            [xc + d / 2.0 * a.cos(), zc + d / 2.0 * a.sin()]
        })
        .collect()
}

/// Posições ao longo do trecho para elementos com espaçamento `s`.
fn posicoes(l: f64, s: f64) -> Vec<f64> {
    let s = s.max(0.1);
    let n = ((l / s).floor() as usize).max(1);
    let folga = (l - (n - 1) as f64 * s) / 2.0;
    (0..n).map(|i| folga + i as f64 * s).collect()
}

/// Perfil ondulado da estaca-prancha em planta, com espessura t (polígono fechado).
fn perfil_prancha(forma: FormaPerfil, b: f64, h: f64, t: f64, l: f64, x0: f64) -> Vec<[f64; 2]> {
    // onda trapezoidal: período 2b (U: duas peças) ou b (Z e W: uma peça por onda)
    let periodo = match forma {
        FormaPerfil::U => 2.0 * b,
        _ => b,
    }
    .max(0.2);
    let mesa = 0.30 * periodo;
    let alma = 0.20 * periodo;
    let mut linha = vec![[x0, 0.0]];
    let mut z = 0.0;
    while z < l - 1e-9 {
        for (dz, dx) in [(mesa / 2.0, 0.0), (alma, h), (mesa, h), (alma, 0.0), (mesa / 2.0, 0.0)] {
            z = (z + dz).min(l);
            linha.push([x0 - h / 2.0 + dx, z]);
            if z >= l {
                break;
            }
        }
    }
    linha[0][0] = x0 - h / 2.0;
    let mut poly: Vec<[f64; 2]> = linha.clone();
    poly.extend(linha.iter().rev().map(|p| [p[0] + t, p[1]]));
    poly.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
    poly
}

impl Modelo3d {
    /// Monta o modelo do projeto num trecho de comprimento `l` (m).
    /// `corte`: maciço só até a metade do trecho (vista em corte).
    pub fn montar(p: &Projeto, r: &Resultado, l: f64, corte: bool) -> Self {
        let l = l.max(1.0);
        let m = p.secao_global(r);
        let mut md = Modelo3d { comprimento: l, ..Default::default() };
        let ext = |secao: &Vec<[f64; 2]>| Forma::Extrusao { secao: secao.clone(), z0: 0.0, z1: l };
        let h = p.geometria.h;
        let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut amplia = |x: f64| {
            xmin = xmin.min(x);
            xmax = xmax.max(x);
        };

        match p.tipo {
            TipoContencao::Gravidade => {
                let g = p.preparar_gravidade();
                let mat = match g.material {
                    crate::tipos::gravidade::Material::Gabiao => Material::Gabiao,
                    crate::tipos::gravidade::Material::ConcretoCiclopico | crate::tipos::gravidade::Material::ConcretoSimples => Material::Concreto,
                    _ => Material::Pedra,
                };
                for (i, s) in m.muros.iter().enumerate() {
                    s.iter().for_each(|q| amplia(q[0]));
                    md.pecas.push(Peca {
                        nome: format!("{} — {}", if i == 0 { "Base" } else { "Corpo do muro" }, g.material.nome()),
                        classe: if i == 0 { Classe::Sapata } else { Classe::Muro },
                        material: mat,
                        forma: ext(s),
                    });
                }
                if g.dente && g.dente_h > 0.0 {
                    // dente sob o pé do paramento frontal (mesma posição do cálculo)
                    let x0 = g.geo().x_pf;
                    md.pecas.push(Peca {
                        nome: format!("Dente de base — {}", g.material.nome()),
                        classe: Classe::Sapata,
                        material: mat,
                        forma: ext(&vec![[x0, -g.dente_h], [x0 + g.dente_b, -g.dente_h], [x0 + g.dente_b, 0.0], [x0, 0.0]]),
                    });
                }
            }
            TipoContencao::Flexao if p.flexao.sistema == crate::tipos::misto::Sistema::Misto => {
                // misto: sapata + painel de blocos + pilares + viga de respaldo
                let fx = p.preparar_flexao();
                let mi = &fx.misto;
                let (bp, hs, ht) = (fx.b_ponta, fx.h_sapata, fx.h);
                if let Some(s) = m.muros.first() {
                    s.iter().for_each(|q| amplia(q[0]));
                    md.pecas.push(Peca { nome: "Sapata corrida (concreto armado)".into(), classe: Classe::Sapata, material: Material::Concreto, forma: ext(s) });
                }
                let hv = 0.19; // viga de respaldo em canaleta (altura de uma fiada)
                md.pecas.push(Peca {
                    nome: format!("Painel de alvenaria estrutural (bloco de {} cm)", fmt0(100.0 * mi.bloco)),
                    classe: Classe::Muro,
                    material: Material::Bloco,
                    forma: ext(&vec![[bp, hs], [bp + mi.bloco, hs], [bp + mi.bloco, ht - hv], [bp, ht - hv]]),
                });
                md.pecas.push(Peca {
                    nome: "Viga de respaldo (concreto armado)".into(),
                    classe: Classe::Viga,
                    material: Material::Concreto,
                    forma: ext(&vec![[bp - 0.005, ht - hv], [bp + mi.bloco, ht - hv], [bp + mi.bloco, ht], [bp - 0.005, ht]]),
                });
                amplia(bp + mi.pilar_h);
                for (k, z) in posicoes(l, mi.espac_pilar).into_iter().enumerate() {
                    // face externa 5 mm à frente do painel (sem disputa de pixel)
                    md.pecas.push(Peca {
                        nome: format!("Pilar P{} {} × {} cm (concreto armado)", k + 1, fmt0(100.0 * mi.pilar_b), fmt0(100.0 * mi.pilar_h)),
                        classe: Classe::Pilar,
                        material: Material::Concreto,
                        forma: Forma::Extrusao {
                            secao: vec![[bp - 0.005, hs], [bp + mi.pilar_h, hs], [bp + mi.pilar_h, ht - hv], [bp - 0.005, ht - hv]],
                            z0: (z - mi.pilar_b / 2.0).max(0.0),
                            z1: (z + mi.pilar_b / 2.0).min(l),
                        },
                    });
                }
                if fx.dente && fx.dente_h > 0.0 {
                    let xc = bp + fx.t_base / 2.0;
                    md.pecas.push(Peca {
                        nome: "Dente de base (concreto armado)".into(),
                        classe: Classe::Sapata,
                        material: Material::Concreto,
                        forma: ext(&vec![[xc - fx.dente_b / 2.0, -fx.dente_h], [xc + fx.dente_b / 2.0, -fx.dente_h], [xc + fx.dente_b / 2.0, 0.0], [xc - fx.dente_b / 2.0, 0.0]]),
                    });
                }
            }
            TipoContencao::Flexao => {
                for (i, s) in m.muros.iter().enumerate() {
                    s.iter().for_each(|q| amplia(q[0]));
                    md.pecas.push(Peca {
                        nome: if i == 0 { "Sapata corrida (concreto armado)".into() } else { "Parede do muro (concreto armado)".into() },
                        classe: if i == 0 { Classe::Sapata } else { Classe::Muro },
                        material: Material::Concreto,
                        forma: ext(s),
                    });
                }
                let fx = p.preparar_flexao();
                if fx.dente && fx.dente_h > 0.0 {
                    // dente sob a parede (centro em b_ponta + t_base/2, como no cálculo)
                    let xc = fx.b_ponta + fx.t_base / 2.0;
                    md.pecas.push(Peca {
                        nome: "Dente de base (concreto armado)".into(),
                        classe: Classe::Sapata,
                        material: Material::Concreto,
                        forma: ext(&vec![[xc - fx.dente_b / 2.0, -fx.dente_h], [xc + fx.dente_b / 2.0, -fx.dente_h], [xc + fx.dente_b / 2.0, 0.0], [xc - fx.dente_b / 2.0, 0.0]]),
                    });
                }
            }
            TipoContencao::Atirantada => {
                for s in &m.muros {
                    s.iter().for_each(|q| amplia(q[0]));
                    md.pecas.push(Peca { nome: "Cortina de concreto armado".into(), classe: Classe::Muro, material: Material::Concreto, forma: ext(s) });
                }
            }
            TipoContencao::Prancha => {
                let e = p.preparar_prancha();
                let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(1.5 * h);
                let (forma, b, hp, t, mat) = match e.catalogo() {
                    Some(Catalogo::Aco(c)) => (c.forma, c.b / 1000.0, c.h / 1000.0, c.t / 1000.0, Material::Aco),
                    Some(Catalogo::Pvc(c)) => (c.forma, c.b / 1000.0, c.h / 1000.0, c.t / 1000.0, Material::Pvc),
                    None => (FormaPerfil::Z, 0.60, 0.30, 0.010, Material::Aco),
                };
                amplia(-hp);
                amplia(hp);
                md.pecas.push(Peca {
                    nome: format!("Estaca-prancha {}", e.nome_perfil),
                    classe: Classe::EstacaPrancha,
                    material: mat,
                    forma: Forma::Vertical { planta: perfil_prancha(forma, b, hp, t.max(0.004), l, 0.0), y0: -lt, y1: 0.0 },
                });
                md.apoios_embutida(&e.parede, h, lt, l, -hp / 2.0, e.viga_h);
            }
            TipoContencao::Estacas => {
                let e = p.preparar_estacas();
                let lt = r.verificacoes.iter().find(|v| v.nome.starts_with("Comprimento total")).map(|v| v.valor).unwrap_or(1.5 * h);
                amplia(-e.diametro);
                amplia(e.diametro);
                for (i, z) in posicoes(l, e.espacamento).into_iter().enumerate() {
                    md.pecas.push(Peca {
                        nome: format!("Estaca escavada Ø{} cm — E{}", crate::resultado::fmt(100.0 * e.diametro), i + 1),
                        classe: Classe::Estaca,
                        material: Material::Concreto,
                        forma: Forma::Vertical { planta: circulo(0.0, z, e.diametro, 20), y0: -lt, y1: 0.0 },
                    });
                }
                md.apoios_embutida(&e.parede, h, lt, l, -e.diametro / 2.0, 0.5);
            }
            TipoContencao::Grampeado => {
                let g = p.preparar_grampeado();
                let x_ft = g.h * g.incl_face.to_radians().tan();
                let e = g.espessura_face.max(0.05);
                amplia(-e);
                amplia(x_ft);
                md.pecas.push(Peca {
                    nome: "Face em concreto projetado".into(),
                    classe: Classe::Muro,
                    material: Material::Concreto,
                    forma: ext(&vec![[-e, 0.0], [0.0, 0.0], [x_ft, g.h], [x_ft - e, g.h]]),
                });
            }
        }

        // reforços do cálculo (tirantes da cortina e grampos)
        let (espac, furo) = match p.tipo {
            TipoContencao::Atirantada => (p.atirantada.espac_h, p.atirantada.diam_furo),
            TipoContencao::Grampeado => (p.grampeado.sh, p.grampeado.diam_furo),
            _ => (0.0, 0.0),
        };
        if espac > 0.0 {
            for (k, rf) in r.reforcos.iter().enumerate() {
                let fim = |s: f64| [rf.p0[0] + rf.dir[0] * s, rf.p0[1] + rf.dir[1] * s];
                amplia(fim(rf.comprimento)[0]);
                for z in posicoes(l, espac) {
                    let p3 = |q: [f64; 2]| pt(q[0], q[1], z);
                    if p.tipo == TipoContencao::Atirantada {
                        // face frontal da cortina (x = −espessura) no eixo do tirante
                        let s_face = -p.atirantada.espessura / rf.dir[0].max(0.1);
                        md.pecas.push(Peca {
                            nome: format!("Tirante nível {} — trecho livre", k + 1),
                            classe: Classe::Tirante,
                            material: Material::TiranteLivre,
                            forma: Forma::Barra { p0: p3(fim(s_face)), p1: p3(fim(rf.livre)), diam: 0.04 },
                        });
                        md.cabeca(k + 1, p3(fim(s_face)), [rf.dir[0], rf.dir[1], 0.0], p.atirantada.placa.max(0.15), true);
                        md.pecas.push(Peca {
                            nome: format!("Tirante nível {} — bulbo", k + 1),
                            classe: Classe::Tirante,
                            material: Material::Bulbo,
                            forma: Forma::Barra { p0: p3(fim(rf.livre)), p1: p3(fim(rf.comprimento)), diam: furo.max(0.05) },
                        });
                    } else {
                        md.pecas.push(Peca {
                            nome: format!("Grampo linha {}", k + 1),
                            classe: Classe::Grampo,
                            material: Material::Grampo,
                            forma: Forma::Barra { p0: p3(rf.p0), p1: p3(fim(rf.comprimento)), diam: furo.max(0.05) },
                        });
                        // trecho na face de concreto projetado e cabeça na face externa
                        let e = p.grampeado.espessura_face.max(0.05);
                        let frente = [rf.p0[0] - e, rf.p0[1]];
                        md.pecas.push(Peca {
                            nome: format!("Grampo linha {} — barra na face", k + 1),
                            classe: Classe::Grampo,
                            material: Material::Aco,
                            forma: Forma::Barra { p0: p3(frente), p1: p3(rf.p0), diam: (p.grampeado.diam_barra / 1000.0).max(0.012) },
                        });
                        md.cabeca(k + 1, p3(frente), [rf.dir[0], rf.dir[1], 0.0], p.grampeado.placa.max(0.10), false);
                    }
                }
            }
        }
        md.drenagem(p, l);
        for pc in &md.pecas {
            if let Forma::Barra { p1, .. } = pc.forma {
                amplia(p1[0]);
            }
        }

        // maciço de solo: volumes opacos (camadas da sondagem e reaterro), sem
        // atravessar as peças; com `corte`, vai só até a metade do trecho para
        // mostrar o interior (reforços, drenagem e a face do corte)
        let folga = 0.8 * h.max(2.0);
        let (x0, x1) = (xmin - folga, xmax + folga);
        let z_solo = if corte { 0.5 * l } else { l };
        // muros: o terreno à frente fica na cota do embutimento Df até a face (cobre o ressalto da base)
        let frente = match p.tipo {
            TipoContencao::Gravidade => Some((p.geometria.engaste.max(0.0), p.preparar_gravidade().geo().x_cr)),
            TipoContencao::Flexao => {
                let fx = p.preparar_flexao();
                Some((p.geometria.engaste.max(0.0), fx.b_ponta + fx.t_topo))
            }
            _ => None,
        };
        md.solos(p, &m, x0, x1, z_solo, l, frente);
        md
    }

    /// Volumes de solo por fatias verticais da seção, descontadas as peças.
    fn solos(&mut self, p: &Projeto, m: &crate::global::Modelo, x0: f64, x1: f64, z1: f64, l: f64, frente: Option<(f64, f64)>) {
        let y_sup0 = |x: f64| -> f64 {
            for s in m.sup.windows(2) {
                let (a, b) = (s[0], s[1]);
                if a[0] <= x && x <= b[0] && (b[0] - a[0]).abs() > 1e-12 {
                    return a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]);
                }
            }
            if x < m.sup.first().map(|q| q[0]).unwrap_or(0.0) { m.sup[0][1] } else { m.sup.last().map(|q| q[1]).unwrap_or(0.0) }
        };
        // muros: à frente da crista o terreno fica na cota Df (o corpo é recortado como obstáculo)
        let y_sup = |x: f64| -> f64 {
            match frente {
                Some((df, x_crista)) if x < x_crista => df,
                _ => y_sup0(x),
            }
        };
        // obstáculos: seções das peças contínuas e retângulos das paredes verticais
        let mut obst: Vec<Vec<[f64; 2]>> = Vec::new();
        for pc in &self.pecas {
            match &pc.forma {
                Forma::Extrusao { secao, z0, z1: zf } if *z0 < 1e-6 && (*zf - l).abs() < 1e-6 => obst.push(secao.clone()),
                Forma::Vertical { planta, y0, y1 } => {
                    let (a, b) = planta.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| (a.min(q[0]), b.max(q[0])));
                    obst.push(vec![[a, *y0], [b, *y0], [b, *y1], [a, *y1]]);
                }
                _ => {}
            }
        }
        // recorte pelo contorno convexo: canal da calha (seção em U) e vãos ficam sem solo
        let obst: Vec<Vec<[f64; 2]>> = obst.iter().map(|o| envoltoria(o)).collect();
        let y_min = obst.iter().flatten().map(|q| q[1]).fold(m.y_fundo, f64::min);
        let y_fundo = y_min - 1.0;
        // intervalo vertical ocupado por um polígono na abscissa x
        let intervalo = |poly: &[[f64; 2]], x: f64| -> Option<(f64, f64)> {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                if (a[0] - x) * (b[0] - x) <= 0.0 && (b[0] - a[0]).abs() > 1e-12 {
                    let y = a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]);
                    lo = lo.min(y);
                    hi = hi.max(y);
                }
            }
            (hi > lo).then_some((lo, hi))
        };
        // abscissas de quebra
        let mut xs = vec![x0, x1];
        for q in m.sup.iter().chain(obst.iter().flatten()).chain(m.reaterro.iter().flatten()) {
            if q[0] > x0 && q[0] < x1 {
                xs.push(q[0]);
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        // camadas da sondagem (profundidade a partir de y_ref)
        let camadas = p.sondagem.camadas_calc();
        let mut cortes: Vec<f64> = camadas.iter().map(|c| m.y_ref - c.ate).filter(|y| *y > y_fundo).collect();
        cortes.push(f64::INFINITY);
        cortes.push(y_fundo);
        cortes.sort_by(|a, b| a.total_cmp(b));
        // divisas de camada cruzando faces inclinadas (tardoz, talude): novas quebras em x
        {
            let mut extra = Vec::new();
            let arestas = obst.iter().chain(m.reaterro.iter()).flat_map(|o| (0..o.len()).map(move |i| (o[i], o[(i + 1) % o.len()]))).chain(m.sup.windows(2).map(|s| (s[0], s[1])));
            for (a, b) in arestas {
                for &yc in &cortes {
                    if yc.is_finite() && (a[1] - yc) * (b[1] - yc) < 0.0 {
                        let x = a[0] + (b[0] - a[0]) * (yc - a[1]) / (b[1] - a[1]);
                        if x > x0 && x < x1 {
                            extra.push(x);
                        }
                    }
                }
            }
            xs.extend(extra);
            xs.sort_by(|a, b| a.total_cmp(b));
            xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        }
        let material_em = |x: f64, y: f64| -> (Material, String) {
            if m.reaterro.iter().any(|r| intervalo(r, x).is_some_and(|(a, b)| y >= a && y <= b)) {
                return (Material::Reaterro, "Reaterro compactado".into());
            }
            match p.sondagem.camada_em((m.y_ref - y).max(0.0)) {
                Some(c) => {
                    let d = c.descricao.to_lowercase();
                    let t = format!("{:?}", c.tipo);
                    let mat = if d.contains("aterro") {
                        Material::SoloAterro
                    } else if t.starts_with("Areia") {
                        Material::SoloAreia
                    } else if t.starts_with("Silte") {
                        Material::SoloSilte
                    } else {
                        Material::SoloArgila
                    };
                    (mat, format!("Solo natural — {}", c.descricao))
                }
                None => (Material::SoloArgila, "Solo natural".into()),
            }
        };
        let mut baldes: Vec<(Material, String, Vec<Tri>)> = Vec::new();
        for w in xs.windows(2) {
            let (xa, xb) = (w[0], w[1]);
            if xb - xa < 1e-6 {
                continue;
            }
            let (ea, eb, xm) = (xa + 1e-7, xb - 1e-7, 0.5 * (xa + xb));
            // coluna de solo nas duas bordas, menos os obstáculos (topologia pelo meio da faixa)
            let mut col = vec![(y_fundo, y_sup(ea), y_fundo, y_sup(eb))];
            let mut ocup: Vec<(f64, f64, f64, f64, f64)> = obst
                .iter()
                .filter_map(|o| {
                    let (lm, _) = intervalo(o, xm)?;
                    let (la, ha) = intervalo(o, ea).or(intervalo(o, xm))?;
                    let (lb, hb) = intervalo(o, eb).or(intervalo(o, xm))?;
                    Some((lm, la, ha, lb, hb))
                })
                .collect();
            ocup.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, la, ha, lb, hb) in ocup {
                col = col
                    .into_iter()
                    .flat_map(|(a0, a1, b0, b1)| {
                        let mut r = Vec::new();
                        if la > a0 + 1e-6 || lb > b0 + 1e-6 {
                            r.push((a0, la.min(a1), b0, lb.min(b1)));
                        }
                        if ha < a1 - 1e-6 || hb < b1 - 1e-6 {
                            r.push((ha.max(a0), a1, hb.max(b0), b1));
                        }
                        r
                    })
                    .collect();
            }
            for (a0, a1, b0, b1) in col {
                for k in cortes.windows(2) {
                    let (c0, c1) = (k[0], k[1]);
                    let (pa0, pa1) = (a0.max(c0), a1.min(c1));
                    let (pb0, pb1) = (b0.max(c0), b1.min(c1));
                    if pa1 - pa0 < 1e-4 && pb1 - pb0 < 1e-4 {
                        continue;
                    }
                    let mut secao = vec![[xa, pa0.min(pa1)], [xb, pb0.min(pb1)], [xb, pb1], [xa, pa1]];
                    secao.dedup_by(|u, v| (u[0] - v[0]).abs() < 1e-9 && (u[1] - v[1]).abs() < 1e-5);
                    if secao.len() < 3 {
                        continue;
                    }
                    let ym = 0.25 * (pa0 + pa1 + pb0 + pb1);
                    let (mat, nome) = material_em(xm, ym);
                    let tris = prisma(&secao, |q| pt(q[0], q[1], 0.0), |q| pt(q[0], q[1], z1));
                    match baldes.iter_mut().find(|(mm, n, _)| *mm == mat && *n == nome) {
                        Some((_, _, v)) => v.extend(tris),
                        None => baldes.push((mat, nome, tris)),
                    }
                }
            }
        }
        for (material, nome, tris) in baldes {
            self.pecas.push(Peca { nome, classe: Classe::Terreno, material, forma: Forma::Malha(tris) });
        }
    }

    /// Tirantes/estroncas de paredes embutidas ancoradas (comprimento indicativo).
    fn apoios_embutida(&mut self, par: &crate::tipos::embutida::ParedeEmbutida, h: f64, lt: f64, l: f64, x_face: f64, viga_h: f64) {
        if par.apoio != Apoio::Ancorada {
            return;
        }
        let a = par.incl_apoio.to_radians();
        let comp = (0.6 * lt).max(4.0).max(h);
        let livre = 0.45 * comp;
        let dir = [a.cos(), -a.sin()];
        let y0 = -par.prof_apoio;
        // viga de travamento (longarina) na face escavada, na cota dos tirantes
        let (vb, vh) = (0.30, viga_h.clamp(0.20, 1.0));
        let xv = x_face - vb;
        self.pecas.push(Peca {
            nome: "Viga de travamento (longarina)".into(),
            classe: Classe::Viga,
            material: Material::Concreto,
            forma: Forma::Extrusao { secao: vec![[xv, y0 - vh / 2.0], [x_face, y0 - vh / 2.0], [x_face, y0 + vh / 2.0], [xv, y0 + vh / 2.0]], z0: 0.0, z1: l },
        });
        for z in posicoes(l, par.espac_apoio) {
            let q = |s: f64| pt(dir[0] * s, y0 + dir[1] * s, z);
            let s_face = xv / dir[0].max(0.1);
            self.pecas.push(Peca { nome: "Tirante — trecho livre".into(), classe: Classe::Tirante, material: Material::TiranteLivre, forma: Forma::Barra { p0: q(s_face), p1: q(livre), diam: 0.04 } });
            self.cabeca(1, q(s_face), [dir[0], dir[1], 0.0], 0.25, true);
            self.pecas.push(Peca { nome: "Tirante — bulbo".into(), classe: Classe::Tirante, material: Material::Bulbo, forma: Forma::Barra { p0: q(livre), p1: q(comp), diam: 0.15 } });
        }
        self.notas.push(format!(
            "Tirantes da parede ancorada com comprimento indicativo ({} m): dimensionar o bulbo e o trecho livre conforme a NBR 5629.",
            crate::resultado::fmt(comp)
        ));
    }

    /// Cabeça de ancoragem na face externa: placa perpendicular ao eixo e,
    /// para tirantes, bloco de cunhas com a ponta da cordoalha; para grampos,
    /// porca sextavada com a ponta da barra rosqueada.
    fn cabeca(&mut self, nivel: usize, face: [f64; 3], dir: [f64; 3], placa: f64, tirante: bool) {
        let a = norm(dir);
        let (u, w) = base_eixo(a);
        let tp = if tirante { 0.032 } else { 0.019 };
        let fora = |s: f64| [face[0] - a[0] * s, face[1] - a[1] * s, face[2] - a[2] * s];
        let tipo = if tirante { "Tirante" } else { "Grampo" };
        self.pecas.push(Peca {
            nome: format!("{tipo} nível {nivel} — placa de ancoragem {}×{} cm", fmt0(100.0 * placa), fmt0(100.0 * placa)),
            classe: Classe::Placa,
            material: Material::Aco,
            forma: Forma::Malha(caixa_orientada(fora(tp / 2.0), a, u, w, tp / 2.0, placa / 2.0, placa / 2.0)),
        });
        let (comp, d, ponta, dp) = if tirante { (0.10, 0.11, 0.10, 0.04) } else { (0.05, 0.055, 0.06, 0.025) };
        self.pecas.push(Peca {
            nome: format!("{tipo} nível {nivel} — {}", if tirante { "cabeça (bloco de cunhas)" } else { "porca" }),
            classe: Classe::Cabeca,
            material: Material::Aco,
            forma: Forma::Barra { p0: fora(tp), p1: fora(tp + comp), diam: d },
        });
        self.pecas.push(Peca {
            nome: format!("{tipo} nível {nivel} — ponta {}", if tirante { "da cordoalha" } else { "da barra" }),
            classe: Classe::Cabeca,
            material: Material::Aco,
            forma: Forma::Barra { p0: fora(tp + comp), p1: fora(tp + comp + ponta), diam: dp },
        });
    }

    /// Drenagem: barbacãs (em quincôncio), camada drenante junto ao tardoz,
    /// dreno de pé, canaletas e drenos horizontais profundos.
    fn drenagem(&mut self, p: &Projeto, l: f64) {
        let dr = &p.drenagem;
        let gd = p.geo_drenagem();
        let d_barb = (dr.diam_barbaca_mm / 1000.0).max(0.04);
        let bolsa_c = crate::drenagem::BOLSA_COMP;
        for (i, (t, f)) in gd.barbacas.iter().enumerate() {
            // o tubo já entra 20 cm no maciço e sai 15 cm da face (geo_drenagem)
            let desl = if i % 2 == 1 { dr.esp_h / 2.0 } else { 0.0 };
            for z in posicoes(l, dr.esp_h) {
                let z = z + desl;
                if z > l - 0.05 {
                    continue;
                }
                self.pecas.push(Peca {
                    nome: format!("Barbacã PVC Ø{} mm — fila {}", fmt0(dr.diam_barbaca_mm), i + 1),
                    classe: Classe::Tubo,
                    material: Material::Pvc,
                    forma: Forma::Barra { p0: pt(t[0], t[1], z), p1: pt(f[0], f[1], z), diam: d_barb },
                });
                // bolsa de brita envolta em geotêxtil na entrada do tubo
                if let Some(b) = gd.bolsas.get(i) {
                    self.pecas.push(Peca {
                        nome: format!("Bolsa drenante de brita + geotêxtil — fila {}", i + 1),
                        classe: Classe::Filtro,
                        material: Material::Brita,
                        forma: Forma::Extrusao { secao: b.to_vec(), z0: (z - bolsa_c / 2.0).max(0.0), z1: (z + bolsa_c / 2.0).min(l) },
                    });
                }
            }
        }
        if let (Some((p0, p1)), true) = (gd.filtro, gd.filtro_continuo) {
            let e = dr.esp_filtro.max(0.02);
            let (mat, nome) = match dr.material_filtro {
                crate::drenagem::MaterialFiltro::Geocomposto => (Material::Geocomposto, "Geocomposto drenante"),
                _ => (Material::Areia, "Filtro de areia grossa"),
            };
            self.pecas.push(Peca {
                nome: format!("{nome} junto ao tardoz (e = {} cm)", fmt0(100.0 * e)),
                classe: Classe::Filtro,
                material: mat,
                forma: Forma::Extrusao { secao: vec![p0, [p0[0] + e, p0[1]], [p1[0] + e, p1[1]], p1], z0: 0.0, z1: l },
            });
        }
        if dr.dreno_pe {
            let dpe = (dr.diam_dreno_pe_mm / 1000.0).max(0.06);
            let centro = match (gd.envelope_pe, gd.filtro) {
                (Some(env), _) => {
                    self.pecas.push(Peca {
                        nome: "Envelope de brita + geotêxtil do dreno de pé".into(),
                        classe: Classe::Filtro,
                        material: Material::Brita,
                        forma: Forma::Extrusao { secao: env.to_vec(), z0: 0.0, z1: l },
                    });
                    Some([(env[0][0] + env[1][0]) / 2.0, (env[0][1] + env[2][1]) / 2.0])
                }
                (None, Some((p0, _))) if gd.filtro_continuo => Some([p0[0] + dr.esp_filtro / 2.0, p0[1] + dpe * 0.7]),
                _ => None,
            };
            if let Some([x, y]) = centro {
                self.pecas.push(Peca {
                    nome: format!("Dreno de pé — tubo perfurado Ø{} mm", fmt0(dr.diam_dreno_pe_mm)),
                    classe: Classe::Tubo,
                    material: Material::Pvc,
                    forma: Forma::Barra { p0: pt(x, y, 0.0), p1: pt(x, y, l), diam: dpe },
                });
            }
        }
        for c in &gd.canaletas {
            let (xc, y) = (c[0], c[1]);
            let u = vec![
                [xc - 0.15, y], [xc - 0.10, y - 0.25], [xc + 0.10, y - 0.25], [xc + 0.15, y],
                [xc + 0.10, y], [xc + 0.075, y - 0.20], [xc - 0.075, y - 0.20], [xc - 0.10, y],
            ];
            self.pecas.push(Peca { nome: "Canaleta de concreto".into(), classe: Classe::Canaleta, material: Material::Concreto, forma: Forma::Extrusao { secao: u, z0: 0.0, z1: l } });
        }
        for (a, b) in &gd.dhp {
            for z in posicoes(l, dr.dhp_espacamento) {
                self.pecas.push(Peca {
                    nome: "Dreno horizontal profundo (DHP)".into(),
                    classe: Classe::Tubo,
                    material: Material::Pvc,
                    forma: Forma::Barra { p0: pt(a[0], a[1], z), p1: pt(b[0], b[1], z), diam: 0.05 },
                });
            }
        }
    }

    /// Caixa envolvente (mín, máx).
    pub fn caixa(&self) -> ([f64; 3], [f64; 3]) {
        let (mut a, mut b) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for pc in &self.pecas {
            for t in pc.forma.triangulos(6) {
                for v in t {
                    for k in 0..3 {
                        a[k] = a[k].min(v[k]);
                        b[k] = b[k].max(v[k]);
                    }
                }
            }
        }
        if !a[0].is_finite() {
            return ([0.0; 3], [1.0; 3]);
        }
        (a, b)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::projeto::TIPOS_CONTENCAO;

    #[test]
    fn triangula_poligono_concavo() {
        // "L": área 3
        let l = [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
        let t = triangular(&l);
        assert_eq!(t.len(), 4);
        let a: f64 = t.iter().map(|k| area(&[l[k[0]], l[k[1]], l[k[2]]]).abs()).sum();
        assert!((a - 3.0).abs() < 1e-9);
    }

    #[test]
    fn todo_tipo_gera_modelo() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let r = p.calcular();
            let m = Modelo3d::montar(&p, &r, 10.0, false);
            assert!(m.pecas.iter().any(|x| x.classe != Classe::Terreno), "{}", tipo.nome());
            assert!(m.pecas.iter().any(|x| x.classe == Classe::Terreno));
            for pc in &m.pecas {
                let t = pc.forma.triangulos(8);
                assert!(!t.is_empty(), "{}: {}", tipo.nome(), pc.nome);
                assert!(t.iter().flatten().flatten().all(|v| v.is_finite()));
            }
            let (a, b) = m.caixa();
            assert!(b[2] - a[2] >= 9.9, "trecho de 10 m");
            if tipo == TipoContencao::Grampeado || tipo == TipoContencao::Atirantada {
                assert!(m.pecas.iter().any(|x| matches!(x.classe, Classe::Grampo | Classe::Tirante)), "{}", tipo.nome());
                assert!(m.pecas.iter().any(|x| x.classe == Classe::Placa) && m.pecas.iter().any(|x| x.classe == Classe::Cabeca), "{}: cabeças", tipo.nome());
            }
            if matches!(tipo, TipoContencao::Gravidade | TipoContencao::Flexao | TipoContencao::Atirantada | TipoContencao::Grampeado) {
                assert!(m.pecas.iter().any(|x| x.classe == Classe::Tubo && x.nome.starts_with("Barbacã")), "{}: barbacãs", tipo.nome());
                assert!(m.pecas.iter().any(|x| x.classe == Classe::Filtro), "{}: filtro", tipo.nome());
                assert!(m.pecas.iter().any(|x| x.nome.starts_with("Dreno de pé")), "{}: dreno de pé", tipo.nome());
            }
        }
    }
}
