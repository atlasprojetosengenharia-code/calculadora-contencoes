//! Memorial de cálculo em Word (.docx), sem dependências: pacote OOXML escrito
//! à mão (ZIP sem compressão). A página tem margem superior folgada e um
//! cabeçalho vazio, para o usuário aplicar o papel timbrado do escritório.
//! A seção transversal vai em SVG (vetorial, Word 2016+) com PNG de reserva.

use crate::resultado::{Bloco, Criterio, Resultado, Verificacao};

const MARINHO: &str = "13253E";
/// Largura útil da página A4 com as margens adotadas (twips).
const LARGURA: u32 = 9354;

// ---------------------------------------------------------------------------
// ZIP (somente armazenamento)

struct Zip {
    saida: Vec<u8>,
    central: Vec<u8>,
    n: u16,
}

impl Zip {
    fn novo() -> Self {
        Self { saida: Vec::new(), central: Vec::new(), n: 0 }
    }

    fn adicionar(&mut self, nome: &str, dados: &[u8]) {
        let crc = crate::raster::crc32(dados);
        let desloc = self.saida.len() as u32;
        let (hora, data) = (0u16, 0x5821u16); // 01/01/2024 00:00
        let mut cab = Vec::new();
        cab.extend(0x0403_4B50u32.to_le_bytes());
        cab.extend(20u16.to_le_bytes()); // versão necessária
        cab.extend(0x0800u16.to_le_bytes()); // nomes em UTF-8
        cab.extend(0u16.to_le_bytes()); // sem compressão
        cab.extend(hora.to_le_bytes());
        cab.extend(data.to_le_bytes());
        cab.extend(crc.to_le_bytes());
        cab.extend((dados.len() as u32).to_le_bytes());
        cab.extend((dados.len() as u32).to_le_bytes());
        cab.extend((nome.len() as u16).to_le_bytes());
        cab.extend(0u16.to_le_bytes());
        cab.extend(nome.as_bytes());
        self.saida.extend(&cab);
        self.saida.extend(dados);

        let c = &mut self.central;
        c.extend(0x0201_4B50u32.to_le_bytes());
        c.extend(20u16.to_le_bytes());
        c.extend(20u16.to_le_bytes());
        c.extend(0x0800u16.to_le_bytes());
        c.extend(0u16.to_le_bytes());
        c.extend(hora.to_le_bytes());
        c.extend(data.to_le_bytes());
        c.extend(crc.to_le_bytes());
        c.extend((dados.len() as u32).to_le_bytes());
        c.extend((dados.len() as u32).to_le_bytes());
        c.extend((nome.len() as u16).to_le_bytes());
        c.extend([0u8; 12]); // extra, comentário, disco, atributos internos/externos
        c.extend(desloc.to_le_bytes());
        c.extend(nome.as_bytes());
        self.n += 1;
    }

    fn fechar(mut self) -> Vec<u8> {
        let inicio = self.saida.len() as u32;
        let tam = self.central.len() as u32;
        self.saida.extend(&self.central);
        self.saida.extend(0x0605_4B50u32.to_le_bytes());
        self.saida.extend([0u8; 4]);
        self.saida.extend(self.n.to_le_bytes());
        self.saida.extend(self.n.to_le_bytes());
        self.saida.extend(tam.to_le_bytes());
        self.saida.extend(inicio.to_le_bytes());
        self.saida.extend(0u16.to_le_bytes());
        self.saida
    }
}

// ---------------------------------------------------------------------------
// XML do documento

fn esc(t: &str) -> String {
    t.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

#[derive(Default, Clone)]
struct Fmt<'a> {
    negrito: bool,
    italico: bool,
    cor: Option<&'a str>,
    tam: Option<u32>,
    fonte: Option<&'a str>,
}

fn run(t: &str, f: &Fmt) -> String {
    let mut pr = String::new();
    if let Some(fo) = f.fonte {
        pr += &format!(r#"<w:rFonts w:ascii="{fo}" w:hAnsi="{fo}" w:cs="{fo}"/>"#);
    }
    if f.negrito {
        pr += "<w:b/>";
    }
    if f.italico {
        pr += "<w:i/>";
    }
    if let Some(c) = f.cor {
        pr += &format!(r#"<w:color w:val="{c}"/>"#);
    }
    if let Some(s) = f.tam {
        pr += &format!(r#"<w:sz w:val="{s}"/><w:szCs w:val="{s}"/>"#);
    }
    let rpr = if pr.is_empty() { String::new() } else { format!("<w:rPr>{pr}</w:rPr>") };
    format!(r#"<w:r>{rpr}<w:t xml:space="preserve">{}</w:t></w:r>"#, esc(t))
}

fn par(runs: &str, estilo: Option<&str>, alinh: Option<&str>) -> String {
    let mut pr = String::new();
    if let Some(e) = estilo {
        pr += &format!(r#"<w:pStyle w:val="{e}"/>"#);
    }
    if let Some(a) = alinh {
        pr += &format!(r#"<w:jc w:val="{a}"/>"#);
    }
    let ppr = if pr.is_empty() { String::new() } else { format!("<w:pPr>{pr}</w:pPr>") };
    format!("<w:p>{ppr}{runs}</w:p>")
}

fn texto(t: &str) -> String {
    par(&run(t, &Fmt::default()), None, None)
}

fn titulo(t: &str, nivel: u8) -> String {
    par(&run(t, &Fmt::default()), Some(if nivel == 1 { "Heading1" } else { "Heading2" }), None)
}

/// Texto pré-formatado (colunas alinhadas com espaços): fonte monoespaçada.
fn mono(t: &str) -> String {
    let f = Fmt { fonte: Some("Consolas"), tam: Some(15), ..Default::default() };
    par(&run(t, &f), Some("Mono"), None)
}

fn celula(conteudo: &str, largura: u32, fundo: Option<&str>, span: u32) -> String {
    let mut pr = format!(r#"<w:tcW w:w="{largura}" w:type="dxa"/>"#);
    if span > 1 {
        pr += &format!(r#"<w:gridSpan w:val="{span}"/>"#);
    }
    if let Some(f) = fundo {
        pr += &format!(r#"<w:shd w:val="clear" w:color="auto" w:fill="{f}"/>"#);
    }
    format!("<w:tc><w:tcPr>{pr}</w:tcPr>{conteudo}</w:tc>")
}

fn tabela(grade: &[u32], linhas: &[String]) -> String {
    let g: String = grade.iter().map(|w| format!(r#"<w:gridCol w:w="{w}"/>"#)).collect();
    format!(
        r#"<w:tbl><w:tblPr><w:tblStyle w:val="Grade"/><w:tblW w:w="{LARGURA}" w:type="dxa"/><w:tblLayout w:type="fixed"/></w:tblPr><w:tblGrid>{g}</w:tblGrid>{}</w:tbl>{}"#,
        linhas.concat(),
        // parágrafo mínimo após a tabela (separação e exigência do Word entre tabelas)
        r#"<w:p><w:pPr><w:spacing w:after="60"/></w:pPr></w:p>"#
    )
}

fn linha(celulas: &[String], cabecalho: bool) -> String {
    let pr = if cabecalho { "<w:trPr><w:cantSplit/><w:tblHeader/></w:trPr>" } else { "<w:trPr><w:cantSplit/></w:trPr>" };
    format!("<w:tr>{pr}{}</w:tr>", celulas.concat())
}

fn formula(f: &crate::resultado::Formula) -> String {
    let cam = Fmt { fonte: Some("Cambria"), ..Default::default() };
    let rot = Fmt { negrito: true, cor: Some("57606A"), tam: Some(17), ..Default::default() };
    let l = |nome: &str, txt: &str, negrito: bool| {
        let f2 = Fmt { negrito, ..cam.clone() };
        linha(
            &[
                celula(&par(&run(nome, &rot), None, None), 1700, Some("F6F8FA"), 1),
                celula(&par(&run(txt, &f2), None, None), LARGURA - 1700, None, 1),
            ],
            false,
        )
    };
    let mut linhas = vec![linha(
        &[celula(&par(&run(&f.titulo, &Fmt { negrito: true, ..Default::default() }), None, None), LARGURA, Some("DDEAFC"), 2)],
        false,
    )];
    linhas.push(l("Equação", &f.equacao, false));
    if !f.substituicao.is_empty() {
        linhas.push(l("Substituição", &f.substituicao, false));
    }
    linhas.push(l("Resultado", &f.resultado, true));
    tabela(&[1700, LARGURA - 1700], &linhas)
}

fn norma(n: &crate::resultado::RefNorma) -> String {
    let c = par(&run(&format!("{} — {}", n.norma, n.item), &Fmt { negrito: true, cor: Some("6639BA"), tam: Some(18), ..Default::default() }), None, None)
        + &par(&run(n.requisito, &Fmt { italico: true, tam: Some(18), ..Default::default() }), None, None);
    tabela(&[LARGURA], &[linha(&[celula(&c, LARGURA, Some("F3EEFC"), 1)], false)])
}

fn verificacoes(vs: &[&Verificacao]) -> String {
    if vs.is_empty() {
        return String::new();
    }
    let cab = Fmt { negrito: true, cor: Some("FFFFFF"), tam: Some(18), ..Default::default() };
    let mut linhas = vec![linha(
        &[
            celula(&par(&run("Verificação", &cab), None, None), 5100, Some(MARINHO), 1),
            celula(&par(&run("Valor / critério", &cab), None, None), 2900, Some(MARINHO), 1),
            celula(&par(&run("Situação", &cab), None, None), LARGURA - 8000, Some(MARINHO), 1),
        ],
        true,
    )];
    for v in vs {
        let (fundo, cor) = match v.criterio {
            Criterio::Info => ("EEF0F2", "57606A"),
            _ if v.ok() => ("DAFBE1", "1A7F37"),
            _ => ("FFEBE9", "CF222E"),
        };
        let pequeno = Fmt { tam: Some(18), ..Default::default() };
        linhas.push(linha(
            &[
                celula(&par(&run(&v.nome, &pequeno), None, None), 5100, None, 1),
                celula(&par(&run(&v.expressao(), &Fmt { fonte: Some("Cambria"), tam: Some(18), ..Default::default() }), None, None), 2900, None, 1),
                celula(
                    &par(&run(v.status(), &Fmt { negrito: true, cor: Some(cor), tam: Some(16), ..Default::default() }), None, Some("center")),
                    LARGURA - 8000,
                    Some(fundo),
                    1,
                ),
            ],
            false,
        ));
    }
    tabela(&[5100, 2900, LARGURA - 8000], &linhas)
}

fn tabela_armacao(res: &Resultado) -> String {
    let larg = [500u32, 2300, 700, 800, 1150, 900, 1000, 1000, 1004];
    let cab = Fmt { negrito: true, cor: Some("FFFFFF"), tam: Some(16), ..Default::default() };
    let corpo = Fmt { tam: Some(16), ..Default::default() };
    let mut linhas = vec![linha(
        &crate::armadura::CABECALHO.iter().zip(larg).map(|(t, w)| celula(&par(&run(t, &cab), None, None), w, Some(MARINHO), 1)).collect::<Vec<_>>(),
        true,
    )];
    for a in &res.armaduras {
        linhas.push(linha(&a.celulas().iter().zip(larg).map(|(t, w)| celula(&par(&run(t, &corpo), None, None), w, None, 1)).collect::<Vec<_>>(), false));
    }
    tabela(&larg, &linhas)
}

fn figura(n: usize, descr: &str, cx_cm: f64, proporcao: f64) -> String {
    let cx = (cx_cm * 360_000.0) as i64;
    let cy = (cx as f64 * proporcao) as i64;
    format!(
        r#"<w:p><w:pPr><w:keepNext/><w:jc w:val="center"/></w:pPr><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{n}" name="Figura {n}" descr="{descr}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="0" name="fig{n}.png"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rIdImg{n}"><a:extLst><a:ext uri="{{96DAC541-7B7A-43D3-8B79-37D633B846F1}}"><asvg:svgBlip xmlns:asvg="http://schemas.microsoft.com/office/drawing/2016/SVG/main" r:embed="rIdSvg{n}"/></a:ext></a:extLst></a:blip><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#
    )
}

fn corpo(res: &Resultado, titulo_memorial: &str, versao: &str, figs: &[(String, f64)]) -> String {
    let mut x = String::new();
    x += &par(&run("MEMORIAL DE CÁLCULO", &Fmt { negrito: true, cor: Some("57606A"), tam: Some(20), ..Default::default() }), None, None);
    x += &par(&run(titulo_memorial, &Fmt::default()), Some("Title"), None);
    let (cor, fundo, msg) = if res.atende() {
        ("1A7F37", "DAFBE1", "ATENDE a todas as verificações")
    } else {
        let n = res.verificacoes.iter().filter(|v| !v.ok()).count();
        ("CF222E", "FFEBE9", if n == 1 { "NÃO ATENDE — 1 verificação reprovada" } else { "NÃO ATENDE — verificações reprovadas" })
    };
    x += &tabela(
        &[LARGURA],
        &[linha(&[celula(&par(&run(msg, &Fmt { negrito: true, cor: Some(cor), ..Default::default() }), None, Some("center")), LARGURA, Some(fundo), 1)], false)],
    );

    for (i, (t, prop)) in figs.iter().enumerate() {
        x += &titulo(t, 1);
        x += &figura(i + 1, t, 16.0, *prop);
        let legenda = if i == 0 {
            "Esquema sem escala de impressão; medidas em centímetros (cotas) e metros (demais legendas)."
        } else {
            "Detalhes sem escala; medidas em centímetros, salvo indicação."
        };
        x += &par(&run(legenda, &Fmt { italico: true, cor: Some("57606A"), tam: Some(16), ..Default::default() }), None, Some("center"));
    }

    let mut pend: Vec<&Verificacao> = Vec::new();
    for b in &res.blocos {
        if let Bloco::Verif(i) = b {
            pend.push(&res.verificacoes[*i]);
            continue;
        }
        x += &verificacoes(&pend);
        pend.clear();
        match b {
            Bloco::Titulo(t) => x += &titulo(t, 1),
            Bloco::Texto(t) => {
                if t.contains("   ") {
                    x += &mono(t);
                } else {
                    x += &texto(t);
                }
            }
            Bloco::Formula(f) => x += &formula(f),
            Bloco::Norma(n) => x += &norma(n),
            Bloco::Verif(_) => {}
        }
    }
    x += &verificacoes(&pend);

    if !res.armaduras.is_empty() {
        x += &titulo("Tabela de armação (comprimentos aproximados)", 1);
        x += &tabela_armacao(res);
    }
    x += &titulo("Resumo das verificações", 1);
    let todas: Vec<&Verificacao> = res.verificacoes.iter().collect();
    x += &verificacoes(&todas);
    if !res.avisos.is_empty() {
        x += &titulo("Avisos", 1);
        for a in &res.avisos {
            x += &par(&run(&format!("• {a}"), &Fmt::default()), Some("Aviso"), None);
        }
    }
    x += &par(
        &run(
            "Referências normativas: sínteses dos requisitos — consulte o texto integral das normas ABNT.",
            &Fmt { italico: true, cor: Some("57606A"), tam: Some(16), ..Default::default() },
        ),
        None,
        None,
    );
    x += &titulo("Responsabilidade técnica", 1);
    x += &texto("Responsável técnico: ______________________________________    CREA/CAU: ____________________");
    x += &texto("Data: ____ / ____ / ________                       Assinatura: ______________________________________");
    x += &par(&run(&format!("Gerado por {} {versao}.", crate::sobre::NOME), &Fmt { italico: true, cor: Some("57606A"), tam: Some(16), ..Default::default() }), None, None);
    x
}

const ESTILOS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:eastAsia="Calibri" w:cs="Calibri"/><w:sz w:val="21"/><w:szCs w:val="21"/><w:lang w:val="pt-BR"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="80" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="160"/></w:pPr><w:rPr><w:b/><w:color w:val="13253E"/><w:sz w:val="36"/><w:szCs w:val="36"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:pBdr><w:bottom w:val="single" w:sz="8" w:space="2" w:color="13253E"/></w:pBdr><w:spacing w:before="280" w:after="100"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:color w:val="13253E"/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="200" w:after="60"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:color w:val="13253E"/><w:sz w:val="23"/><w:szCs w:val="23"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Mono"><w:name w:val="Pre-formatado"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Aviso"><w:name w:val="Aviso"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="284" w:hanging="284"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Header"><w:name w:val="header"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Footer"><w:name w:val="footer"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="table" w:default="1" w:styleId="TabelaNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblInd w:w="0" w:type="dxa"/><w:tblCellMar><w:top w:w="0" w:type="dxa"/><w:left w:w="108" w:type="dxa"/><w:bottom w:w="0" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
<w:style w:type="table" w:styleId="Grade"><w:name w:val="Table Grid"/><w:basedOn w:val="TabelaNormal"/><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/><w:left w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/><w:right w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="C9D1D9"/></w:tblBorders><w:tblCellMar><w:top w:w="40" w:type="dxa"/><w:left w:w="100" w:type="dxa"/><w:bottom w:w="40" w:type="dxa"/><w:right w:w="100" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
</w:styles>"#;

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#;

/// Gera o memorial em .docx. `figuras` = (título, desenho): a primeira é a seção transversal;
/// as demais (detalhes construtivos, posições de armação) são opcionais.
pub fn gerar(res: &Resultado, titulo_memorial: &str, versao: &str, figuras: &[(String, crate::resultado::Desenho)]) -> Vec<u8> {
    let lp = 1600usize;
    let mut imagens: Vec<(Vec<u8>, String)> = Vec::new();
    let mut proporcoes: Vec<(String, f64)> = Vec::new();
    for (t, d) in figuras {
        let ap = crate::raster::altura_para(d, lp);
        imagens.push((crate::raster::desenho_png(d, lp, ap), crate::desenho::svg(d, lp as f64, ap as f64)));
        proporcoes.push((t.clone(), ap as f64 / lp as f64));
    }
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {NS}><w:body>{}<w:sectPr><w:headerReference w:type="default" r:id="rIdH"/><w:footerReference w:type="default" r:id="rIdF"/><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1985" w:right="1134" w:bottom="1418" w:left="1418" w:header="709" w:footer="567" w:gutter="0"/></w:sectPr></w:body></w:document>"#,
        corpo(res, titulo_memorial, versao, &proporcoes)
    );
    let cabecalho = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:hdr {NS}><w:p><w:pPr><w:pStyle w:val="Header"/></w:pPr></w:p></w:hdr>"#);
    let rodape = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:ftr {NS}><w:p><w:pPr><w:pStyle w:val="Footer"/><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:color w:val="57606A"/><w:sz w:val="16"/></w:rPr><w:t xml:space="preserve">{} — Página </w:t></w:r><w:fldSimple w:instr=" PAGE "><w:r><w:rPr><w:color w:val="57606A"/><w:sz w:val="16"/></w:rPr><w:t>1</w:t></w:r></w:fldSimple><w:r><w:rPr><w:color w:val="57606A"/><w:sz w:val="16"/></w:rPr><w:t xml:space="preserve"> de </w:t></w:r><w:fldSimple w:instr=" NUMPAGES "><w:r><w:rPr><w:color w:val="57606A"/><w:sz w:val="16"/></w:rPr><w:t>1</w:t></w:r></w:fldSimple></w:p></w:ftr>"#,
        esc(&format!("{} {versao}", crate::sobre::NOME))
    );
    let tipos = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="svg" ContentType="image/svg+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#;
    let mut rels_doc = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdS" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdH" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/><Relationship Id="rIdF" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>"#,
    );
    for n in 1..=imagens.len() {
        rels_doc += &format!(
            r#"<Relationship Id="rIdImg{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/fig{n}.png"/><Relationship Id="rIdSvg{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/fig{n}.svg"/>"#
        );
    }
    rels_doc += "</Relationships>";
    let nucleo = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>{}</dc:title><dc:creator>{}</dc:creator></cp:coreProperties>"#,
        esc(&format!("Memorial de cálculo — {titulo_memorial}")),
        esc(crate::sobre::EMPRESA)
    );

    let mut z = Zip::novo();
    z.adicionar("[Content_Types].xml", tipos.as_bytes());
    z.adicionar("_rels/.rels", rels.as_bytes());
    z.adicionar("docProps/core.xml", nucleo.as_bytes());
    z.adicionar("word/document.xml", doc.as_bytes());
    z.adicionar("word/styles.xml", ESTILOS.as_bytes());
    z.adicionar("word/header1.xml", cabecalho.as_bytes());
    z.adicionar("word/footer1.xml", rodape.as_bytes());
    z.adicionar("word/_rels/document.xml.rels", rels_doc.as_bytes());
    for (n, (png, svg)) in imagens.iter().enumerate() {
        z.adicionar(&format!("word/media/fig{}.png", n + 1), png);
        z.adicionar(&format!("word/media/fig{}.svg", n + 1), svg.as_bytes());
    }
    z.fechar()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pacote_docx_tem_as_partes_e_o_conteudo() {
        let p = crate::projeto::Projeto::default();
        let r = p.calcular();
        let figs = vec![("Seção transversal".to_string(), r.desenho.clone()), ("Barras".to_string(), crate::barras::montar(&r.armaduras, 1.0, 2))];
        let bytes = gerar(&r, "Muro de gravidade — teste & <ok>", "0.0.0", &figs);
        // assinatura ZIP e fim do diretório central
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert!(bytes.windows(4).any(|w| w == b"PK\x05\x06"));
        for parte in ["[Content_Types].xml", "word/document.xml", "word/styles.xml", "word/header1.xml", "word/media/fig1.png", "word/media/fig2.svg"] {
            assert!(bytes.windows(parte.len()).any(|w| w == parte.as_bytes()), "falta {parte}");
        }
        let amostra = std::env::var("DOCX_AMOSTRA");
        if let Ok(arq) = amostra {
            std::fs::write(arq, &bytes).unwrap();
        }
    }
}
