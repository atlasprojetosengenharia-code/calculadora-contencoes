//! Sistema de drenagem: barbacãs, filtro (brita/geotêxtil), dreno de pé,
//! canaleta de crista e drenos sub-horizontais profundos (DHP).
//!
//! A condição de drenagem define a poropressão adotada no maciço contido.
//! NBR 11682 7.3.1 e) exige o detalhamento dos elementos de drenagem
//! (dimensões, materiais, declividade, selagem e saída d'água) e a Seção 10
//! a sua manutenção; NBR 16920-2 9.7.1 / 12: contenções sem drenagem interna
//! devem considerar a água no dimensionamento.

use crate::normas;
use crate::projeto::TipoContencao;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};

/// Critérios de projeto (prática usual — não são exigências da ABNT).
pub const DIAM_MIN_BARBACA_MM: f64 = 75.0;
pub const DIAM_USUAL_MAX_MM: f64 = 100.0;
pub const ESP_MAX_MALHA: f64 = 2.0;
pub const ESP_MIN_FILTRO: f64 = 0.30;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Condicao {
    /// Drenagem eficiente: poropressão desprezada no maciço contido.
    Drenada,
    /// Lâmina d'água parcial atrás da contenção (fração da altura).
    Parcial,
    /// Sem drenagem: NA da sondagem.
    SemDrenagem,
}

impl Condicao {
    pub fn nome(&self) -> &'static str {
        match self {
            Condicao::Drenada => "Drenada (sistema eficiente)",
            Condicao::Parcial => "Parcialmente drenada",
            Condicao::SemDrenagem => "Sem drenagem (NA da sondagem)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MaterialFiltro {
    BritaGeotextil,
    Geocomposto,
    AreiaGrossa,
}

impl MaterialFiltro {
    pub fn nome(&self) -> &'static str {
        match self {
            MaterialFiltro::BritaGeotextil => "Brita envolta em geotêxtil",
            MaterialFiltro::Geocomposto => "Geocomposto drenante",
            MaterialFiltro::AreiaGrossa => "Areia grossa (filtro granular)",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Drenagem {
    pub condicao: Condicao,
    /// Altura d'água atrás da contenção como fração da altura contida (Parcial).
    pub fracao_parcial: f64,
    /// Idem, na condição sem drenagem: sem saída, a água da chuva acumula no
    /// aterro (1,0 = maciço saturado até a crista, hidrostática; pior caso).
    pub fracao_sem: f64,

    pub barbacas: bool,
    pub diam_barbaca_mm: f64,
    pub esp_h: f64,
    pub esp_v: f64,
    /// Altura da 1ª fila acima do terreno à frente / base (m).
    pub altura_1a_fila: f64,
    /// Declividade dos barbacãs para fora (%).
    pub declividade: f64,

    pub filtro: bool,
    pub material_filtro: MaterialFiltro,
    pub esp_filtro: f64,

    pub dreno_pe: bool,
    pub diam_dreno_pe_mm: f64,
    pub canaleta_crista: bool,

    pub dhp: bool,
    pub dhp_comprimento: f64,
    pub dhp_espacamento: f64,
}

impl Default for Drenagem {
    fn default() -> Self {
        Self {
            condicao: Condicao::Drenada,
            fracao_parcial: 0.33,
            fracao_sem: 1.0,
            barbacas: true,
            diam_barbaca_mm: 75.0,
            esp_h: 1.5,
            esp_v: 1.0,
            altura_1a_fila: 0.3,
            declividade: 5.0,
            filtro: true,
            material_filtro: MaterialFiltro::BritaGeotextil,
            esp_filtro: 0.30,
            dreno_pe: true,
            diam_dreno_pe_mm: 100.0,
            canaleta_crista: true,
            dhp: false,
            dhp_comprimento: 6.0,
            dhp_espacamento: 3.0,
        }
    }
}

impl Drenagem {
    /// Profundidade do NA (a partir do topo do maciço contido) adotada no
    /// cálculo, dada a condição de drenagem.
    pub fn na_efetivo(&self, na_sondagem: Option<f64>, altura: f64, tipo: TipoContencao) -> Option<f64> {
        let lamina = |f: f64| {
            let z = altura * (1.0 - f.clamp(0.0, 1.0));
            Some(match na_sondagem {
                Some(n) => n.min(z),
                None => z,
            })
        };
        match self.condicao_efetiva(tipo) {
            Condicao::Drenada => None,
            Condicao::Parcial => lamina(self.fracao_parcial),
            Condicao::SemDrenagem => lamina(self.fracao_sem),
        }
    }

    /// O sistema atende ao que a condição 'drenada' pressupõe? Contenções com
    /// face precisam de camada drenante no tardoz e de saída d'água (barbacãs,
    /// ou DHP nas atirantadas/grampeadas); nas embutidas a condição só define
    /// a poropressão adotada.
    pub fn eficaz(&self, tipo: TipoContencao) -> bool {
        match tipo {
            TipoContencao::Prancha | TipoContencao::Estacas => true,
            // NBR 16920-2, 12.1: geodrenos na face, barbacãs ou DHP, juntos ou isolados
            TipoContencao::Grampeado => self.filtro || self.barbacas || self.dhp,
            TipoContencao::Atirantada => (self.filtro && self.barbacas) || self.dhp,
            _ => self.filtro && self.barbacas,
        }
    }

    /// Condição realmente usada no cálculo: 'drenada' sem sistema eficaz é
    /// tratada como sem drenagem.
    pub fn condicao_efetiva(&self, tipo: TipoContencao) -> Condicao {
        if self.condicao == Condicao::Drenada && !self.eficaz(tipo) {
            Condicao::SemDrenagem
        } else {
            self.condicao
        }
    }

    /// Alturas (acima da base) das filas de barbacãs.
    pub fn filas(&self, altura: f64) -> Vec<f64> {
        let mut v = Vec::new();
        if !self.barbacas || self.esp_v <= 0.05 {
            return v;
        }
        let mut y = self.altura_1a_fila.max(0.1);
        while y <= altura - 0.3 && v.len() < 60 {
            v.push(y);
            y += self.esp_v;
        }
        v
    }

    /// Desenha um barbacã do tardoz (`xt`) até a face (`xf`) na cota `y`,
    /// descendo com a declividade adotada.
    pub fn barbaca(&self, d: &mut Desenho, xt: f64, xf: f64, y: f64) {
        let queda = (xt - xf).abs() * self.declividade / 100.0;
        d.linha(Estilo::Barbaca, vec![[xt, y], [xf, y - queda]]);
    }

    /// Faixa de filtro paralela ao tardoz, do ponto `p0` (inferior) ao `p1`
    /// (superior), com espessura medida em +x (lado do maciço).
    pub fn faixa_filtro(&self, d: &mut Desenho, p0: [f64; 2], p1: [f64; 2]) {
        if !self.filtro {
            return;
        }
        let e = self.esp_filtro;
        let quad = vec![p0, [p0[0] + e, p0[1]], [p1[0] + e, p1[1]], p1];
        d.poli(Estilo::Dreno, quad.clone());
        if e >= 0.12 {
            match self.material_filtro {
                MaterialFiltro::AreiaGrossa => d.pontilhado(&quad, e / 4.0),
                MaterialFiltro::BritaGeotextil => d.pedras(&quad, e * 0.3),
                MaterialFiltro::Geocomposto => {}
            }
        }
        if self.dreno_pe {
            let r = (self.diam_dreno_pe_mm / 1000.0).max(0.06);
            d.circulo(Estilo::Barbaca, [p0[0] + e / 2.0, p0[1] + r * 1.4], r);
        }
    }

    /// Memorial e quantitativos por metro linear de contenção.
    /// `larguras` = comprimento de cada barbacã (espessura do paramento na fila);
    /// `altura_filtro` = altura da camada drenante.
    pub fn relatar(&self, r: &mut Resultado, altura: f64, larguras: &[f64], altura_filtro: f64, tipo: TipoContencao) {
        r.titulo("Drenagem — barbacãs, colchão drenante e drenos (por metro linear)");
        r.norma(normas::NBR11682_DRENAGEM);

        // 1. Condição de drenagem e água no cálculo
        let cond = self.condicao_efetiva(tipo);
        r.linha(format!("Condição adotada no cálculo: {}", cond.nome()));
        if cond != self.condicao {
            r.aviso(
                "Condição 'drenada' selecionada, mas sem o sistema que a sustenta (camada drenante + barbacãs, ou DHP): \
                 o cálculo foi feito como 'sem drenagem'.",
            );
        }
        if self.condicao == Condicao::Drenada && !matches!(tipo, TipoContencao::Prancha | TipoContencao::Estacas) {
            r.verif(Verificacao::min(
                "Drenagem: camada drenante + saída d'água (barbacãs ou DHP) previstas para a condição 'drenada'",
                if self.eficaz(tipo) { 1.0 } else { 0.0 },
                1.0,
                "",
            ));
        }
        match cond {
            Condicao::Drenada => {
                r.formula(
                    "Água no maciço contido — condição drenada",
                    "u = 0  (poropressão desprezada atrás da contenção)",
                    "Sistema de drenagem eficiente e mantido",
                    "Empuxo hidrostático U = 0 kN/m",
                );
                r.norma(normas::NBR11682_MANUTENCAO);
                r.aviso(
                    "Poropressão desprezada no maciço contido: exige sistema de drenagem eficiente e plano de manutenção (NBR 11682 Seção 10).",
                );
            }
            Condicao::Parcial => {
                let hw = self.fracao_parcial.clamp(0.0, 1.0) * altura;
                r.formula(
                    "Água no maciço contido — lâmina parcial",
                    "hw = f·H ;  U = ½·γw·hw²",
                    format!("hw = {}·{} ;  U = ½·10·{}²", fmt(self.fracao_parcial), fmt(altura), fmt(hw)),
                    format!("hw = {} m ;  U = {} kN/m (incluído no empuxo)", fmt(hw), fmt(0.5 * 10.0 * hw * hw)),
                );
            }
            Condicao::SemDrenagem => {
                let hw = self.fracao_sem.clamp(0.0, 1.0) * altura;
                r.norma(normas::NBR16920_DRENAGEM);
                r.formula(
                    "Água no maciço contido — sem drenagem",
                    "hw = f·H ;  U = ½·γw·hw² ;  solo submerso (γ') abaixo do nível d'água",
                    format!("hw = {}·{} ;  U = ½·10·{}²", fmt(self.fracao_sem), fmt(altura), fmt(hw)),
                    format!("hw = {} m ;  U = {} kN/m (incluído no empuxo; o NA da sondagem vale se for mais alto)", fmt(hw), fmt(0.5 * 10.0 * hw * hw)),
                );
                r.aviso(
                    "Contenção sem drenagem interna: a água acumula atrás da estrutura e entra no empuxo (NBR 16920-2, 12; NBR 11682 7.3.1). Em geral o dimensionamento só fecha com drenagem eficiente e mantida.",
                );
            }
        }

        // 2. Barbacãs
        if self.barbacas && !larguras.is_empty() {
            let por_m = larguras.len() as f64 / self.esp_h;
            let por_m2 = 1.0 / (self.esp_h * self.esp_v);
            let d = self.diam_barbaca_mm / 1000.0;
            let area_furo = std::f64::consts::PI * d * d / 4.0;
            let tubo: f64 = larguras.iter().sum::<f64>() / self.esp_h;
            let l_max = larguras.iter().cloned().fold(0.0, f64::max);
            r.linha(format!(
                "Barbacãs: tubo PVC Ø {} mm, malha {} m (h) × {} m (v), {} filas, 1ª fila a {} m, declividade {} % para fora",
                format!("{:.0}", self.diam_barbaca_mm),
                fmt(self.esp_h),
                fmt(self.esp_v),
                larguras.len(),
                fmt(self.altura_1a_fila),
                fmt(self.declividade)
            ));
            r.formula(
                "Malha de barbacãs",
                "n/m = nº de filas / Sh ;  n/m² = 1 / (Sh·Sv) ;  A_dren = (n/m²)·π·Ø²/4 ;  L_tubo = Σ e_fila / Sh",
                format!(
                    "n/m = {} / {} ;  n/m² = 1 / ({}·{}) ;  A_dren = {}·π·{}²/4",
                    larguras.len(),
                    fmt(self.esp_h),
                    fmt(self.esp_h),
                    fmt(self.esp_v),
                    fmt(por_m2),
                    fmt(d)
                ),
                format!(
                    "{} barbacãs/m ;  {} barbacãs/m² ;  A_dren = {} cm²/m² ;  tubo ≈ {} m/m",
                    fmt(por_m),
                    fmt(por_m2),
                    fmt(por_m2 * area_furo * 1e4),
                    fmt(tubo)
                ),
            );
            r.formula(
                "Caimento do barbacã",
                "Δh = L·i",
                format!("Δh = {} m·{} %", fmt(l_max), fmt(self.declividade)),
                format!(
                    "Δh = {} cm na fila mais longa (saída livre na face; extremidade interna protegida com geotêxtil)",
                    fmt(l_max * self.declividade)
                ),
            );
            r.verif(Verificacao::min(
                format!(
                    "Barbacãs: Ø ≥ {} mm (critério de projeto; usual {}–{} mm)",
                    format!("{:.0}", DIAM_MIN_BARBACA_MM),
                    format!("{:.0}", DIAM_MIN_BARBACA_MM),
                    format!("{:.0}", DIAM_USUAL_MAX_MM)
                ),
                self.diam_barbaca_mm,
                DIAM_MIN_BARBACA_MM,
                "mm",
            ));
            r.verif(Verificacao::max(
                format!("Barbacãs: espaçamento horizontal ≤ {} m (critério de projeto)", fmt(ESP_MAX_MALHA)),
                self.esp_h,
                ESP_MAX_MALHA,
                "m",
            ));
            r.verif(Verificacao::max(
                format!("Barbacãs: espaçamento vertical ≤ {} m (critério de projeto)", fmt(ESP_MAX_MALHA)),
                self.esp_v,
                ESP_MAX_MALHA,
                "m",
            ));
            if self.diam_barbaca_mm > DIAM_USUAL_MAX_MM {
                r.aviso(format!(
                    "Barbacãs com Ø {} mm, acima da faixa usual de {}–{} mm: verificar o enfraquecimento do paramento.",
                    format!("{:.0}", self.diam_barbaca_mm),
                    format!("{:.0}", DIAM_MIN_BARBACA_MM),
                    format!("{:.0}", DIAM_USUAL_MAX_MM)
                ));
            }
        } else if self.barbacas {
            r.linha("Barbacãs: não aplicáveis a este tipo de contenção ou altura insuficiente para a malha informada.");
        }

        // 3. Colchão drenante (filtro)
        if self.filtro && altura_filtro > 0.0 {
            let vol = self.esp_filtro * altura_filtro;
            r.formula(
                format!("Colchão drenante — {}", self.material_filtro.nome()),
                "h_f = H − 0,30 (selagem) ;  V = e·h_f ;  A_geotêxtil = 2·(h_f + e)",
                format!("h_f = {} m ;  V = {}·{}", fmt(altura_filtro), fmt(self.esp_filtro), fmt(altura_filtro)),
                if self.material_filtro == MaterialFiltro::BritaGeotextil {
                    format!(
                        "V = {} m³/m de brita ;  geotêxtil ≈ {} m²/m",
                        fmt(vol),
                        fmt(2.0 * altura_filtro + 2.0 * self.esp_filtro)
                    )
                } else {
                    format!("V = {} m³/m", fmt(vol))
                },
            );
            if self.material_filtro != MaterialFiltro::Geocomposto {
                r.formula(
                    "Critérios de filtro (Terzaghi) — conferir com a granulometria do solo e do material drenante",
                    "Retenção: D15,f / D85,s ≤ 4 a 5 ;  Permeabilidade: D15,f / D15,s ≥ 4 a 5",
                    "D15,f = diâmetro a 15 % do filtro ;  D85,s e D15,s = diâmetros do solo protegido",
                    "Com geotêxtil envolvendo a brita, a retenção é dada pela abertura de filtração do geotêxtil",
                );
                r.verif(Verificacao::min(
                    format!("Colchão drenante: espessura ≥ {} m (critério de projeto)", fmt(ESP_MIN_FILTRO)),
                    self.esp_filtro,
                    ESP_MIN_FILTRO,
                    "m",
                ));
            }
            r.linha("Topo do filtro selado com solo argiloso compactado (≈ 0,3 m) para impedir a entrada de águas superficiais.");
        }

        // 4. Demais dispositivos
        if self.dreno_pe {
            r.linha(format!(
                "Dreno de pé: tubo perfurado Ø {} mm, 1,0 m/m, envolto no colchão, com saída livre e declividade longitudinal ≥ 1 %",
                fmt(self.diam_dreno_pe_mm)
            ));
        }
        if self.canaleta_crista {
            r.linha("Canaleta de crista: 1,0 m/m, interceptando o escoamento superficial a montante.");
        }
        if self.dhp {
            r.formula(
                "Drenos sub-horizontais profundos (DHP)",
                "L_dren/m = L / s",
                format!("L_dren/m = {} / {}", fmt(self.dhp_comprimento), fmt(self.dhp_espacamento.max(0.5))),
                format!("{} m de dreno por metro de contenção", fmt(self.dhp_comprimento / self.dhp_espacamento.max(0.5))),
            );
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn memorial_de_drenagem() {
        let d = Drenagem::default();
        let mut r = Resultado::default();
        d.relatar(&mut r, 4.0, &[0.6, 0.5, 0.4], 3.7, TipoContencao::Gravidade);
        let nomes: Vec<&str> = r.verificacoes.iter().map(|v| v.nome.as_str()).collect();
        assert!(nomes.iter().any(|n| n.contains("Ø ≥ 75 mm")));
        assert!(nomes.iter().any(|n| n.contains("espessura ≥ 0,30 m")));
        assert!(r.atende(), "padrões devem atender");
        let t = r.texto_completo("x");
        assert!(t.contains("Malha de barbacãs") && t.contains("Colchão drenante") && t.contains("7.3.1 e)"));
        // Ø 50 mm e malha 2,5 m reprovam
        let mut d2 = d.clone();
        d2.diam_barbaca_mm = 50.0;
        d2.esp_h = 2.5;
        let mut r2 = Resultado::default();
        d2.relatar(&mut r2, 4.0, &[0.6], 3.7, TipoContencao::Gravidade);
        assert_eq!(r2.verificacoes.iter().filter(|v| !v.ok()).count(), 2);
        // sem barbacãs/filtro aplicáveis (ex.: estaca-prancha): nenhuma verificação
        let mut r3 = Resultado::default();
        d.relatar(&mut r3, 4.0, &[], 0.0, TipoContencao::Prancha);
        assert!(r3.verificacoes.is_empty());
    }
}
