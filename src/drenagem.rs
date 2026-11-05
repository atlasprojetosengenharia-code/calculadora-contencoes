//! Sistema de drenagem: barbacãs, filtro (brita/geotêxtil), dreno de pé,
//! canaleta de crista e drenos sub-horizontais profundos (DHP).
//!
//! A condição de drenagem define a poropressão adotada no maciço contido.
//! NBR 11682 7.3.1 e) exige o detalhamento dos elementos de drenagem
//! (dimensões, materiais, declividade, selagem e saída d'água) e a Seção 10
//! a sua manutenção; NBR 16920-2 9.7.1 / 12: contenções sem drenagem interna
//! devem considerar a água no dimensionamento.

use crate::resultado::{fmt, Desenho, Estilo, Resultado};
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
    pub fn na_efetivo(&self, na_sondagem: Option<f64>, altura: f64) -> Option<f64> {
        match self.condicao {
            Condicao::Drenada => None,
            Condicao::Parcial => {
                let z = altura * (1.0 - self.fracao_parcial.clamp(0.0, 1.0));
                Some(match na_sondagem {
                    Some(n) => n.min(z),
                    None => z,
                })
            }
            Condicao::SemDrenagem => na_sondagem,
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
        d.poli(Estilo::Dreno, vec![p0, [p0[0] + e, p0[1]], [p1[0] + e, p1[1]], p1]);
        if self.dreno_pe {
            let r = (self.diam_dreno_pe_mm / 1000.0).max(0.06);
            d.circulo(Estilo::Barbaca, [p0[0] + e / 2.0, p0[1] + r * 1.4], r);
        }
    }

    /// Memorial e quantitativos por metro linear de contenção.
    /// `larguras` = comprimento de cada barbacã (espessura do paramento na fila);
    /// `altura_filtro` = altura da camada drenante.
    pub fn relatar(&self, r: &mut Resultado, altura: f64, larguras: &[f64], altura_filtro: f64) {
        r.titulo("Drenagem (NBR 11682 7.3.1 e) — por metro linear");
        r.linha(format!("Condição adotada no cálculo: {}", self.condicao.nome()));
        match self.condicao {
            Condicao::Drenada => r.aviso(
                "Poropressão desprezada no maciço contido: exige sistema de drenagem eficiente e plano de manutenção (NBR 11682 Seção 10).",
            ),
            Condicao::Parcial => r.linha(format!(
                "Lâmina d'água considerada: {} da altura contida ({} m)",
                fmt(self.fracao_parcial),
                fmt(self.fracao_parcial * altura)
            )),
            Condicao::SemDrenagem => r.aviso(
                "Contenção sem drenagem interna: dimensionada com o NA da sondagem (NBR 16920-2, 12). Verifique se o NA informado é o crítico.",
            ),
        }
        if self.barbacas && !larguras.is_empty() {
            let por_m = larguras.len() as f64 / self.esp_h;
            let tubo: f64 = larguras.iter().sum::<f64>() / self.esp_h;
            r.linha(format!(
                "Barbacãs: tubo PVC Ø {} mm, malha {} m (h) × {} m (v), 1ª fila a {} m, declividade {} % para fora",
                fmt(self.diam_barbaca_mm), fmt(self.esp_h), fmt(self.esp_v), fmt(self.altura_1a_fila), fmt(self.declividade)
            ));
            r.linha(format!(
                "  {} filas → {} barbacãs/m ; tubo ≈ {} m/m ; extremidade interna protegida com geotêxtil/tela",
                larguras.len(), fmt(por_m), fmt(tubo)
            ));
        } else if self.barbacas {
            r.linha("Barbacãs: altura insuficiente para a malha informada.");
        }
        if self.filtro {
            let vol = self.esp_filtro * altura_filtro;
            r.linha(format!(
                "Filtro: {} com espessura {} m e altura {} m → {} m³/m",
                self.material_filtro.nome(), fmt(self.esp_filtro), fmt(altura_filtro), fmt(vol)
            ));
            if self.material_filtro == MaterialFiltro::BritaGeotextil {
                r.linha(format!(
                    "  Geotêxtil envolvendo a brita ≈ {} m²/m",
                    fmt(2.0 * altura_filtro + 2.0 * self.esp_filtro)
                ));
            }
            r.linha("  Topo do filtro selado com solo argiloso compactado (≈ 0,3 m) para impedir a entrada de águas superficiais.");
        }
        if self.dreno_pe {
            r.linha(format!(
                "Dreno de pé: tubo perfurado Ø {} mm, 1,0 m/m, com saída livre e declividade longitudinal ≥ 1 %",
                fmt(self.diam_dreno_pe_mm)
            ));
        }
        if self.canaleta_crista {
            r.linha("Canaleta de crista: 1,0 m/m, interceptando o escoamento superficial a montante.");
        }
        if self.dhp {
            r.linha(format!(
                "DHP: drenos sub-horizontais profundos L = {} m a cada {} m → {} m de dreno por metro",
                fmt(self.dhp_comprimento), fmt(self.dhp_espacamento), fmt(self.dhp_comprimento / self.dhp_espacamento.max(0.5))
            ));
        }
        if !self.barbacas && !self.filtro && !self.dhp && self.condicao == Condicao::Drenada {
            r.aviso("Condição 'drenada' sem nenhum elemento de drenagem definido — incoerente.");
        }
    }
}
