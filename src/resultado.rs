//! Estruturas comuns de saída: verificações, memorial e desenho da seção.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Criterio {
    /// valor ≥ limite
    Minimo,
    /// valor ≤ limite
    Maximo,
    /// Apenas informativo.
    Info,
}

#[derive(Clone, Debug)]
pub struct Verificacao {
    pub nome: String,
    pub valor: f64,
    pub limite: f64,
    pub criterio: Criterio,
    pub unidade: String,
}

impl Verificacao {
    pub fn min(nome: impl Into<String>, valor: f64, limite: f64, unidade: &str) -> Self {
        Self { nome: nome.into(), valor, limite, criterio: Criterio::Minimo, unidade: unidade.into() }
    }
    pub fn max(nome: impl Into<String>, valor: f64, limite: f64, unidade: &str) -> Self {
        Self { nome: nome.into(), valor, limite, criterio: Criterio::Maximo, unidade: unidade.into() }
    }
    pub fn info(nome: impl Into<String>, valor: f64, unidade: &str) -> Self {
        Self { nome: nome.into(), valor, limite: 0.0, criterio: Criterio::Info, unidade: unidade.into() }
    }
    pub fn ok(&self) -> bool {
        match self.criterio {
            // +∞ (sem esforço instabilizante) atende; NaN e −∞ não.
            Criterio::Minimo => !self.valor.is_nan() && self.valor >= self.limite - 1e-9,
            Criterio::Maximo => self.valor.is_finite() && self.valor <= self.limite + 1e-9,
            Criterio::Info => true,
        }
    }
    /// Símbolo de comparação do critério.
    pub fn simbolo(&self) -> &'static str {
        match self.criterio {
            Criterio::Minimo => "≥",
            Criterio::Maximo => "≤",
            Criterio::Info => "=",
        }
    }
    /// Expressão da comparação, ex.: "1,65 ≥ 1,50" (ou só o valor, se informativa).
    pub fn expressao(&self) -> String {
        let u = if self.unidade.is_empty() { String::new() } else { format!(" {}", self.unidade) };
        match self.criterio {
            Criterio::Info => format!("{}{u}", fmt(self.valor)),
            _ => format!("{}{u} {} {}{u}", fmt(self.valor), self.simbolo(), fmt(self.limite)),
        }
    }
    /// Etiqueta de status: ATENDIDO / NÃO ATENDIDO / INFORMATIVO.
    pub fn status(&self) -> &'static str {
        match self.criterio {
            Criterio::Info => "INFORMATIVO",
            _ if self.ok() => "ATENDIDO",
            _ => "NÃO ATENDIDO",
        }
    }
    pub fn texto(&self) -> String {
        format!("{}: {} → [ {} ]", self.nome, self.expressao(), self.status())
    }
}

/// Formata um número para o memorial, com vírgula decimal (padrão ABNT).
pub fn fmt(v: f64) -> String {
    let v = if v.abs() < 0.005 { 0.0 } else { v };
    let s = if !v.is_finite() {
        return if v.is_nan() { "—".into() } else if v > 0.0 { "∞".into() } else { "−∞".into() };
    } else if v.abs() >= 1000.0 {
        format!("{v:.0}")
    } else if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    };
    s.replace('.', ",")
}

/// Fórmula do memorial: equação teórica → substituição → resultado.
#[derive(Clone, Debug)]
pub struct Formula {
    pub titulo: String,
    pub equacao: String,
    pub substituicao: String,
    pub resultado: String,
}

/// Referência normativa (síntese do requisito — ver `crate::normas`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefNorma {
    pub norma: &'static str,
    pub item: &'static str,
    pub requisito: &'static str,
}

/// Elemento do memorial de cálculo, na ordem em que aparece.
#[derive(Clone, Debug)]
pub enum Bloco {
    Titulo(String),
    Texto(String),
    Formula(Formula),
    Norma(RefNorma),
    /// Índice em `Resultado::verificacoes`.
    Verif(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Estilo {
    Concreto,
    Aco,
    Solo,
    SoloFundacao,
    Agua,
    Tirante,
    Bulbo,
    Empuxo,
    EmpuxoNegativo,
    Carga,
    Ruptura,
    Momento,
    Cota,
    /// Material drenante (brita/geotêxtil).
    Dreno,
    /// Barbacãs e drenos sub-horizontais.
    Barbaca,
    /// Barra de armadura (vista longitudinal).
    Armadura,
    /// Barra de armadura em seção (ponto).
    ArmaduraSecao,
    /// Hachuras e texturas de materiais (solo, pedra, concreto, areia, brita).
    Hachura,
}

/// Polígono CONVEXO em coordenadas do mundo (m), x → direita, y ↑.
/// Gerador pseudo-aleatório determinístico (texturas estáveis entre quadros).
struct Lcg(u32);

impl Lcg {
    fn prox(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f64 / (1u32 << 24) as f64
    }
}

fn limites(pts: &[[f64; 2]]) -> (f64, f64, f64, f64) {
    pts.iter().fold((f64::MAX, f64::MIN, f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0]), a.2.min(p[1]), a.3.max(p[1])))
}

fn sentido(pts: &[[f64; 2]]) -> f64 {
    let a: f64 = (0..pts.len())
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    if a >= 0.0 { 1.0 } else { -1.0 }
}

/// Recorta a reta `o + t·d` pelo polígono convexo (Cyrus-Beck): intervalo de `t`.
fn recorta_convexo(pts: &[[f64; 2]], o: [f64; 2], d: [f64; 2]) -> Option<(f64, f64)> {
    let sg = sentido(pts);
    let (mut t0, mut t1) = (-1e9f64, 1e9f64);
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let n = [-(b[1] - a[1]) * sg, (b[0] - a[0]) * sg];
        let num = n[0] * (o[0] - a[0]) + n[1] * (o[1] - a[1]);
        let den = n[0] * d[0] + n[1] * d[1];
        if den.abs() < 1e-12 {
            if num < 0.0 {
                return None;
            }
        } else {
            let t = -num / den;
            if den > 0.0 { t0 = t0.max(t) } else { t1 = t1.min(t) }
        }
    }
    (t1 > t0).then_some((t0, t1))
}

/// O ponto está dentro do polígono convexo, com folga `margem` das arestas?
fn dentro_convexo(pts: &[[f64; 2]], p: [f64; 2], margem: f64) -> bool {
    let sg = sentido(pts);
    (0..pts.len()).all(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        let n = [-(b[1] - a[1]) * sg, (b[0] - a[0]) * sg];
        let len = (n[0] * n[0] + n[1] * n[1]).sqrt().max(1e-12);
        (n[0] * (p[0] - a[0]) + n[1] * (p[1] - a[1])) / len >= margem
    })
}

#[derive(Clone, Debug)]
pub struct Poligono {
    pub pontos: Vec<[f64; 2]>,
    pub estilo: Estilo,
}

#[derive(Clone, Debug)]
pub struct Linha {
    pub pontos: Vec<[f64; 2]>,
    pub estilo: Estilo,
    pub tracejada: bool,
}

#[derive(Clone, Debug)]
pub struct Texto {
    pub pos: [f64; 2],
    pub texto: String,
}

#[derive(Clone, Debug, Default)]
pub struct Desenho {
    pub poligonos: Vec<Poligono>,
    pub linhas: Vec<Linha>,
    pub textos: Vec<Texto>,
    /// Setas de carga: (origem, destino).
    pub setas: Vec<([f64; 2], [f64; 2])>,
}

impl Desenho {
    pub fn poli(&mut self, estilo: Estilo, pontos: Vec<[f64; 2]>) {
        self.poligonos.push(Poligono { pontos, estilo });
    }
    pub fn linha(&mut self, estilo: Estilo, pontos: Vec<[f64; 2]>) {
        self.linhas.push(Linha { pontos, estilo, tracejada: false });
    }
    pub fn tracejada(&mut self, estilo: Estilo, pontos: Vec<[f64; 2]>) {
        self.linhas.push(Linha { pontos, estilo, tracejada: true });
    }
    pub fn texto(&mut self, pos: [f64; 2], texto: impl Into<String>) {
        self.textos.push(Texto { pos, texto: texto.into() });
    }
    pub fn seta(&mut self, de: [f64; 2], ate: [f64; 2]) {
        self.setas.push((de, ate));
    }
    /// Círculo aproximado por polígono regular (convexo).
    pub fn circulo(&mut self, estilo: Estilo, c: [f64; 2], r: f64) {
        let pts = (0..16)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / 16.0;
                [c[0] + r * a.cos(), c[1] + r * a.sin()]
            })
            .collect();
        self.poli(estilo, pts);
    }
    pub fn retangulo(&mut self, estilo: Estilo, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.poli(estilo, vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
    }

    // ---- desenho técnico: texturas, cotas, níveis e canaletas -------------------

    /// Acrescenta o conteúdo de `o` deslocado de (`dx`, `dy`).
    pub fn anexar_deslocado(&mut self, o: &Desenho, dx: f64, dy: f64) {
        let mv = |p: &[f64; 2]| [p[0] + dx, p[1] + dy];
        for p in &o.poligonos {
            self.poligonos.push(Poligono { pontos: p.pontos.iter().map(mv).collect(), estilo: p.estilo });
        }
        for l in &o.linhas {
            self.linhas.push(Linha { pontos: l.pontos.iter().map(mv).collect(), estilo: l.estilo, tracejada: l.tracejada });
        }
        for t in &o.textos {
            self.textos.push(Texto { pos: mv(&t.pos), texto: t.texto.clone() });
        }
        for (a, b) in &o.setas {
            self.setas.push((mv(a), mv(b)));
        }
    }

    /// Hachura com retas de ângulo `ang` (graus) e espaçamento `esp` (m),
    /// recortada pelo polígono convexo `pts`.
    pub fn hachura(&mut self, pts: &[[f64; 2]], ang: f64, esp: f64) {
        if pts.len() < 3 || esp <= 0.0 {
            return;
        }
        let th = ang.to_radians();
        let d = [th.cos(), th.sin()];
        let u = [-th.sin(), th.cos()];
        let proj: Vec<f64> = pts.iter().map(|p| p[0] * u[0] + p[1] * u[1]).collect();
        let (mn, mx) = proj.iter().fold((f64::MAX, f64::MIN), |a, &v| (a.0.min(v), a.1.max(v)));
        if !(mx - mn).is_finite() || (mx - mn) / esp > 400.0 {
            return;
        }
        let mut c = mn + esp / 2.0;
        while c < mx {
            let o = [c * u[0], c * u[1]];
            if let Some((t0, t1)) = recorta_convexo(pts, o, d) {
                self.linha(
                    Estilo::Hachura,
                    vec![[o[0] + t0 * d[0], o[1] + t0 * d[1]], [o[0] + t1 * d[0], o[1] + t1 * d[1]]],
                );
            }
            c += esp;
        }
    }

    /// Pedras irregulares (hexágonos) inteiras dentro do polígono convexo — alvenaria
    /// de pedra, concreto ciclópico e brita.
    pub fn pedras(&mut self, pts: &[[f64; 2]], tam: f64) {
        if pts.len() < 3 || tam <= 0.0 {
            return;
        }
        let (x0, x1, y0, y1) = limites(pts);
        let mut rng = Lcg(0x9E37_79B9);
        let mut aceitas: Vec<([f64; 2], f64)> = Vec::new();
        for _ in 0..1200 {
            if aceitas.len() >= 260 {
                break;
            }
            let r = tam * (0.32 + 0.22 * rng.prox());
            let c = [x0 + rng.prox() * (x1 - x0), y0 + rng.prox() * (y1 - y0)];
            if !dentro_convexo(pts, c, 1.2 * r) {
                continue;
            }
            if aceitas.iter().any(|(q, rq)| ((c[0] - q[0]).powi(2) + (c[1] - q[1]).powi(2)).sqrt() < 1.08 * (r + rq)) {
                continue;
            }
            aceitas.push((c, r));
            let rot = rng.prox() * std::f64::consts::TAU;
            let raios: Vec<f64> = (0..6).map(|_| r * (0.8 + 0.3 * rng.prox())).collect();
            let anel: Vec<[f64; 2]> = (0..=6)
                .map(|i| {
                    let k = i % 6;
                    let a = rot + std::f64::consts::TAU * k as f64 / 6.0;
                    [c[0] + raios[k] * a.cos(), c[1] + raios[k] * a.sin()]
                })
                .collect();
            self.linha(Estilo::Hachura, anel);
        }
    }

    /// Pontilhado (concreto, areia): pequenos traços em grade com deslocamento pseudo-aleatório.
    pub fn pontilhado(&mut self, pts: &[[f64; 2]], esp: f64) {
        if pts.len() < 3 || esp <= 0.0 {
            return;
        }
        let (x0, x1, y0, y1) = limites(pts);
        if ((x1 - x0) / esp) * ((y1 - y0) / esp) > 3000.0 {
            return;
        }
        let mut rng = Lcg(0x85EB_CA6B);
        let mut y = y0 + esp / 2.0;
        while y < y1 {
            let mut x = x0 + esp / 2.0;
            while x < x1 {
                let p = [x + (rng.prox() - 0.5) * 0.8 * esp, y + (rng.prox() - 0.5) * 0.8 * esp];
                if dentro_convexo(pts, p, 0.1 * esp) {
                    let l = 0.22 * esp;
                    self.linha(Estilo::Hachura, vec![p, [p[0] + l, p[1] + 0.3 * l]]);
                }
                x += esp;
            }
            y += esp;
        }
    }

    /// Cota horizontal em `y` entre `x0` e `x1`; as linhas de extensão partem de `y_ref`.
    /// `sz` é a unidade de escala (≈ altura do texto, m).
    pub fn cota_h(&mut self, x0: f64, x1: f64, y: f64, y_ref: f64, sz: f64, texto: impl Into<String>) {
        let s = if y >= y_ref { 1.0 } else { -1.0 };
        for x in [x0, x1] {
            self.linha(Estilo::Cota, vec![[x, y_ref + s * 0.3 * sz], [x, y + s * 0.5 * sz]]);
            self.linha(Estilo::Cota, vec![[x - 0.35 * sz, y - 0.35 * sz], [x + 0.35 * sz, y + 0.35 * sz]]);
        }
        self.linha(Estilo::Cota, vec![[x0, y], [x1, y]]);
        self.texto([(x0 + x1) / 2.0, y + s * 1.0 * sz], texto);
    }

    /// Cota vertical em `x` entre `y0` e `y1`; as linhas de extensão partem de `x_ref`.
    pub fn cota_v(&mut self, y0: f64, y1: f64, x: f64, x_ref: f64, sz: f64, texto: impl Into<String>) {
        let s = if x >= x_ref { 1.0 } else { -1.0 };
        let t: String = texto.into();
        for y in [y0, y1] {
            self.linha(Estilo::Cota, vec![[x_ref + s * 0.3 * sz, y], [x + s * 0.5 * sz, y]]);
            self.linha(Estilo::Cota, vec![[x - 0.35 * sz, y - 0.35 * sz], [x + 0.35 * sz, y + 0.35 * sz]]);
        }
        self.linha(Estilo::Cota, vec![[x, y0], [x, y1]]);
        self.texto([x + s * (0.7 * sz + 0.3 * sz * t.chars().count() as f64), (y0 + y1) / 2.0], t);
    }

    /// Marca de nível (triângulo sobre a linha de cota) com legenda.
    pub fn nivel(&mut self, x: f64, y: f64, sz: f64, texto: impl Into<String>) {
        self.linha(Estilo::Cota, vec![[x, y], [x + 7.0 * sz, y]]);
        self.poli(Estilo::Cota, vec![[x, y], [x - 0.6 * sz, y + 1.0 * sz], [x + 0.6 * sz, y + 1.0 * sz]]);
        self.texto([x + 3.6 * sz, y + 1.0 * sz], texto);
    }

    /// Canaleta de concreto (seção trapezoidal 0,20 × 0,20 m, paredes de 5 cm) com o topo em `y`.
    pub fn canaleta(&mut self, xc: f64, y: f64) {
        let ext = vec![[xc - 0.15, y], [xc - 0.10, y - 0.25], [xc + 0.10, y - 0.25], [xc + 0.15, y]];
        let int = vec![[xc - 0.10, y], [xc - 0.075, y - 0.20], [xc + 0.075, y - 0.20], [xc + 0.10, y]];
        let mut e = ext.clone();
        e.push(ext[0]);
        self.linha(Estilo::Concreto, e);
        self.linha(Estilo::Concreto, int);
    }

    /// Hachura o solo (45°) e a fundação (135°) de qualquer desenho já montado.
    pub fn hachurar_solos(&mut self) {
        let solos: Vec<(Vec<[f64; 2]>, bool)> = self
            .poligonos
            .iter()
            .filter(|p| matches!(p.estilo, Estilo::Solo | Estilo::SoloFundacao))
            .map(|p| (p.pontos.clone(), p.estilo == Estilo::SoloFundacao))
            .collect();
        if solos.is_empty() {
            return;
        }
        let todos: Vec<[f64; 2]> = solos.iter().flat_map(|(p, _)| p.iter().copied()).collect();
        let (x0, x1, y0, y1) = limites(&todos);
        let diag = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let esp = diag / 60.0;
        for (p, fund) in solos {
            if fund {
                self.hachura(&p, 135.0, esp * 1.6);
            } else {
                self.hachura(&p, 45.0, esp);
            }
        }
    }

    /// Diagrama de pressões desenhado em faixas trapezoidais (sempre convexas).
    /// `pts` = (y, valor); o valor é desenhado na direção +x a partir de `x0`
    /// multiplicado por `escala`.
    pub fn diagrama(&mut self, x0: f64, escala: f64, pts: &[(f64, f64)]) {
        if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
            let mut contorno = vec![[x0, a.0]];
            contorno.extend(pts.iter().map(|&(y, p)| [x0 + p * escala, y]));
            contorno.push([x0, b.0]);
            self.linha(Estilo::Empuxo, contorno);
        }
        for w in pts.windows(2) {
            let (y0, p0) = w[0];
            let (y1, p1) = w[1];
            let estilo = if p0 + p1 >= 0.0 { Estilo::Empuxo } else { Estilo::EmpuxoNegativo };
            self.poli(
                estilo,
                vec![[x0, y0], [x0 + p0 * escala, y0], [x0 + p1 * escala, y1], [x0, y1]],
            );
        }
    }

    /// Setas de sobrecarga sobre uma superfície `y = sup(x)` (terrapleno em talude).
    pub fn sobrecarga_sup(&mut self, x0: f64, x1: f64, altura: f64, sup: &dyn Fn(f64) -> f64) {
        let n = ((x1 - x0) / altura).clamp(3.0, 12.0) as usize;
        let mut topo = Vec::new();
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            let y = sup(x);
            self.seta([x, y + altura], [x, y]);
            topo.push([x, y + altura]);
        }
        self.linha(Estilo::Carga, topo);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Resultado {
    pub verificacoes: Vec<Verificacao>,
    /// Memorial de cálculo estruturado.
    pub blocos: Vec<Bloco>,
    pub avisos: Vec<String>,
    pub desenho: Desenho,
    /// Tabela de armação (detalhamento do aço).
    pub armaduras: Vec<crate::armadura::Armadura>,
    /// Reforços (tirantes/grampos) para a estabilidade global.
    pub reforcos: Vec<crate::global::Reforco>,
}

impl Resultado {
    pub fn titulo(&mut self, t: &str) {
        self.blocos.push(Bloco::Titulo(t.into()));
    }
    pub fn linha(&mut self, t: impl Into<String>) {
        self.blocos.push(Bloco::Texto(t.into()));
    }
    pub fn verif(&mut self, v: Verificacao) {
        self.verificacoes.push(v);
        self.blocos.push(Bloco::Verif(self.verificacoes.len() - 1));
    }
    /// Fórmula em três passos: equação teórica, substituição e resultado.
    pub fn formula(
        &mut self,
        titulo: impl Into<String>,
        equacao: impl Into<String>,
        substituicao: impl Into<String>,
        resultado: impl Into<String>,
    ) {
        self.blocos.push(Bloco::Formula(Formula {
            titulo: titulo.into(),
            equacao: equacao.into(),
            substituicao: substituicao.into(),
            resultado: resultado.into(),
        }));
    }
    /// Caixa de referência normativa (cada referência aparece uma única vez).
    pub fn norma(&mut self, r: RefNorma) {
        if !self.blocos.iter().any(|b| matches!(b, Bloco::Norma(x) if *x == r)) {
            self.blocos.push(Bloco::Norma(r));
        }
    }
    pub fn aviso(&mut self, t: impl Into<String>) {
        self.avisos.push(t.into());
    }
    pub fn atende(&self) -> bool {
        self.verificacoes.iter().all(|v| v.ok())
    }

    /// Memorial em texto puro (copiar / exportar .txt).
    pub fn texto_completo(&self, titulo: &str) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "CALCULADORA DE CONTENÇÕES — {titulo}");
        let _ = writeln!(s, "{}", "=".repeat(72));
        for b in &self.blocos {
            let _ = match b {
                Bloco::Titulo(t) => writeln!(s, "\n== {t} =="),
                Bloco::Texto(t) => writeln!(s, "{t}"),
                Bloco::Formula(f) => {
                    let _ = writeln!(s, "  ┌─ {}", f.titulo);
                    let _ = writeln!(s, "  │ Equação:      {}", f.equacao);
                    if !f.substituicao.is_empty() {
                        let _ = writeln!(s, "  │ Substituição: {}", f.substituicao);
                    }
                    writeln!(s, "  └ Resultado:    {}", f.resultado)
                }
                Bloco::Norma(n) => writeln!(s, "  ▌ {} — {}\n  ▌ {}", n.norma, n.item, n.requisito),
                Bloco::Verif(i) => writeln!(s, "  {}", self.verificacoes[*i].texto()),
            };
        }
        if !self.armaduras.is_empty() {
            let _ = writeln!(s, "\n== TABELA DE ARMAÇÃO (comprimentos aproximados) ==");
            let _ = writeln!(s, "  {}", crate::armadura::CABECALHO.join(" | "));
            for a in &self.armaduras {
                let _ = writeln!(s, "  {}", a.celulas().join(" | "));
            }
        }
        let _ = writeln!(s, "\n== RESUMO DAS VERIFICAÇÕES ==");
        for v in &self.verificacoes {
            let _ = writeln!(s, "  {}", v.texto());
        }
        if !self.avisos.is_empty() {
            let _ = writeln!(s, "\n== AVISOS ==");
            for a in &self.avisos {
                let _ = writeln!(s, "  - {a}");
            }
        }
        let _ = writeln!(s, "\nReferências normativas: sínteses dos requisitos — consulte o texto integral das normas ABNT.");
        s
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn fs_infinito_atende() {
        // FS = ∞ (sem esforço instabilizante) atende a um mínimo
        assert!(Verificacao::min("FS", f64::INFINITY, 1.5, "").ok());
        assert!(!Verificacao::min("FS", f64::NAN, 1.5, "").ok());
        assert!(!Verificacao::min("FS", f64::NEG_INFINITY, 1.5, "").ok());
        assert!(!Verificacao::min("FS", 1.49, 1.5, "").ok());
        // em um máximo, ∞ continua reprovando (ex.: seção insuficiente)
        assert!(!Verificacao::max("taxa", f64::INFINITY, 4.0, "%").ok());
    }

    #[test]
    fn expressao_e_status() {
        let v = Verificacao::min("FS deslizamento", 1.65, 1.5, "");
        assert_eq!(v.expressao(), "1,65 ≥ 1,50");
        assert_eq!(v.status(), "ATENDIDO");
        assert_eq!(v.texto(), "FS deslizamento: 1,65 ≥ 1,50 → [ ATENDIDO ]");
        let v = Verificacao::max("e", 0.4, 0.3, "m");
        assert_eq!(v.expressao(), "0,40 m ≤ 0,30 m");
        assert_eq!(v.status(), "NÃO ATENDIDO");
        assert_eq!(fmt(1234.6), "1235");
        assert_eq!(fmt(f64::INFINITY), "∞");
    }

    #[test]
    fn blocos_na_ordem_e_norma_sem_repeticao() {
        let mut r = Resultado::default();
        let n = RefNorma { norma: "NBR X", item: "1", requisito: "req" };
        r.titulo("T");
        r.norma(n);
        r.formula("F", "a = b", "a = 1", "a = 1 m");
        r.verif(Verificacao::min("FS", 2.0, 1.5, ""));
        r.norma(n);
        assert_eq!(r.blocos.len(), 4);
        assert!(matches!(r.blocos[3], Bloco::Verif(0)));
        let t = r.texto_completo("x");
        assert!(t.contains("Substituição: a = 1") && t.contains("[ ATENDIDO ]"));
    }
}
