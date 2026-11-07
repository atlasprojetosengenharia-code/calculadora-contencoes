//! Perfil de solo, nível d'água e carregamentos externos.
//!
//! Convenção: `z` é a profundidade (m) medida a partir do topo do terreno
//! contido, positiva para baixo.

use serde::{Deserialize, Serialize};

/// Peso específico da água (kN/m³).
pub const GAMMA_W: f64 = 10.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camada {
    pub nome: String,
    /// Espessura (m). A última camada é considerada semi-infinita.
    pub espessura: f64,
    /// Peso específico natural, acima do NA (kN/m³).
    pub gamma: f64,
    /// Peso específico saturado, abaixo do NA (kN/m³).
    pub gamma_sat: f64,
    /// Ângulo de atrito efetivo (graus).
    pub phi: f64,
    /// Coesão efetiva (kPa).
    pub c: f64,
}

impl Camada {
    pub fn nova(nome: &str, espessura: f64, gamma: f64, gamma_sat: f64, phi: f64, c: f64) -> Self {
        Self { nome: nome.to_string(), espessura, gamma, gamma_sat, phi, c }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Perfil {
    pub camadas: Vec<Camada>,
    /// Considerar nível d'água no lado contido.
    pub usar_na: bool,
    /// Profundidade do NA a partir do topo do terreno contido (m).
    pub prof_na: f64,
}

impl Perfil {
    pub fn simples(gamma: f64, phi: f64, c: f64) -> Self {
        Self {
            camadas: vec![Camada::nova("Aterro", 10.0, gamma, gamma + 2.0, phi, c)],
            usar_na: false,
            prof_na: 3.0,
        }
    }

    pub fn na(&self) -> Option<f64> {
        if self.usar_na { Some(self.prof_na.max(0.0)) } else { None }
    }

    /// Camada que contém a profundidade `z`.
    pub fn camada_em(&self, z: f64) -> &Camada {
        let mut topo = 0.0;
        for (i, c) in self.camadas.iter().enumerate() {
            if z < topo + c.espessura || i == self.camadas.len() - 1 {
                return c;
            }
            topo += c.espessura;
        }
        self.camadas.last().expect("perfil sem camadas")
    }

    /// Tensão vertical efetiva (kPa) na profundidade `z`, considerando o terreno
    /// iniciando na profundidade `z_ini` (para o lado passivo, `z_ini` = cota de
    /// escavação) e o NA na profundidade `na` (medida do mesmo referencial).
    pub fn sigma_v_ef_desde(&self, z_ini: f64, z: f64, na: Option<f64>) -> f64 {
        if z <= z_ini {
            return 0.0;
        }
        let mut sigma = 0.0;
        let mut topo = 0.0;
        let n = self.camadas.len();
        for (i, c) in self.camadas.iter().enumerate() {
            let base = if i == n - 1 { f64::INFINITY } else { topo + c.espessura };
            let a = topo.max(z_ini);
            let b = base.min(z);
            if b > a {
                match na {
                    Some(zw) if zw < b => {
                        let seco = (zw - a).max(0.0);
                        let sub = b - a.max(zw);
                        sigma += c.gamma * seco + (c.gamma_sat - GAMMA_W) * sub;
                    }
                    _ => sigma += c.gamma * (b - a),
                }
            }
            if base >= z {
                break;
            }
            topo = base;
        }
        sigma
    }

    pub fn sigma_v_ef(&self, z: f64) -> f64 {
        self.sigma_v_ef_desde(0.0, z, self.na())
    }

    /// Poropressão hidrostática (kPa) no lado contido.
    pub fn u(&self, z: f64) -> f64 {
        match self.na() {
            Some(zw) if z > zw => GAMMA_W * (z - zw),
            _ => 0.0,
        }
    }

    /// Peso específico médio (natural) entre 0 e `h` — usado no empuxo sísmico
    /// e no peso de cunhas de solo.
    pub fn gamma_medio(&self, h: f64) -> f64 {
        if h <= 0.0 {
            return self.camadas[0].gamma;
        }
        self.sigma_v_ef_desde(0.0, h, None) / h
    }

    /// Ângulo de atrito médio ponderado pela espessura entre 0 e `h`.
    pub fn phi_medio(&self, h: f64) -> f64 {
        let n = 200;
        let dz = h / n as f64;
        (0..n).map(|i| self.camada_em((i as f64 + 0.5) * dz).phi).sum::<f64>() / n as f64
    }
}

/// Carregamentos externos aplicados ao terreno contido.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cargas {
    /// Sobrecarga uniforme distribuída no terrapleno (kPa).
    pub q_uniforme: f64,
    /// Carga em faixa (Boussinesq, parede rígida).
    pub usar_faixa: bool,
    pub q_faixa: f64,
    /// Distância da face da contenção à borda mais próxima da faixa (m).
    pub faixa_dist: f64,
    /// Largura da faixa (m).
    pub faixa_larg: f64,
    /// Ação sísmica (Mononobe-Okabe).
    pub usar_sismo: bool,
    /// Coeficiente sísmico horizontal kh (= ah/g).
    pub kh: f64,
    /// Coeficiente sísmico vertical kv.
    pub kv: f64,
}

impl Default for Cargas {
    fn default() -> Self {
        Self {
            q_uniforme: 20.0,
            usar_faixa: false,
            q_faixa: 20.0,
            faixa_dist: 1.0,
            faixa_larg: 2.0,
            usar_sismo: false,
            kh: 0.05,
            kv: 0.0,
        }
    }
}

impl Cargas {
    /// NBR 11682 7.3.3: sobrecarga acidental mínima de 20 kPa no terrapleno.
    pub fn avisar_minima(&self, r: &mut crate::resultado::Resultado, minimo: f64) {
        if self.q_uniforme + 1e-9 < minimo {
            r.aviso(format!(
                "Sobrecarga q = {:.1} kPa inferior ao mínimo de {:.0} kPa (NBR 11682, 7.3.3) — deve ser justificada.",
                self.q_uniforme, minimo
            ));
        }
    }

    /// Pressão horizontal devida à carga em faixa na profundidade `z` (kPa).
    /// Solução de Boussinesq com fator 2 para parede indeslocável.
    pub fn pressao_faixa(&self, z: f64) -> f64 {
        if !self.usar_faixa || z <= 1e-6 {
            return 0.0;
        }
        let t1 = (self.faixa_dist / z).atan();
        let t2 = ((self.faixa_dist + self.faixa_larg) / z).atan();
        let beta = t2 - t1;
        let alfa = (t1 + t2) / 2.0;
        (2.0 * self.q_faixa / std::f64::consts::PI) * (beta - beta.sin() * (2.0 * alfa).cos())
    }
}

/// Solo de fundação (abaixo da base de muros).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SoloFundacao {
    pub gamma: f64,
    /// Peso específico saturado (kN/m³) — usado abaixo do NA.
    pub gamma_sat: f64,
    pub phi: f64,
    pub c: f64,
    /// Ângulo de atrito solo-base como fração de φ (tipicamente 2/3).
    pub frac_delta_base: f64,
    /// Adesão solo-base como fração de c.
    pub frac_adesao: f64,
    /// Profundidade do NA abaixo da cota da base (m); ≤ 0 = NA na base ou
    /// acima; `None` = sem água. Definido no cálculo, não é salvo.
    #[serde(skip)]
    pub na_abaixo_base: Option<f64>,
}

impl Default for SoloFundacao {
    fn default() -> Self {
        Self {
            gamma: 19.0,
            gamma_sat: 20.0,
            phi: 30.0,
            c: 10.0,
            frac_delta_base: 2.0 / 3.0,
            frac_adesao: 0.5,
            na_abaixo_base: None,
        }
    }
}

impl SoloFundacao {
    /// Peso específico submerso γ' = γsat − γw.
    pub fn gamma_sub(&self) -> f64 {
        (self.gamma_sat - GAMMA_W).max(0.0)
    }

    /// γ efetivo para a parcela Nγ da capacidade de carga: γ' com o NA na base
    /// ou acima; interpolação linear quando o NA está a menos de `b_ef` abaixo
    /// da base; γ natural quando mais profundo ou sem água.
    pub fn gamma_nq_gama(&self, b_ef: f64) -> f64 {
        match self.na_abaixo_base {
            None => self.gamma,
            Some(dw) if dw <= 0.0 => self.gamma_sub(),
            Some(dw) if b_ef > 0.0 && dw < b_ef => self.gamma_sub() + (dw / b_ef) * (self.gamma - self.gamma_sub()),
            Some(_) => self.gamma,
        }
    }

    /// Sobrecarga efetiva na cota da base devida ao solo à frente de altura
    /// `df` (kPa), com o trecho abaixo do NA em γ'.
    pub fn sobrecarga_efetiva(&self, df: f64) -> f64 {
        let df = df.max(0.0);
        let sub = match self.na_abaixo_base {
            Some(dw) if dw < 0.0 => (-dw).min(df),
            _ => 0.0,
        };
        self.gamma * (df - sub) + self.gamma_sub() * sub
    }

    /// Junta um NA vindo de outra fonte, mantendo o mais alto (crítico).
    pub fn com_na(&self, outro: Option<f64>) -> Self {
        let mut f = self.clone();
        f.na_abaixo_base = match (self.na_abaixo_base, outro) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        f
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn fundacao_com_na() {
        let mut f = SoloFundacao { gamma: 18.0, gamma_sat: 20.0, ..Default::default() };
        // sem água
        assert_eq!(f.gamma_nq_gama(2.0), 18.0);
        assert_eq!(f.sobrecarga_efetiva(1.0), 18.0);
        // NA na base → γ' = 10
        f.na_abaixo_base = Some(0.0);
        assert!((f.gamma_nq_gama(2.0) - 10.0).abs() < 1e-9);
        // NA a 1 m abaixo, B' = 2 m → 10 + 0,5·(18 − 10) = 14
        f.na_abaixo_base = Some(1.0);
        assert!((f.gamma_nq_gama(2.0) - 14.0).abs() < 1e-9);
        // NA mais fundo que B' → γ natural
        f.na_abaixo_base = Some(3.0);
        assert_eq!(f.gamma_nq_gama(2.0), 18.0);
        // NA 0,4 m acima da base, df = 1 m → 18·0,6 + 10·0,4 = 14,8
        f.na_abaixo_base = Some(-0.4);
        assert!((f.sobrecarga_efetiva(1.0) - 14.8).abs() < 1e-9);
        // com_na mantém o NA mais alto
        let g = f.com_na(Some(-1.0));
        assert_eq!(g.na_abaixo_base, Some(-1.0));
        assert_eq!(SoloFundacao::default().com_na(None).na_abaixo_base, None);
    }
}
