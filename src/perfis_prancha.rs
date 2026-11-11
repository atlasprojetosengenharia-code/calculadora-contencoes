//! Catálogo comercial de estacas-prancha (VPA Infra — catálogo "Estaca Prancha
//! Metálica / VANIL® (PVC)", Tabela de Modelos, p. 11 e p. 28).
//! Valores por metro de parede, exceto o peso da estaca (kg/m de peça).
//! O catálogo não informa a classe do aço: a tensão de escoamento fy é escolhida
//! pelo usuário (ver `ACOS`).

/// Forma do perfil.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Forma {
    U,
    Z,
    W,
}

impl Forma {
    pub fn nome(&self) -> &'static str {
        match self {
            Forma::U => "U",
            Forma::Z => "Z",
            Forma::W => "W",
        }
    }
}

/// Perfil metálico (aço perfilado a frio).
#[derive(Clone, Copy, Debug)]
pub struct PerfilAco {
    pub nome: &'static str,
    pub forma: Forma,
    /// Largura b, altura h e espessura t (mm).
    pub b: f64,
    pub h: f64,
    pub t: f64,
    /// Peso da estaca (kg/m de peça) e da parede (kg/m²).
    pub peso_estaca: f64,
    pub peso_parede: f64,
    /// Área da seção (cm²/m), raio de giração (cm), inércia (cm⁴/m) e módulo elástico (cm³/m).
    #[allow(dead_code)]
    pub area: f64,
    #[allow(dead_code)]
    pub raio: f64,
    pub inercia: f64,
    pub w: f64,
}

/// Perfil de PVC (VANIL®): resistência dada pelo momento admissível do fabricante.
#[derive(Clone, Copy, Debug)]
pub struct PerfilPvc {
    pub nome: &'static str,
    pub forma: Forma,
    pub b: f64,
    pub h: f64,
    pub t: f64,
    pub w: f64,
    pub inercia: f64,
    /// Momento admissível e máximo (kN·m/m) — catálogo.
    pub m_adm: f64,
    #[allow(dead_code)]
    pub m_max: f64,
}

const fn a(nome: &'static str, forma: Forma, b: f64, h: f64, t: f64, pe: f64, pp: f64, area: f64, raio: f64, inercia: f64, w: f64) -> PerfilAco {
    PerfilAco { nome, forma, b, h, t, peso_estaca: pe, peso_parede: pp, area, raio, inercia, w }
}

/// VPA — estacas-prancha metálicas (Tabela de Modelos, p. 11).
pub const ACO: [PerfilAco; 28] = [
    a("VPA-MTU 4", Forma::U, 400.0, 167.57, 6.3, 37.8, 94.5, 48.29, 5.74, 3990.0, 474.0),
    a("VPA-MTU 5", Forma::U, 650.0, 243.51, 6.3, 50.48, 77.53, 61.16, 7.55, 5351.0, 440.0),
    a("VPA-MTU 10", Forma::U, 880.0, 337.28, 6.3, 60.48, 68.73, 75.65, 10.98, 10430.0, 622.0),
    a("VPA-MTU 11", Forma::U, 880.0, 274.26, 9.5, 91.02, 103.64, 113.08, 9.5, 11677.0, 852.0),
    a("VPA-MTU 12", Forma::U, 950.0, 301.54, 8.0, 76.8, 80.84, 101.64, 10.62, 12134.0, 805.0),
    a("VPA-MTU 17", Forma::U, 840.0, 336.46, 9.5, 91.2, 108.57, 112.92, 11.27, 17198.0, 1023.0),
    a("VPA-MTU 18", Forma::U, 790.0, 390.08, 8.0, 76.8, 97.22, 100.04, 12.77, 17552.0, 921.0),
    a("VPA-MTU 29", Forma::U, 1100.0, 415.52, 9.5, 114.0, 103.64, 141.4, 14.86, 28500.0, 1372.0),
    a("VPA-MTU 32", Forma::U, 1010.0, 456.68, 8.0, 96.0, 95.05, 116.92, 16.44, 31173.0, 1366.0),
    a("VPA-MTU 34", Forma::U, 750.0, 496.0, 8.0, 76.8, 102.4, 100.87, 15.69, 33563.0, 1353.0),
    a("VPA-MTU 37", Forma::U, 750.0, 515.29, 8.0, 76.8, 102.4, 99.67, 16.49, 36410.0, 1412.0),
    a("VPA-MTU 40", Forma::U, 1000.0, 497.43, 9.5, 114.0, 114.0, 137.87, 17.01, 39768.0, 1600.0),
    a("VPA-MTU 69", Forma::U, 840.0, 602.47, 9.5, 114.0, 135.71, 137.39, 20.46, 68513.0, 2275.0),
    a("VPA-MTZ 18", Forma::Z, 860.0, 303.43, 9.5, 91.2, 106.05, 110.01, 12.0, 18436.0, 1215.0),
    a("VPA-MTZ 22", Forma::Z, 830.0, 342.63, 8.0, 76.8, 92.53, 94.57, 13.76, 21604.0, 1262.0),
    a("VPA-MTZ 23", Forma::Z, 830.0, 352.63, 8.0, 76.8, 92.53, 95.09, 14.17, 23030.0, 1306.0),
    a("VPA-MTZ 28", Forma::Z, 830.0, 364.43, 9.5, 91.2, 109.88, 111.46, 14.32, 27602.0, 1515.0),
    a("VPA-MTZ 29", Forma::Z, 800.0, 420.0, 8.0, 76.8, 96.0, 92.98, 15.76, 29065.0, 1384.0),
    a("VPA-MTZ 31", Forma::Z, 810.0, 432.86, 8.0, 76.8, 94.81, 94.58, 16.21, 30693.0, 1418.0),
    a("VPA-MTZ 33", Forma::Z, 810.0, 384.72, 9.5, 91.2, 112.59, 112.8, 15.3, 32816.0, 1706.0),
    a("VPA-MTZ 38", Forma::Z, 770.0, 448.86, 8.0, 76.8, 99.74, 94.56, 17.54, 37780.0, 1683.0),
    a("VPA-MTZ 41", Forma::Z, 760.0, 471.63, 8.0, 76.8, 101.05, 94.53, 18.23, 41352.0, 1753.0),
    a("VPA-MTZ 42", Forma::Z, 740.0, 471.63, 8.0, 76.8, 103.78, 93.3, 18.29, 42185.0, 1789.0),
    a("VPA-MTZ 49", Forma::Z, 710.0, 510.4, 8.0, 76.8, 108.17, 92.83, 19.38, 48683.0, 1907.0),
    a("VPA-MTZ 71", Forma::Z, 950.0, 550.47, 9.5, 114.0, 120.0, 92.83, 22.06, 70593.0, 2565.0),
    a("VPA-MTW 1", Forma::W, 300.0, 48.63, 4.75, 15.2, 50.67, 26.8, 1.27, 114.0, 55.0),
    a("VPA-MTW 3", Forma::W, 350.0, 50.0, 6.3, 25.4, 72.578, 32.37, 1.43, 189.0, 64.0),
    a("VPA-MTW 8", Forma::W, 550.0, 89.74, 6.35, 37.8, 68.73, 50.35, 3.56, 875.0, 195.0),
];

/// VPA — estacas-prancha VANIL® de PVC (Tabela de Modelos, p. 28).
pub const PVC: [PerfilPvc; 6] = [
    PerfilPvc { nome: "VPA VNZ-26", forma: Forma::Z, b: 270.0, h: 150.0, t: 3.5, w: 332.0, inercia: 2632.0, m_adm: 5.60, m_max: 11.20 },
    PerfilPvc { nome: "VPA VNU-53", forma: Forma::U, b: 610.0, h: 178.0, t: 6.4, w: 559.0, inercia: 5326.0, m_adm: 13.17, m_max: 26.34 },
    PerfilPvc { nome: "VPA VNU-127", forma: Forma::U, b: 610.0, h: 229.0, t: 8.9, w: 1034.0, inercia: 12773.0, m_adm: 23.72, m_max: 47.44 },
    PerfilPvc { nome: "VPA VNZ-227", forma: Forma::Z, b: 457.0, h: 254.0, t: 11.18, w: 1717.0, inercia: 22709.0, m_adm: 39.0, m_max: 77.92 },
    PerfilPvc { nome: "VPA VNZ-324", forma: Forma::Z, b: 457.0, h: 305.0, t: 11.7, w: 2132.0, inercia: 32486.0, m_adm: 45.22, m_max: 90.43 },
    PerfilPvc { nome: "VPA VNZ-472", forma: Forma::Z, b: 457.0, h: 305.0, t: 18.1, w: 3102.0, inercia: 47236.0, m_adm: 67.37, m_max: 134.74 },
];

/// Aços usuais de estacas-prancha (nome, fy em MPa). O catálogo VPA não especifica
/// a classe: confirme com o fornecedor.
pub const ACOS: [(&str, f64); 6] = [
    ("ASTM A572 Gr. 50 / A588 (fy 345 MPa)", 345.0),
    ("ASTM A328 (fy 270 MPa)", 270.0),
    ("ASTM A36 (fy 250 MPa)", 250.0),
    ("EN 10249 S235JRC (fy 235 MPa)", 235.0),
    ("EN 10249 S275JRC (fy 275 MPa)", 275.0),
    ("EN 10249 S355JOC (fy 355 MPa)", 355.0),
];

/// Perfil de catálogo identificado pelo nome.
#[derive(Clone, Copy, Debug)]
pub enum Catalogo {
    Aco(&'static PerfilAco),
    Pvc(&'static PerfilPvc),
}

pub fn buscar(nome: &str) -> Option<Catalogo> {
    let n = nome.trim();
    ACO.iter().find(|p| p.nome == n).map(Catalogo::Aco).or_else(|| PVC.iter().find(|p| p.nome == n).map(Catalogo::Pvc))
}

/// Perfis de aço em ordem crescente de peso da parede (os mais econômicos primeiro).
pub fn aco_por_peso() -> Vec<&'static PerfilAco> {
    let mut v: Vec<_> = ACO.iter().collect();
    v.sort_by(|a, b| a.peso_parede.partial_cmp(&b.peso_parede).unwrap().then(b.w.partial_cmp(&a.w).unwrap()));
    v
}

/// Perfil de aço mais leve (kg/m²) com W ≥ `w_nec` (cm³/m).
pub fn aco_mais_leve(w_nec: f64) -> Option<&'static PerfilAco> {
    aco_por_peso().into_iter().find(|p| p.w >= w_nec)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn catalogo_vpa_consistente() {
        // dados da tabela: W crescente dentro de cada família não é garantido, mas todos positivos
        for p in ACO.iter().filter(|p| p.forma != Forma::W) {
            assert!(p.w > 0.0 && p.inercia > p.w && p.peso_parede > 0.0, "{}", p.nome);
            // W ≈ I / (h/2) dentro de 15 % (confere a transcrição da tabela)
            let w_calc = p.inercia / (p.h / 20.0);
            assert!((w_calc / p.w - 1.0).abs() < 0.15, "{}: W tabela {} × I/(h/2) {:.0}", p.nome, p.w, w_calc);
        }
        for p in &PVC {
            assert!((p.m_max / p.m_adm - 2.0).abs() < 0.02, "{}", p.nome);
        }
        assert_eq!(ACO.iter().filter(|p| p.forma == Forma::U).count(), 13);
        assert_eq!(ACO.iter().filter(|p| p.forma == Forma::Z).count(), 12);
        assert!(matches!(buscar("VPA-MTZ 49"), Some(Catalogo::Aco(p)) if p.w == 1907.0));
        assert!(matches!(buscar("VPA VNU-127"), Some(Catalogo::Pvc(p)) if p.m_adm == 23.72));
        // mais leve que atende 1300 cm³/m: MTZ 23 (92,53 kg/m², W 1306)
        assert_eq!(aco_mais_leve(1300.0).unwrap().nome, "VPA-MTZ 23");
        assert!(aco_mais_leve(3000.0).is_none());
    }
}
