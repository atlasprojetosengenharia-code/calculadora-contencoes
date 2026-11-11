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
    Terreno,
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
            Material::Terreno => "Terreno",
        }
    }
    /// Cor RGB (0–1) e transparência (0 = opaco).
    pub fn cor(&self) -> ([f64; 3], f64) {
        match self {
            Material::Concreto => ([0.72, 0.72, 0.70], 0.0),
            Material::Pedra => ([0.66, 0.58, 0.46], 0.0),
            Material::Gabiao => ([0.60, 0.56, 0.50], 0.0),
            Material::Aco => ([0.45, 0.52, 0.62], 0.0),
            Material::Pvc => ([0.55, 0.70, 0.85], 0.0),
            Material::TiranteLivre => ([0.95, 0.60, 0.15], 0.0),
            Material::Bulbo => ([0.80, 0.35, 0.10], 0.0),
            Material::Grampo => ([0.85, 0.20, 0.20], 0.0),
            Material::Terreno => ([0.55, 0.47, 0.30], 0.55),
        }
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
}

#[derive(Clone, Debug)]
pub enum Forma {
    /// Polígono da seção (x, y) extrudado de z0 a z1.
    Extrusao { secao: Vec<[f64; 2]>, z0: f64, z1: f64 },
    /// Polígono em planta (x, z) extrudado verticalmente de y0 a y1.
    Vertical { planta: Vec<[f64; 2]>, y0: f64, y1: f64 },
    /// Barra cilíndrica de p0 a p1.
    Barra { p0: [f64; 3], p1: [f64; 3], diam: f64 },
    /// Superfície do terreno: polilinha (x, y) estendida de z0 a z1.
    Superficie { linha: Vec<[f64; 2]>, z0: f64, z1: f64 },
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
            Forma::Superficie { linha, z0, z1 } => {
                let mut v = Vec::new();
                for s in linha.windows(2) {
                    let (a, b) = (s[0], s[1]);
                    v.push([pt(a[0], a[1], *z0), pt(b[0], b[1], *z0), pt(b[0], b[1], *z1)]);
                    v.push([pt(a[0], a[1], *z0), pt(b[0], b[1], *z1), pt(a[0], a[1], *z1)]);
                }
                v
            }
        }
    }
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
    pub fn montar(p: &Projeto, r: &Resultado, l: f64) -> Self {
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
                md.apoios_embutida(&e.parede, h, lt, l);
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
                md.apoios_embutida(&e.parede, h, lt, l);
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
                        md.pecas.push(Peca {
                            nome: format!("Tirante nível {} — trecho livre", k + 1),
                            classe: Classe::Tirante,
                            material: Material::TiranteLivre,
                            forma: Forma::Barra { p0: p3(rf.p0), p1: p3(fim(rf.livre)), diam: 0.04 },
                        });
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
                    }
                }
            }
        }
        for pc in &md.pecas {
            if let Forma::Barra { p1, .. } = pc.forma {
                amplia(p1[0]);
            }
        }

        // terreno: superfície do cálculo, recortada em torno da estrutura
        let folga = 0.8 * h.max(2.0);
        let (x0, x1) = (xmin - folga, xmax + folga);
        let mut linha: Vec<[f64; 2]> = Vec::new();
        let y_em = |x: f64| -> f64 {
            for s in m.sup.windows(2) {
                let (a, b) = (s[0], s[1]);
                if (a[0] <= x && x <= b[0]) && (b[0] - a[0]).abs() > 1e-12 {
                    return a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]);
                }
            }
            m.sup.last().map(|q| q[1]).unwrap_or(0.0)
        };
        linha.push([x0, y_em(x0)]);
        for q in &m.sup {
            if q[0] > x0 && q[0] < x1 {
                linha.push(*q);
            }
        }
        linha.push([x1, y_em(x1)]);
        md.pecas.push(Peca { nome: "Terreno".into(), classe: Classe::Terreno, material: Material::Terreno, forma: Forma::Superficie { linha, z0: 0.0, z1: l } });
        md
    }

    /// Tirantes/estroncas de paredes embutidas ancoradas (comprimento indicativo).
    fn apoios_embutida(&mut self, par: &crate::tipos::embutida::ParedeEmbutida, h: f64, lt: f64, l: f64) {
        if par.apoio != Apoio::Ancorada {
            return;
        }
        let a = par.incl_apoio.to_radians();
        let comp = (0.6 * lt).max(4.0).max(h);
        let livre = 0.45 * comp;
        let dir = [a.cos(), -a.sin()];
        let y0 = -par.prof_apoio;
        for z in posicoes(l, par.espac_apoio) {
            let q = |s: f64| pt(dir[0] * s, y0 + dir[1] * s, z);
            self.pecas.push(Peca { nome: "Tirante — trecho livre".into(), classe: Classe::Tirante, material: Material::TiranteLivre, forma: Forma::Barra { p0: q(0.0), p1: q(livre), diam: 0.04 } });
            self.pecas.push(Peca { nome: "Tirante — bulbo".into(), classe: Classe::Tirante, material: Material::Bulbo, forma: Forma::Barra { p0: q(livre), p1: q(comp), diam: 0.15 } });
        }
        self.notas.push(format!(
            "Tirantes da parede ancorada com comprimento indicativo ({} m): dimensionar o bulbo e o trecho livre conforme a NBR 5629.",
            crate::resultado::fmt(comp)
        ));
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
            let m = Modelo3d::montar(&p, &r, 10.0);
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
            }
        }
    }
}
