//! Gerador mínimo de PDF (1.4) sem dependências: páginas A4 com texto em
//! Helvetica / Helvetica-Bold (fontes padrão do PDF, WinAnsiEncoding),
//! linhas e retângulos. Usado pelo manual do usuário.

use std::fmt::Write;

/// Largura A4 (pt).
pub const LARGURA: f64 = 595.28;
/// Altura A4 (pt).
pub const ALTURA: f64 = 841.89;

/// Larguras da Helvetica (1/1000 em) para ASCII 32..=126 (AFM padrão Adobe).
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // espaço … /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 0 … ?
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // @ … O
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // P … _
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // ` … o
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, // p … ~
];

/// Letra-base de um caractere acentuado (para estimar a largura).
fn base(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'ê' | 'è' => 'e',
        'É' | 'Ê' | 'È' => 'E',
        'í' | 'î' => 'i',
        'Í' | 'Î' => 'I',
        'ó' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ü' => 'u',
        'Ú' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        _ => ch,
    }
}

/// Substituições para caracteres fora do Windows-1252.
fn equivalente(ch: char) -> Option<&'static str> {
    Some(match ch {
        'φ' | 'ϕ' => "phi",
        'γ' => "gama",
        'σ' => "sigma",
        'τ' => "tau",
        'δ' => "delta",
        'β' => "beta",
        'α' => "alfa",
        'μ' => "mi",
        'Δ' => "Delta",
        'Σ' => "Soma",
        '−' => "-",
        '∞' => "inf",
        '≥' => ">=",
        '≤' => "<=",
        '≈' => "~",
        '→' | '➡' => "->",
        '←' => "<-",
        '√' => "raiz",
        '⅔' => "2/3",
        '′' => "'",
        '″' => "\"",
        '✓' | '✔' => "OK",
        _ => return None,
    })
}

/// Ícones/emoji (sem glifo na Helvetica).
fn pictograma(ch: char) -> bool {
    matches!(ch as u32, 0x2190..=0x21FF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0x1F000..=0x1FAFF | 0xFE0F)
}

/// Byte Windows-1252 de um caractere (inclui o marcador • = 0x95).
fn byte(ch: char) -> Option<u8> {
    if ch == '•' { Some(0x95) } else { crate::dxf::cp1252(ch) }
}

/// Texto normalizado para o Windows-1252 (com equivalentes ASCII).
pub fn normalizar(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    let mut pular_espaco = false;
    for ch in t.chars() {
        if pular_espaco && ch == ' ' {
            pular_espaco = false;
            continue;
        }
        pular_espaco = false;
        if byte(ch).is_some() {
            s.push(ch);
        } else if let Some(e) = equivalente(ch) {
            s.push_str(e);
        } else if pictograma(ch) {
            // ícones de interface (emoji): omitidos junto com o espaço seguinte
            pular_espaco = true;
        } else if !ch.is_control() {
            s.push('?');
        }
    }
    s
}

/// String literal PDF "( … )" em Windows-1252, com escapes.
fn literal(t: &str) -> Vec<u8> {
    let mut v = vec![b'('];
    for ch in normalizar(t).chars() {
        let b = byte(ch).unwrap_or(b'?');
        if matches!(b, b'(' | b')' | b'\\') {
            v.push(b'\\');
        }
        v.push(b);
    }
    v.push(b')');
    v
}

/// Largura de um texto (pt) na Helvetica (negrito ≈ +8 %).
pub fn largura(t: &str, tamanho: f64, negrito: bool) -> f64 {
    let w: u32 = normalizar(t)
        .chars()
        .map(|c| {
            let c = base(c) as u32;
            if (32..=126).contains(&c) { HELV[(c - 32) as usize] as u32 } else { 556 }
        })
        .sum();
    w as f64 / 1000.0 * tamanho * if negrito { 1.08 } else { 1.0 }
}

/// Quebra um texto em linhas que caibam em `largura_max` (pt).
pub fn quebrar(t: &str, tamanho: f64, negrito: bool, largura_max: f64) -> Vec<String> {
    let mut linhas = Vec::new();
    let mut atual = String::new();
    for palavra in t.split_whitespace() {
        let tentativa = if atual.is_empty() { palavra.to_string() } else { format!("{atual} {palavra}") };
        if atual.is_empty() || largura(&tentativa, tamanho, negrito) <= largura_max {
            atual = tentativa;
        } else {
            linhas.push(std::mem::take(&mut atual));
            atual = palavra.to_string();
        }
    }
    if !atual.is_empty() {
        linhas.push(atual);
    }
    linhas
}

/// Cor RGB (0–1).
#[derive(Clone, Copy)]
pub struct Cor(pub f64, pub f64, pub f64);

pub const PRETO: Cor = Cor(0.10, 0.12, 0.16);
pub const CINZA: Cor = Cor(0.42, 0.45, 0.50);
/// Azul-marinho institucional Atlas (#13253E).
pub const AZUL: Cor = Cor(19.0 / 255.0, 37.0 / 255.0, 62.0 / 255.0);
pub const CLARO: Cor = Cor(0.93, 0.95, 0.98);
pub const BRANCO: Cor = Cor(1.0, 1.0, 1.0);

/// Página: fluxo de conteúdo PDF (origem no canto inferior esquerdo, pt).
#[derive(Default)]
pub struct Pagina {
    conteudo: Vec<u8>,
}

impl Pagina {
    fn op(&mut self, s: &str) {
        self.conteudo.extend_from_slice(s.as_bytes());
        self.conteudo.push(b'\n');
    }

    /// Texto com a linha de base em (x, y).
    pub fn texto(&mut self, x: f64, y: f64, tamanho: f64, negrito: bool, cor: Cor, t: &str) {
        let fonte = if negrito { "F2" } else { "F1" };
        self.op(&format!("BT /{fonte} {tamanho:.2} Tf {:.3} {:.3} {:.3} rg {x:.2} {y:.2} Td", cor.0, cor.1, cor.2));
        let mut s = literal(t);
        s.extend_from_slice(b" Tj ET\n");
        self.conteudo.extend_from_slice(&s);
    }

    /// Texto alinhado à direita em `xd`.
    pub fn texto_direita(&mut self, xd: f64, y: f64, tamanho: f64, negrito: bool, cor: Cor, t: &str) {
        let w = largura(t, tamanho, negrito);
        self.texto(xd - w, y, tamanho, negrito, cor, t);
    }

    pub fn linha(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, espessura: f64, cor: Cor) {
        self.op(&format!(
            "{:.3} {:.3} {:.3} RG {espessura:.2} w {x0:.2} {y0:.2} m {x1:.2} {y1:.2} l S",
            cor.0, cor.1, cor.2
        ));
    }

    pub fn retangulo(&mut self, x: f64, y: f64, w: f64, h: f64, cor: Cor) {
        self.op(&format!("{:.3} {:.3} {:.3} rg {x:.2} {y:.2} {w:.2} {h:.2} re f", cor.0, cor.1, cor.2));
    }
}

/// Imagem de uma cor com máscara de opacidade (logotipos da marca).
pub struct Imagem<'a> {
    /// Nome do recurso usado em `Pagina::imagem` (ex.: "Logo").
    pub nome: &'static str,
    pub largura: usize,
    pub altura: usize,
    pub alfa: &'a [u8],
    pub cor: Cor,
}

/// Compressão RunLength (filtro /RunLengthDecode do PDF).
fn run_length(dados: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(dados.len() / 4);
    let mut i = 0;
    while i < dados.len() {
        // repetição
        let mut r = 1;
        while i + r < dados.len() && dados[i + r] == dados[i] && r < 128 {
            r += 1;
        }
        if r >= 2 {
            out.push((257 - r) as u8);
            out.push(dados[i]);
            i += r;
            continue;
        }
        // trecho literal até a próxima repetição
        let ini = i;
        while i < dados.len() && i - ini < 128 && !(i + 1 < dados.len() && dados[i + 1] == dados[i]) {
            i += 1;
        }
        if i == ini {
            i += 1;
        }
        out.push((i - ini - 1) as u8);
        out.extend_from_slice(&dados[ini..i]);
    }
    out.push(128);
    out
}

impl Pagina {
    /// Desenha a imagem `nome` com o canto inferior esquerdo em (x, y).
    pub fn imagem(&mut self, nome: &str, x: f64, y: f64, w: f64, h: f64) {
        self.op(&format!("q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /{nome} Do Q"));
    }
}

/// Monta o arquivo PDF a partir das páginas.
pub fn montar(paginas: &[Pagina], imagens: &[Imagem], titulo: &str, autor: &str) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    let n = paginas.len();
    let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, corpo: &[u8]| {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        out.extend_from_slice(corpo);
        out.extend_from_slice(b"\nendobj\n");
    };
    let fluxo = |dic: String, dados: &[u8]| -> Vec<u8> {
        let mut v = format!("{dic}\nstream\n").into_bytes();
        v.extend_from_slice(dados);
        v.extend_from_slice(b"\nendstream");
        v
    };
    // 1 catálogo, 2 árvore de páginas, 3 e 4 fontes, 5 info, depois (imagem, máscara)
    // por imagem e (página, conteúdo) por página
    let primeira_pagina = 6 + 2 * imagens.len();
    obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: String = (0..n).map(|i| format!("{} 0 R ", primeira_pagina + 2 * i)).collect();
    obj(&mut out, &mut offsets, format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").as_bytes());
    obj(&mut out, &mut offsets, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    obj(&mut out, &mut offsets, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>");
    let mut info = b"<< /Title ".to_vec();
    info.extend(literal(titulo));
    info.extend_from_slice(b" /Author ");
    info.extend(literal(autor));
    info.extend_from_slice(b" /Producer ");
    info.extend(literal(crate::sobre::NOME));
    info.extend_from_slice(b" >>");
    obj(&mut out, &mut offsets, &info);
    let mut xobjetos = String::new();
    for (k, im) in imagens.iter().enumerate() {
        let id = 6 + 2 * k;
        let _ = write!(xobjetos, "/{} {id} 0 R ", im.nome);
        // base 1×1 na cor; a máscara (outra resolução, permitido pelo PDF 1.4) dá a forma
        let rgb = [im.cor.0, im.cor.1, im.cor.2].map(|c| (c * 255.0).round() as u8);
        let dic = format!(
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /SMask {} 0 R /Length 3 >>",
            id + 1
        );
        obj(&mut out, &mut offsets, &fluxo(dic, &rgb));
        let rl = run_length(im.alfa);
        let dic = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray \
             /BitsPerComponent 8 /Filter /RunLengthDecode /Length {} >>",
            im.largura,
            im.altura,
            rl.len()
        );
        obj(&mut out, &mut offsets, &fluxo(dic, &rl));
    }
    for (i, p) in paginas.iter().enumerate() {
        obj(
            &mut out,
            &mut offsets,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {LARGURA} {ALTURA}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> /XObject << {xobjetos}>> >> /Contents {} 0 R >>",
                primeira_pagina + 1 + 2 * i
            )
            .as_bytes(),
        );
        obj(&mut out, &mut offsets, &fluxo(format!("<< /Length {} >>", p.conteudo.len()), &p.conteudo));
    }
    let xref = out.len();
    let mut x = String::new();
    let _ = writeln!(x, "xref\n0 {}", offsets.len() + 1);
    x.push_str("0000000000 65535 f \n");
    for o in &offsets {
        let _ = writeln!(x, "{o:010} 00000 n ");
    }
    let _ = write!(x, "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1);
    out.extend_from_slice(x.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_valido_com_acentos() {
        let mut p = Pagina::default();
        p.texto(50.0, 800.0, 12.0, false, PRETO, "Contenção (σ ≥ 2) \\ teste");
        let b = montar(&[p], &[], "Título", "Autor");
        assert!(b.starts_with(b"%PDF-1.4"));
        assert!(b.ends_with(b"%%EOF\n"));
        // "ç" e "ã" como bytes Windows-1252; σ e ≥ trocados; parênteses e \ escapados
        assert!(b.windows(2).any(|w| w == [0xE7, 0xE3]));
        let achar = |agulha: &[u8]| b.windows(agulha.len()).rposition(|w| w == agulha);
        assert!(achar(b"\\(sigma >= 2\\) \\\\ teste").is_some());
        // startxref aponta para a tabela xref, e cada entrada para "N 0 obj"
        let i = achar(b"startxref\n").unwrap() + 10;
        let fim = b[i..].iter().position(|&c| c == b'\n').unwrap();
        let xref: usize = std::str::from_utf8(&b[i..i + fim]).unwrap().parse().unwrap();
        assert!(b[xref..].starts_with(b"xref"));
        let tabela = std::str::from_utf8(&b[xref..]).unwrap();
        for (k, l) in tabela.lines().skip(3).take(5).enumerate() {
            let off: usize = l[..10].parse().unwrap();
            assert!(b[off..].starts_with(format!("{} 0 obj", k + 1).as_bytes()), "objeto {}", k + 1);
        }
        assert_eq!(quebrar("a b c", 10.0, false, 1.0).len(), 3);
    }

    /// Decodificador de referência do /RunLengthDecode.
    fn decodificar(rl: &[u8]) -> Vec<u8> {
        let (mut out, mut i) = (Vec::new(), 0);
        while i < rl.len() && rl[i] != 128 {
            let n = rl[i] as usize;
            if n < 128 {
                out.extend_from_slice(&rl[i + 1..i + 2 + n]);
                i += n + 2;
            } else {
                out.extend(std::iter::repeat(rl[i + 1]).take(257 - n));
                i += 2;
            }
        }
        out
    }

    #[test]
    fn run_length_ida_e_volta() {
        let mut dados = vec![0u8; 1000];
        dados.extend((0..300).map(|i| (i * 7 % 251) as u8));
        dados.extend(vec![255u8; 129]);
        dados.push(3);
        let rl = run_length(&dados);
        assert_eq!(decodificar(&rl), dados);
        let m = crate::marca::logotipo();
        let rl = run_length(m.alfa);
        assert_eq!(decodificar(&rl), m.alfa);
        assert!(rl.len() < m.alfa.len() / 2, "compressão {} de {}", rl.len(), m.alfa.len());
    }
}
