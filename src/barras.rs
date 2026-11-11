//! Posições de armadura isoladas, fora da seção: cada barra é desenhada sozinha com
//! as dobras, ganchos, pino de dobramento e comprimentos desenvolvidos (NBR 6118:2023,
//! 9.4.2.3 e 9.4.6.1, Tabelas 9.1 e 9.2), para facilitar o corte e a dobra.

use crate::armadura::{self, Armadura, Forma};
use crate::resultado::{fmt, Desenho, Estilo};
use std::f64::consts::PI;

/// Polilinha com cantos arredondados (raio `r`) a 90°.
fn arredondar(pts: &[[f64; 2]], r: f64) -> Vec<[f64; 2]> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (a, b, c) = (pts[i - 1], pts[i], pts[i + 1]);
        let (u, v) = ([a[0] - b[0], a[1] - b[1]], [c[0] - b[0], c[1] - b[1]]);
        let (lu, lv) = ((u[0] * u[0] + u[1] * u[1]).sqrt(), (v[0] * v[0] + v[1] * v[1]).sqrt());
        let rr = r.min(0.45 * lu).min(0.45 * lv);
        let (u, v) = ([u[0] / lu, u[1] / lu], [v[0] / lv, v[1] / lv]);
        let p0 = [b[0] + u[0] * rr, b[1] + u[1] * rr];
        let p1 = [b[0] + v[0] * rr, b[1] + v[1] * rr];
        // arco (aproximado por 8 pontos) com centro em b + (u + v)·rr
        let c0 = [b[0] + (u[0] + v[0]) * rr, b[1] + (u[1] + v[1]) * rr];
        let (a0, a1) = ((p0[1] - c0[1]).atan2(p0[0] - c0[0]), (p1[1] - c0[1]).atan2(p1[0] - c0[0]));
        let mut da = a1 - a0;
        while da > PI {
            da -= 2.0 * PI;
        }
        while da < -PI {
            da += 2.0 * PI;
        }
        for k in 0..=8 {
            let a = a0 + da * k as f64 / 8.0;
            out.push([c0[0] + rr * a.cos(), c0[1] + rr * a.sin()]);
        }
    }
    out.push(pts[pts.len() - 1]);
    out
}

fn cm(v: f64) -> String {
    format!("{:.0}", v * 100.0)
}

/// Desenha a posição `a` com o centro da célula em (`cx`, `cy`), escala `k`.
fn posicao(d: &mut Desenho, a: &Armadura, cx: f64, cy: f64, k: f64) {
    let t = |x: f64, y: f64| [cx + k * x, cy + k * y];
    let sz = 0.05 * k;
    let phi = a.bitola_mm;
    let mut cabecalho = a.descricao();
    if a.comprimento_m > 0.0 {
        cabecalho += &format!(" — C = {} cm", cm(a.comprimento_m));
    } else if !matches!(a.forma, Forma::Tela) {
        cabecalho += " — corrida";
    }
    d.texto(t(0.0, 1.25), cabecalho);
    let mut notas: Vec<String> = Vec::new();
    match a.forma {
        Forma::Reta => {
            d.linha(Estilo::Armadura, vec![t(-1.1, 0.1), t(1.1, 0.1)]);
            if a.comprimento_m > 0.0 {
                d.cota_h(cx - 1.1 * k, cx + 1.1 * k, cy - 0.15 * k, cy + 0.1 * k, sz, cm(a.comprimento_m));
            }
            if a.lb_cm > 0.0 {
                notas.push(format!("ancoragem reta lb,nec = {} cm (NBR 6118 9.4.2.5)", fmt(a.lb_cm)));
            }
            notas.push("barra reta, sem dobras".into());
        }
        Forma::EmL { longa, curta } => {
            let alt = (curta / longa.max(0.1) * 2.2).clamp(0.45, 0.95);
            let r = 0.5 * armadura::pino_mm(phi, false) / 1000.0 * (2.2 / longa.max(0.5)).clamp(0.5, 3.0);
            let r = r.clamp(0.06, 0.2);
            let pts = [t(-1.1, 0.35), t(1.1, 0.35), t(1.1, 0.35 - alt)];
            d.linha(Estilo::Armadura, arredondar(&pts, r * k));
            d.cota_h(cx - 1.1 * k, cx + 1.1 * k, cy + 0.65 * k, cy + 0.35 * k, sz, cm(longa));
            d.cota_v(cy + (0.35 - alt) * k, cy + 0.35 * k, cx + 1.45 * k, cx + 1.1 * k, sz, cm(curta));
            let pino = armadura::pino_mm(phi, false);
            notas.push(format!(
                "dobra a 90°: pino D = {} Ø = {} cm (NBR 6118 Tab. 9.1) ; ponta reta ≥ 8Ø = {} cm (9.4.2.3 c)",
                fmt(pino / phi),
                fmt(pino / 10.0),
                fmt(armadura::gancho_reto_cm(phi))
            ));
        }
        Forma::EmU { vao, perna } => {
            let alt = (perna / vao.max(0.1) * 2.2).clamp(0.45, 0.8);
            let pts = [t(-1.1, 0.35 - alt), t(-1.1, 0.35), t(1.1, 0.35), t(1.1, 0.35 - alt)];
            d.linha(Estilo::Armadura, arredondar(&pts, 0.12 * k));
            d.cota_h(cx - 1.1 * k, cx + 1.1 * k, cy + 0.65 * k, cy + 0.35 * k, sz, cm(vao));
            d.cota_v(cy + (0.35 - alt) * k, cy + 0.35 * k, cx + 1.45 * k, cx + 1.1 * k, sz, cm(perna));
            d.cota_v(cy + (0.35 - alt) * k, cy + 0.35 * k, cx - 1.45 * k, cx - 1.1 * k, sz, cm(perna));
            let pino = armadura::pino_mm(phi, false);
            notas.push(format!(
                "dobras a 90° nas duas pontas: pino D = {} Ø = {} cm (Tab. 9.1) ; ponta reta ≥ 8Ø = {} cm",
                fmt(pino / phi),
                fmt(pino / 10.0),
                fmt(armadura::gancho_reto_cm(phi))
            ));
        }
        Forma::Estribo { b, h } => {
            let larg = 1.5;
            let alt = (larg * h / b.max(0.05)).clamp(0.55, 1.6);
            let (x0, x1, y0, y1) = (-larg / 2.0, larg / 2.0, 0.55 - alt, 0.55);
            let pts = [t(x1, y1), t(x1, y0), t(x0, y0), t(x0, y1), t(x1, y1)];
            d.linha(Estilo::Armadura, arredondar(&pts, 0.10 * k));
            // ganchos a 135° no canto superior direito, voltados para o interior
            let g = 0.30;
            d.linha(Estilo::Armadura, vec![t(x1, y1), t(x1 - 0.8 * g, y1 - 0.8 * g)]);
            d.linha(Estilo::Armadura, vec![t(x1, y1 - 0.0), t(x1 - 1.0 * g, y1 - 0.5 * g)]);
            d.cota_h(cx + x0 * k, cx + x1 * k, cy + (y1 + 0.2) * k, cy + y1 * k, sz, cm(b));
            d.cota_v(cy + y0 * k, cy + y1 * k, cx + (x1 + 0.4) * k, cx + x1 * k, sz, cm(h));
            let pino = armadura::pino_mm(phi, true);
            notas.push(format!(
                "gancho a 135°: ponta reta ≥ 5Øt = {} cm (mín. 5 cm) — NBR 6118 9.4.6.1 ; pino D = {} Øt = {} cm (Tab. 9.2)",
                fmt(armadura::gancho_estribo_cm(phi)),
                fmt(pino / phi),
                fmt(pino / 10.0)
            ));
            notas.push("dimensões externas b × h (cm); emendas e ganchos alternados nos cantos".into());
        }
        Forma::Helice { d: dm, passo } => {
            let n = 48;
            let pts: Vec<[f64; 2]> = (0..=n)
                .map(|i| {
                    let a = 2.0 * PI * i as f64 / n as f64;
                    t(0.55 * a.cos(), 0.1 + 0.55 * a.sin())
                })
                .collect();
            d.linha(Estilo::Armadura, pts);
            d.cota_h(cx - 0.55 * k, cx + 0.55 * k, cy - 0.6 * k, cy - 0.45 * k, sz, cm(dm));
            notas.push(format!("hélice contínua de Ø {} cm, passo {} cm; ponta com gancho a 135° (5Øt ≥ 5 cm) e transpasse de 2 voltas", cm(dm), fmt(passo * 100.0)));
        }
        Forma::Tela => {
            let (x0, x1, y0, y1) = (-0.9, 0.9, -0.3, 0.8);
            d.linha(Estilo::Armadura, vec![t(x0, y0), t(x1, y0), t(x1, y1), t(x0, y1), t(x0, y0)]);
            let mut x = x0 + 0.3;
            while x < x1 - 0.01 {
                d.linha(Estilo::Armadura, vec![t(x, y0), t(x, y1)]);
                x += 0.3;
            }
            let mut y = y0 + 0.3;
            while y < y1 - 0.01 {
                d.linha(Estilo::Armadura, vec![t(x0, y), t(x1, y)]);
                y += 0.3;
            }
            notas.push("tela soldada nas duas direções, no meio da espessura; traspasse de 2 malhas".into());
        }
    }
    // notas quebradas em linhas curtas, para não invadir a célula vizinha
    let mut linhas: Vec<String> = Vec::new();
    for n in &notas {
        let mut atual = String::new();
        for palavra in n.split_whitespace() {
            if !atual.is_empty() && atual.chars().count() + 1 + palavra.chars().count() > 44 {
                linhas.push(std::mem::take(&mut atual));
            }
            if !atual.is_empty() {
                atual.push(' ');
            }
            atual.push_str(palavra);
        }
        if !atual.is_empty() {
            linhas.push(atual);
        }
    }
    for (i, l) in linhas.iter().enumerate() {
        d.texto(t(0.0, -0.62 - 0.15 * i as f64), l.clone());
    }
}

/// Posições de armação isoladas em grade, a partir da origem, na escala `k`.
pub fn montar(arm: &[Armadura], k: f64, colunas: usize) -> Desenho {
    let mut d = Desenho::default();
    for (i, a) in arm.iter().enumerate() {
        let (col, lin) = (i % colunas.max(1), i / colunas.max(1));
        posicao(&mut d, a, col as f64 * 4.8 * k, -(lin as f64) * 3.6 * k, k);
    }
    d
}

/// Uma única posição, centrada na origem (cartões da tela).
pub fn montar_uma(arm: &[Armadura], i: usize) -> Desenho {
    let mut d = Desenho::default();
    if let Some(a) = arm.get(i) {
        posicao(&mut d, a, 0.0, 0.0, 1.0);
    }
    d
}

/// Anexa as posições à direita do desenho (DXF).
pub fn anexar(d: &mut Desenho, arm: &[Armadura]) {
    if arm.is_empty() {
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
    let k = ((y1 - y0) / 1.6).max(2.0);
    let det = montar(arm, k, 2);
    d.anexar_deslocado(&det, x1 + 0.10 * (x1 - x0) + 3.4 * k, y1 - 0.6 * k);
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn pinos_e_ganchos_da_nbr6118() {
        // Tabela 9.1 (CA-50): < 20 mm → 5 Ø ; ≥ 20 mm → 8 Ø
        assert_eq!(armadura::pino_mm(16.0, false), 80.0);
        assert_eq!(armadura::pino_mm(20.0, false), 160.0);
        // Tabela 9.2 (estribos, CA-50): ≤ 10 → 3 Øt ; 10 < Ø < 20 → 5 Øt ; ≥ 20 → 8 Øt
        assert_eq!(armadura::pino_mm(6.3, true), 18.9);
        assert_eq!(armadura::pino_mm(12.5, true), 62.5);
        // gancho de estribo a 135°: 5 Øt, mínimo 5 cm ; ângulo reto de barra: 8 Ø
        assert_eq!(armadura::gancho_estribo_cm(6.3), 5.0);
        assert!((armadura::gancho_estribo_cm(12.5) - 6.25).abs() < 1e-9);
        assert!((armadura::gancho_reto_cm(16.0) - 12.8).abs() < 1e-9);
    }

    #[test]
    fn cantos_arredondados_mantem_extremos() {
        let p = arredondar(&[[0.0, 0.0], [1.0, 0.0], [1.0, -1.0]], 0.2);
        assert_eq!(p[0], [0.0, 0.0]);
        assert_eq!(*p.last().unwrap(), [1.0, -1.0]);
        assert!(p.len() > 8);
    }

    #[test]
    fn cada_tipo_gera_posicoes_com_forma_e_normas() {
        for tipo in crate::projeto::TIPOS_CONTENCAO {
            let mut p = crate::projeto::Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 5.0;
            p.pre_dimensionar();
            let r = p.calcular();
            if r.armaduras.is_empty() {
                continue;
            }
            let d = montar(&r.armaduras, 1.0, 2);
            assert!(d.linhas.len() >= r.armaduras.len(), "{}: sem desenho das barras", tipo.nome());
            assert!(d.textos.iter().all(|t| t.pos[0].is_finite() && t.pos[1].is_finite()));
            for a in &r.armaduras {
                assert!(d.textos.iter().any(|t| t.texto.starts_with(&a.descricao())), "{}: falta {}", tipo.nome(), a.pos);
            }
        }
        // o muro de flexão tem barras dobradas (N4 e N7 em L; N5 em U)
        let mut p = crate::projeto::Projeto::default();
        p.tipo = crate::projeto::TipoContencao::Flexao;
        p.geometria.h = 5.0;
        let r = p.calcular();
        let forma = |pos: &str| r.armaduras.iter().find(|a| a.pos == pos).unwrap().forma.clone();
        assert!(matches!(forma("N7"), Forma::EmL { .. }) && matches!(forma("N4"), Forma::EmL { .. }) && matches!(forma("N5"), Forma::EmU { .. }));
        let d = montar(&r.armaduras, 1.0, 2);
        let todo = d.textos.iter().map(|t| t.texto.as_str()).collect::<Vec<_>>().join(" ");
        assert!(todo.contains("Tab. 9.1") && todo.contains("8Ø"));
        // vigas com estribos fechados com gancho a 135°
        let mut p = crate::projeto::Projeto::default();
        p.tipo = crate::projeto::TipoContencao::Atirantada;
        let r = p.calcular();
        let d = montar(&r.armaduras, 1.0, 2);
        let todo = d.textos.iter().map(|t| t.texto.as_str()).collect::<Vec<_>>().join(" ");
        assert!(todo.contains("gancho a 135°") && todo.contains("9.4.6.1"));
    }
}
