//! Contenções embutidas: estaca-prancha e paredes de estacas.
//! Parede em balanço (Blum simplificado) ou com uma linha de apoio
//! (suporte livre — free earth support). Cálculo por metro de parede.

use crate::concreto::{as_circular, Concreto};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil, GAMMA_W};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Apoio {
    Balanco,
    Ancorada,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParedeEmbutida {
    /// Altura escavada (m).
    pub h: f64,
    pub apoio: Apoio,
    /// Profundidade da linha de tirantes/estroncas (m).
    pub prof_apoio: f64,
    /// Inclinação do tirante com a horizontal (graus).
    pub incl_apoio: f64,
    /// Espaçamento horizontal entre apoios (m).
    pub espac_apoio: f64,
    pub usar_na_frente: bool,
    /// Profundidade do NA no lado escavado, a partir do topo do terreno contido (m).
    pub prof_na_frente: f64,
    /// Atrito solo-parede no lado passivo (graus) — usado em Coulomb.
    pub delta_passivo: f64,
    pub perfil: Perfil,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
}

impl Default for ParedeEmbutida {
    fn default() -> Self {
        let mut perfil = Perfil::simples(18.0, 30.0, 0.0);
        perfil.camadas[0].espessura = 30.0;
        Self {
            h: 4.0,
            apoio: Apoio::Balanco,
            prof_apoio: 1.0,
            incl_apoio: 15.0,
            espac_apoio: 2.5,
            usar_na_frente: false,
            prof_na_frente: 4.0,
            delta_passivo: 10.0,
            perfil,
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Solucao {
    /// Ficha teórica (m).
    pub d0: f64,
    /// Ficha de projeto (m).
    pub d: f64,
    /// Força no apoio (kN/m) — 0 em balanço; contraforça de Blum em balanço.
    pub t: f64,
    pub contraforca: f64,
    pub m_max: f64,
    pub z_m_max: f64,
    pub v_max: f64,
    /// (z, p líquida)
    pub pressoes: Vec<(f64, f64)>,
    /// (z, M)
    pub momentos: Vec<(f64, f64)>,
}

impl ParedeEmbutida {
    fn na_frente(&self) -> Option<f64> {
        if self.usar_na_frente { Some(self.prof_na_frente.max(self.h)) } else { None }
    }

    fn pressao_passiva(&self, z: f64, fs_p: f64) -> f64 {
        if z <= self.h {
            return 0.0;
        }
        let cam = self.perfil.camada_em(z);
        let kp = self.empuxo.coef_passivo(cam.phi, self.delta_passivo);
        let sv = self.perfil.sigma_v_ef_desde(self.h, z, self.na_frente());
        let incl = if self.empuxo.teoria == Teoria::Coulomb { rad(self.delta_passivo) } else { 0.0 };
        let pp = (kp * sv + 2.0 * cam.c * kp.sqrt()) * incl.cos() / fs_p;
        let u = match self.na_frente() {
            Some(zw) if z > zw => GAMMA_W * (z - zw),
            _ => 0.0,
        };
        pp + u
    }

    fn incremento_sismico(&self, z: f64, dp: f64) -> f64 {
        if z >= self.h || dp <= 0.0 {
            return 0.0;
        }
        2.0 * dp / self.h * (1.0 - z / self.h)
    }

    /// Pressão líquida horizontal (kPa): + empurra a parede para a escavação.
    pub fn pressao_liquida(&self, z: f64, fs_p: f64, dp_sis: f64) -> f64 {
        self.empuxo.pressao_horizontal_total(&self.perfil, &self.cargas, 0.0, z)
            + self.incremento_sismico(z, dp_sis)
            - self.pressao_passiva(z, fs_p)
    }

    fn equilibrio(&self, d: f64, fs_p: f64, dp: f64) -> f64 {
        let zt = self.h + d;
        let n = 600;
        match self.apoio {
            Apoio::Balanco => integrar(|z| self.pressao_liquida(z, fs_p, dp) * (zt - z), 0.0, zt, n).0,
            Apoio::Ancorada => {
                let za = self.prof_apoio;
                integrar(|z| self.pressao_liquida(z, fs_p, dp) * (z - za), 0.0, zt, n).0
            }
        }
    }

    pub fn resolver(&self, fat: &Fatores, sismo: bool) -> Result<Solucao, String> {
        let fs_p = fat.fs_passivo.max(1.0);
        let dp = if sismo && self.cargas.usar_sismo {
            let (dp, _, _) = self.empuxo.incremento_sismico(&self.perfil, &self.cargas, 0.0, self.h);
            dp * self.empuxo.inclinacao_ativa(0.0).cos()
        } else {
            0.0
        };
        if self.apoio == Apoio::Ancorada && (self.prof_apoio <= 0.0 || self.prof_apoio >= self.h) {
            return Err("Profundidade do apoio deve estar entre 0 e a altura escavada.".into());
        }

        // Busca da ficha teórica: primeira mudança de sinal da equação de equilíbrio
        let passo = (0.05 * self.h).max(0.05);
        let d_max = 5.0 * self.h + 10.0;
        let mut a = 1e-3;
        let mut d0 = None;
        if self.equilibrio(a, fs_p, dp) <= 0.0 {
            d0 = Some(0.0);
        }
        while d0.is_none() && a < d_max {
            let b = a + passo;
            if self.equilibrio(b, fs_p, dp) <= 0.0 {
                let (mut lo, mut hi) = (a, b);
                for _ in 0..50 {
                    let m = 0.5 * (lo + hi);
                    if self.equilibrio(m, fs_p, dp) > 0.0 { lo = m } else { hi = m }
                }
                d0 = Some(hi);
            }
            a = b;
        }
        let d0 = d0.ok_or("Não foi encontrada ficha que equilibre a parede — revisar parâmetros (solo, FS do passivo, altura).")?;

        let zt = self.h + d0;
        let fator = match self.apoio {
            Apoio::Balanco => fat.acrescimo_ficha_balanco,
            Apoio::Ancorada => fat.acrescimo_ficha_ancorada,
        };

        // Esforços ao longo da parede (configuração teórica)
        let n = 1000;
        let dz = zt / n as f64;
        let p: Vec<f64> = (0..=n).map(|i| self.pressao_liquida(i as f64 * dz, fs_p, dp)).collect();
        let resultante = integrar(|z| self.pressao_liquida(z, fs_p, dp), 0.0, zt, 2000).0;
        let t = if self.apoio == Apoio::Ancorada { resultante } else { 0.0 };
        let (mut v, mut m) = (0.0, 0.0);
        let mut momentos = vec![(0.0, 0.0)];
        let (mut m_max, mut z_m, mut v_max) = (0.0f64, 0.0, 0.0f64);
        let mut ancora_aplicada = self.apoio == Apoio::Balanco;
        for i in 0..n {
            let z = (i + 1) as f64 * dz;
            let v_ant = v;
            v += 0.5 * (p[i] + p[i + 1]) * dz;
            if !ancora_aplicada && z >= self.prof_apoio {
                v -= t;
                ancora_aplicada = true;
            }
            m += 0.5 * (v_ant + v) * dz;
            if m.abs() > m_max.abs() {
                m_max = m;
                z_m = z;
            }
            v_max = v_max.max(v.abs());
            momentos.push((z, m));
        }
        let pressoes = (0..=n).step_by(10).map(|i| (i as f64 * dz, p[i])).collect();

        Ok(Solucao {
            d0,
            d: d0 * fator,
            t,
            contraforca: if self.apoio == Apoio::Balanco { -resultante } else { 0.0 },
            m_max: m_max.abs(),
            z_m_max: z_m,
            v_max,
            pressoes,
            momentos,
        })
    }

    /// Resolve os casos estático e sísmico e registra o memorial.
    /// Retorna a solução envoltória (maior ficha / momento / força).
    pub fn analisar(&self, r: &mut Resultado, fat: &Fatores) -> Option<Solucao> {
        self.cargas.avisar_minima(r, fat.sobrecarga_minima);
        r.titulo("Dados");
        r.norma(normas::NBR11682_SOBRECARGA);
        r.linha(format!(
            "Altura escavada H = {} m ; {} ; teoria: {} ; δ ativo = {}° ; δ passivo = {}° ; β = {}°",
            fmt(self.h),
            match self.apoio {
                Apoio::Balanco => "parede em balanço".to_string(),
                Apoio::Ancorada => format!("apoio a {} m de profundidade", fmt(self.prof_apoio)),
            },
            self.empuxo.teoria.nome(),
            fmt(self.empuxo.delta),
            fmt(self.delta_passivo),
            fmt(self.empuxo.beta)
        ));
        r.linha(format!(
            "FS no passivo = {} ; acréscimo de ficha = {} ; q = {} kPa",
            fmt(fat.fs_passivo),
            fmt(match self.apoio {
                Apoio::Balanco => fat.acrescimo_ficha_balanco,
                Apoio::Ancorada => fat.acrescimo_ficha_ancorada,
            }),
            fmt(self.cargas.q_uniforme)
        ));
        for (i, c) in self.perfil.camadas.iter().enumerate() {
            let ka = self.empuxo.coef_ativo(c.phi, 0.0);
            let kp = self.empuxo.coef_passivo(c.phi, self.delta_passivo);
            r.linha(format!(
                "Camada {} ({}): esp = {} m ; γ = {} ; φ = {}° ; c = {} kPa ; Ka = {} ; Kp = {}",
                i + 1, c.nome, fmt(c.espessura), fmt(c.gamma), fmt(c.phi), fmt(c.c), fmt(ka), fmt(kp)
            ));
        }

        let c0 = &self.perfil.camadas[0];
        let (ka0, kp0) = (self.empuxo.coef_ativo(c0.phi, 0.0), self.empuxo.coef_passivo(c0.phi, self.delta_passivo));
        r.formula(
            format!("Coeficientes de empuxo — {} (camada 1)", self.empuxo.teoria.nome()),
            match self.empuxo.teoria {
                Teoria::Rankine => "Ka = tan²(45° − φ/2) ;  Kp = tan²(45° + φ/2)",
                Teoria::Coulomb => "Ka, Kp de Coulomb (δ ativo / δ passivo, paramento vertical)",
            },
            format!("φ = {}° ;  δa = {}° ;  δp = {}°", fmt(c0.phi), fmt(self.empuxo.delta), fmt(self.delta_passivo)),
            format!("Ka = {} ;  Kp = {}", fmt(ka0), fmt(kp0)),
        );
        r.formula(
            "Pressão líquida sobre a parede",
            "p(z) = Ka·(σ'v + q) − 2c'·√Ka + u − [Kp·σ'v,p + 2c'·√Kp] / FS_p − u_p",
            format!("q = {} kPa ;  FS_p = {}", fmt(self.cargas.q_uniforme), fmt(fat.fs_passivo.max(1.0))),
            "Diagrama de p(z) desenhado na seção transversal",
        );

        let mut casos = vec![("estático", false)];
        if self.cargas.usar_sismo {
            casos.push(("sísmico", true));
        }
        let mut env: Option<Solucao> = None;
        for (nome, sis) in casos {
            match self.resolver(fat, sis) {
                Ok(s) => {
                    r.titulo(&format!("Ficha e esforços — caso {nome} (por metro)"));
                    let (eq, txt) = match self.apoio {
                        Apoio::Balanco => (
                            "Blum: Σ M_pé = ∫₀^(H+D0) [pa(z) − pp(z)/FS_p]·(H + D0 − z) dz = 0 ;  D = f·D0",
                            "momentos em torno do pé da parede",
                        ),
                        Apoio::Ancorada => (
                            "Suporte livre: Σ M_apoio = ∫₀^(H+D0) [pa(z) − pp(z)/FS_p]·(z − z_a) dz = 0 ;  T = ∫ p_líq dz",
                            "momentos em torno do apoio",
                        ),
                    };
                    r.formula(
                        format!("Ficha — caso {nome} ({txt})"),
                        eq,
                        format!(
                            "FS_p = {} ;  f = {} ;  D0 = {} m",
                            fmt(fat.fs_passivo.max(1.0)),
                            fmt(match self.apoio {
                                Apoio::Balanco => fat.acrescimo_ficha_balanco,
                                Apoio::Ancorada => fat.acrescimo_ficha_ancorada,
                            }),
                            fmt(s.d0)
                        ),
                        format!("D = {} m ;  M_máx = {} kN·m/m ;  V_máx = {} kN/m", fmt(s.d), fmt(s.m_max), fmt(s.v_max)),
                    );
                    r.linha(format!("Ficha teórica D0 = {} m ; ficha de projeto D = {} m", fmt(s.d0), fmt(s.d)));
                    r.linha(format!("Comprimento total da parede = {} m", fmt(self.h + s.d)));
                    if self.apoio == Apoio::Ancorada {
                        r.linha(format!("Reação no apoio T = {} kN/m", fmt(s.t)));
                    } else {
                        r.linha(format!("Contraforça de Blum (abaixo do ponto de rotação) = {} kN/m", fmt(s.contraforca)));
                    }
                    r.linha(format!(
                        "Momento máximo = {} kN·m/m a z = {} m ; cortante máxima = {} kN/m",
                        fmt(s.m_max), fmt(s.z_m_max), fmt(s.v_max)
                    ));
                    env = Some(match env {
                        None => s,
                        Some(e) => {
                            // mantém os diagramas do caso de maior momento e
                            // toma o máximo de cada grandeza
                            let (mut g, o) = if s.m_max >= e.m_max { (s, e) } else { (e, s) };
                            g.d = g.d.max(o.d);
                            g.d0 = g.d0.max(o.d0);
                            g.t = g.t.max(o.t);
                            g.v_max = g.v_max.max(o.v_max);
                            g
                        }
                    });
                }
                Err(e) => {
                    r.aviso(format!("Caso {nome}: {e}"));
                    r.verif(Verificacao::min(format!("Equilíbrio da parede ({nome})"), 0.0, 1.0, ""));
                }
            }
        }
        if let Some(s) = &env {
            r.verif(Verificacao::info("Ficha de projeto D", s.d, "m"));
            r.verif(Verificacao::info("Comprimento total", self.h + s.d, "m"));
            r.verif(Verificacao::info("Momento máximo (característico)", s.m_max, "kN·m/m"));
            if self.apoio == Apoio::Ancorada {
                let ti = s.t * self.espac_apoio / rad(self.incl_apoio).cos();
                r.linha(format!(
                    "Carga por tirante/estronca = T·s / cos i = {} × {} / cos {}° = {} kN",
                    fmt(s.t), fmt(self.espac_apoio), fmt(self.incl_apoio), fmt(ti)
                ));
                r.verif(Verificacao::info("Carga de trabalho por apoio", ti, "kN"));
            }
        }
        env
    }

    /// Desenho comum: solo, água, sobrecarga, apoio e diagramas.
    pub fn desenho(&self, s: Option<&Solucao>, espessura: f64, estilo: Estilo) -> Desenho {
        let mut d = Desenho::default();
        let h = self.h;
        let ficha = s.map(|s| s.d).unwrap_or(0.5 * h);
        let lt = h + ficha;
        let fundo = -(lt + 0.15 * h);
        let xe = 1.1 * lt.max(h);
        let xi = -0.9 * lt.max(h);
        d.retangulo(Estilo::Solo, 0.0, fundo, xe, 0.0);
        d.retangulo(Estilo::Solo, xi, fundo, 0.0, -h);
        // camadas
        let mut z = 0.0;
        for c in &self.perfil.camadas[..self.perfil.camadas.len().saturating_sub(1)] {
            z += c.espessura;
            if z < lt + 0.15 * h {
                d.tracejada(Estilo::Cota, vec![[xi, -z], [xe, -z]]);
            }
        }
        d.retangulo(estilo, -espessura / 2.0, -lt, espessura / 2.0, 0.0);
        if let Some(na) = self.perfil.na() {
            d.tracejada(Estilo::Agua, vec![[0.0, -na], [xe, -na]]);
            d.texto([xe * 0.8, -na + 0.05 * h], "NA");
        }
        if let Some(na) = self.na_frente() {
            d.tracejada(Estilo::Agua, vec![[xi, -na], [0.0, -na]]);
        }
        if self.cargas.q_uniforme > 0.0 {
            d.sobrecarga(0.2, xe, 0.0, 0.08 * lt);
            d.texto([xe * 0.6, 0.14 * lt], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        if self.cargas.usar_faixa {
            let x0 = self.cargas.faixa_dist;
            d.retangulo(Estilo::Carga, x0, 0.0, x0 + self.cargas.faixa_larg, 0.05 * lt);
        }
        if self.apoio == Apoio::Ancorada {
            let i = rad(self.incl_apoio);
            let l = 0.7 * h;
            d.linha(Estilo::Tirante, vec![[0.0, -self.prof_apoio], [l * i.cos(), -self.prof_apoio - l * i.sin()]]);
        }
        if let Some(s) = s {
            let pmax = s.pressoes.iter().map(|p| p.1.abs()).fold(1e-6, f64::max);
            let pts: Vec<(f64, f64)> = s.pressoes.iter().map(|&(z, p)| (-z, p)).collect();
            d.diagrama(espessura / 2.0, 0.25 * lt / pmax, &pts);
            let mmax = s.momentos.iter().map(|m| m.1.abs()).fold(1e-6, f64::max);
            let esc = 0.25 * lt / mmax;
            let linha: Vec<[f64; 2]> = s.momentos.iter().step_by(5).map(|&(z, m)| [-espessura / 2.0 - m.abs() * esc, -z]).collect();
            d.linha(Estilo::Momento, linha);
            d.texto([-espessura / 2.0 - 0.25 * lt - 0.12 * h, -s.z_m_max], format!("Mmáx = {} kN·m/m", fmt(s.m_max)));
            d.texto([0.25 * lt + 0.15 * h, -0.35 * h], format!("p líquida (máx. {} kPa)", fmt(pmax)));
            d.tracejada(Estilo::Cota, vec![[xi, -lt], [xi * 0.3, -lt]]);
            d.texto([xi * 0.6, -lt + 0.05 * h], format!("D = {} m", fmt(s.d)));
        }
        d.texto([xi * 0.6, -h / 2.0], format!("H = {} m", fmt(h)));
        d
    }
}

// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EstacaPrancha {
    pub parede: ParedeEmbutida,
    pub nome_perfil: String,
    /// Módulo resistente elástico do perfil (cm³/m).
    pub w_perfil: f64,
    /// Tensão de escoamento do aço (MPa).
    pub fy: f64,
}

impl Default for EstacaPrancha {
    fn default() -> Self {
        Self { parede: ParedeEmbutida::default(), nome_perfil: "Perfil a definir".into(), w_perfil: 1300.0, fy: 270.0 }
    }
}

impl EstacaPrancha {
    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let s = self.parede.analisar(&mut r, fat);
        if let Some(s) = &s {
            r.titulo("Dimensionamento do perfil (NBR 8800 — flexão elástica)");
            let md = fat.gamma_f * s.m_max;
            let fyd = self.fy / fat.gamma_a1;
            let w_nec = md * 1e6 / fyd / 1000.0;
            r.linha(format!(
                "Md = γf·M = {} kN·m/m ; fyd = fy/γa1 = {} MPa ; Wnec = Md/fyd = {} cm³/m",
                fmt(md), fmt(fyd), fmt(w_nec)
            ));
            r.norma(normas::NBR8800_FLEXAO);
            r.formula(
                "Módulo resistente necessário do perfil",
                "Md = γf·M_máx ;  fyd = fy / γa1 ;  W_nec = Md / fyd",
                format!("Md = {}·{} ;  fyd = {} / {}", fmt(fat.gamma_f), fmt(s.m_max), fmt(self.fy), fmt(fat.gamma_a1)),
                format!("W_nec = {} cm³/m ;  W_perfil = {} cm³/m", fmt(w_nec), fmt(self.w_perfil)),
            );
            r.verif(Verificacao::min(format!("W do perfil ({})", self.nome_perfil), self.w_perfil, w_nec, "cm³/m"));
            r.aviso("Verificar também deslocamentos e a classe da seção (flambagem local) conforme catálogo do fabricante.");
        }
        r.aviso("Estabilidade global (NBR 11682) e ruptura de fundo ainda não implementadas.");
        r.desenho = self.parede.desenho(s.as_ref(), 0.012 * self.parede.h.max(4.0), Estilo::Aco);
        r
    }
}

// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParedeEstacas {
    pub parede: ParedeEmbutida,
    pub diametro: f64,
    pub espacamento: f64,
    pub n_barras: usize,
    pub concreto: Concreto,
}

impl Default for ParedeEstacas {
    fn default() -> Self {
        let mut c = Concreto::default();
        c.cobrimento = 0.05;
        Self { parede: ParedeEmbutida::default(), diametro: 0.5, espacamento: 0.6, n_barras: 10, concreto: c }
    }
}

impl ParedeEstacas {
    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let s = self.parede.analisar(&mut r, fat);
        let c = &self.concreto;
        if self.espacamento < self.diametro * 0.999 {
            r.linha("Estacas secantes (espaçamento < diâmetro).");
        }
        if let Some(s) = &s {
            r.titulo("Dimensionamento da estaca (NBR 6118 — seção circular)");
            let m = s.m_max * self.espacamento;
            let v = s.v_max * self.espacamento;
            let md = fat.gamma_f * m;
            let c_eixo = c.cobrimento + 0.01 + 0.0125;
            r.linha(format!(
                "Por estaca: M = {} kN·m ; V = {} kN ; Md = {} kN·m ; Vd = {} kN",
                fmt(m), fmt(v), fmt(md), fmt(fat.gamma_f * v)
            ));
            let ac = std::f64::consts::PI * self.diametro.powi(2) / 4.0 * 1e4;
            let as_min = 0.004 * ac;
            match as_circular(md, self.diametro, c_eixo, self.n_barras.max(6), c.fck, c.fyk, fat.gamma_c, fat.gamma_s) {
                Some(a) => {
                    r.linha(format!(
                        "As calculada = {} cm² ({} barras) ; As mín (0,4% Ac) = {} cm² ; adotar {} cm²",
                        fmt(a), self.n_barras, fmt(as_min), fmt(a.max(as_min))
                    ));
                    r.norma(normas::NBR6118_FLEXAO);
                    r.norma(normas::NBR6118_ARM_LONGITUDINAL);
                    r.formula(
                        "Armadura longitudinal da estaca (seção circular, flexão simples)",
                        "M_Rd(As) ≥ Md  (diagrama retangular 0,9·0,85·ηc·fcd ; εcu = 3,5 ‰) ;  0,4 % ≤ As/Ac ≤ 4 %",
                        format!(
                            "Md = {}·{} kN·m ;  Ø = {} m ;  {} barras ;  fck = {} MPa",
                            fmt(fat.gamma_f), fmt(m), fmt(self.diametro), self.n_barras, fmt(c.fck)
                        ),
                        format!("As = {} cm² ;  As,mín = {} cm² ;  taxa = {} %", fmt(a), fmt(as_min), fmt(100.0 * a.max(as_min) / ac)),
                    );
                    r.verif(Verificacao::max("Taxa de armadura longitudinal", 100.0 * a.max(as_min) / ac, 4.0, "%"));
                }
                None => {
                    r.aviso("Seção da estaca insuficiente mesmo com 4% de armadura — aumentar diâmetro ou reduzir espaçamento.");
                    r.verif(Verificacao::max("Taxa de armadura longitudinal", f64::INFINITY, 4.0, "%"));
                }
            }
            r.aviso("Cortante na estaca: dimensionar estribos/espiral conforme NBR 6118 (não verificado automaticamente).");
        }
        if self.espacamento > 3.0 * self.diametro {
            r.aviso("Espaçamento > 3D: verificar efeito de arco do solo entre estacas e necessidade de parede entre estacas.");
        }
        r.aviso("Empuxo passivo calculado como parede contínua; para estacas espaçadas a largura efetiva do passivo é limitada (≈ 3D).");
        r.aviso("Estabilidade global (NBR 11682) e ruptura de fundo ainda não implementadas.");
        r.desenho = self.parede.desenho(s.as_ref(), self.diametro, Estilo::Concreto);
        r
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::fatores::Fatores;

    #[test]
    fn balanco_blum_areia() {
        // Rankine, φ=30°, γ=18, sem sobrecarga, FS passivo = 1:
        // Ka(H+D)^3 = Kp D^3  ->  D0 = 4 / (9^(1/3) - 1) = 3,703 m
        let mut p = ParedeEmbutida::default();
        p.cargas.q_uniforme = 0.0;
        let mut f = Fatores::default();
        f.fs_passivo = 1.0;
        let s = p.resolver(&f, false).unwrap();
        assert!((s.d0 - 3.703).abs() < 0.02, "D0 = {}", s.d0);
    }

    #[test]
    fn ancorada_suporte_livre() {
        // Suporte livre, apoio no topo (z=0,001), φ=30°, γ=18, H=4, FS=1:
        // momento em torno do apoio: Ka (H+D)^3 /3 = Kp D^2 (H + 2D/3)/2
        let mut p = ParedeEmbutida::default();
        p.cargas.q_uniforme = 0.0;
        p.apoio = Apoio::Ancorada;
        p.prof_apoio = 0.001;
        let mut f = Fatores::default();
        f.fs_passivo = 1.0;
        let s = p.resolver(&f, false).unwrap();
        let g = |d: f64| (1.0 / 3.0) * (4.0 + d).powi(3) / 3.0 - 3.0 * d * d * (4.0 + 2.0 * d / 3.0) / 2.0;
        assert!(g(s.d0).abs() < 0.5, "D0 = {} ; resíduo = {}", s.d0, g(s.d0));
        // T = Pa - Pp
        let t = 0.5 * 18.0 / 3.0 * (4.0 + s.d0).powi(2) - 0.5 * 18.0 * 3.0 * s.d0 * s.d0;
        assert!((s.t - t).abs() < 0.5, "T = {} (esperado {t})", s.t);
    }
}
