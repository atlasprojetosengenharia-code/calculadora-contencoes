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
}

/// Polígono CONVEXO em coordenadas do mundo (m), x → direita, y ↑.
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

    /// Setas de sobrecarga uniforme sobre um trecho horizontal.
    pub fn sobrecarga(&mut self, x0: f64, x1: f64, y: f64, altura: f64) {
        let n = ((x1 - x0) / altura).clamp(3.0, 12.0) as usize;
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i as f64 / n as f64;
            self.seta([x, y + altura], [x, y]);
        }
        self.linha(Estilo::Carga, vec![[x0, y + altura], [x1, y + altura]]);
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
