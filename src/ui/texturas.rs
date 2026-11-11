//! Texturas procedurais do modelo 3D (repetíveis sem emenda), simulando os
//! materiais reais: concreto, brita, manta geotêxtil, solos (aterro escuro,
//! argila vermelha, areia, silte), alvenaria de pedra e gabião.


pub const N: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tex {
    Concreto,
    Brita,
    Geotextil,
    SoloEscuro,
    Argila,
    Areia,
    Silte,
    Pedra,
    Gabiao,
    /// Blocos estruturais de concreto em amarração (ladrilho = 2 blocos × 4 fiadas = 0,8 m).
    Blocos,
}

pub const TODAS: [Tex; 10] = [Tex::Concreto, Tex::Brita, Tex::Geotextil, Tex::SoloEscuro, Tex::Argila, Tex::Areia, Tex::Silte, Tex::Pedra, Tex::Gabiao, Tex::Blocos];

// ---------- ruído repetível ----------

fn hash(x: i64, y: i64, s: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ s.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h % 1_000_003) as f32 / 1_000_003.0
}

/// Ruído de valor repetível com `p` células por lado (u, v em [0, 1)).
fn ruido(u: f32, v: f32, p: i64, s: u64) -> f32 {
    let (x, y) = (u * p as f32, v * p as f32);
    let (ix, iy) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let h = |a: i64, b: i64| hash(a.rem_euclid(p), b.rem_euclid(p), s);
    let a = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let b = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

/// Soma de oitavas (fbm), em [0, 1].
fn fbm(u: f32, v: f32, p: i64, oitavas: u32, s: u64) -> f32 {
    let (mut t, mut amp, mut soma, mut pp) = (0.0, 0.5, 0.0, p);
    for k in 0..oitavas {
        t += amp * ruido(u, v, pp, s + k as u64);
        soma += amp;
        amp *= 0.5;
        pp *= 2;
    }
    t / soma
}

/// Voronoi repetível: (id da célula, d1, d2) em unidades de célula.
fn voronoi(u: f32, v: f32, p: i64, s: u64) -> (u64, f32, f32) {
    let (id, d1, d2, _) = voronoi_vet(u, v, p, s);
    (id, d1, d2)
}

/// Voronoi com o vetor do pixel ao centro da célula (para relevo).
fn voronoi_vet(u: f32, v: f32, p: i64, s: u64) -> (u64, f32, f32, [f32; 2]) {
    let (x, y) = (u * p as f32, v * p as f32);
    let (ix, iy) = (x.floor() as i64, y.floor() as i64);
    let (mut d1, mut d2, mut id, mut vet) = (9.0f32, 9.0f32, 0u64, [0.0f32; 2]);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (cx, cy) = (ix + dx, iy + dy);
            let (wx, wy) = (cx.rem_euclid(p), cy.rem_euclid(p));
            let px = cx as f32 + 0.1 + 0.8 * hash(wx, wy, s);
            let py = cy as f32 + 0.1 + 0.8 * hash(wx, wy, s + 7);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = (wx * 7919 + wy) as u64;
                vet = [x - px, y - py];
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (id, d1, d2, vet)
}


fn mult(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn mistura(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn imagem(f: impl Fn(f32, f32, usize, usize) -> [f32; 3]) -> Vec<[f32; 3]> {
    let mut v = Vec::with_capacity(N * N);
    for j in 0..N {
        for i in 0..N {
            v.push(f(i as f32 / N as f32, j as f32 / N as f32, i, j));
        }
    }
    v
}

/// Traços (fibras, fissuras) somados à imagem, com repetição nas bordas.
fn tracos(img: &mut [[f32; 3]], n: usize, comp: (f32, f32), delta: (f32, f32), s: u64, curvo: bool) {
    for k in 0..n {
        let k = k as i64;
        let (mut x, mut y) = (hash(k, 1, s) * N as f32, hash(k, 2, s) * N as f32);
        let mut ang = hash(k, 3, s) * std::f32::consts::TAU;
        let l = comp.0 + (comp.1 - comp.0) * hash(k, 4, s);
        let d = delta.0 + (delta.1 - delta.0) * hash(k, 5, s);
        let passos = l as i64;
        for t in 0..passos {
            let (i, j) = ((x as i64).rem_euclid(N as i64) as usize, (y as i64).rem_euclid(N as i64) as usize);
            let p = &mut img[j * N + i];
            *p = [p[0] + d, p[1] + d, p[2] + d];
            if curvo {
                ang += (hash(k, 100 + t, s) - 0.5) * 0.9;
            }
            x += ang.cos();
            y += ang.sin();
        }
    }
}

fn gerar(t: Tex) -> Vec<[f32; 3]> {
    match t {
        Tex::Concreto => {
            let mut img = imagem(|u, v, i, j| {
                let m = fbm(u, v, 4, 5, 11) - 0.5;
                let g = fbm(u, v, 64, 2, 12) - 0.5;
                let mut c = 0.58 + 0.22 * m + 0.08 * g;
                let poro = hash(i as i64, j as i64, 13);
                if poro > 0.9965 {
                    c *= 0.45;
                } else if poro < 0.004 {
                    c *= 1.15;
                }
                [c, c, c * 0.98]
            });
            tracos(&mut img, 9, (120.0, 380.0), (-0.22, -0.12), 14, true);
            img
        }
        Tex::Brita => imagem(|u, v, i, j| {
            let (id, d1, d2, vet) = voronoi_vet(u, v, 18, 21);
            let borda = d2 - d1;
            if borda < 0.06 {
                return [0.13, 0.13, 0.14];
            }
            // pedra britada: tom próprio, faces facetadas e luz vinda de cima à esquerda
            let tom = 0.62 + 0.22 * hash(id as i64, 0, 22);
            let ang = hash(id as i64, 9, 24) * 6.283;
            let faceta = ((vet[0] * ang.cos() + vet[1] * ang.sin()) * 3.0).floor() * 0.05;
            let relevo = 0.92 - 0.40 * (vet[0] * 0.6 + vet[1] * 0.8) - 0.20 * d1 + faceta;
            let pinta = hash(i as i64, j as i64, 23);
            let mut c = tom * relevo;
            if pinta < 0.10 {
                c *= 0.6;
            } else if pinta > 0.96 {
                c = (c * 1.2).min(0.97);
            }
            let sombra = ((borda - 0.06) / 0.10).clamp(0.0, 1.0);
            let c = c * (0.50 + 0.50 * sombra);
            [c, c, c * 1.01]
        }),
        Tex::Geotextil => {
            let mut img = imagem(|u, v, _, _| {
                let c = 0.84 + 0.05 * (fbm(u, v, 32, 3, 31) - 0.5);
                [c, c, c * 1.01]
            });
            tracos(&mut img, 9000, (6.0, 22.0), (-0.07, 0.06), 32, true);
            img
        }
        Tex::SoloEscuro => imagem(|u, v, i, j| {
            let base = [0.26, 0.19, 0.14];
            let m = fbm(u, v, 6, 4, 41);
            let (id, d1, _) = voronoi(u, v, 48, 42);
            let torrao = if d1 < 0.30 { 0.10 * (0.30 - d1) / 0.30 * (hash(id as i64, 1, 43) - 0.3) } else { 0.0 };
            let g = hash(i as i64, j as i64, 44);
            let mut c = mult(base, 0.80 + 0.45 * m + torrao * 3.0 + 0.18 * (g - 0.5));
            if g > 0.992 {
                c = mistura(c, [0.75, 0.62, 0.40], 0.6); // grãos de areia brilhando
            }
            c
        }),
        Tex::Argila => imagem(|u, v, i, j| {
            let escura = [0.62, 0.33, 0.18];
            let clara = [0.85, 0.58, 0.38];
            let m = fbm(u, v, 3, 4, 51);
            let mut c = mistura(escura, clara, (m - 0.42) * 2.2);
            let (id, d1, _) = voronoi(u, v, 30, 52);
            if d1 < 0.22 && hash(id as i64, 2, 53) > 0.55 {
                // torrões: relevo com luz de cima
                let k = 1.0 + 0.35 * (0.22 - d1) / 0.22;
                c = mult(c, k);
            }
            let g = hash(i as i64, j as i64, 54);
            mult(c, 0.90 + 0.18 * g)
        }),
        Tex::Areia => imagem(|u, v, i, j| {
            let m = fbm(u, v, 12, 3, 61);
            let g = hash(i as i64, j as i64, 62);
            mult([0.84, 0.73, 0.53], 0.86 + 0.12 * m + 0.20 * (g - 0.5))
        }),
        Tex::Silte => imagem(|u, v, i, j| {
            let m = fbm(u, v, 5, 4, 71);
            let g = hash(i as i64, j as i64, 72);
            mult(mistura([0.58, 0.45, 0.30], [0.76, 0.63, 0.44], m * 1.4 - 0.2), 0.92 + 0.12 * g)
        }),
        Tex::Pedra => imagem(|u, v, _, _| {
            let (id, d1, d2) = voronoi(u, v, 7, 81);
            if d2 - d1 < 0.09 {
                let a = 0.70 + 0.06 * fbm(u, v, 64, 2, 82);
                return [a, a * 0.98, a * 0.93]; // argamassa
            }
            let tom = 0.75 + 0.35 * hash(id as i64, 3, 83);
            let det = fbm(u, v, 48, 3, 84);
            mult([0.52, 0.47, 0.40], tom * (0.85 + 0.30 * det) * (1.08 - 0.25 * d1))
        }),
        Tex::Blocos => imagem(|u, v, i, j| {
            // 4 fiadas de 19 cm + junta de 1 cm ; blocos de 39 cm + junta, desencontrados meia peça
            let fiada = (v * 4.0).floor();
            let fv = v * 4.0 - fiada;
            let desloc = if fiada as i64 % 2 == 1 { 0.25 } else { 0.0 };
            let fu = ((u + desloc) * 2.0).fract();
            let junta_h = fv < 0.05;
            let junta_v = fu < 0.025;
            if junta_h || junta_v {
                let a = 0.74 + 0.04 * fbm(u, v, 64, 2, 101);
                return [a, a * 0.99, a * 0.96];
            }
            let bloco = ((u + desloc) * 2.0).floor() as i64 + 7 * fiada as i64;
            let tom = 0.60 + 0.06 * hash(bloco, 5, 102);
            let g = hash(i as i64, j as i64, 103);
            let poro = if g > 0.985 { 0.75 } else { 1.0 };
            let c = tom * (0.94 + 0.10 * fbm(u, v, 48, 3, 104)) * (0.95 + 0.08 * g) * poro;
            [c, c, c * 0.98]
        }),
        Tex::Gabiao => imagem(|u, v, _, _| {
            // malha hexagonal de arame (aprox. losangos) sobre pedras de mão
            let (a, b) = ((u + v) * 8.0, (u - v + 1.0) * 8.0);
            let fio = |t: f32| {
                let f = t - t.floor();
                f.min(1.0 - f) < 0.035
            };
            if fio(a) || fio(b) {
                return [0.62, 0.64, 0.66];
            }
            let (id, d1, d2) = voronoi(u, v, 12, 91);
            if d2 - d1 < 0.10 {
                return [0.12, 0.11, 0.10]; // vazios entre as pedras
            }
            let tom = 0.70 + 0.40 * hash(id as i64, 4, 92);
            let det = fbm(u, v, 64, 2, 93);
            mult([0.55, 0.52, 0.48], tom * (0.85 + 0.25 * det) * (1.10 - 0.35 * d1))
        }),
    }
}

/// Textura pronta para amostragem (RGB 8 bits, N × N), gerada na primeira vez
/// que é usada e guardada para o resto da sessão.
pub fn pixels(t: Tex) -> &'static [[u8; 3]] {
    use std::sync::OnceLock;
    static CACHE: [OnceLock<Vec<[u8; 3]>>; 10] = [const { OnceLock::new() }; 10];
    let i = TODAS.iter().position(|x| *x == t).unwrap_or(0);
    CACHE[i].get_or_init(|| {
        gerar(t).into_iter().map(|c| [(c[0].clamp(0.0, 1.0) * 255.0) as u8, (c[1].clamp(0.0, 1.0) * 255.0) as u8, (c[2].clamp(0.0, 1.0) * 255.0) as u8]).collect()
    })
}

/// Amostra a textura em (u, v) com repetição (vizinho mais próximo).
#[inline]
pub fn amostrar(px: &[[u8; 3]], u: f32, v: f32) -> [u8; 3] {
    let i = ((u - u.floor()) * N as f32) as usize % N;
    let j = ((v - v.floor()) * N as f32) as usize % N;
    px[j * N + i]
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn texturas_repetem_sem_emenda() {
        // o ruído e o Voronoi são periódicos: valor em u e em u + 1 coincidem
        for &(u, v) in &[(0.01f32, 0.37f32), (0.99, 0.5), (0.5, 0.999)] {
            assert!((ruido(u, v, 8, 1) - ruido(u + 1.0, v, 8, 1)).abs() < 1e-4);
            let (a, ..) = voronoi(u, v, 10, 2);
            let (b, ..) = voronoi(u + 1.0, v + 1.0, 10, 2);
            assert_eq!(a, b);
        }
        for t in TODAS {
            let img = gerar(t);
            assert_eq!(img.len(), N * N);
            assert!(img.iter().all(|c| c.iter().all(|x| x.is_finite())), "{t:?}");
        }
    }
}

#[cfg(test)]
mod amostra {
    /// Grava uma prancha com as texturas (conferência visual): TEXTURAS_PNG=caminho.
    #[test]
    #[ignore]
    fn prancha() {
        let Ok(caminho) = std::env::var("TEXTURAS_PNG") else { return };
        let lado = 256;
        let (w, h) = (lado * 5, lado * 2);
        let mut px = vec![0u8; w * h * 3];
        for (k, t) in super::TODAS.iter().enumerate() {
            let img = super::gerar(*t);
            let (ox, oy) = ((k % 5) * lado, (k / 5) * lado);
            for j in 0..lado {
                for i in 0..lado {
                    let c = img[j * super::N + i];
                    let o = ((oy + j) * w + ox + i) * 3;
                    for q in 0..3 {
                        px[o + q] = (c[q].clamp(0.0, 1.0) * 255.0) as u8;
                    }
                }
            }
        }
        let im = crate::raster::Imagem { w, h, px };
        std::fs::write(caminho, im.png()).unwrap();
    }
}
