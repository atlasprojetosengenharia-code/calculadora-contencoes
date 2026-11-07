//! Sondagem SPT: camadas, correlações de parâmetros geotécnicos e
//! importação de arquivos CSV/TXT.
//!
//! Correlações (estimativas — devem ser confirmadas por ensaios, NBR 11682 Anexo D):
//! - γ: Godoy (1972), em função da consistência (coesivos) ou compacidade (granulares);
//! - φ' (granulares): Teixeira (1996) φ' = √(20·N) + 15  ou  Godoy (1983) φ' = 28 + 0,4·N;
//! - coesivos não drenados: Teixeira e Godoy (1996) Su = 10·N (kPa), φ = 0;
//! - coesivos drenados: valores padrão definidos pelo usuário.

use crate::solo::{Camada, Perfil};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TipoSolo {
    Areia,
    AreiaSiltosa,
    AreiaArgilosa,
    SilteArenoso,
    SilteArgiloso,
    ArgilaArenosa,
    ArgilaSiltosa,
    Argila,
    Aterro,
}

pub const TIPOS: [TipoSolo; 9] = [
    TipoSolo::Areia,
    TipoSolo::AreiaSiltosa,
    TipoSolo::AreiaArgilosa,
    TipoSolo::SilteArenoso,
    TipoSolo::SilteArgiloso,
    TipoSolo::ArgilaArenosa,
    TipoSolo::ArgilaSiltosa,
    TipoSolo::Argila,
    TipoSolo::Aterro,
];

impl TipoSolo {
    pub fn nome(&self) -> &'static str {
        match self {
            TipoSolo::Areia => "Areia",
            TipoSolo::AreiaSiltosa => "Areia siltosa",
            TipoSolo::AreiaArgilosa => "Areia argilosa",
            TipoSolo::SilteArenoso => "Silte arenoso",
            TipoSolo::SilteArgiloso => "Silte argiloso",
            TipoSolo::ArgilaArenosa => "Argila arenosa",
            TipoSolo::ArgilaSiltosa => "Argila siltosa",
            TipoSolo::Argila => "Argila",
            TipoSolo::Aterro => "Aterro",
        }
    }
    pub fn granular(&self) -> bool {
        matches!(
            self,
            TipoSolo::Areia | TipoSolo::AreiaSiltosa | TipoSolo::AreiaArgilosa | TipoSolo::SilteArenoso | TipoSolo::Aterro
        )
    }
    /// Cor (RGB) para o perfil estratigráfico.
    pub fn cor(&self) -> [u8; 3] {
        match self {
            TipoSolo::Areia => [222, 196, 120],
            TipoSolo::AreiaSiltosa => [208, 184, 128],
            TipoSolo::AreiaArgilosa => [196, 164, 112],
            TipoSolo::SilteArenoso => [176, 160, 128],
            TipoSolo::SilteArgiloso => [156, 136, 116],
            TipoSolo::ArgilaArenosa => [168, 120, 88],
            TipoSolo::ArgilaSiltosa => [150, 104, 84],
            TipoSolo::Argila => [134, 88, 72],
            TipoSolo::Aterro => [120, 120, 112],
        }
    }

    /// Classificação automática a partir da descrição (ex.: "Areia fina argilosa").
    pub fn da_descricao(desc: &str) -> Option<TipoSolo> {
        let s = desc.to_lowercase();
        if s.contains("aterro") {
            return Some(TipoSolo::Aterro);
        }
        let pos = |k: &str| s.find(k);
        let principais = [("areia", 0), ("silte", 1), ("argila", 2)];
        let (_, princ, ini) = principais
            .iter()
            .filter_map(|&(k, i)| pos(k).map(|p| (p, i, p + k.len())))
            .min_by_key(|t| t.0)?;
        // qualificador predominante = o primeiro que aparece após o nome principal
        let resto = &s[ini..];
        let qualif = ["aren", "silt", "argil"]
            .iter()
            .filter_map(|k| resto.find(k).map(|p| (p, *k)))
            .min_by_key(|t| t.0)
            .map(|t| t.1);
        Some(match (princ, qualif) {
            (0, Some("silt")) => TipoSolo::AreiaSiltosa,
            (0, Some("argil")) => TipoSolo::AreiaArgilosa,
            (0, _) => TipoSolo::Areia,
            (1, Some("aren")) => TipoSolo::SilteArenoso,
            (1, _) => TipoSolo::SilteArgiloso,
            (_, Some("aren")) => TipoSolo::ArgilaArenosa,
            (_, Some("silt")) => TipoSolo::ArgilaSiltosa,
            _ => TipoSolo::Argila,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MetodoPhi {
    Teixeira1996,
    Godoy1983,
}

impl MetodoPhi {
    pub fn nome(&self) -> &'static str {
        match self {
            MetodoPhi::Teixeira1996 => "Teixeira (1996): φ' = √(20N) + 15",
            MetodoPhi::Godoy1983 => "Godoy (1983): φ' = 28 + 0,4N",
        }
    }
}

/// Modo de entrada dos parâmetros do solo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ModoSolo {
    /// Sondagem completa, camada a camada.
    Completo,
    /// Um único NSPT e tipo de solo predominante (solo homogêneo).
    Simplificado,
}

impl ModoSolo {
    pub fn nome(&self) -> &'static str {
        match self {
            ModoSolo::Completo => "SPT completo (camadas)",
            ModoSolo::Simplificado => "Entrada simplificada na base",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CondicaoCoesivos {
    /// Longo prazo: c' e φ' padrão definidos pelo usuário.
    Drenada,
    /// Curto prazo: Su = 10·N (Teixeira e Godoy, 1996), φ = 0.
    NaoDrenada,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CamadaSpt {
    /// Profundidade final da camada (m) — a inicial é a final da anterior.
    pub ate: f64,
    pub nspt: f64,
    pub descricao: String,
    pub tipo: TipoSolo,
    /// Parâmetros ajustados manualmente (não recalcular).
    pub manual: bool,
    pub gamma: f64,
    pub gamma_sat: f64,
    pub c: f64,
    pub phi: f64,
}

impl CamadaSpt {
    pub fn nova(ate: f64, nspt: f64, descricao: &str) -> Self {
        let tipo = TipoSolo::da_descricao(descricao).unwrap_or(TipoSolo::Areia);
        Self { ate, nspt, descricao: descricao.into(), tipo, manual: false, gamma: 18.0, gamma_sat: 20.0, c: 0.0, phi: 30.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Sondagem {
    pub identificacao: String,
    pub camadas: Vec<CamadaSpt>,
    pub usar_na: bool,
    pub prof_na: f64,
    pub metodo_phi: MetodoPhi,
    pub coesivos: CondicaoCoesivos,
    pub c_drenado: f64,
    pub phi_drenado: f64,
    /// Modo de entrada (projetos antigos abrem como `Completo`).
    pub modo: ModoSolo,
    /// Entrada simplificada: NSPT representativo da base.
    pub simples_nspt: f64,
    /// Entrada simplificada: tipo de solo predominante na fundação.
    pub simples_tipo: TipoSolo,
}

impl Default for Sondagem {
    fn default() -> Self {
        let mut s = Self {
            identificacao: "SP-01".into(),
            camadas: vec![
                CamadaSpt::nova(1.5, 4.0, "Aterro argilo-arenoso"),
                CamadaSpt::nova(4.0, 7.0, "Argila siltosa, média"),
                CamadaSpt::nova(8.0, 14.0, "Areia fina argilosa, medianamente compacta"),
                CamadaSpt::nova(15.0, 28.0, "Areia média, compacta"),
            ],
            usar_na: true,
            prof_na: 6.0,
            metodo_phi: MetodoPhi::Teixeira1996,
            coesivos: CondicaoCoesivos::Drenada,
            c_drenado: 5.0,
            phi_drenado: 25.0,
            modo: ModoSolo::Completo,
            simples_nspt: 10.0,
            simples_tipo: TipoSolo::AreiaArgilosa,
        };
        s.recalcular();
        s
    }
}

/// γ natural e saturado (kN/m³) — Godoy (1972).
fn gamma_godoy(tipo: TipoSolo, n: f64) -> (f64, f64) {
    if tipo.granular() {
        // compacidade: fofa <5, pouco compacta 5-8, med. compacta 9-18, compacta 19-40, muito compacta >40
        // (γ úmido, γ saturado)
        if n < 9.0 {
            (18.0, 19.0)
        } else if n < 19.0 {
            (19.0, 20.0)
        } else {
            (20.0, 21.0)
        }
    } else {
        // consistência: muito mole ≤2, mole 3-5, média 6-10, rija 11-19, dura ≥20
        let g = if n <= 2.0 {
            13.0
        } else if n <= 5.0 {
            15.0
        } else if n <= 10.0 {
            17.0
        } else if n <= 19.0 {
            19.0
        } else {
            21.0
        };
        (g, g)
    }
}

pub fn compacidade(tipo: TipoSolo, n: f64) -> &'static str {
    if tipo.granular() {
        match n {
            x if x < 5.0 => "fofa",
            x if x < 9.0 => "pouco compacta",
            x if x < 19.0 => "medianamente compacta",
            x if x <= 40.0 => "compacta",
            _ => "muito compacta",
        }
    } else {
        match n {
            x if x <= 2.0 => "muito mole",
            x if x <= 5.0 => "mole",
            x if x <= 10.0 => "média",
            x if x <= 19.0 => "rija",
            _ => "dura",
        }
    }
}

impl Sondagem {
    pub fn de(&self, i: usize) -> f64 {
        if i == 0 { 0.0 } else { self.camadas[i - 1].ate }
    }

    /// Parâmetros estimados por correlação: (γ, γsat, c', φ').
    pub fn parametros(&self, tipo: TipoSolo, nspt: f64) -> (f64, f64, f64, f64) {
        let n = nspt.max(0.0);
        let (g, gs) = gamma_godoy(tipo, n);
        let (c, phi) = if tipo.granular() {
            let phi = match self.metodo_phi {
                MetodoPhi::Teixeira1996 => (20.0 * n).sqrt() + 15.0,
                MetodoPhi::Godoy1983 => 28.0 + 0.4 * n,
            };
            (0.0, phi.min(45.0))
        } else {
            match self.coesivos {
                CondicaoCoesivos::NaoDrenada => (10.0 * n, 0.0),
                CondicaoCoesivos::Drenada => (self.c_drenado, self.phi_drenado),
            }
        };
        (g, gs, c, phi)
    }

    /// Recalcula os parâmetros das camadas não marcadas como manuais.
    pub fn recalcular(&mut self) {
        let novos: Vec<Option<(f64, f64, f64, f64)>> =
            self.camadas.iter().map(|c| (!c.manual).then(|| self.parametros(c.tipo, c.nspt))).collect();
        for (c, p) in self.camadas.iter_mut().zip(novos) {
            if let Some((g, gs, coes, phi)) = p {
                c.gamma = g;
                c.gamma_sat = gs;
                c.c = coes;
                c.phi = phi;
            }
        }
    }

    /// Camada única da entrada simplificada (solo homogêneo até 100 m).
    pub fn camada_simplificada(&self) -> CamadaSpt {
        let (g, gs, c, phi) = self.parametros(self.simples_tipo, self.simples_nspt);
        CamadaSpt {
            ate: 100.0,
            nspt: self.simples_nspt.max(0.0),
            descricao: format!("{} (entrada simplificada)", self.simples_tipo.nome()),
            tipo: self.simples_tipo,
            manual: false,
            gamma: g,
            gamma_sat: gs,
            c,
            phi,
        }
    }

    /// Camadas usadas no cálculo, conforme o modo de entrada.
    pub fn camadas_calc(&self) -> Vec<CamadaSpt> {
        match self.modo {
            ModoSolo::Completo => self.camadas.clone(),
            ModoSolo::Simplificado => vec![self.camada_simplificada()],
        }
    }

    /// Ordena e remove camadas de espessura nula.
    pub fn normalizar(&mut self) {
        self.camadas.sort_by(|a, b| a.ate.partial_cmp(&b.ate).unwrap_or(std::cmp::Ordering::Equal));
        let mut ant = 0.0;
        self.camadas.retain(|c| {
            let ok = c.ate > ant + 1e-6;
            if ok {
                ant = c.ate;
            }
            ok
        });
    }

    /// Une camadas consecutivas do mesmo tipo e descrição (N médio ponderado).
    pub fn agrupar(&mut self) {
        let inicios: Vec<f64> = (0..self.camadas.len()).map(|i| self.de(i)).collect();
        // (camada, profundidade inicial)
        let mut novas: Vec<(CamadaSpt, f64)> = Vec::new();
        for (i, c) in std::mem::take(&mut self.camadas).into_iter().enumerate() {
            if let Some((u, ini_u)) = novas.last_mut() {
                if u.tipo == c.tipo && u.descricao.trim().eq_ignore_ascii_case(c.descricao.trim()) && !u.manual && !c.manual {
                    let e1 = u.ate - *ini_u;
                    let e2 = c.ate - inicios[i];
                    u.nspt = ((u.nspt * e1 + c.nspt * e2) / (e1 + e2)).round();
                    u.ate = c.ate;
                    continue;
                }
            }
            novas.push((c, inicios[i]));
        }
        self.camadas = novas.into_iter().map(|(c, _)| c).collect();
        self.recalcular();
    }

    pub fn na(&self) -> Option<f64> {
        if self.usar_na { Some(self.prof_na) } else { None }
    }

    pub fn camada_em(&self, z: f64) -> Option<CamadaSpt> {
        let cs = self.camadas_calc();
        cs.iter().find(|c| z < c.ate).or(cs.last()).cloned()
    }

    /// Converte em perfil de cálculo (z = 0 no topo da sondagem).
    pub fn perfil(&self, na: Option<f64>) -> Perfil {
        let mut camadas = Vec::new();
        let cs = self.camadas_calc();
        for (i, c) in cs.iter().enumerate() {
            let de = if i == 0 { 0.0 } else { cs[i - 1].ate };
            let esp = (c.ate - de).max(0.01);
            camadas.push(Camada {
                nome: if c.descricao.is_empty() { c.tipo.nome().into() } else { c.descricao.clone() },
                espessura: esp,
                gamma: c.gamma,
                gamma_sat: c.gamma_sat,
                phi: c.phi,
                c: c.c,
            });
        }
        if camadas.is_empty() {
            camadas.push(Camada::nova("Solo", 10.0, 18.0, 20.0, 30.0, 0.0));
        }
        Perfil { camadas, usar_na: na.is_some(), prof_na: na.unwrap_or(0.0) }
    }

    /// Médias ponderadas pela espessura (γ, φ, c) entre 0 e `h`.
    pub fn medias(&self, h: f64) -> (f64, f64, f64) {
        let n = 200;
        let dz = h.max(0.1) / n as f64;
        let (mut g, mut p, mut c) = (0.0, 0.0, 0.0);
        for i in 0..n {
            if let Some(cam) = self.camada_em((i as f64 + 0.5) * dz) {
                g += cam.gamma;
                p += cam.phi;
                c += cam.c;
            }
        }
        (g / n as f64, p / n as f64, c / n as f64)
    }

    pub fn tem_coesivo_padrao(&self) -> bool {
        self.coesivos == CondicaoCoesivos::Drenada && self.camadas_calc().iter().any(|c| !c.tipo.granular() && !c.manual)
    }
}

/// Importa sondagem de texto CSV/TXT. Formatos aceitos (uma linha por registro):
/// - `prof_inicial ; prof_final ; NSPT ; descrição`
/// - `profundidade ; NSPT ; descrição` (ensaio a cada metro — camada até a profundidade)
/// - `NA ; profundidade` define o nível d'água.
/// Separador `;`, tabulação ou `,`. Com `;` ou tabulação, aceita vírgula decimal.
/// Linhas de cabeçalho/comentário (iniciadas por `#` ou não numéricas) são ignoradas.
pub fn importar(texto: &str) -> Result<(Vec<CamadaSpt>, Option<f64>), String> {
    let mut camadas: Vec<CamadaSpt> = Vec::new();
    let mut na = None;
    for (num, bruta) in texto.lines().enumerate() {
        let l = bruta.trim().trim_start_matches('\u{feff}');
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let sep = if l.contains(';') { ';' } else if l.contains('\t') { '\t' } else { ',' };
        let campos: Vec<&str> = l.split(sep).map(|s| s.trim()).collect();
        let numero = |s: &str| -> Option<f64> {
            let s = if sep != ',' { s.replace(',', ".") } else { s.to_string() };
            s.trim().parse::<f64>().ok()
        };
        if campos[0].eq_ignore_ascii_case("na") || campos[0].to_lowercase().starts_with("nível") || campos[0].to_lowercase().starts_with("nivel") {
            if let Some(v) = campos.get(1).and_then(|s| numero(s)) {
                na = Some(v);
            }
            continue;
        }
        let nums: Vec<f64> = campos.iter().map_while(|s| numero(s)).collect();
        let desc = campos[nums.len().min(campos.len())..].join(" ").trim().to_string();
        match nums.len() {
            0 => continue, // cabeçalho
            1 => return Err(format!("Linha {}: dados insuficientes ('{}')", num + 1, l)),
            2 => {
                let (z, n) = (nums[0], nums[1]);
                camadas.push(CamadaSpt::nova(z, n, &desc));
            }
            _ => {
                let (z0, z1, n) = (nums[0], nums[1], nums[2]);
                if z1 <= z0 {
                    return Err(format!("Linha {}: profundidade final ≤ inicial", num + 1));
                }
                camadas.push(CamadaSpt::nova(z1, n, &desc));
            }
        }
    }
    if camadas.is_empty() {
        return Err("Nenhuma camada encontrada no arquivo.".into());
    }
    // descrição vazia herda a anterior
    for i in 1..camadas.len() {
        if camadas[i].descricao.is_empty() {
            camadas[i].descricao = camadas[i - 1].descricao.clone();
            camadas[i].tipo = camadas[i - 1].tipo;
        }
    }
    Ok((camadas, na))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn classificacao() {
        assert_eq!(TipoSolo::da_descricao("Areia fina argilosa"), Some(TipoSolo::AreiaArgilosa));
        assert_eq!(TipoSolo::da_descricao("ARGILA SILTO-ARENOSA"), Some(TipoSolo::ArgilaSiltosa));
        assert_eq!(TipoSolo::da_descricao("Silte arenoso"), Some(TipoSolo::SilteArenoso));
        assert_eq!(TipoSolo::da_descricao("Aterro de argila"), Some(TipoSolo::Aterro));
        assert_eq!(TipoSolo::da_descricao("solo"), None);
    }

    #[test]
    fn correlacoes() {
        let mut s = Sondagem::default();
        s.camadas = vec![CamadaSpt::nova(3.0, 20.0, "Areia"), CamadaSpt::nova(6.0, 8.0, "Argila")];
        s.recalcular();
        assert!((s.camadas[0].phi - 35.0).abs() < 1e-9); // √400 + 15
        assert_eq!(s.camadas[0].gamma, 20.0);
        s.coesivos = CondicaoCoesivos::NaoDrenada;
        s.recalcular();
        assert_eq!(s.camadas[1].c, 80.0);
        assert_eq!(s.camadas[1].phi, 0.0);
        assert_eq!(s.camadas[1].gamma, 17.0);
    }

    #[test]
    fn entrada_simplificada() {
        let mut s = Sondagem::default();
        s.modo = ModoSolo::Simplificado;
        s.simples_tipo = TipoSolo::Areia;
        s.simples_nspt = 20.0;
        let cs = s.camadas_calc();
        assert_eq!(cs.len(), 1);
        assert!((cs[0].phi - 35.0).abs() < 1e-9 && cs[0].c == 0.0 && cs[0].gamma == 20.0);
        // perfil homogêneo e fundação na mesma camada, qualquer profundidade
        let p = s.perfil(None);
        assert_eq!(p.camadas.len(), 1);
        assert!((s.camada_em(37.0).unwrap().phi - 35.0).abs() < 1e-9);
        // camadas da sondagem completa permanecem intactas
        assert_eq!(s.camadas.len(), 4);
        // argila não drenada: Su = 10·N
        s.simples_tipo = TipoSolo::Argila;
        s.simples_nspt = 8.0;
        s.coesivos = CondicaoCoesivos::NaoDrenada;
        let c = s.camada_simplificada();
        assert_eq!((c.c, c.phi, c.gamma), (80.0, 0.0, 17.0));
        s.modo = ModoSolo::Completo;
        assert_eq!(s.camadas_calc().len(), 4);
    }

    #[test]
    fn importacao_formatos() {
        let txt = "Prof;N;Descrição\n1;3;Argila mole\n2;4;\n3;12;Areia fina\nNA;2,5\n";
        let (c, na) = importar(txt).unwrap();
        assert_eq!(c.len(), 3);
        assert_eq!(c[1].tipo, TipoSolo::Argila);
        assert_eq!(na, Some(2.5));
        let txt2 = "0,0;2,5;6;Silte argiloso\n2,5;7,0;15;Areia siltosa";
        let (c2, _) = importar(txt2).unwrap();
        assert_eq!(c2[1].ate, 7.0);
        assert_eq!(c2[1].tipo, TipoSolo::AreiaSiltosa);
    }
}
