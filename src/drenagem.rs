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
/// Barbacã: entrada no maciço (coleta) e saída além da face (pingadeira), m.
pub const BARBACA_ENTRADA: f64 = 0.20;
pub const BARBACA_SAIDA: f64 = 0.15;
/// Bolsa drenante de brita envolta em geotêxtil na entrada de cada barbacã: lado da seção e extensão ao longo da contenção (m).
pub const BOLSA_LADO: f64 = 0.30;
pub const BOLSA_COMP: f64 = 0.40;
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
            MaterialFiltro::AreiaGrossa => "Areia lavada (filtro granular)",
        }
    }
}

/// Areia do filtro granular (classificação granulométrica da NBR 6502).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TipoAreia {
    Grossa,
    Media,
}

impl TipoAreia {
    pub fn nome(&self) -> &'static str {
        match self {
            TipoAreia::Grossa => "Areia grossa",
            TipoAreia::Media => "Areia média",
        }
    }
    pub fn faixa(&self) -> &'static str {
        match self {
            TipoAreia::Grossa => "0,6 a 2,0 mm",
            TipoAreia::Media => "0,2 a 0,6 mm",
        }
    }
}

/// Brita do material drenante e das bolsas dos barbacãs (graduações usuais da NBR 7211).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TipoBrita {
    N1,
    N2,
    N3,
}

impl TipoBrita {
    pub fn nome(&self) -> &'static str {
        match self {
            TipoBrita::N1 => "Brita nº 1",
            TipoBrita::N2 => "Brita nº 2",
            TipoBrita::N3 => "Brita nº 3",
        }
    }
    pub fn faixa(&self) -> &'static str {
        match self {
            TipoBrita::N1 => "≈ 9,5 a 19 mm",
            TipoBrita::N2 => "≈ 19 a 25 mm",
            TipoBrita::N3 => "≈ 25 a 38 mm",
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
    /// Areia do filtro granular.
    pub areia: TipoAreia,
    /// Brita do material drenante e das bolsas dos barbacãs.
    pub brita: TipoBrita,
    /// Gramatura mínima do geotêxtil não tecido (g/m²).
    pub gramatura: f64,

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
            areia: TipoAreia::Grossa,
            brita: TipoBrita::N2,
            gramatura: 200.0,
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

    /// Especificação dos materiais da drenagem (memorial e notas do desenho).
    pub fn especificacao(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.filtro {
            v.push(match self.material_filtro {
                MaterialFiltro::BritaGeotextil => format!(
                    "Material drenante: {} ({}), limpa e lavada, de rocha sã (NBR 7211), envolta em geotêxtil não tecido agulhado de poliéster ou polipropileno com gramatura mínima de {} g/m²; espessura de {} m.",
                    self.brita.nome(), self.brita.faixa(), format!("{:.0}", self.gramatura), fmt(self.esp_filtro)
                ),
                MaterialFiltro::AreiaGrossa => format!(
                    "Material drenante: {} lavada ({}, NBR 6502), isenta de matéria orgânica, torrões e finos, lançada em camadas e adensada; espessura de {} m. Geotêxtil não tecido (≥ {} g/m²) entre a areia e o solo quando o critério de filtro não for atendido.",
                    self.areia.nome(), self.areia.faixa(), fmt(self.esp_filtro), format!("{:.0}", self.gramatura)
                ),
                MaterialFiltro::Geocomposto => format!(
                    "Material drenante: geocomposto drenante (núcleo drenante com geotêxtil não tecido filtrante de gramatura mínima {} g/m²), fixado ao paramento e ligado ao dreno de pé.",
                    format!("{:.0}", self.gramatura)
                ),
            });
        }
        if self.barbacas {
            v.push(format!(
                "Barbacãs: tubo de PVC rígido Ø {} mm, caimento de {} % para fora, perfurado (furos a cada 5 cm) no trecho interno, que fica numa bolsa de {} ({}) envolta em geotêxtil ou tela de nylon; malha de {} × {} m, 1ª fila a {} m do terreno.",
                format!("{:.0}", self.diam_barbaca_mm), fmt(self.declividade), self.brita.nome().to_lowercase(), self.brita.faixa(),
                fmt(self.esp_h), fmt(self.esp_v), fmt(self.altura_1a_fila)
            ));
        }
        if self.dreno_pe {
            v.push(format!(
                "Dreno de pé: tubo perfurado Ø {} mm (PVC rígido ou PEAD corrugado), envolto em {} e geotêxtil, declividade longitudinal ≥ 1 % e saída livre.",
                format!("{:.0}", self.diam_dreno_pe_mm), self.brita.nome().to_lowercase()
            ));
        }
        if self.canaleta_crista {
            v.push("Canaletas de crista e de pé: concreto, seção trapezoidal 0,20 × 0,20 m (paredes de 5 cm), com desnível mínimo de 5 cm até a borda para garantir o acesso da água.".into());
        }
        if self.dhp {
            v.push(format!(
                "DHP: tubo perfurado envolvido por filtro (geotêxtil), {} m de comprimento a cada {} m, com leve inclinação para fora.",
                fmt(self.dhp_comprimento), fmt(self.dhp_espacamento)
            ));
        }
        v.push("Topo do material drenante selado com solo argiloso compactado (≈ 0,3 m).".into());
        v.push("Critérios de filtro (Terzaghi/Cedergren — Maccaferri, Manual Técnico, 4.4.3): D15(filtro) < 5·D85(solo) ; 4·D15(solo) < D15(filtro) < 20·D15(solo) ; D50(filtro) < 25·D50(solo). Se a granulometria do material não atender, envolver com geotêxtil.".into());
        v
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

    /// Quantitativos das bolsas de brita + geotêxtil (nos barbacãs) e do envelope do dreno de pé.
    pub fn relatar_bolsas(&self, r: &mut Resultado, filas: usize, pe: bool) {
        let l = BOLSA_LADO.max(self.esp_filtro.min(0.5));
        let por_m = filas as f64 / self.esp_h.max(0.1);
        let v_bolsas = por_m * l * l * BOLSA_COMP;
        let v_pe = if pe { l * l } else { 0.0 };
        let a_geo = por_m * (4.0 * l * BOLSA_COMP + 2.0 * l * l) + if pe { 4.0 * l } else { 0.0 };
        r.formula(
            "Bolsas drenantes de brita envoltas em geotêxtil",
            "uma bolsa l × l × c na entrada de cada barbacã ;  V = (n/m)·l²·c + l² (dreno de pé)",
            format!("l = {} m ;  c = {} m ;  n/m = {} ;  dreno de pé: {}", fmt(l), fmt(BOLSA_COMP), fmt(por_m), if pe { "envelope l × l contínuo" } else { "não" }),
            format!("V = {} m³/m de brita ;  geotêxtil ≈ {} m²/m", fmt(v_bolsas + v_pe), fmt(a_geo)),
        );
        r.linha("A brita com geotêxtil fica só onde há tubos (bolsas nos barbacãs e envelope do dreno de pé); o restante do tardoz é reaterro compactado.");
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
                    "Δh = {} cm no tubo mais longo (entra 20 cm no maciço e sai 15 cm da face, como pingadeira; entrada envolvida por bolsa de brita e geotêxtil)",
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
                    "Critérios de filtro (Terzaghi/Cedergren) — conferir com a granulometria do solo e do material drenante",
                    "D15,f < 5·D85,s ;  4·D15,s < D15,f < 20·D15,s ;  D50,f < 25·D50,s",
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
        r.titulo("Detalhamento construtivo — materiais da drenagem");
        for l in self.especificacao() {
            r.linha(l);
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
