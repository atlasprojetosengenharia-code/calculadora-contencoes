//! Fatores de segurança e coeficientes normativos — editáveis na aba
//! "Fatores / Normas". Valores padrão conforme as normas fornecidas:
//! NBR 11682:2009, NBR 6122:2019, NBR 5629:2018, NBR 16920-2:2021 e
//! NBR 6118:2023.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Nivel {
    Alto,
    Medio,
    Baixo,
}

impl Nivel {
    pub fn nome(&self) -> &'static str {
        match self {
            Nivel::Alto => "Alto",
            Nivel::Medio => "Médio",
            Nivel::Baixo => "Baixo",
        }
    }
}

/// NBR 11682:2009 Tabela 3 (idêntica à NBR 5629 Tab. B.3 e NBR 16920-2 Tab. B.3):
/// linhas = danos materiais/ambientais, colunas = vidas humanas.
pub fn fs_tabela3(vidas: Nivel, danos: Nivel) -> f64 {
    let i = |n: Nivel| match n {
        Nivel::Alto => 0,
        Nivel::Medio => 1,
        Nivel::Baixo => 2,
    };
    const T: [[f64; 3]; 3] = [[1.5, 1.5, 1.4], [1.5, 1.4, 1.3], [1.4, 1.3, 1.2]];
    T[i(danos)][i(vidas)]
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fixacao {
    Rosca,
    Clavete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Fatores {
    // --- NBR 11682 Tabela 3 — estabilidade (global, cunhas, interna/externa) ---
    pub nivel_vidas: Nivel,
    pub nivel_danos: Nivel,
    /// Nota 1 da Tabela 3: grande variabilidade dos ensaios → +10 %.
    pub majorar_variabilidade: bool,

    // --- NBR 11682:2009 Tabela 4 — muros de gravidade e de flexão ---
    // Tombamento 2,0 ; deslizamento na base 1,5 ; capacidade de carga 3,0
    // (ou critérios da NBR 6122, Nota da Tab. 4)
    pub fs_deslizamento: f64,
    pub fs_tombamento: f64,
    /// NBR 6122 Tabela 1 — método analítico: FSg = 3,0.
    pub fs_capacidade_carga: f64,
    /// NBR 11682 7.3.3.1: resultante no terço central → e ≤ B/6.
    pub excentricidade_max: f64,
    /// NBR 6122 7.6.2: área comprimida ≥ 2/3 da base (ações características).
    pub area_comprimida_min: f64,
    /// Exceção justificada à NBR 11682 7.3.3.1: terço central (e ≤ B/6)
    /// verificado só com as ações permanentes (a combinação total segue coberta
    /// pela área comprimida ≥ 2/3 da NBR 6122 7.6.2). Padrão: desligado.
    pub terco_central_permanente: bool,

    // --- Caso sísmico (muros) ---
    pub fs_deslizamento_sismo: f64,
    pub fs_tombamento_sismo: f64,
    pub fs_capacidade_carga_sismo: f64,

    // --- Contenções embutidas ---
    /// Redução do passivo à frente de muros (NBR 6122 7.6.3: mínimo 2,0).
    pub reducao_passivo_muros: f64,
    /// FS sobre o passivo em paredes embutidas (critério de projeto — não
    /// definido nas normas fornecidas; usual 1,5 a 2,0).
    pub fs_passivo: f64,
    pub acrescimo_ficha_balanco: f64,
    pub acrescimo_ficha_ancorada: f64,

    // --- NBR 5629:2018 — tirantes ---
    pub tirante_permanente: bool,
    /// Fator do aço: σadm = 0,9·fyk / fator (1,75 permanente; 1,50 provisório).
    pub fs_tirante_aco_perm: f64,
    pub fs_tirante_aco_prov: f64,
    /// Fator do bulbo (B.2.4.1): 1,75 permanente; 1,50 provisório.
    pub fs_tirante_bulbo_perm: f64,
    pub fs_tirante_bulbo_prov: f64,
    pub tirante_fixacao: Fixacao,
    /// Comprimento livre mínimo (B.2.7): 3,0 m rosca / 5,0 m clavetes.
    pub tirante_livre_rosca: f64,
    pub tirante_livre_clavete: f64,
    /// Critério de projeto (não normativo): folga além da cunha ativa, × H.
    pub tirante_folga_h: f64,
    /// Critério de projeto (não normativo): bulbo mínimo (m).
    pub tirante_bulbo_min: f64,

    // --- NBR 16920-2:2021 — solo grampeado ---
    /// 9.8: FS global mínimo na estabilidade interna (arrancamento e tração).
    pub fs_grampo_interna: f64,
    /// 9.1: γm da armação de aço.
    pub gamma_m_grampo: f64,

    // --- NBR 6118:2023 / NBR 8800 ---
    pub gamma_f: f64,
    pub gamma_c: f64,
    pub gamma_s: f64,
    pub gamma_a1: f64,

    /// NBR 11682 7.3.3: sobrecarga acidental mínima (kPa).
    pub sobrecarga_minima: f64,
}

impl Default for Fatores {
    fn default() -> Self {
        Self {
            nivel_vidas: Nivel::Alto,
            nivel_danos: Nivel::Medio,
            majorar_variabilidade: false,

            fs_deslizamento: 1.5,
            fs_tombamento: 2.0,
            fs_capacidade_carga: 3.0,
            excentricidade_max: 1.0 / 6.0,
            area_comprimida_min: 2.0 / 3.0,
            terco_central_permanente: false,

            fs_deslizamento_sismo: 1.5,
            fs_tombamento_sismo: 2.0,
            fs_capacidade_carga_sismo: 3.0,

            reducao_passivo_muros: 2.0,
            fs_passivo: 2.0,
            acrescimo_ficha_balanco: 1.2,
            acrescimo_ficha_ancorada: 1.0,

            tirante_permanente: true,
            fs_tirante_aco_perm: 1.75,
            fs_tirante_aco_prov: 1.50,
            fs_tirante_bulbo_perm: 1.75,
            fs_tirante_bulbo_prov: 1.50,
            tirante_fixacao: Fixacao::Rosca,
            tirante_livre_rosca: 3.0,
            tirante_livre_clavete: 5.0,
            tirante_folga_h: 0.15,
            tirante_bulbo_min: 3.0,

            fs_grampo_interna: 1.5,
            gamma_m_grampo: 1.15,

            gamma_f: 1.4,
            gamma_c: 1.4,
            gamma_s: 1.15,
            gamma_a1: 1.10,

            sobrecarga_minima: 20.0,
        }
    }
}

impl Fatores {
    /// FS mínimo de estabilidade (NBR 11682 Tab. 3, com Nota 1 se marcada).
    pub fn fs_global(&self) -> f64 {
        let f = fs_tabela3(self.nivel_vidas, self.nivel_danos);
        if self.majorar_variabilidade { f * 1.1 } else { f }
    }
    pub fn fs_tirante_aco(&self) -> f64 {
        if self.tirante_permanente { self.fs_tirante_aco_perm } else { self.fs_tirante_aco_prov }
    }
    pub fn fs_tirante_bulbo(&self) -> f64 {
        if self.tirante_permanente { self.fs_tirante_bulbo_perm } else { self.fs_tirante_bulbo_prov }
    }
    pub fn tirante_livre_min(&self) -> f64 {
        match self.tirante_fixacao {
            Fixacao::Rosca => self.tirante_livre_rosca,
            Fixacao::Clavete => self.tirante_livre_clavete,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tabela3_nbr11682() {
        assert_eq!(fs_tabela3(Nivel::Alto, Nivel::Alto), 1.5);
        assert_eq!(fs_tabela3(Nivel::Baixo, Nivel::Alto), 1.4); // vidas baixo, danos alto
        assert_eq!(fs_tabela3(Nivel::Alto, Nivel::Baixo), 1.4); // vidas alto, danos baixo
        assert_eq!(fs_tabela3(Nivel::Medio, Nivel::Medio), 1.4);
        assert_eq!(fs_tabela3(Nivel::Baixo, Nivel::Baixo), 1.2);
        // NBR 11682:2009 Tabela 4 — muros de contenção
        let d = Fatores::default();
        assert_eq!((d.fs_tombamento, d.fs_deslizamento, d.fs_capacidade_carga), (2.0, 1.5, 3.0));
        let mut f = Fatores::default();
        f.majorar_variabilidade = true;
        assert!((f.fs_global() - 1.65).abs() < 1e-9);
    }
}
