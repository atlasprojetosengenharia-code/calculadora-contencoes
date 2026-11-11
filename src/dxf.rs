//! Exportação da seção transversal e da armação em DXF R12, compatível com
//! AutoCAD e demais programas CAD (abrir e "Salvar como" .dwg). Unidades:
//! metros. Uma camada por tipo de elemento. Texto em Windows-1252
//! ($DWGCODEPAGE = ANSI_1252); símbolos fora dessa tabela como \U+XXXX.

use crate::armadura::Armadura;
use crate::resultado::{Desenho, Estilo};
use std::fmt::Write;

/// (nome da camada, cor ACI) por estilo de desenho.
fn camada(e: Estilo) -> (&'static str, u8) {
    match e {
        Estilo::Concreto => ("CONCRETO", 8),
        Estilo::Aco => ("ACO", 5),
        Estilo::Solo => ("SOLO", 42),
        Estilo::SoloFundacao => ("SOLO-FUNDACAO", 32),
        Estilo::Agua => ("AGUA", 150),
        Estilo::Tirante => ("TIRANTE", 1),
        Estilo::Bulbo => ("BULBO", 14),
        Estilo::Empuxo | Estilo::EmpuxoNegativo => ("DIAGRAMA", 30),
        Estilo::Carga => ("CARGA", 30),
        Estilo::Ruptura => ("RUPTURA", 6),
        Estilo::Momento => ("MOMENTO", 200),
        Estilo::Cota => ("COTA", 9),
        Estilo::Dreno => ("DRENAGEM", 4),
        Estilo::Barbaca => ("DRENAGEM", 4),
        Estilo::Armadura | Estilo::ArmaduraSecao => ("ARMADURA", 1),
        Estilo::Hachura => ("HACHURA", 253),
        Estilo::Reaterro => ("REATERRO", 44),
    }
}

const CAMADAS: [(&str, u8); 20] = [
    ("CONCRETO", 8),
    ("ACO", 5),
    ("SOLO", 42),
    ("SOLO-FUNDACAO", 32),
    ("AGUA", 150),
    ("TIRANTE", 1),
    ("BULBO", 14),
    ("DIAGRAMA", 30),
    ("CARGA", 30),
    ("RUPTURA", 6),
    ("MOMENTO", 200),
    ("COTA", 9),
    ("DRENAGEM", 4),
    ("ARMADURA", 1),
    ("HACHURA", 253),
    ("REATERRO", 44),
    ("TEXTO", 7),
    ("TABELA", 7),
    ("CARIMBO", 7),
    ("NOTAS", 7),
];

/// Byte Windows-1252 de um caractere, se representável.
pub(crate) fn cp1252(ch: char) -> Option<u8> {
    let c = ch as u32;
    match ch {
        _ if c < 0x80 => Some(c as u8),
        _ if (0xA0..=0xFF).contains(&c) => Some(c as u8),
        '€' => Some(0x80),
        '…' => Some(0x85),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '–' => Some(0x96),
        '—' => Some(0x97),
        _ => None,
    }
}

/// Texto DXF: quebras de linha viram espaço; caracteres fora do Windows-1252
/// viram \U+XXXX (interpretado pelo AutoCAD).
fn texto_dxf(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for ch in t.chars() {
        // códigos de controle do AutoCAD e equivalentes ASCII (fontes SHX não têm esses glifos)
        let sub = match ch {
            'Ø' | 'ø' | '∅' => Some("%%c"),
            '°' => Some("%%d"),
            '±' => Some("%%p"),
            '—' | '–' => Some("-"),
            '²' => Some("2"),
            '³' => Some("3"),
            '≈' => Some("~"),
            '·' => Some("."),
            '×' => Some("x"),
            '≥' => Some(">="),
            '≤' => Some("<="),
            _ => None,
        };
        if let Some(t) = sub {
            s.push_str(t);
        } else if ch == '\n' || ch == '\r' {
            s.push(' ');
        } else if cp1252(ch).is_some() {
            s.push(ch);
        } else {
            let _ = write!(s, "\\U+{:04X}", ch as u32);
        }
    }
    s
}

/// Largura aproximada de um texto em caracteres (escape \U+XXXX conta como 1).
fn largura(t: &str) -> usize {
    t.chars().count()
}

struct Dxf {
    s: String,
}

impl Dxf {
    fn par(&mut self, codigo: i32, valor: impl std::fmt::Display) {
        let _ = write!(self.s, "{codigo}\r\n{valor}\r\n");
    }
    fn num(&mut self, codigo: i32, v: f64) {
        let v = if v.is_finite() { v } else { 0.0 };
        self.par(codigo, format!("{v:.4}"));
    }
    fn linha(&mut self, lay: &str, a: [f64; 2], b: [f64; 2], tracejada: bool) {
        self.par(0, "LINE");
        self.par(8, lay);
        if tracejada {
            self.par(6, "DASHED");
        }
        self.num(10, a[0]);
        self.num(20, a[1]);
        self.num(30, 0.0);
        self.num(11, b[0]);
        self.num(21, b[1]);
        self.num(31, 0.0);
    }
    fn polilinha(&mut self, lay: &str, pts: &[[f64; 2]], fechada: bool, tracejada: bool) {
        if pts.len() < 2 {
            return;
        }
        self.par(0, "POLYLINE");
        self.par(8, lay);
        if tracejada {
            self.par(6, "DASHED");
        }
        self.par(66, 1);
        self.num(10, 0.0);
        self.num(20, 0.0);
        self.num(30, 0.0);
        self.par(70, if fechada { 1 } else { 0 });
        for p in pts {
            self.par(0, "VERTEX");
            self.par(8, lay);
            self.num(10, p[0]);
            self.num(20, p[1]);
            self.num(30, 0.0);
        }
        self.par(0, "SEQEND");
        self.par(8, lay);
    }
    fn circulo(&mut self, lay: &str, c: [f64; 2], r: f64) {
        self.par(0, "CIRCLE");
        self.par(8, lay);
        self.num(10, c[0]);
        self.num(20, c[1]);
        self.num(30, 0.0);
        self.num(40, r);
    }
    /// Texto; `centro` = alinhado pelo meio (72=1, 73=2).
    fn texto(&mut self, lay: &str, p: [f64; 2], altura: f64, t: &str, centro: bool) {
        self.par(0, "TEXT");
        self.par(8, lay);
        self.num(10, p[0]);
        self.num(20, p[1]);
        self.num(30, 0.0);
        self.num(40, altura);
        self.par(1, texto_dxf(t));
        if centro {
            self.par(72, 1);
            self.par(73, 2);
            self.num(11, p[0]);
            self.num(21, p[1]);
            self.num(31, 0.0);
        }
    }
}

/// Gera o DXF: seção, tabela de armação, notas e carimbo.
pub fn gerar(d: &Desenho, armaduras: &[Armadura], titulo: &str, notas: &[String], versao: &str) -> Vec<u8> {
    // extensão do desenho
    let mut pts: Vec<[f64; 2]> = d.poligonos.iter().flat_map(|p| p.pontos.iter().copied()).collect();
    pts.extend(d.linhas.iter().flat_map(|l| l.pontos.iter().copied()));
    pts.retain(|p| p[0].is_finite() && p[1].is_finite());
    let (mut x0, mut x1, mut y0, mut y1) = (0.0f64, 1.0f64, 0.0f64, 1.0f64);
    if !pts.is_empty() {
        x0 = pts.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
        x1 = pts.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
        y0 = pts.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
        y1 = pts.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
    }
    let tam = (x1 - x0).max(y1 - y0).max(1.0);
    let ht = (tam * 0.012).clamp(0.05, 0.5); // altura de texto (m)

    let mut f = Dxf { s: String::with_capacity(256 * 1024) };
    // HEADER
    f.par(0, "SECTION");
    f.par(2, "HEADER");
    f.par(9, "$ACADVER");
    f.par(1, "AC1009");
    f.par(9, "$DWGCODEPAGE");
    f.par(3, "ANSI_1252");
    f.par(9, "$INSBASE");
    f.num(10, 0.0);
    f.num(20, 0.0);
    f.num(30, 0.0);
    f.par(9, "$EXTMIN");
    f.num(10, x0 - tam);
    f.num(20, y0 - tam);
    f.num(30, 0.0);
    f.par(9, "$EXTMAX");
    f.num(10, x1 + tam);
    f.num(20, y1 + tam * 0.3);
    f.num(30, 0.0);
    f.par(9, "$LTSCALE");
    f.num(40, (tam / 20.0).max(0.05));
    f.par(0, "ENDSEC");
    // TABLES: tipos de linha e camadas
    f.par(0, "SECTION");
    f.par(2, "TABLES");
    f.par(0, "TABLE");
    f.par(2, "LTYPE");
    f.par(70, 2);
    f.par(0, "LTYPE");
    f.par(2, "CONTINUOUS");
    f.par(70, 0);
    f.par(3, "Solid line");
    f.par(72, 65);
    f.par(73, 0);
    f.num(40, 0.0);
    f.par(0, "LTYPE");
    f.par(2, "DASHED");
    f.par(70, 0);
    f.par(3, "__ __ __");
    f.par(72, 65);
    f.par(73, 2);
    f.num(40, 0.75);
    f.num(49, 0.5);
    f.num(49, -0.25);
    f.par(0, "ENDTAB");
    f.par(0, "TABLE");
    f.par(2, "LAYER");
    f.par(70, CAMADAS.len());
    for (nome, cor) in CAMADAS {
        f.par(0, "LAYER");
        f.par(2, nome);
        f.par(70, 0);
        f.par(62, cor);
        f.par(6, "CONTINUOUS");
    }
    f.par(0, "ENDTAB");
    f.par(0, "ENDSEC");

    // ENTITIES
    f.par(0, "SECTION");
    f.par(2, "ENTITIES");
    for p in &d.poligonos {
        // faixas do diagrama de pressões: só o contorno (desenhado como linha)
        if matches!(p.estilo, Estilo::Empuxo | Estilo::EmpuxoNegativo) {
            continue;
        }
        let (lay, _) = camada(p.estilo);
        if p.estilo == Estilo::ArmaduraSecao && p.pontos.len() >= 3 {
            // ponto de armadura em seção → círculo
            let n = p.pontos.len() as f64;
            let c = [p.pontos.iter().map(|q| q[0]).sum::<f64>() / n, p.pontos.iter().map(|q| q[1]).sum::<f64>() / n];
            let r = ((p.pontos[0][0] - c[0]).powi(2) + (p.pontos[0][1] - c[1]).powi(2)).sqrt();
            f.circulo(lay, c, r);
        } else {
            f.polilinha(lay, &p.pontos, true, false);
        }
    }
    for l in &d.linhas {
        let (lay, _) = camada(l.estilo);
        f.polilinha(lay, &l.pontos, false, l.tracejada);
    }
    for (a, b) in &d.setas {
        f.linha("CARGA", *a, *b, false);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let n = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (ux, uy) = (dx / n, dy / n);
        let t = ht * 0.8;
        f.polilinha(
            "CARGA",
            &[
                *b,
                [b[0] - t * ux + 0.4 * t * uy, b[1] - t * uy - 0.4 * t * ux],
                [b[0] - t * ux - 0.4 * t * uy, b[1] - t * uy + 0.4 * t * ux],
            ],
            true,
            false,
        );
    }
    for t in &d.textos {
        f.texto("TEXTO", t.pos, ht, &t.texto, true);
    }

    // Tabela de armação (abaixo do desenho)
    let mut y = y0 - 3.0 * ht;
    let xt = x0;
    if !armaduras.is_empty() {
        f.texto("TABELA", [xt, y], ht * 1.4, "TABELA DE ARMACAO (CA-50) - comprimentos aproximados", false);
        y -= 2.0 * ht;
        // colunas dimensionadas pelo texto mais longo (≈ 0,9·altura por caractere)
        let linhas: Vec<[String; 9]> = armaduras.iter().map(|a| a.celulas()).collect();
        let mut col = [0.0f64; 9];
        let mut x = 0.0;
        for i in 0..9 {
            col[i] = x;
            let w = linhas.iter().map(|l| largura(&l[i])).chain([largura(crate::armadura::CABECALHO[i])]).max().unwrap_or(4);
            x += (w as f64 + 2.0) * 0.9;
        }
        let larg_tab = x;
        for (c, t) in col.iter().zip(crate::armadura::CABECALHO) {
            f.texto("TABELA", [xt + c * ht, y], ht, t, false);
        }
        f.linha("TABELA", [xt, y - 0.4 * ht], [xt + larg_tab * ht, y - 0.4 * ht], false);
        y -= 1.6 * ht;
        for a in armaduras {
            let vals = a.celulas();
            for (c, t) in col.iter().zip(vals.iter()) {
                f.texto("TABELA", [xt + c * ht, y], ht, t, false);
            }
            y -= 1.5 * ht;
        }
        y -= ht;
    }
    // Notas
    if !notas.is_empty() {
        f.texto("NOTAS", [xt, y], ht * 1.2, "NOTAS", false);
        y -= 1.6 * ht;
        for n in notas {
            f.texto("NOTAS", [xt, y], ht * 0.9, &format!("- {n}"), false);
            y -= 1.3 * ht;
        }
        y -= ht;
    }
    // Carimbo
    let (cx, cw, ch) = (xt, 40.0 * ht, 5.0 * ht);
    let cy = y - ch;
    f.polilinha("CARIMBO", &[[cx, cy], [cx + cw, cy], [cx + cw, cy + ch], [cx, cy + ch]], true, false);
    f.texto("CARIMBO", [cx + ht, cy + ch - 1.5 * ht], ht * 1.2, titulo, false);
    f.texto("CARIMBO", [cx + ht, cy + ch - 3.0 * ht], ht * 0.9, &format!("Calculadora de Contencoes {versao} - unidades em metros"), false);
    f.texto(
        "CARIMBO",
        [cx + ht, cy + 0.8 * ht],
        ht * 0.8,
        "Esquema para projeto executivo - conferir cotas, cobrimentos e dobras antes da fabricacao",
        false,
    );
    f.par(0, "ENDSEC");
    f.par(0, "EOF");
    f.s.chars().map(|ch| cp1252(ch).unwrap_or(b'?')).collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::armadura::Forma;

    #[test]
    fn dxf_r12_valido() {
        let mut d = Desenho::default();
        d.retangulo(Estilo::Concreto, 0.0, 0.0, 1.0, 3.0);
        d.tracejada(Estilo::Agua, vec![[1.0, 1.0], [3.0, 1.0]]);
        d.linha(Estilo::Armadura, vec![[0.9, 0.05], [0.9, 2.95]]);
        d.circulo(Estilo::ArmaduraSecao, [0.5, 0.5], 0.01);
        d.seta([2.0, 3.5], [2.0, 3.0]);
        d.texto([0.5, 3.2], "φ = 30° — ação");
        let a = Armadura {
            pos: "N1".into(),
            elemento: "Parede".into(),
            bitola_mm: 12.5,
            espac_cm: Some(15.0),
            quantidade: "7/m".into(),
            comprimento_m: 3.2,
            as_nec: 7.5,
            as_ef: 8.18,
            unidade: "cm²/m",
            lb_cm: 40.0,
            forma: Forma::Reta,
        };
        let b = gerar(&d, &[a], "Teste", &["nota".into()], "0.3.0");
        // Windows-1252: "ç" (0xE7) como byte; Ø vira %%c (código do AutoCAD); φ vira escape U+
        assert!(b.contains(&0xE7));
        assert!(!b.contains(&b'?'), "nenhum caractere perdido");
        let s: String = b.iter().map(|&c| if c < 0x80 { c as char } else { '#' }).collect();
        assert!(s.starts_with("0\r\nSECTION\r\n2\r\nHEADER"));
        assert!(s.trim_end().ends_with("0\r\nEOF"));
        assert!(s.contains("\\U+03C6") && s.contains("%%c12,5") && !s.contains("\\U+00D8"));
        assert!(s.contains("ARMADURA") && s.contains("CIRCLE") && s.contains("DASHED"));
        // pares código/valor: número par de linhas
        assert_eq!(s.lines().count() % 2, 0);
        let n_poly = s.matches("\r\nPOLYLINE\r\n").count();
        let n_seq = s.matches("\r\nSEQEND\r\n").count();
        assert_eq!(n_poly, n_seq);
    }
}
