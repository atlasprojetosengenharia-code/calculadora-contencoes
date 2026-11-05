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
            Criterio::Minimo => self.valor.is_finite() && self.valor >= self.limite - 1e-9,
            Criterio::Maximo => self.valor.is_finite() && self.valor <= self.limite + 1e-9,
            Criterio::Info => true,
        }
    }
    pub fn texto(&self) -> String {
        let v = fmt(self.valor);
        match self.criterio {
            Criterio::Minimo => format!(
                "{}: {} {} (mín. {}) -> {}",
                self.nome, v, self.unidade, fmt(self.limite), if self.ok() { "OK" } else { "NÃO ATENDE" }
            ),
            Criterio::Maximo => format!(
                "{}: {} {} (máx. {}) -> {}",
                self.nome, v, self.unidade, fmt(self.limite), if self.ok() { "OK" } else { "NÃO ATENDE" }
            ),
            Criterio::Info => format!("{}: {} {}", self.nome, v, self.unidade),
        }
    }
}

pub fn fmt(v: f64) -> String {
    let v = if v.abs() < 0.005 { 0.0 } else { v };
    if !v.is_finite() {
        "∞".into()
    } else if v.abs() >= 1000.0 {
        format!("{v:.0}")
    } else if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
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
    pub memorial: Vec<String>,
    pub avisos: Vec<String>,
    pub desenho: Desenho,
}

impl Resultado {
    pub fn titulo(&mut self, t: &str) {
        self.memorial.push(String::new());
        self.memorial.push(format!("== {t} =="));
    }
    pub fn linha(&mut self, t: impl Into<String>) {
        self.memorial.push(t.into());
    }
    pub fn verif(&mut self, v: Verificacao) {
        self.memorial.push(format!("  {}", v.texto()));
        self.verificacoes.push(v);
    }
    pub fn aviso(&mut self, t: impl Into<String>) {
        self.avisos.push(t.into());
    }
    pub fn atende(&self) -> bool {
        self.verificacoes.iter().all(|v| v.ok())
    }
    pub fn texto_completo(&self, titulo: &str) -> String {
        let mut s = format!("CALCULADORA DE CONTENÇÕES — {titulo}\n");
        s.push_str(&"=".repeat(60));
        s.push('\n');
        for l in &self.memorial {
            s.push_str(l);
            s.push('\n');
        }
        s.push_str("\n== RESUMO DAS VERIFICAÇÕES ==\n");
        for v in &self.verificacoes {
            s.push_str(&format!("  {}\n", v.texto()));
        }
        if !self.avisos.is_empty() {
            s.push_str("\n== AVISOS ==\n");
            for a in &self.avisos {
                s.push_str(&format!("  - {a}\n"));
            }
        }
        s
    }
}
