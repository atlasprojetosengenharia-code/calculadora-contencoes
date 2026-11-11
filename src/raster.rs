//! Rasterizador mínimo da seção transversal para PNG (sem dependências):
//! polígonos, linhas com suavização, setas e texto em fonte bitmap 5×7.
//! Usado como imagem de reserva no memorial .docx (o mesmo desenho segue
//! também em SVG, vetorial, para o Word atual).

use crate::resultado::{Desenho, Estilo};

pub struct Imagem {
    pub w: usize,
    pub h: usize,
    pub(crate) px: Vec<u8>,
}

/// Fonte 5×7 clássica (ASCII 32–126), uma coluna por byte, bit 0 = topo.
const FONTE: [[u8; 5]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], [0x00, 0x00, 0x5F, 0x00, 0x00], [0x00, 0x07, 0x00, 0x07, 0x00], [0x14, 0x7F, 0x14, 0x7F, 0x14],
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], [0x23, 0x13, 0x08, 0x64, 0x62], [0x36, 0x49, 0x56, 0x20, 0x50], [0x00, 0x08, 0x07, 0x03, 0x00],
    [0x00, 0x1C, 0x22, 0x41, 0x00], [0x00, 0x41, 0x22, 0x1C, 0x00], [0x2A, 0x1C, 0x7F, 0x1C, 0x2A], [0x08, 0x08, 0x3E, 0x08, 0x08],
    [0x00, 0x80, 0x70, 0x30, 0x00], [0x08, 0x08, 0x08, 0x08, 0x08], [0x00, 0x00, 0x60, 0x60, 0x00], [0x20, 0x10, 0x08, 0x04, 0x02],
    [0x3E, 0x51, 0x49, 0x45, 0x3E], [0x00, 0x42, 0x7F, 0x40, 0x00], [0x72, 0x49, 0x49, 0x49, 0x46], [0x21, 0x41, 0x49, 0x4D, 0x33],
    [0x18, 0x14, 0x12, 0x7F, 0x10], [0x27, 0x45, 0x45, 0x45, 0x39], [0x3C, 0x4A, 0x49, 0x49, 0x31], [0x41, 0x21, 0x11, 0x09, 0x07],
    [0x36, 0x49, 0x49, 0x49, 0x36], [0x46, 0x49, 0x49, 0x29, 0x1E], [0x00, 0x00, 0x14, 0x00, 0x00], [0x00, 0x40, 0x34, 0x00, 0x00],
    [0x00, 0x08, 0x14, 0x22, 0x41], [0x14, 0x14, 0x14, 0x14, 0x14], [0x00, 0x41, 0x22, 0x14, 0x08], [0x02, 0x01, 0x59, 0x09, 0x06],
    [0x3E, 0x41, 0x5D, 0x59, 0x4E], [0x7C, 0x12, 0x11, 0x12, 0x7C], [0x7F, 0x49, 0x49, 0x49, 0x36], [0x3E, 0x41, 0x41, 0x41, 0x22],
    [0x7F, 0x41, 0x41, 0x41, 0x3E], [0x7F, 0x49, 0x49, 0x49, 0x41], [0x7F, 0x09, 0x09, 0x09, 0x01], [0x3E, 0x41, 0x41, 0x51, 0x73],
    [0x7F, 0x08, 0x08, 0x08, 0x7F], [0x00, 0x41, 0x7F, 0x41, 0x00], [0x20, 0x40, 0x41, 0x3F, 0x01], [0x7F, 0x08, 0x14, 0x22, 0x41],
    [0x7F, 0x40, 0x40, 0x40, 0x40], [0x7F, 0x02, 0x1C, 0x02, 0x7F], [0x7F, 0x04, 0x08, 0x10, 0x7F], [0x3E, 0x41, 0x41, 0x41, 0x3E],
    [0x7F, 0x09, 0x09, 0x09, 0x06], [0x3E, 0x41, 0x51, 0x21, 0x5E], [0x7F, 0x09, 0x19, 0x29, 0x46], [0x26, 0x49, 0x49, 0x49, 0x32],
    [0x03, 0x01, 0x7F, 0x01, 0x03], [0x3F, 0x40, 0x40, 0x40, 0x3F], [0x1F, 0x20, 0x40, 0x20, 0x1F], [0x3F, 0x40, 0x38, 0x40, 0x3F],
    [0x63, 0x14, 0x08, 0x14, 0x63], [0x03, 0x04, 0x78, 0x04, 0x03], [0x61, 0x59, 0x49, 0x4D, 0x43], [0x00, 0x7F, 0x41, 0x41, 0x00],
    [0x02, 0x04, 0x08, 0x10, 0x20], [0x00, 0x41, 0x41, 0x7F, 0x00], [0x04, 0x02, 0x01, 0x02, 0x04], [0x40, 0x40, 0x40, 0x40, 0x40],
    [0x00, 0x01, 0x02, 0x04, 0x00], [0x20, 0x54, 0x54, 0x54, 0x78], [0x7F, 0x48, 0x44, 0x44, 0x38], [0x38, 0x44, 0x44, 0x44, 0x28],
    [0x38, 0x44, 0x44, 0x48, 0x7F], [0x38, 0x54, 0x54, 0x54, 0x18], [0x08, 0x7E, 0x09, 0x01, 0x02], [0x08, 0x14, 0x54, 0x54, 0x3C],
    [0x7F, 0x08, 0x04, 0x04, 0x78], [0x00, 0x44, 0x7D, 0x40, 0x00], [0x20, 0x40, 0x44, 0x3D, 0x00], [0x7F, 0x10, 0x28, 0x44, 0x00],
    [0x00, 0x41, 0x7F, 0x40, 0x00], [0x7C, 0x04, 0x78, 0x04, 0x78], [0x7C, 0x08, 0x04, 0x04, 0x78], [0x38, 0x44, 0x44, 0x44, 0x38],
    [0x7C, 0x14, 0x14, 0x14, 0x08], [0x08, 0x14, 0x14, 0x18, 0x7C], [0x7C, 0x08, 0x04, 0x04, 0x08], [0x48, 0x54, 0x54, 0x54, 0x24],
    [0x04, 0x3F, 0x44, 0x40, 0x20], [0x3C, 0x40, 0x40, 0x20, 0x7C], [0x1C, 0x20, 0x40, 0x20, 0x1C], [0x3C, 0x40, 0x30, 0x40, 0x3C],
    [0x44, 0x28, 0x10, 0x28, 0x44], [0x4C, 0x50, 0x50, 0x50, 0x3C], [0x44, 0x64, 0x54, 0x4C, 0x44], [0x00, 0x08, 0x36, 0x41, 0x00],
    [0x00, 0x00, 0x7F, 0x00, 0x00], [0x00, 0x41, 0x36, 0x08, 0x00], [0x08, 0x04, 0x08, 0x10, 0x08],
];

/// Letra equivalente (sem acento) para a fonte ASCII.
fn ascii(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' => 'e',
        'í' | 'ì' | 'î' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'Á' | 'À' | 'Â' | 'Ã' => 'A',
        'É' | 'Ê' => 'E',
        'Í' => 'I',
        'Ó' | 'Ô' | 'Õ' | 'Ø' => 'O',
        'Ú' => 'U',
        'Ç' => 'C',
        'γ' => 'g',
        'φ' => 'f',
        'β' => 'b',
        'ω' => 'w',
        'θ' => 'q',
        'σ' | 'Σ' => 'S',
        'δ' => 'd',
        '·' | '×' => '*',
        '−' | '–' | '—' => '-',
        '≤' => '<',
        '≥' => '>',
        '±' => '+',
        '²' | '³' => '^',
        '°' => 'o',
        'ⓘ' => 'i',
        c if (' '..='~').contains(&c) => c,
        _ => '?',
    }
}

impl Imagem {
    pub fn nova(w: usize, h: usize, fundo: [u8; 3]) -> Self {
        let mut px = Vec::with_capacity(w * h * 3);
        for _ in 0..w * h {
            px.extend_from_slice(&fundo);
        }
        Self { w, h, px }
    }

    fn misturar(&mut self, x: i32, y: i32, cor: [u8; 3], a: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || a <= 0.0 {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 3;
        let a = a.min(1.0);
        for k in 0..3 {
            self.px[i + k] = (self.px[i + k] as f32 * (1.0 - a) + cor[k] as f32 * a).round() as u8;
        }
    }

    /// Segmento com largura `larg` (px) e bordas suavizadas.
    pub fn linha(&mut self, a: (f32, f32), b: (f32, f32), larg: f32, cor: [u8; 3], alfa: f32) {
        let r = larg.max(0.8) / 2.0;
        let (x0, x1) = (a.0.min(b.0) - r - 1.0, a.0.max(b.0) + r + 1.0);
        let (y0, y1) = (a.1.min(b.1) - r - 1.0, a.1.max(b.1) + r + 1.0);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = (dx * dx + dy * dy).max(1e-6);
        for y in (y0.floor().max(0.0) as i32)..=(y1.ceil().min(self.h as f32 - 1.0) as i32) {
            for x in (x0.floor().max(0.0) as i32)..=(x1.ceil().min(self.w as f32 - 1.0) as i32) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = (((px - a.0) * dx + (py - a.1) * dy) / l2).clamp(0.0, 1.0);
                let d = ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt();
                let cob = (r + 0.5 - d).clamp(0.0, 1.0);
                if cob > 0.0 {
                    self.misturar(x, y, cor, cob * alfa);
                }
            }
        }
    }

    /// Preenchimento de polígono (par-ímpar, sem suavização; o contorno vai em `linha`).
    pub fn poligono(&mut self, pts: &[(f32, f32)], cor: [u8; 3], alfa: f32) {
        if pts.len() < 3 || alfa <= 0.0 {
            return;
        }
        let (ymin, ymax) = pts.iter().fold((f32::MAX, f32::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
        for y in (ymin.floor().max(0.0) as i32)..=(ymax.ceil().min(self.h as f32 - 1.0) as i32) {
            let yc = y as f32 + 0.5;
            let mut xs = Vec::new();
            for i in 0..pts.len() {
                let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
                if (p.1 <= yc && q.1 > yc) || (q.1 <= yc && p.1 > yc) {
                    xs.push(p.0 + (yc - p.1) / (q.1 - p.1) * (q.0 - p.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for par in xs.chunks(2) {
                if let [a, b] = par {
                    for x in (a.round() as i32)..(b.round() as i32) {
                        self.misturar(x, y, cor, alfa);
                    }
                }
            }
        }
    }

    /// Texto centralizado em `(cx, cy)`; `esc` = pixels por ponto da fonte.
    pub fn texto(&mut self, cx: f32, cy: f32, t: &str, esc: i32, cor: [u8; 3]) {
        let n = t.chars().count() as i32;
        let largura = n * 6 * esc;
        let (x0, y0) = (cx as i32 - largura / 2, cy as i32 - 4 * esc);
        for (k, ch) in t.chars().enumerate() {
            let c = ascii(ch);
            let glifo = FONTE[(c as usize).saturating_sub(32).min(94)];
            for (col, bits) in glifo.iter().enumerate() {
                for lin in 0..7 {
                    if bits >> lin & 1 == 1 {
                        for dx in 0..esc {
                            for dy in 0..esc {
                                self.misturar(x0 + (k as i32 * 6 + col as i32) * esc + dx, y0 + lin * esc + dy, cor, 1.0);
                            }
                        }
                    }
                }
            }
        }
    }

    /// PNG (RGB, 8 bits) com deflate de códigos fixos e repetições (distância 1).
    pub fn png(&self) -> Vec<u8> {
        let mut bruto = Vec::with_capacity((self.w * 3 + 1) * self.h);
        for y in 0..self.h {
            bruto.push(0);
            bruto.extend_from_slice(&self.px[y * self.w * 3..(y + 1) * self.w * 3]);
        }
        let mut z = vec![0x78, 0x01];
        z.extend(deflate_fixo(&bruto));
        z.extend(adler32(&bruto).to_be_bytes());
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend((self.w as u32).to_be_bytes());
        ihdr.extend((self.h as u32).to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        bloco(&mut out, b"IHDR", &ihdr);
        bloco(&mut out, b"IDAT", &z);
        bloco(&mut out, b"IEND", &[]);
        out
    }
}

fn bloco(out: &mut Vec<u8>, tipo: &[u8; 4], dados: &[u8]) {
    out.extend((dados.len() as u32).to_be_bytes());
    let mut c = tipo.to_vec();
    c.extend(dados);
    out.extend(&c);
    out.extend(crc32(&c).to_be_bytes());
}

pub fn crc32(d: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in d {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Escritor de bits (LSB primeiro), como exige o deflate.
struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    fn bits(&mut self, valor: u32, n: u32) {
        self.acc |= valor << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// Código de Huffman (enviado do bit mais significativo para o menos).
    fn huff(&mut self, codigo: u32, n: u32) {
        let mut r = 0;
        for i in 0..n {
            r |= ((codigo >> i) & 1) << (n - 1 - i);
        }
        self.bits(r, n);
    }
}

fn simbolo_fixo(b: &mut Bits, v: u32) {
    match v {
        0..=143 => b.huff(0x30 + v, 8),
        144..=255 => b.huff(0x190 + (v - 144), 9),
        256..=279 => b.huff(v - 256, 7),
        _ => b.huff(0xC0 + (v - 280), 8),
    }
}

fn deflate_fixo(d: &[u8]) -> Vec<u8> {
    // (comprimento mínimo, bits extras) dos códigos 257..285
    const LEN: [(u32, u32); 29] = [
        (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 1), (13, 1), (15, 1), (17, 1), (19, 2), (23, 2),
        (27, 2), (31, 2), (35, 3), (43, 3), (51, 3), (59, 3), (67, 4), (83, 4), (99, 4), (115, 4), (131, 5), (163, 5), (195, 5),
        (227, 5), (258, 0),
    ];
    let mut b = Bits { out: Vec::new(), acc: 0, n: 0 };
    b.bits(1, 1); // bloco final
    b.bits(1, 2); // Huffman fixo
    let mut i = 0;
    while i < d.len() {
        // repetição do byte anterior (distância 1)
        let mut run = 0;
        if i > 0 {
            while i + run < d.len() && run < 258 && d[i + run] == d[i - 1] {
                run += 1;
            }
        }
        if run >= 3 {
            let k = LEN.iter().rposition(|&(m, _)| m <= run as u32).unwrap();
            simbolo_fixo(&mut b, 257 + k as u32);
            if LEN[k].1 > 0 {
                b.bits(run as u32 - LEN[k].0, LEN[k].1);
            }
            b.huff(0, 5); // distância 1
            i += run;
        } else {
            simbolo_fixo(&mut b, d[i] as u32);
            i += 1;
        }
    }
    simbolo_fixo(&mut b, 256);
    if b.n > 0 {
        b.out.push(b.acc as u8);
    }
    b.out
}

fn rgb(c: eframe::egui::Color32) -> ([u8; 3], f32) {
    let a = c.a() as f32 / 255.0;
    if c.a() == 0 {
        return ([0, 0, 0], 0.0);
    }
    let f = 255.0 / c.a() as f32;
    ([(c.r() as f32 * f).min(255.0) as u8, (c.g() as f32 * f).min(255.0) as u8, (c.b() as f32 * f).min(255.0) as u8], a)
}

/// Altura (px) de uma imagem de largura `w` que acomoda o desenho com a sua proporção.
pub fn altura_para(d: &Desenho, w: usize) -> usize {
    let mut pts: Vec<[f64; 2]> = d.poligonos.iter().flat_map(|p| p.pontos.iter().copied()).collect();
    pts.extend(d.linhas.iter().flat_map(|l| l.pontos.iter().copied()));
    pts.extend(d.textos.iter().map(|t| t.pos));
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in pts.iter().filter(|p| p[0].is_finite() && p[1].is_finite()) {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    if !(x1 > x0 && y1 > y0) {
        return w * 5 / 8;
    }
    let esc = (w as f64 - 80.0) / (x1 - x0);
    (((y1 - y0) * esc + 80.0) as usize).clamp(w / 4, w * 3 / 2)
}

/// Seção transversal (tema claro) como PNG.
pub fn desenho_png(d: &Desenho, w: usize, h: usize) -> Vec<u8> {
    let mut img = Imagem::nova(w, h, [255, 255, 255]);
    let mut pts: Vec<[f64; 2]> = d.poligonos.iter().flat_map(|p| p.pontos.iter().copied()).collect();
    pts.extend(d.linhas.iter().flat_map(|l| l.pontos.iter().copied()));
    pts.extend(d.setas.iter().flat_map(|(a, b)| [*a, *b]));
    pts.extend(d.textos.iter().map(|t| t.pos));
    let pts: Vec<[f64; 2]> = pts.into_iter().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
    if pts.is_empty() {
        return img.png();
    }
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in &pts {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    let margem = 40.0;
    let esc = ((w as f64 - 2.0 * margem) / (x1 - x0).max(1e-6)).min((h as f64 - 2.0 * margem) / (y1 - y0).max(1e-6));
    let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let t = |p: [f64; 2]| (((w as f64 / 2.0) + (p[0] - mx) * esc) as f32, ((h as f64 / 2.0) - (p[1] - my) * esc) as f32);

    for p in &d.poligonos {
        let (fill, stroke) = super::desenho::cor_claro(p.estilo);
        let v: Vec<(f32, f32)> = p.pontos.iter().map(|&q| t(q)).collect();
        let (c, a) = rgb(fill);
        img.poligono(&v, c, a);
        if !matches!(p.estilo, Estilo::Empuxo | Estilo::EmpuxoNegativo) {
            let (cs, as_) = rgb(stroke.color);
            for i in 0..v.len() {
                img.linha(v[i], v[(i + 1) % v.len()], stroke.width, cs, as_);
            }
        }
    }
    for l in &d.linhas {
        let (_, stroke) = super::desenho::cor_claro(l.estilo);
        let (cs, as_) = rgb(stroke.color);
        let v: Vec<(f32, f32)> = l.pontos.iter().map(|&q| t(q)).collect();
        for par in v.windows(2) {
            if l.tracejada {
                let (dx, dy) = (par[1].0 - par[0].0, par[1].1 - par[0].1);
                let comp = (dx * dx + dy * dy).sqrt();
                let n = (comp / 9.0).ceil().max(1.0) as i32;
                for k in 0..n {
                    if k % 2 == 0 {
                        let (u0, u1) = (k as f32 / n as f32, ((k + 1) as f32 / n as f32).min(1.0));
                        img.linha((par[0].0 + dx * u0, par[0].1 + dy * u0), (par[0].0 + dx * u1, par[0].1 + dy * u1), stroke.width, cs, as_);
                    }
                }
            } else {
                img.linha(par[0], par[1], stroke.width, cs, as_);
            }
        }
    }
    let (_, carga) = super::desenho::cor_claro(Estilo::Carga);
    let (cc, ca) = rgb(carga.color);
    for (a, b) in &d.setas {
        let (pa, pb) = (t(*a), t(*b));
        img.linha(pa, pb, 1.5, cc, ca);
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let n = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / n, dy / n);
        for s in [1.0f32, -1.0] {
            img.linha(pb, (pb.0 - 7.0 * ux + s * 3.5 * uy, pb.1 - 7.0 * uy - s * 3.5 * ux), 1.5, cc, ca);
        }
    }
    for tx in &d.textos {
        let (cx, cy) = t(tx.pos);
        img.texto(cx, cy, &tx.texto, 1, [31, 35, 40]);
    }
    img.png()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn png_valido_e_comprimido() {
        let mut img = Imagem::nova(64, 48, [255, 255, 255]);
        img.linha((2.0, 2.0), (60.0, 40.0), 2.0, [200, 0, 0], 1.0);
        img.poligono(&[(10.0, 10.0), (30.0, 10.0), (20.0, 30.0)], [0, 0, 200], 1.0);
        img.texto(32.0, 24.0, "H = 5,00 m", 1, [0, 0, 0]);
        let png = img.png();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert!(png.len() < 64 * 48 * 3 / 2, "fundo branco deve comprimir: {} bytes", png.len());
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
