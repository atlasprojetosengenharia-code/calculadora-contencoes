//! Detalhes construtivos ao lado da seção: drenagem (barbacã e canaleta, no padrão
//! das pranchas do Caderno de Projetos de Contenções da UFPR), cabeça e seção do
//! grampo, e tirante (trechos livre e ancorado, cabeça de ancoragem). Servem para a
//! tela de resultados e para o DXF.

use crate::drenagem::Drenagem;
use crate::projeto::{Projeto, TipoContencao};
use crate::resultado::{fmt, Desenho, Resultado, Estilo};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Detalhe {
    /// Vista superior do muro misto (pilares, blocos e estacas).
    PlantaMisto,
    /// Seções de concreto armado do muro misto (com barras, estribos e cobrimento).
    SecaoPilar,
    SecaoViga,
    SecaoBaldrame,
    BlocoEstacas,
    SecaoEstaca,
    Canaleta,
    Barbaca,
    CabecaGrampo,
    SecaoGrampo,
    TiranteLongitudinal,
    CabecaTirante,
}

fn lista(proj: &Projeto) -> Vec<Detalhe> {
    let dr = &proj.drenagem;
    let mut v = Vec::new();
    match proj.tipo {
        TipoContencao::Flexao if proj.flexao.sistema == crate::tipos::misto::Sistema::Misto => {
            v.extend([Detalhe::PlantaMisto, Detalhe::SecaoPilar, Detalhe::SecaoViga]);
            if proj.flexao.misto.fundacao_estacas() {
                v.extend([Detalhe::SecaoBaldrame, Detalhe::BlocoEstacas, Detalhe::SecaoEstaca]);
            }
        }
        TipoContencao::Grampeado => {
            v.push(Detalhe::CabecaGrampo);
            v.push(Detalhe::SecaoGrampo);
        }
        TipoContencao::Atirantada => {
            v.push(Detalhe::TiranteLongitudinal);
            v.push(Detalhe::CabecaTirante);
        }
        _ => {}
    }
    if !matches!(proj.tipo, TipoContencao::Prancha | TipoContencao::Estacas) {
        if dr.canaleta_crista {
            v.push(Detalhe::Canaleta);
        }
        if dr.barbacas {
            v.push(Detalhe::Barbaca);
        }
    }
    v
}

/// Há detalhes construtivos para este projeto?
pub fn existem(proj: &Projeto) -> bool {
    !lista(proj).is_empty()
}

fn desenhar_item(d: &mut Desenho, det: Detalhe, cx: f64, cy: f64, k: f64, proj: &Projeto, res: &Resultado) {
    match det {
        Detalhe::PlantaMisto => proj.preparar_flexao().misto.planta(d, cx, cy, k),
        Detalhe::SecaoPilar | Detalhe::SecaoViga | Detalhe::SecaoBaldrame | Detalhe::BlocoEstacas | Detalhe::SecaoEstaca => secao_misto(d, det, cx, cy, k, proj, res),
        Detalhe::Canaleta => canaleta(d, cx, cy, k),
        Detalhe::Barbaca => barbaca(d, cx, cy, k, &proj.drenagem),
        Detalhe::CabecaGrampo => cabeca_grampo(d, cx, cy, k, proj),
        Detalhe::SecaoGrampo => secao_grampo(d, cx, cy, k, proj),
        Detalhe::TiranteLongitudinal => tirante_longitudinal(d, cx, cy, k, proj, res),
        Detalhe::CabecaTirante => cabeca_tirante(d, cx, cy, k, proj),
    }
}

/// Quantidade de detalhes do projeto.
pub fn quantidade(proj: &Projeto) -> usize {
    lista(proj).len()
}

/// Um único detalhe (o `i`-ésimo), centrado na origem — para cartões da tela.
pub fn montar_um(proj: &Projeto, res: &Resultado, i: usize) -> Desenho {
    let mut d = Desenho::default();
    if let Some(det) = lista(proj).get(i).copied() {
        desenhar_item(&mut d, det, 0.0, 0.0, 1.0, proj, res);
    }
    d
}

/// Monta os detalhes em grade (`colunas` por linha) a partir da origem, na escala `k`.
pub fn montar(proj: &Projeto, res: &Resultado, k: f64, colunas: usize) -> Desenho {
    let mut d = Desenho::default();
    for (i, det) in lista(proj).into_iter().enumerate() {
        let (col, lin) = (i % colunas.max(1), i / colunas.max(1));
        desenhar_item(&mut d, det, col as f64 * 4.4 * k, -(lin as f64) * 3.2 * k, k, proj, res);
    }
    d
}

/// Anexa os detalhes à direita do desenho (DXF).
pub fn anexar(d: &mut Desenho, proj: &Projeto, res: &Resultado) {
    if !existem(proj) {
        return;
    }
    let mut pts: Vec<[f64; 2]> = d.poligonos.iter().flat_map(|p| p.pontos.iter().copied()).collect();
    pts.extend(d.linhas.iter().flat_map(|l| l.pontos.iter().copied()));
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in pts.iter().filter(|p| p[0].is_finite() && p[1].is_finite()) {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    if !(x1 > x0 && y1 > y0) {
        return;
    }
    let k = ((y1 - y0) / 1.4).max(2.0);
    let det = montar(proj, res, k, 2);
    d.anexar_deslocado(&det, x1 + 0.10 * (x1 - x0) + 3.2 * k, y1 - 0.6 * k);
}

// ---------------------------------------------------------------------------

fn canaleta(d: &mut Desenho, cx: f64, cy: f64, k: f64) {
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let sz = 0.05 * k;
    // terreno de cada lado, com o desnível mínimo de 5 cm até a borda da canaleta
    d.linha(Estilo::Solo, vec![t(-0.90, 0.10), t(-0.15, 0.05)]);
    d.linha(Estilo::Solo, vec![t(0.15, 0.05), t(0.90, 0.10)]);
    let ext = [t(-0.15, 0.05), t(-0.10, -0.20), t(0.10, -0.20), t(0.15, 0.05), t(-0.15, 0.05)];
    d.linha(Estilo::Concreto, ext.to_vec());
    d.linha(Estilo::Concreto, vec![t(-0.10, 0.05), t(-0.075, -0.15), t(0.075, -0.15), t(0.10, 0.05)]);
    d.cota_h(cx - 0.10 * k, cx + 0.10 * k, cy + 0.30 * k, cy + 0.05 * k, sz, "20");
    d.cota_v(cy - 0.20 * k, cy + 0.05 * k, cx + 0.32 * k, cx + 0.15 * k, sz, "20");
    d.cota_h(cx - 0.15 * k, cx - 0.10 * k, cy - 0.30 * k, cy - 0.20 * k, sz, "5");
    d.texto(t(0.0, 0.78), "DESNÍVEL MÍNIMO PARA GARANTIR");
    d.texto(t(0.0, 0.64), "O ACESSO DA ÁGUA ÀS CANALETAS (5 cm)");
    d.texto(t(0.0, -0.55), "DETALHAMENTO CANALETA");
    d.texto(t(0.0, -0.69), "SEM ESCALA (medidas em cm)");
}

fn barbaca(d: &mut Desenho, cx: f64, cy: f64, k: f64, dr: &Drenagem) {
    let cy = cy - 0.9 * k;
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    // muro (seção) à esquerda e solo drenado à direita
    let muro = vec![t(-0.45, -0.55), t(0.0, -0.55), t(0.0, 0.55), t(-0.45, 0.55)];
    d.poli(Estilo::Concreto, muro.clone());
    d.pedras(&muro, 0.07 * k);
    // bolsa de material drenante envolta em geotêxtil / tela de nylon
    let bolsa = vec![t(0.0, -0.32), t(0.60, -0.32), t(0.60, 0.32), t(0.0, 0.32)];
    d.poli(Estilo::Dreno, bolsa.clone());
    d.pedras(&bolsa, 0.08 * k);
    d.tracejada(Estilo::Barbaca, vec![t(0.0, -0.36), t(0.66, -0.36), t(0.66, 0.36), t(0.0, 0.36)]);
    // tubo de PVC com 5° de caimento para fora (extremo interno mais alto)
    let r = (dr.diam_barbaca_mm / 2000.0).max(0.0375);
    let inc = 5f64.to_radians().tan();
    let (xi, xf, yi) = (0.52, -0.80, 0.06);
    let eixo = |x: f64| yi - (xi - x) * inc;
    for s in [1.0, -1.0] {
        d.linha(Estilo::Barbaca, vec![t(xi, eixo(xi) + s * r), t(xf, eixo(xf) + s * r)]);
    }
    d.linha(Estilo::Barbaca, vec![t(xf, eixo(xf) - r), t(xf, eixo(xf) + r)]);
    d.linha(Estilo::Barbaca, vec![t(xi, eixo(xi) - r), t(xi, eixo(xi) + r)]);
    let mut x = 0.05;
    while x < xi {
        d.linha(Estilo::Cota, vec![t(x, eixo(x) + r), t(x, eixo(x) + 1.8 * r)]);
        x += 0.05;
    }
    // legendas com chamadas
    d.texto(t(-1.35, 0.40), format!("TUBO DE PVC BARBACÃ Ø {} mm", format!("{:.0}", dr.diam_barbaca_mm)));
    d.linha(Estilo::Cota, vec![t(-1.00, 0.32), t(-0.55, eixo(-0.55) + r)]);
    d.texto(t(0.55, 0.85), format!("MATERIAL DRENANTE — {}", dr.brita.nome().to_uppercase()));
    d.linha(Estilo::Cota, vec![t(0.40, 0.78), t(0.30, 0.20)]);
    d.texto(t(0.70, -0.60), "GEOTÊXTIL OU TELA DE NYLON");
    d.linha(Estilo::Cota, vec![t(0.55, -0.54), t(0.50, -0.34)]);
    d.texto(t(1.15, 0.12), "FUROS A CADA 5 cm");
    d.linha(Estilo::Cota, vec![t(0.85, 0.12), t(0.35, eixo(0.35) + 1.8 * r)]);
    d.texto(t(-0.62, -0.26), "5°");
    d.texto(t(-0.22, 0.68), "MURO");
    d.texto(t(0.0, -0.95), "DETALHAMENTO BARBACÃ");
    d.texto(t(0.0, -1.09), "SEM ESCALA");
}

/// Cabeça do grampo: concreto projetado com tela, placa, porca, barra centralizada na calda.
fn cabeca_grampo(d: &mut Desenho, cx: f64, cy: f64, k: f64, proj: &Projeto) {
    let g = &proj.grampeado;
    let cy = cy - 0.1 * k;
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let sz = 0.05 * k;
    // solo à direita, face de concreto projetado à esquerda (espessura exagerada)
    let solo = vec![t(0.0, -0.75), t(1.45, -0.75), t(1.45, 0.75), t(0.0, 0.75)];
    d.poli(Estilo::Solo, solo.clone());
    d.hachura(&solo, 45.0, 0.07 * k);
    let face = vec![t(-0.16, -0.75), t(0.0, -0.75), t(0.0, 0.75), t(-0.16, 0.75)];
    d.poli(Estilo::Concreto, face.clone());
    d.pontilhado(&face, 0.04 * k);
    d.tracejada(Estilo::Armadura, vec![t(-0.08, -0.72), t(-0.08, 0.72)]);
    // furo com calda e barra (inclinada 15° abaixo da horizontal)
    let inc = 15f64.to_radians().tan();
    let eixo = |x: f64| -(x) * inc;
    let rf = 0.075;
    for s in [1.0, -1.0] {
        d.linha(Estilo::Cota, vec![t(0.0, eixo(0.0) + s * rf), t(1.45, eixo(1.45) + s * rf)]);
    }
    let rb = 0.03;
    for s in [1.0, -1.0] {
        d.linha(Estilo::Tirante, vec![t(-0.34, eixo(-0.34) + s * rb), t(1.45, eixo(1.45) + s * rb)]);
    }
    // placa de cabeça e porca com arruela
    d.retangulo(Estilo::Aco, cx - 0.22 * k, cy + (eixo(-0.22) - 0.17) * k, cx - 0.16 * k, cy + (eixo(-0.22) + 0.17) * k);
    d.retangulo(Estilo::Aco, cx - 0.31 * k, cy + (eixo(-0.31) - 0.065) * k, cx - 0.22 * k, cy + (eixo(-0.31) + 0.065) * k);
    // centralizadores
    for x in [0.35, 0.95] {
        d.linha(Estilo::Aco, vec![t(x, eixo(x) - rf), t(x, eixo(x) + rf)]);
    }
    // legendas
    d.texto(t(-0.35, 0.95), format!("CONCRETO PROJETADO e = {} cm", format!("{:.0}", g.espessura_face * 100.0)));
    d.linha(Estilo::Cota, vec![t(-0.10, 0.88), t(-0.08, 0.70)]);
    d.texto(t(-0.95, 0.50), "TELA SOLDADA");
    d.linha(Estilo::Cota, vec![t(-0.62, 0.50), t(-0.09, 0.40)]);
    d.texto(t(-1.05, -0.18), format!("PLACA {} × {} cm", format!("{:.0}", g.placa * 100.0), format!("{:.0}", g.placa * 100.0)));
    d.linha(Estilo::Cota, vec![t(-0.55, -0.12), t(-0.21, eixo(-0.21) - 0.10)]);
    d.texto(t(-1.15, 0.22), "PORCA E ARRUELA");
    d.linha(Estilo::Cota, vec![t(-0.80, 0.19), t(-0.31, eixo(-0.31) + 0.07)]);
    d.texto(t(0.85, 0.62), "CENTRALIZADOR (≤ 2 m)");
    d.linha(Estilo::Cota, vec![t(0.62, 0.55), t(0.40, eixo(0.35) + rf)]);
    d.texto(t(0.95, -0.52), format!("FURO Ø {} cm COM CALDA", format!("{:.0}", g.diam_furo * 100.0)));
    d.linha(Estilo::Cota, vec![t(0.75, -0.46), t(0.65, eixo(0.65) - rf)]);
    d.texto(t(0.85, -0.92), format!("BARRA Ø {} mm — inclinação {}°", fmt(g.diam_barra), fmt(g.incl_grampo)));
    d.cota_v(cy - 0.75 * k, cy + 0.75 * k, cx + 1.62 * k, cx + 1.45 * k, sz, "SOLO");
    d.texto(t(0.55, -1.12), "DETALHAMENTO CABEÇA DO GRAMPO");
    d.texto(t(0.55, -1.26), "SEM ESCALA");
}

/// Seção transversal do grampo: furo, calda, barra e centralizador.
fn secao_grampo(d: &mut Desenho, cx: f64, cy: f64, k: f64, proj: &Projeto) {
    let g = &proj.grampeado;
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let sz = 0.05 * k;
    let (rf, rb) = (0.55, (0.55 * g.diam_barra / (g.diam_furo * 1000.0)).clamp(0.12, 0.40));
    d.circulo(Estilo::Dreno, t(0.0, 0.0), rf * k);
    d.circulo(Estilo::Tirante, t(0.0, 0.0), rb * k);
    // espaçadores (centralizadores) a 120°
    for a in [90.0f64, 210.0, 330.0] {
        let (c, s) = (a.to_radians().cos(), a.to_radians().sin());
        d.linha(Estilo::Aco, vec![t(rb * c, rb * s), t(rf * c, rf * s)]);
    }
    d.cota_h(cx - rf * k, cx + rf * k, cy - rf * k - 0.25 * k, cy - rf * k, sz, format!("Ø {}", format!("{:.0}", g.diam_furo * 1000.0)));
    d.texto(t(0.0, 0.85), format!("CALDA DE CIMENTO a/c ≤ 0,5 ; cobrimento ≥ {} mm", format!("{:.0}", ((g.diam_furo * 1000.0 - g.diam_barra) / 2.0).max(0.0))));
    d.texto(t(0.0, 0.25), format!("Ø {}", fmt(g.diam_barra)));
    d.texto(t(0.0, -1.18), "SEÇÃO DO GRAMPO");
    d.texto(t(0.0, -1.32), "SEM ESCALA (medidas em mm)");
}

/// Tirante em elevação: painel, cabeça de ancoragem, trecho livre (bainha), selo e bulbo.
fn tirante_longitudinal(d: &mut Desenho, cx: f64, cy: f64, k: f64, proj: &Projeto, res: &Resultado) {
    let c = &proj.atirantada;
    let rf = res.reforcos.first();
    let (l_livre, l_total) = rf.map(|r| (r.livre, r.comprimento)).unwrap_or((6.0, 10.0));
    let l_bulbo = (l_total - l_livre).max(0.5);
    let ft = res.verificacoes.iter().find(|v| v.nome.starts_with("Carga de trabalho máxima")).map(|v| v.valor);
    let cy = cy - 0.1 * k;
    // escala horizontal comprimida (desenho esquemático): livre = 2,1 ; bulbo proporcional (0,9 a 1,6)
    let (xl, xb) = (2.1, (l_bulbo / l_livre.max(1.0) * 2.1).clamp(0.9, 1.6));
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let inc = 15f64.to_radians().tan();
    let eixo = |x: f64| -x * inc;
    // painel de concreto
    d.poli(Estilo::Concreto, vec![t(-0.40, -0.9), t(0.0, -0.9), t(0.0, 0.7), t(-0.40, 0.7)]);
    // solo
    let solo = vec![t(0.0, -0.9), t(xl + xb + 0.1, -0.9), t(xl + xb + 0.1, 0.7), t(0.0, 0.7)];
    d.poli(Estilo::Solo, solo.clone());
    d.hachura(&solo, 45.0, 0.08 * k);
    // trecho livre: barra com bainha
    let r = 0.025;
    for s in [1.0, -1.0] {
        d.linha(Estilo::Tirante, vec![t(-0.45, eixo(-0.45) + s * r), t(xl + xb, eixo(xl + xb) + s * r)]);
    }
    for s in [1.0, -1.0] {
        d.tracejada(Estilo::Dreno, vec![t(0.0, eixo(0.0) + s * 0.07), t(xl, eixo(xl) + s * 0.07)]);
    }
    // bulbo (calda injetada) e selo
    let bulbo = vec![t(xl, eixo(xl) - 0.12), t(xl + xb, eixo(xl + xb) - 0.12), t(xl + xb, eixo(xl + xb) + 0.12), t(xl, eixo(xl) + 0.12)];
    d.poli(Estilo::Bulbo, bulbo.clone());
    d.pontilhado(&bulbo, 0.04 * k);
    d.linha(Estilo::Aco, vec![t(xl, eixo(xl) - 0.16), t(xl, eixo(xl) + 0.16)]);
    // cabeça: placa e porca
    d.retangulo(Estilo::Aco, cx - 0.50 * k, cy + (eixo(-0.45) - 0.28) * k, cx - 0.40 * k, cy + (eixo(-0.45) + 0.28) * k);
    d.retangulo(Estilo::Aco, cx - 0.62 * k, cy + (eixo(-0.56) - 0.08) * k, cx - 0.50 * k, cy + (eixo(-0.56) + 0.08) * k);
    // cotas
    let sz = 0.05 * k;
    d.cota_h(cx, cx + xl * k, cy - 0.62 * k, cy - 0.30 * k, sz, format!("L livre = {} m", fmt(l_livre)));
    d.cota_h(cx + xl * k, cx + (xl + xb) * k, cy - 0.62 * k, cy - 0.30 * k, sz, format!("L bulbo = {} m", fmt(l_bulbo)));
    // legendas
    d.texto(t(-0.35, 0.95), "PAINEL DE CONCRETO");
    d.texto(t(0.9, 0.52), "TRECHO LIVRE: BARRA EM BAINHA PLÁSTICA COM GRAXA");
    d.linha(Estilo::Cota, vec![t(0.6, 0.46), t(0.9, eixo(0.9) + 0.07)]);
    d.texto(t(xl + xb / 2.0, 0.30), format!("BULBO: CALDA INJETADA Ø {} cm", format!("{:.0}", c.diam_furo * 100.0)));
    d.texto(t(xl - 0.45, -0.22), "SELO / OBTURADOR");
    d.texto(t(0.75, -1.12), format!("DETALHAMENTO DO TIRANTE{}", ft.map(|f| format!(" — Ft,máx = {} kN", fmt(f))).unwrap_or_default()));
    d.texto(t(0.75, -1.26), format!("inclinação {}° ; SEM ESCALA (trechos fora de proporção)", fmt(c.inclinacao)));
}

/// Cabeça de ancoragem do tirante: placa de apoio, porca/clavete e proteção anticorrosiva.
fn cabeca_tirante(d: &mut Desenho, cx: f64, cy: f64, k: f64, proj: &Projeto) {
    let c = &proj.atirantada;
    let cy = cy - 0.1 * k;
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let sz = 0.05 * k;
    // painel
    let painel = vec![t(0.0, -0.9), t(0.55, -0.9), t(0.55, 0.9), t(0.0, 0.9)];
    d.poli(Estilo::Concreto, painel.clone());
    d.pontilhado(&painel, 0.045 * k);
    // armadura do painel (duas faces)
    for x in [0.06, 0.49] {
        d.linha(Estilo::Armadura, vec![t(x, -0.85), t(x, 0.85)]);
    }
    // bainha/tubo de passagem
    let inc = 15f64.to_radians().tan();
    let eixo = |x: f64| -(x - 0.55) * inc;
    for s in [1.0, -1.0] {
        d.linha(Estilo::Barbaca, vec![t(0.0, eixo(0.0) + s * 0.07), t(0.55, eixo(0.55) + s * 0.07)]);
    }
    for s in [1.0, -1.0] {
        d.linha(Estilo::Tirante, vec![t(-0.55, eixo(-0.55) + s * 0.03), t(1.25, eixo(1.25) + s * 0.03)]);
    }
    // placa de apoio, porca/cunha e capa de proteção
    d.retangulo(Estilo::Aco, cx - 0.10 * k, cy + (eixo(-0.10) - 0.30) * k, cx + 0.0, cy + (eixo(-0.10) + 0.30) * k);
    d.retangulo(Estilo::Aco, cx - 0.24 * k, cy + (eixo(-0.24) - 0.09) * k, cx - 0.10 * k, cy + (eixo(-0.24) + 0.09) * k);
    d.tracejada(Estilo::Dreno, vec![t(-0.50, eixo(-0.50) - 0.17), t(-0.50, eixo(-0.50) + 0.17), t(-0.02, eixo(-0.02) + 0.34), t(-0.02, eixo(-0.02) - 0.34), t(-0.50, eixo(-0.50) - 0.17)]);
    // cotas
    d.cota_v(cy - 0.30 * k + eixo(-0.10) * k, cy + 0.30 * k + eixo(-0.10) * k, cx - 0.78 * k, cx - 0.10 * k, sz, format!("{:.0}", c.placa * 100.0));
    d.cota_h(cx, cx + 0.55 * k, cy + 1.05 * k, cy + 0.9 * k, sz, format!("{:.0}", c.espessura * 100.0));
    // legendas
    d.texto(t(-1.15, 0.45), "CAPA DE PROTEÇÃO");
    d.texto(t(-1.15, 0.31), "(graute / calda + tinta)");
    d.linha(Estilo::Cota, vec![t(-0.85, 0.36), t(-0.45, eixo(-0.50) + 0.16)]);
    d.texto(t(-1.2, -0.62), "PLACA DE APOIO");
    d.linha(Estilo::Cota, vec![t(-0.80, -0.56), t(-0.05, eixo(-0.05) - 0.27)]);
    d.texto(t(-1.25, -0.88), "PORCA / CLAVETES");
    d.linha(Estilo::Cota, vec![t(-0.85, -0.82), t(-0.17, eixo(-0.17) - 0.08)]);
    d.texto(t(0.95, 0.62), "TUBO DE PASSAGEM (bainha)");
    d.linha(Estilo::Cota, vec![t(0.80, 0.56), t(0.50, eixo(0.50) + 0.08)]);
    d.texto(t(0.95, 0.88), "ARMADURA DO PAINEL");
    d.linha(Estilo::Cota, vec![t(0.80, 0.82), t(0.50, 0.70)]);
    d.texto(t(0.25, -1.12), "DETALHAMENTO CABEÇA DO TIRANTE");
    d.texto(t(0.25, -1.26), "SEM ESCALA (medidas em cm)");
}

#[cfg(test)]
mod testes {
    use super::*;

    fn proj(tipo: TipoContencao) -> Projeto {
        let mut p = Projeto::default();
        p.tipo = tipo;
        p.geometria.h = 5.0;
        p
    }

    #[test]
    fn detalhes_por_tipo() {
        for (tipo, esperado) in [
            (TipoContencao::Gravidade, vec!["BARBACÃ", "CANALETA"]),
            (TipoContencao::Grampeado, vec!["CABEÇA DO GRAMPO", "SEÇÃO DO GRAMPO"]),
            (TipoContencao::Atirantada, vec!["DO TIRANTE", "CABEÇA DO TIRANTE"]),
        ] {
            let p = proj(tipo);
            let r = p.calcular();
            let d = montar(&p, &r, 1.0, 2);
            for e in esperado {
                assert!(d.textos.iter().any(|t| t.texto.contains(e)), "{}: falta o detalhe '{e}'", tipo.nome());
            }
            assert!(d.textos.iter().all(|t| t.pos[0].is_finite() && t.pos[1].is_finite()));
        }
        let p = proj(TipoContencao::Prancha);
        assert!(!existem(&p));
    }

    #[test]
    fn anexa_detalhes_a_direita_do_desenho() {
        let p = proj(TipoContencao::Grampeado);
        let r = p.calcular();
        let mut d = r.desenho.clone();
        let xmax = d.linhas.iter().flat_map(|l| l.pontos.iter()).map(|q| q[0]).fold(f64::MIN, f64::max);
        let antes = d.textos.len();
        anexar(&mut d, &p, &r);
        assert!(d.textos.len() > antes + 8);
        assert!(d.textos[antes..].iter().all(|t| t.pos[0] > xmax - 1e-6), "detalhes à direita da seção");
    }
}

// ---------------------------------------------------------------------------
// Seções de concreto armado do muro misto (NBR 6118: cobrimento, barras, estribos)
// ---------------------------------------------------------------------------

fn arm<'a>(res: &'a Resultado, pos: &str) -> Option<&'a crate::armadura::Armadura> {
    res.armaduras.iter().find(|a| a.pos == pos)
}

fn qtd(a: &crate::armadura::Armadura) -> usize {
    a.quantidade.split(|c: char| !c.is_ascii_digit()).find(|t| !t.is_empty()).and_then(|t| t.parse().ok()).unwrap_or(2).max(1)
}

/// Seção retangular b × h (m) centrada em (cx, cy): concreto, estribo no cobrimento,
/// barras (posição relativa ao centro, Ø mm), cotas e rótulos.
#[allow(clippy::too_many_arguments)]
fn secao_ret(d: &mut Desenho, cx: f64, cy: f64, k: f64, b: f64, h: f64, cob: f64, barras: &[([f64; 2], f64)], phi_t: Option<f64>, titulo: &str, rotulos: &[String]) {
    let esc = k * (2.4 / b).min(1.9 / h);
    let t = |x: f64, y: f64| [cx + esc * x, cy + esc * y];
    let (bx, hy) = (b / 2.0, h / 2.0);
    d.poli(Estilo::Concreto, vec![t(-bx, -hy), t(bx, -hy), t(bx, hy), t(-bx, hy)]);
    if let Some(ft) = phi_t {
        let o = cob + ft / 2000.0;
        let g = 0.06_f64.min(0.25 * b);
        // estribo fechado com ganchos a 135° no canto superior esquerdo
        d.linha(Estilo::Armadura, vec![t(-bx + o + g, hy - o - g * 0.7), t(-bx + o, hy - o), t(bx - o, hy - o), t(bx - o, -hy + o), t(-bx + o, -hy + o), t(-bx + o, hy - o), t(-bx + o + g * 0.7, hy - o - g)]);
    }
    for (p, phi) in barras {
        d.circulo(Estilo::ArmaduraSecao, t(p[0], p[1]), (phi / 2000.0 * esc).max(0.018 * k));
    }
    d.cota_h(t(-bx, 0.0)[0], t(bx, 0.0)[0], cy - esc * hy - 0.22 * k, cy - esc * hy, 0.07 * k, format!("{} cm", fmt(100.0 * b)));
    d.cota_v(t(0.0, -hy)[1], t(0.0, hy)[1], cx - esc * bx - 0.22 * k, cx - esc * bx, 0.07 * k, format!("{} cm", fmt(100.0 * h)));
    d.texto([cx, cy + esc * hy + 0.30 * k], titulo);
    for (i, r) in rotulos.iter().enumerate() {
        d.texto([cx, cy - esc * hy - (0.48 + 0.16 * i as f64) * k], r.clone());
    }
}

/// Barras distribuídas numa linha de a até b (inclusive).
fn linha_barras(a: [f64; 2], b: [f64; 2], n: usize, phi: f64) -> Vec<([f64; 2], f64)> {
    (0..n)
        .map(|i| {
            let s = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.5 };
            ([a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s], phi)
        })
        .collect()
}

fn circulo_pts(cx: f64, cy: f64, r: f64) -> Vec<[f64; 2]> {
    (0..=32)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 32.0;
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

fn secao_misto(d: &mut Desenho, det: Detalhe, cx: f64, cy: f64, k: f64, proj: &Projeto, res: &Resultado) {
    let fx = proj.preparar_flexao();
    let mi = &fx.misto;
    let cob = fx.concreto.cobrimento;
    let desc = |p: &str| arm(res, p).map(|a| format!("{} — {}", a.descricao(), a.elemento)).unwrap_or_default();
    let cobr = format!("cobrimento nominal {} cm", fmt(100.0 * cob));
    match det {
        Detalhe::SecaoPilar => {
            let (Some(n1), Some(n3)) = (arm(res, "N1"), arm(res, "N3")) else { return };
            // b ao longo do muro (horizontal no desenho), h para o aterro (vertical, aterro em cima)
            let (b, h) = (mi.pilar_b, mi.pilar_h);
            let (ft, f) = (n3.bitola_mm / 1000.0, n1.bitola_mm / 1000.0);
            let o = cob + ft + f / 2.0;
            let mut barras = linha_barras([-b / 2.0 + o, h / 2.0 - o], [b / 2.0 - o, h / 2.0 - o], qtd(n1), n1.bitola_mm);
            let o2 = cob + ft + 0.005;
            barras.extend(linha_barras([-b / 2.0 + o2, -h / 2.0 + o2], [b / 2.0 - o2, -h / 2.0 + o2], 2, 10.0));
            secao_ret(d, cx, cy, k, b, h, cob, &barras, Some(n3.bitola_mm), "SECAO DO PILAR (aterro em cima)", &[desc("N1"), desc("N3"), format!("2 Ø10 porta-estribos na face externa ; {cobr}")]);
        }
        Detalhe::SecaoViga => {
            let (Some(n8), Some(n12)) = (arm(res, "N8"), arm(res, "N12")) else { return };
            // b perpendicular ao muro (horizontal no desenho, aterro à direita), h vertical
            let (b, h) = (mi.viga_b, mi.viga_h);
            let (ft, f) = (n12.bitola_mm / 1000.0, n8.bitola_mm / 1000.0);
            let o = cob + ft + f / 2.0;
            let n = qtd(n8);
            let mut barras = linha_barras([-b / 2.0 + o, -h / 2.0 + o], [-b / 2.0 + o, h / 2.0 - o], n, n8.bitola_mm);
            barras.extend(linha_barras([b / 2.0 - o, -h / 2.0 + o], [b / 2.0 - o, h / 2.0 - o], n, n8.bitola_mm));
            secao_ret(d, cx, cy, k, b, h, cob, &barras, Some(n12.bitola_mm), "SECAO DA VIGA INTERMEDIARIA (aterro a direita)", &[desc("N8"), desc("N12"), cobr]);
        }
        Detalhe::SecaoBaldrame => {
            let (Some(n9), Some(n13)) = (arm(res, "N9"), arm(res, "N13")) else { return };
            let (b, h) = (mi.baldrame_b, mi.baldrame_h);
            let (ft, f) = (n13.bitola_mm / 1000.0, n9.bitola_mm / 1000.0);
            let o = cob + ft + f / 2.0;
            let n = qtd(n9);
            let mut barras = linha_barras([-b / 2.0 + o, -h / 2.0 + o], [b / 2.0 - o, -h / 2.0 + o], n, n9.bitola_mm);
            barras.extend(linha_barras([-b / 2.0 + o, h / 2.0 - o], [b / 2.0 - o, h / 2.0 - o], n, n9.bitola_mm));
            secao_ret(d, cx, cy, k, b, h, cob, &barras, Some(n13.bitola_mm), "SECAO DA VIGA BALDRAME", &[desc("N9"), desc("N13"), cobr]);
        }
        Detalhe::SecaoEstaca => {
            let (Some(n10), Some(n14)) = (arm(res, "N10"), arm(res, "N14")) else { return };
            let dd = mi.diam_estaca;
            let esc = k * 1.9 / dd;
            let r = dd / 2.0;
            d.linha(Estilo::Concreto, circulo_pts(cx, cy, esc * r));
            let rh = r - cob - n14.bitola_mm / 2000.0;
            d.linha(Estilo::Armadura, circulo_pts(cx, cy, esc * rh));
            let n = qtd(n10);
            let rb = rh - n14.bitola_mm / 2000.0 - n10.bitola_mm / 2000.0;
            for i in 0..n {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                d.circulo(Estilo::ArmaduraSecao, [cx + esc * rb * a.cos(), cy + esc * rb * a.sin()], (n10.bitola_mm / 2000.0 * esc).max(0.018 * k));
            }
            d.texto([cx, cy + esc * r + 0.30 * k], format!("SECAO DA ESTACA Ø{} cm", fmt(100.0 * dd)));
            for (i, t) in [desc("N10"), desc("N14"), cobr].iter().enumerate() {
                d.texto([cx, cy - esc * r - (0.35 + 0.16 * i as f64) * k], t.clone());
            }
        }
        Detalhe::BlocoEstacas => {
            let Some(n11) = arm(res, "N11") else { return };
            let (e, lb, _bb, hb) = mi.geometria_bloco();
            let dd = mi.diam_estaca;
            let esc = k * (3.0 / lb).min(1.4 / hb);
            let t = |x: f64, y: f64| [cx + esc * x, cy + esc * y];
            // corte perpendicular ao muro: bloco, estacas embutidas 5 cm, pilar e tirante em U
            d.poli(Estilo::Concreto, vec![t(-lb / 2.0, -hb / 2.0), t(lb / 2.0, -hb / 2.0), t(lb / 2.0, hb / 2.0), t(-lb / 2.0, hb / 2.0)]);
            for s in [-1.0, 1.0] {
                let x = s * e / 2.0;
                d.poli(Estilo::Concreto, vec![t(x - dd / 2.0, -hb / 2.0 - 0.6 * hb), t(x + dd / 2.0, -hb / 2.0 - 0.6 * hb), t(x + dd / 2.0, -hb / 2.0 + 0.05), t(x - dd / 2.0, -hb / 2.0 + 0.05)]);
            }
            let ap = mi.pilar_h;
            d.poli(Estilo::Concreto, vec![t(-ap / 2.0, hb / 2.0), t(ap / 2.0, hb / 2.0), t(ap / 2.0, hb), t(-ap / 2.0, hb)]);
            let f = n11.bitola_mm / 1000.0;
            let y = -hb / 2.0 + cob + 0.05 + f / 2.0;
            let perna = (hb - 2.0 * cob).min(12.0 * f + 0.1);
            let xa = lb / 2.0 - cob;
            d.linha(Estilo::Armadura, vec![t(-xa, y + perna), t(-xa, y), t(xa, y), t(xa, y + perna)]);
            if let Some(n1) = arm(res, "N1") {
                // arranques do pilar com dobra de 90° (12Ø) sobre o tirante
                let x1 = ap / 2.0 - cob - n1.bitola_mm / 2000.0;
                let fy = y + 0.03;
                d.linha(Estilo::Armadura, vec![t(x1 - 12.0 * n1.bitola_mm / 1000.0, fy), t(x1, fy), t(x1, 0.95 * hb)]);
            }
            let y_cota = cy - esc * (hb / 2.0 + 0.6 * hb) - 0.2 * k;
            d.cota_h(t(-lb / 2.0, 0.0)[0], t(lb / 2.0, 0.0)[0], y_cota, y_cota + 0.2 * k, 0.07 * k, format!("{} m", fmt(lb)));
            d.cota_v(t(0.0, -hb / 2.0)[1], t(0.0, hb / 2.0)[1], cx - esc * lb / 2.0 - 0.2 * k, cx - esc * lb / 2.0, 0.07 * k, format!("{} m", fmt(hb)));
            d.texto([cx, cy + esc * hb + 0.25 * k], format!("BLOCO SOBRE 2 ESTACAS - corte perpendicular ao muro (e = {} m)", fmt(e)));
            for (i, s) in [desc("N11"), "tirante concentrado sobre as estacas (faixa ≤ 1,2·Ø) ; estacas embutidas 5 cm".to_string(), desc("N1")].iter().enumerate() {
                d.texto([cx, y_cota - (0.40 + 0.16 * i as f64) * k], s.clone());
            }
        }
        _ => {}
    }
}
