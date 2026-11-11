//! Detalhes construtivos da drenagem (barbacã e canaleta), anexados ao lado da
//! seção na exportação DXF, no padrão das pranchas de projeto: tubo de PVC com
//! caimento de 5°, bolsa de material drenante envolta em geotêxtil, e canaleta de
//! concreto 0,20 × 0,20 m.

use crate::drenagem::Drenagem;
use crate::projeto::TipoContencao;
use crate::resultado::{Desenho, Estilo};

/// Anexa os detalhes à direita do desenho (somente tipos com face e drenagem prevista).
pub fn anexar(d: &mut Desenho, dr: &Drenagem, tipo: TipoContencao) {
    if matches!(tipo, TipoContencao::Prancha | TipoContencao::Estacas) || !(dr.barbacas || dr.canaleta_crista) {
        return;
    }
    let mut pts: Vec<[f64; 2]> = d.poligonos.iter().flat_map(|p| p.pontos.iter().copied()).collect();
    pts.extend(d.linhas.iter().flat_map(|l| l.pontos.iter().copied()));
    let (mut x1, mut y0, mut y1) = (f64::MIN, f64::MAX, f64::MIN);
    let mut x0 = f64::MAX;
    for p in pts.iter().filter(|p| p[0].is_finite() && p[1].is_finite()) {
        x0 = x0.min(p[0]);
        x1 = x1.max(p[0]);
        y0 = y0.min(p[1]);
        y1 = y1.max(p[1]);
    }
    if !(x1 > x0 && y1 > y0) {
        return;
    }
    let k = ((y1 - y0) / 2.2).max(1.4);
    let xo = x1 + 0.10 * (x1 - x0) + 3.0 * k;
    let mut yo = y1 - 0.5 * k;
    if dr.canaleta_crista {
        canaleta(d, xo, yo - 0.3 * k, k);
        yo -= 2.6 * k;
    }
    if dr.barbacas {
        barbaca(d, xo, yo - 1.0 * k, k, dr);
    }
}

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
    d.texto(t(0.55, 0.85), "MATERIAL DRENANTE");
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn anexa_detalhes_a_direita_do_desenho() {
        let mut d = Desenho::default();
        d.retangulo(Estilo::Concreto, 0.0, 0.0, 2.0, 4.0);
        let antes = d.linhas.len();
        anexar(&mut d, &Drenagem::default(), TipoContencao::Gravidade);
        assert!(d.linhas.len() > antes + 20);
        assert!(d.textos.iter().any(|t| t.texto.contains("BARBACÃ")) && d.textos.iter().any(|t| t.texto.contains("CANALETA")));
        // todo o detalhe fica à direita da seção
        assert!(d.textos.iter().all(|t| t.pos[0] > 2.0));
        // sem detalhe para as embutidas
        let mut e = Desenho::default();
        e.retangulo(Estilo::Concreto, 0.0, 0.0, 2.0, 4.0);
        anexar(&mut e, &Drenagem::default(), TipoContencao::Prancha);
        assert!(e.textos.is_empty());
    }
}
