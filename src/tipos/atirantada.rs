//! Cortina atirantada — vários níveis de tirantes (NBR 5629 para tirantes,
//! NBR 6118 para o painel). Cargas nos tirantes por áreas de influência.

use crate::armadura::{self, Armadura, Forma};
use crate::concreto::{flexao_retangular, Concreto};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::fatores::Fatores;
use crate::normas;
use crate::resultado::{fmt, Desenho, Estilo, Resultado, Verificacao};
use crate::solo::{Cargas, Perfil};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Diagrama {
    /// Distribuição teórica (Rankine/Coulomb).
    Teorico,
    /// Envoltória aparente retangular (Terzaghi-Peck): p = 1,3·Pa/H.
    Aparente,
}

/// Aço do tirante, para o dimensionamento pela tração.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AcoTirante {
    /// Barra roscada de alta resistência ST 85/105 (fyk = 850 MPa).
    Barra,
    /// Cordoalhas de 7 fios Ø 12,7 mm CP 190 RB (fpyk = 1710 MPa), em feixe.
    Cordoalha,
}

impl AcoTirante {
    pub fn nome(&self) -> &'static str {
        match self {
            AcoTirante::Barra => "Barra roscada ST 85/105",
            AcoTirante::Cordoalha => "Cordoalhas CP 190 RB Ø 12,7 mm",
        }
    }

    /// Menor opção comercial cuja carga admissível 0,9·fyk·As/FS atende `carga` (kN):
    /// (descrição, As em cm², fyk em MPa, atende?).
    pub fn escolher(&self, carga: f64, fs: f64) -> (String, f64, f64, bool) {
        let opcoes: Vec<(String, f64, f64)> = match self {
            AcoTirante::Barra => vec![
                ("barra ST 85/105 Ø 26,5 mm".into(), 5.52, 850.0),
                ("barra ST 85/105 Ø 32 mm".into(), 8.04, 850.0),
                ("barra ST 85/105 Ø 36 mm".into(), 10.18, 850.0),
                ("barra ST 85/105 Ø 40 mm".into(), 12.57, 850.0),
            ],
            AcoTirante::Cordoalha => (1..=12)
                .map(|n| (format!("{n} cordoalha(s) CP 190 RB Ø 12,7 mm"), 0.987 * n as f64, 1710.0))
                .collect(),
        };
        for (nome, a, f) in &opcoes {
            if 0.9 * f * a / 10.0 / fs >= carga {
                return (nome.clone(), *a, *f, true);
            }
        }
        let (nome, a, f) = opcoes.last().unwrap().clone();
        (nome, a, f, false)
    }
}

fn verdadeiro() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CortinaAtirantada {
    /// Dimensionar o aço de cada tirante pela tração (NBR 5629).
    #[serde(default = "verdadeiro")]
    pub auto_dim: bool,
    pub tipo_aco: AcoTirante,
    pub h: f64,
    /// Profundidades dos níveis de tirantes (m).
    pub niveis: Vec<f64>,
    pub espac_h: f64,
    pub inclinacao: f64,
    /// Diâmetro da perfuração / bulbo (m).
    pub diam_furo: f64,
    /// Resistência ao cisalhamento no contato bulbo-solo qs (kPa).
    pub qs: f64,
    /// fyk do aço do tirante (MPa).
    pub fyk_tirante: f64,
    /// Área de aço do tirante adotado (cm²).
    pub area_aco: f64,
    pub espessura: f64,
    /// Dimensão da placa de ancoragem (m) — para punção.
    pub placa: f64,
    /// Ficha abaixo da escavação (m).
    pub ficha: f64,
    pub delta_passivo: f64,
    /// Altura da viga de coroamento (m).
    pub viga_h: f64,
    pub diagrama: Diagrama,
    pub concreto: Concreto,
    pub perfil: Perfil,
    pub cargas: Cargas,
    pub empuxo: ParamEmpuxo,
}

impl Default for CortinaAtirantada {
    fn default() -> Self {
        let mut perfil = Perfil::simples(19.0, 30.0, 5.0);
        perfil.camadas[0].espessura = 30.0;
        Self {
            auto_dim: true,
            tipo_aco: AcoTirante::Barra,
            h: 8.0,
            niveis: vec![1.2, 2.8],
            espac_h: 2.5,
            inclinacao: 15.0,
            diam_furo: 0.10,
            qs: 150.0,
            fyk_tirante: 850.0,
            area_aco: 8.04,
            espessura: 0.30,
            placa: 0.30,
            ficha: 1.0,
            delta_passivo: 0.0,
            viga_h: 0.5,
            diagrama: Diagrama::Aparente,
            concreto: Concreto::default(),
            perfil,
            cargas: Cargas::default(),
            empuxo: ParamEmpuxo::default(),
        }
    }
}

struct Tirante {
    z: f64,
    f_m: f64,
    carga: f64,
    l_livre: f64,
    l_bulbo: f64,
    /// Aço adotado (descrição, As em cm², fyk em MPa) e carga admissível do aço (kN).
    nome_aco: String,
    area: f64,
    fyk: f64,
    t_adm: f64,
}

impl CortinaAtirantada {
    fn niveis_ordenados(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self.niveis.iter().copied().filter(|&z| z > 0.0 && z < self.h).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    }

    /// Pressão horizontal total de cálculo (kPa) na profundidade z.
    fn pressao(&self, z: f64, p_aparente: f64, dp_sis: f64) -> f64 {
        let solo = match self.diagrama {
            Diagrama::Aparente => p_aparente,
            Diagrama::Teorico => {
                self.empuxo.pressao_ativa(&self.perfil, &self.cargas, 0.0, z) * self.empuxo.inclinacao_ativa(0.0).cos()
            }
        };
        let sis = if z < self.h && dp_sis > 0.0 { 2.0 * dp_sis / self.h * (1.0 - z / self.h) } else { 0.0 };
        solo + self.perfil.u(z) + self.cargas.pressao_faixa(z) + sis
    }

    /// Esforços na ficha (kN/m), entre H e H + ficha: (passivo característico
    /// à frente, ações atrás da ficha). As ações atrás = empuxo ativo teórico
    /// (componente horizontal) + água atrás − água à frente + carga em faixa.
    /// NA à frente: o do perfil, rebaixado no mínimo até o fundo da escavação.
    fn esforcos_ficha(&self) -> (f64, f64) {
        let h = self.h;
        let na_frente = self.perfil.na().map(|n| n.max(h));
        let dp = if self.empuxo.teoria == Teoria::Coulomb { self.delta_passivo } else { 0.0 };
        let pp = integrar(
            |z| {
                let c = self.perfil.camada_em(z);
                let kp = self.empuxo.coef_passivo(c.phi, dp);
                let sv = self.perfil.sigma_v_ef_desde(h, z, na_frente);
                (kp * sv + 2.0 * c.c * kp.sqrt()) * rad(dp).cos()
            },
            h,
            h + self.ficha,
            200,
        )
        .0;
        let u_frente = |z: f64| match na_frente {
            Some(zw) if z > zw => crate::solo::GAMMA_W * (z - zw),
            _ => 0.0,
        };
        let ativo = integrar(
            |z| self.empuxo.pressao_horizontal_total(&self.perfil, &self.cargas, 0.0, z) - u_frente(z),
            h,
            h + self.ficha,
            200,
        )
        .0;
        (pp, ativo)
    }

    pub fn calcular(&self, fat: &Fatores) -> Resultado {
        let mut r = Resultado::default();
        let h = self.h;
        let niveis = self.niveis_ordenados();
        let incl = rad(self.inclinacao);
        if niveis.is_empty() {
            r.aviso("Defina ao menos um nível de tirantes entre 0 e H.");
            r.verif(Verificacao::min("Níveis de tirantes", 0.0, 1.0, ""));
            r.desenho = self.desenho(&[], 0.0);
            return r;
        }

        let pa_h = integrar(
            |z| self.empuxo.pressao_ativa(&self.perfil, &self.cargas, 0.0, z) * self.empuxo.inclinacao_ativa(0.0).cos(),
            0.0,
            h,
            600,
        )
        .0;
        let p_ap = 1.3 * pa_h / h;

        r.titulo("Dados");
        r.linha(format!(
            "H = {} m ; {} níveis de tirantes ; espaçamento horizontal = {} m ; inclinação = {}°",
            fmt(h), niveis.len(), fmt(self.espac_h), fmt(self.inclinacao)
        ));
        r.linha(format!(
            "Teoria: {} ; diagrama: {} ; Pa,h (teórico) = {} kN/m",
            self.empuxo.teoria.nome(),
            match self.diagrama {
                Diagrama::Teorico => "teórico".to_string(),
                Diagrama::Aparente => format!("aparente retangular p = 1,3·Pa/H = {} kPa", fmt(p_ap)),
            },
            fmt(pa_h)
        ));

        r.norma(normas::NBR11682_SOBRECARGA);
        if self.diagrama == Diagrama::Aparente {
            r.formula(
                "Envoltória aparente de pressões (Terzaghi-Peck)",
                "p = 1,3·Pa,h / H ;  Pa,h = ∫₀ᴴ (Ka·σ'v + Ka·q − 2c'·√Ka)·cos i dz",
                format!("p = 1,3·{} / {}", fmt(pa_h), fmt(h)),
                format!("p = {} kPa (constante; água e carga em faixa somadas à parte)", fmt(p_ap)),
            );
        } else {
            r.formula(
                "Empuxo ativo teórico",
                "Pa,h = ∫₀ᴴ (Ka·σ'v + Ka·q − 2c'·√Ka)·cos i dz",
                format!("H = {} m ;  q = {} kPa", fmt(h), fmt(self.cargas.q_uniforme)),
                format!("Pa,h = {} kN/m", fmt(pa_h)),
            );
        }

        // casos
        let mut dps = vec![("estático", 0.0)];
        if self.cargas.usar_sismo {
            let (dp, _, _) = self.empuxo.incremento_sismico(&self.perfil, &self.cargas, 0.0, h);
            dps.push(("sísmico", dp * self.empuxo.inclinacao_ativa(0.0).cos()));
        }

        // Cargas por área de influência (envoltória dos casos)
        let n = niveis.len();
        let mut f_m = vec![0.0f64; n];
        let mut r_base: f64 = 0.0;
        let mut p_max: f64 = 0.0;
        for &(nome, dp) in &dps {
            let p = |z: f64| self.pressao(z, p_ap, dp);
            r.titulo(&format!("Cargas nos tirantes — caso {nome} (áreas de influência)"));
            for i in 0..n {
                let a = if i == 0 { 0.0 } else { 0.5 * (niveis[i - 1] + niveis[i]) };
                let b = if i == n - 1 { 0.5 * (niveis[i] + h) } else { 0.5 * (niveis[i] + niveis[i + 1]) };
                let f = integrar(p, a, b, 300).0;
                r.linha(format!("Nível {} (z = {} m): faixa {}–{} m → {} kN/m", i + 1, fmt(niveis[i]), fmt(a), fmt(b), fmt(f)));
                f_m[i] = f_m[i].max(f);
            }
            let rb = integrar(p, 0.5 * (niveis[n - 1] + h), h, 300).0;
            r.linha(format!("Reação na base da escavação = {} kN/m", fmt(rb)));
            r_base = r_base.max(rb);
            p_max = p_max.max((0..=100).map(|i| p(h * i as f64 / 100.0)).fold(0.0, f64::max));
        }

        // Tirantes
        let phi_m = rad(self.perfil.phi_medio(h));
        let rho = PI / 4.0 + phi_m / 2.0;
        let mut tirantes = Vec::new();
        r.titulo("Tirantes (NBR 5629)");
        r.linha(format!(
            "Superfície de ruptura: plano pela base da escavação a {}° com a horizontal (45° + φ/2)",
            fmt(rho.to_degrees())
        ));
        let rt = self.fyk_tirante * self.area_aco / 10.0;
        let perm = fat.tirante_permanente;
        let fs_aco = fat.fs_tirante_aco();
        let t_adm = 0.9 * rt / fs_aco;
        r.linha(format!(
            "Tirante {} ; fixação por {} ; {}",
            if perm { "permanente" } else { "provisório" },
            match fat.tirante_fixacao {
                crate::fatores::Fixacao::Rosca => "rosca",
                crate::fatores::Fixacao::Clavete => "clavetes",
            },
            if self.auto_dim {
                format!(
                    "aço dimensionado pela tração de cada tirante ({}): 0,9·fyk·As/{} ≥ Ft (4.5.2.1)",
                    self.tipo_aco.nome(),
                    fmt(fs_aco)
                )
            } else {
                format!(
                    "aço informado: fyk·As = {} kN ; carga admissível 0,9·fyk·As/{} = {} kN (4.5.2.1)",
                    fmt(rt), fmt(fs_aco), fmt(t_adm)
                )
            }
        ));
        r.linha(format!(
            "Bulbo: L = {} · Ft / (π · D · qs) (B.2.4.1) ; trecho livre ≥ {} m (B.2.7)",
            fmt(fat.fs_tirante_bulbo()), fmt(fat.tirante_livre_min())
        ));
        r.norma(normas::NBR5629_ACO);
        if !self.auto_dim {
            r.formula(
                "Carga admissível do aço do tirante",
                "T_adm = 0,9·fyk·As / FS",
                format!("T_adm = 0,9·{} MPa·{} cm² / {}", fmt(self.fyk_tirante), fmt(self.area_aco), fmt(fs_aco)),
                format!("fyk·As = {} kN ;  T_adm = {} kN", fmt(rt), fmt(t_adm)),
            );
        }
        r.norma(normas::NBR5629_BULBO);
        r.norma(normas::NBR5629_TRECHO_LIVRE);
        for (i, &z) in niveis.iter().enumerate() {
            let carga = f_m[i] * self.espac_h / incl.cos();
            let t_corte = (h - z) / (incl.sin() + incl.cos() * rho.tan());
            let l_livre = (t_corte + fat.tirante_folga_h * h).max(fat.tirante_livre_min());
            let l_bulbo =
                (fat.fs_tirante_bulbo() * carga / (PI * self.diam_furo * self.qs)).max(fat.tirante_bulbo_min);
            r.linha(format!(
                "Tirante {}: z = {} m ; carga de trabalho Ft = {} kN ; L livre = {} m ; L bulbo = {} m ; L total = {} m",
                i + 1, fmt(z), fmt(carga), fmt(l_livre), fmt(l_bulbo), fmt(l_livre + l_bulbo)
            ));
            r.formula(
                format!("Tirante {} (z = {} m)", i + 1, fmt(z)),
                "Ft = F·s / cos i ;  L_bulbo = FS·Ft / (π·D·qs) ;  L_livre = t_cunha + folga ≥ L_mín",
                format!(
                    "Ft = {}·{} / cos {}° ;  L_bulbo = {}·Ft / (π·{}·{})",
                    fmt(f_m[i]), fmt(self.espac_h), fmt(self.inclinacao), fmt(fat.fs_tirante_bulbo()),
                    fmt(self.diam_furo), fmt(self.qs)
                ),
                format!(
                    "Ft = {} kN ;  L_livre = {} m ;  L_bulbo = {} m ;  L_total = {} m",
                    fmt(carga), fmt(l_livre), fmt(l_bulbo), fmt(l_livre + l_bulbo)
                ),
            );
            // aço: dimensionado pela tração (menor opção que atende) ou informado
            let (nome_aco, as_i, fyk_i, cabe) = if self.auto_dim {
                self.tipo_aco.escolher(carga, fs_aco)
            } else {
                ("aço informado".to_string(), self.area_aco, self.fyk_tirante, true)
            };
            let t_adm_i = 0.9 * fyk_i * as_i / 10.0 / fs_aco;
            if self.auto_dim {
                r.formula(
                    format!("Aço do tirante {} — dimensionamento pela tração", i + 1),
                    "As,nec = Ft·FS / (0,9·fyk) ;  adotar a menor opção comercial com As ≥ As,nec",
                    format!("As,nec = {}·{} / (0,9·{} MPa)", fmt(carga), fmt(fs_aco), fmt(fyk_i)),
                    format!(
                        "As,nec = {} cm² ;  adotado: {} (As = {} cm², T_adm = {} kN)",
                        fmt(carga * fs_aco / (0.9 * fyk_i) * 10.0), nome_aco, fmt(as_i), fmt(t_adm_i)
                    ),
                );
                if !cabe {
                    r.aviso(format!("Tirante {}: nenhuma opção de {} resiste à carga; reduza o espaçamento horizontal ou use mais níveis.", i + 1, self.tipo_aco.nome()));
                }
            }
            r.verif(Verificacao::max(format!("Tirante {}: Ft ≤ 0,9·fyk·As/{}", i + 1, fmt(fs_aco)), carga, t_adm_i, "kN"));
            let f_ens = if perm { 1.75 } else { 1.50 };
            r.verif(Verificacao::max(
                format!("Tirante {}: carga de ensaio {}·Ft ≤ 0,9·fyk·As (D.3.4)", i + 1, fmt(f_ens)),
                f_ens * carga,
                0.9 * fyk_i * as_i / 10.0,
                "kN",
            ));
            tirantes.push(Tirante { z, f_m: f_m[i], carga, l_livre, l_bulbo, nome_aco, area: as_i, fyk: fyk_i, t_adm: t_adm_i });
        }
        let t_max = tirantes.iter().map(|t| t.carga).fold(0.0, f64::max);
        r.verif(Verificacao::info("Carga de trabalho máxima Ft", t_max, "kN"));
        // reforços para a estabilidade global (capacidades admissíveis)
        let fsb = fat.fs_tirante_bulbo();
        r.reforcos = tirantes
            .iter()
            .map(|t| crate::global::Reforco {
                p0: [0.0, -t.z],
                dir: [incl.cos(), -incl.sin()],
                comprimento: t.l_livre + t.l_bulbo,
                livre: t.l_livre,
                arr_por_m: PI * self.diam_furo * self.qs / (fsb * self.espac_h),
                aco: t.t_adm / self.espac_h,
            })
            .collect();
        // Ensaios de recebimento — NBR 5629 Tabela D.1
        let (fa, fb, ta, tb) = if perm { (1.75, 1.40, "A", "B") } else { (1.50, 1.20, "C", "D") };
        r.linha(format!(
            "Ensaios de recebimento (Tab. D.1): tipo {ta} (≥10% dos tirantes) até {fa}·Ft = {} kN ; tipo {tb} (demais) até {fb}·Ft = {} kN",
            fmt(fa * t_max), fmt(fb * t_max)
        ));
        r.norma(normas::NBR5629_ENSAIOS);
        r.formula(
            "Carga máxima de ensaio de recebimento",
            "P_ensaio = f·Ft,máx ≤ 0,9·fyk·As",
            format!("P_ensaio = {}·{}", fmt(fa), fmt(t_max)),
            format!(
                "P_ensaio,máx = {} kN ;  menor 0,9·fyk·As entre os tirantes = {} kN (verificação por tirante acima)",
                fmt(fa * t_max),
                fmt(tirantes.iter().map(|t| 0.9 * t.fyk * t.area / 10.0).fold(f64::INFINITY, f64::min))
            ),
        );
        let v_tir: f64 = tirantes.iter().map(|t| t.f_m * incl.tan()).sum();
        r.linha(format!("Componente vertical dos tirantes transmitida à cortina = {} kN/m", fmt(v_tir)));

        // Ficha
        r.titulo("Ficha / apoio na base");
        if self.ficha > 0.0 {
            let fs_p = fat.fs_passivo.max(1.0);
            let (pp, ativo) = self.esforcos_ficha();
            let disponivel = pp / fs_p - ativo;
            r.linha(format!(
                "Passivo na ficha = {} kN/m ; reduzido (÷{}) = {} kN/m",
                fmt(pp), fmt(fs_p), fmt(pp / fs_p)
            ));
            r.linha(format!(
                "Ativo + água líquida + faixa atrás da ficha = {} kN/m ; resistência disponível = {} kN/m",
                fmt(ativo), fmt(disponivel)
            ));
            r.formula(
                "Equilíbrio da ficha",
                "R_disp = Ep / FS_p − (Ea + Uatrás − Ufrente + faixa)  ≥  R_base",
                format!("R_disp = {} / {} − {}", fmt(pp), fmt(fs_p), fmt(ativo)),
                format!("R_disp = {} kN/m ;  R_base = {} kN/m", fmt(disponivel), fmt(r_base)),
            );
            r.verif(Verificacao::min("Passivo reduzido − ativo na ficha ≥ reação na base", disponivel, r_base, "kN/m"));
        } else {
            r.aviso("Sem ficha: a reação na base deve ser resistida por apoio em rocha/viga de coroamento.");
        }

        // Painel
        let as_painel = self.painel(&mut r, fat, &niveis, p_max, t_max);
        self.detalhamento(&mut r, &niveis, &tirantes, as_painel);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        r.aviso("Comprimentos de bulbo e qs devem ser confirmados por ensaios de tirantes (NBR 5629, Anexo D).");
        r.aviso(format!(
            "Estabilidade global ainda não implementada — FS mínimo exigido = {} (NBR 5629 Tab. B.3 / NBR 11682 Tab. 3{}).",
            fmt(if fat.tirante_permanente { fat.fs_global() } else { 1.2 }),
            if fat.tirante_permanente { "" } else { "; obra provisória: mínimo 1,2" }
        ));
        r.aviso("Folga do trecho livre além da cunha ativa e bulbo mínimo são critérios de projeto (editáveis em Fatores), não exigências da NBR 5629.");
        r.desenho = self.desenho(&tirantes, rho);
        self.desenhar_armaduras(&mut r.desenho, &r.armaduras);
        r
    }

    fn painel(&self, r: &mut Resultado, fat: &Fatores, niveis: &[f64], p_max: f64, t_max: f64) -> [f64; 2] {
        let c = &self.concreto;
        let e = self.espessura;
        let d = e - c.cobrimento - 0.01;
        let mut lv: f64 = 0.0;
        for w in niveis.windows(2) {
            lv = lv.max(w[1] - w[0]);
        }
        lv = lv.max(self.h - niveis[niveis.len() - 1]);
        let a_topo = niveis[0];
        r.titulo("Painel da cortina (NBR 6118) — por metro");
        r.linha(format!(
            "p máx = {} kPa ; vão vertical = {} m ; vão horizontal = {} m ; balanço superior = {} m ; e = {} m",
            fmt(p_max), fmt(lv), fmt(self.espac_h), fmt(a_topo), fmt(e)
        ));
        let mv = (p_max * lv * lv / 10.0).max(p_max * a_topo * a_topo / 2.0);
        let mh = p_max * self.espac_h * self.espac_h / 10.0;
        let mut as_ad = [0.0; 2];
        for (k, (nome, m)) in [("vertical", mv), ("horizontal", mh)].into_iter().enumerate() {
            let md = fat.gamma_f * m;
            let f = flexao_retangular(md, 1.0, e, d, c.fck, c.fyk, fat.gamma_c, fat.gamma_s);
            r.linha(format!(
                "Direção {nome}: M ≈ pL²/10 = {} kN·m/m ; Md = {} ; As = {} cm²/m (mín. {})",
                fmt(m), fmt(md), fmt(f.as_calc), fmt(f.as_min)
            ));
            r.norma(normas::NBR6118_FLEXAO);
            r.norma(normas::NBR6118_AS_MIN);
            r.formula(
                format!("Painel — direção {nome}"),
                "M = p·L²/10 (vão) ou p·a²/2 (balanço) ;  Md = γf·M ;  As = Md / (fyd·(d − y/2))",
                format!("p = {} kPa ;  Md = {}·{} ;  d = {} cm", fmt(p_max), fmt(fat.gamma_f), fmt(m), fmt(d * 100.0)),
                format!("Md = {} kN·m/m ;  x/d = {} ;  As = {} cm²/m (mín. {})", fmt(md), fmt(f.x_d), fmt(f.as_calc), fmt(f.as_min)),
            );
            r.verif(Verificacao::max(format!("Painel {nome}: x/d"), f.x_d, 0.45, ""));
            as_ad[k] = f.as_adotar();
        }
        // Punção na placa de ancoragem (NBR 6118 19.5.3.2, contorno C')
        let dm = d * 1000.0;
        let u = 4.0 * self.placa * 1000.0 + 4.0 * PI * dm;
        let rho = ((as_ad[0] / 1e4 / d) * (as_ad[1] / 1e4 / d)).sqrt().min(0.02);
        let trd1 = crate::concreto::tau_rd1_puncao(d, rho, c.fck);
        let fsd = fat.gamma_f * t_max * rad(self.inclinacao).cos();
        let tsd = fsd * 1000.0 / (u * dm);
        r.linha(format!(
            "Punção: FSd = {} kN ; u = {} m ; τSd = {} MPa ; τRd1 = {} MPa",
            fmt(fsd), fmt(u / 1000.0), fmt(tsd), fmt(trd1)
        ));
        r.norma(normas::NBR6118_PUNCAO);
        r.formula(
            "Punção da placa de ancoragem no contorno C'",
            "u = 4·a + 4·π·d ;  τSd = FSd / (u·d) ;  τRd1 = 0,13·(1 + √(20/d))·(100·ρ·fck)^(1/3)",
            format!("FSd = {}·{}·cos {}° ;  a = {} m ;  d = {} cm", fmt(fat.gamma_f), fmt(t_max), fmt(self.inclinacao), fmt(self.placa), fmt(d * 100.0)),
            format!("u = {} m ;  τSd = {} MPa ;  τRd1 = {} MPa", fmt(u / 1000.0), fmt(tsd), fmt(trd1)),
        );
        r.verif(Verificacao::max("Punção na placa: τSd ≤ τRd1", tsd, trd1, "MPa"));
        as_ad
    }

    /// Detalhamento construtivo: tirantes, painel, viga de coroamento e blocos de ancoragem.
    fn detalhamento(&self, r: &mut Resultado, niveis: &[f64], tirantes: &[Tirante], as_painel: [f64; 2]) {
        let c = &self.concreto;
        let (e, cob) = (self.espessura, c.cobrimento);
        r.titulo("Detalhamento construtivo — cortina atirantada");
        r.norma(normas::NBR5629_INICIO_BULBO);
        r.linha(format!(
            "Tirantes: {} linhas ; espaçamento horizontal {} m ; inclinação {}° com a horizontal ; furo Ø {} cm",
            niveis.len(),
            fmt(self.espac_h),
            fmt(self.inclinacao),
            fmt(self.diam_furo * 100.0)
        ));
        let esp_v: Vec<String> = niveis.windows(2).map(|w| fmt(w[1] - w[0])).collect();
        r.linha(format!(
            "Linhas de tirantes a {} m do topo ; espaçamento vertical entre linhas: {} m",
            niveis.iter().map(|z| fmt(*z)).collect::<Vec<_>>().join(" ; "),
            if esp_v.is_empty() { "—".into() } else { esp_v.join(" ; ") }
        ));
        for (i, t) in tirantes.iter().enumerate() {
            r.linha(format!(
                "T{} (z = {} m): trecho livre {} m + trecho ancorado (bulbo) {} m = {} m ; Ft = {} kN ; aço: {} (As = {} cm², T_adm = {} kN)",
                i + 1,
                fmt(t.z),
                fmt(t.l_livre),
                fmt(t.l_bulbo),
                fmt(t.l_livre + t.l_bulbo),
                fmt(t.carga),
                t.nome_aco,
                fmt(t.area),
                fmt(t.t_adm)
            ));
        }
        let l_min = tirantes.iter().map(|t| t.l_livre).fold(f64::INFINITY, f64::min);
        if l_min.is_finite() {
            r.verif(Verificacao::min("Início do bulbo ≥ 3,0 m do paramento (NBR 5629 5.5.1 a)", l_min, 3.0, "m"));
        }

        // Painel: armadura nas duas faces e nas duas direções
        let mut tab = Vec::new();
        for (pos, elem, as_nec, comp) in [
            ("N1", "Painel — vertical (cada face)", as_painel[0], self.h + self.ficha - 2.0 * cob),
            ("N2", "Painel — horizontal (cada face)", as_painel[1], 0.0),
        ] {
            let (phi, s, as_ef) = armadura::malha_principal(as_nec, e);
            let lb = armadura::lb_basico(phi, c.fck, c.fyk, 1.4, 1.15, pos == "N1");
            tab.push(Armadura {
                pos: pos.into(),
                elemento: elem.into(),
                bitola_mm: phi,
                espac_cm: Some(s),
                quantidade: format!("{}/m por face", (100.0 / s).ceil() as i64),
                comprimento_m: comp,
                as_nec,
                as_ef,
                unidade: "cm²/m",
                lb_cm: armadura::lb_nec(lb, phi, as_nec, as_ef),
                forma: Forma::Reta,
            });
        }
        let bv = e.max(0.30);
        tab.extend(armadura::viga_construtiva("N3", "N4", "Viga de coroamento", bv, self.viga_h, c.fck, c.fyk, cob));
        r.formula(
            "Viga de coroamento (armadura construtiva)",
            "As,mín = ρmín·b·h ;  barras Ø12,5 (mín. 4) ;  estribos Ø6,3 c/15",
            format!("b = {} m ;  h = {} m ;  fck = {} MPa", fmt(bv), fmt(self.viga_h), fmt(c.fck)),
            format!("As,mín = {} cm² → {} Ø12,5", fmt(tab[2].as_nec), tab[2].quantidade),
        );
        r.linha(format!(
            "Blocos de ancoragem: placa de aço {} × {} m perpendicular ao eixo do tirante (inclinação {}°), apoiada em nicho/bloco de concreto da cortina; a punção na placa é verificada na seção do painel.",
            fmt(self.placa),
            fmt(self.placa),
            fmt(self.inclinacao)
        ));
        r.linha(format!(
            "Painel de {} m de espessura, concreto C{}, cobrimento {} cm ; armadura N1/N2 nas duas faces (momentos positivos entre linhas e negativos nos tirantes).",
            fmt(e),
            fmt(c.fck),
            fmt(cob * 100.0)
        ));
        r.armaduras.extend(tab);
    }

    /// Armaduras do painel e viga de coroamento no desenho.
    fn desenhar_armaduras(&self, d: &mut Desenho, arm: &[Armadura]) {
        let (e, cob) = (self.espessura, self.concreto.cobrimento);
        let lt = self.h + self.ficha;
        for x in [-cob, -(e - cob)] {
            d.linha(Estilo::Armadura, vec![[x, -lt + cob], [x, -cob]]);
        }
        let mut y = -lt + 0.2;
        while y < -0.1 {
            d.circulo(Estilo::ArmaduraSecao, [-cob - 0.015, y], 0.012);
            d.circulo(Estilo::ArmaduraSecao, [-(e - cob) + 0.015, y], 0.012);
            y += 0.25;
        }
        d.retangulo(Estilo::Aco, -e.max(0.30), -self.viga_h, 0.0, 0.0);
        if let Some(a) = arm.iter().find(|a| a.pos == "N1") {
            d.texto([-e - 0.18 * self.h, -0.55 * self.h], a.descricao());
        }
        if let Some(a) = arm.iter().find(|a| a.pos == "N2") {
            d.texto([-e - 0.18 * self.h, -0.62 * self.h], a.descricao());
        }
        d.linha(Estilo::Cota, vec![[-e.max(0.30), -self.viga_h / 2.0], [-e - 0.10 * self.h, -0.12 * self.h]]);
        d.texto([-e - 0.18 * self.h, -0.15 * self.h], "viga de coroamento");
    }


    fn desenho(&self, tirantes: &[Tirante], rho: f64) -> Desenho {
        let mut d = Desenho::default();
        let h = self.h;
        let lt = h + self.ficha;
        let fundo = -(lt + 0.15 * h);
        let l_max = tirantes.iter().map(|t| t.l_livre + t.l_bulbo).fold(h, f64::max);
        let xe = l_max * 1.05;
        let xi = -0.5 * h;
        // terrapleno em talude (β) a partir do topo da cortina
        let tb = rad(self.empuxo.beta).tan();
        let sup = move |x: f64| x.max(0.0) * tb;
        d.poli(Estilo::Solo, vec![[0.0, fundo], [xe, fundo], [xe, sup(xe)], [0.0, 0.0]]);
        d.retangulo(Estilo::Solo, xi, fundo, 0.0, -h);
        d.retangulo(Estilo::Concreto, -self.espessura, -lt, 0.0, 0.0);
        if let Some(na) = self.perfil.na() {
            d.tracejada(Estilo::Agua, vec![[0.0, -na], [xe, -na]]);
            d.texto([xe * 0.9, -na + 0.04 * h], "NA");
        }
        if self.cargas.q_uniforme > 0.0 {
            d.sobrecarga_sup(0.2, xe, 0.06 * h, &sup);
            d.texto([xe * 0.6, sup(xe * 0.6) + 0.17 * h], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        if rho > 0.0 {
            // o plano de ruptura termina na superfície do terrapleno
            let x_r = if rho.tan() > tb + 1e-9 { h / (rho.tan() - tb) } else { h / rho.tan() };
            d.tracejada(Estilo::Ruptura, vec![[0.0, -h], [x_r, sup(x_r)]]);
        }
        let i = rad(self.inclinacao);
        for (k, t) in tirantes.iter().enumerate() {
            let p0 = [0.0, -t.z];
            // trecho através do painel, da cabeça de ancoragem (face externa) até o tardoz
            let cab = [-self.espessura, -t.z + self.espessura * i.tan()];
            d.linha(Estilo::Tirante, vec![cab, p0]);
            d.retangulo(Estilo::Aco, cab[0] - 0.04 * h.min(6.0) / 6.0 - 0.02, cab[1] - 0.12, cab[0], cab[1] + 0.12);
            let p1 = [t.l_livre * i.cos(), -t.z - t.l_livre * i.sin()];
            let l = t.l_livre + t.l_bulbo;
            let p2 = [l * i.cos(), -t.z - l * i.sin()];
            d.linha(Estilo::Tirante, vec![p0, p1]);
            d.linha(Estilo::Bulbo, vec![p1, p2]);
            d.texto([p2[0] + 0.04 * h, p2[1] + 0.03 * h], format!("T{} = {} kN", k + 1, fmt(t.carga)));
        }
        // diagrama de pressões desenhado no lado escavado
        let ap = 1.3
            * integrar(
                |z| self.empuxo.pressao_ativa(&self.perfil, &self.cargas, 0.0, z) * self.empuxo.inclinacao_ativa(0.0).cos(),
                0.0,
                h,
                300,
            )
            .0
            / h;
        let pts: Vec<(f64, f64)> = (0..=40)
            .map(|k| {
                let z = h * k as f64 / 40.0;
                (-z, self.pressao(z, ap, 0.0))
            })
            .collect();
        let pmax = pts.iter().map(|p| p.1.abs()).fold(1e-6, f64::max);
        d.diagrama(-self.espessura, -0.35 * h / pmax, &pts);
        d.texto([-self.espessura - 0.2 * h, 0.05 * h], format!("p máx = {} kPa", fmt(pmax)));
        d.texto([xi * 0.7, -h * 0.5], format!("H = {} m", fmt(h)));
        d
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ficha_desconta_ativo() {
        // Rankine φ=30°, γ=19, c=0, q=20, sem água, H=8, ficha=1:
        // ativo = Ka·∫(19z + 20)dz [8;9] = (161,5 + 20)/3 = 60,5 kN/m
        // passivo = Kp·∫19(z − 8)dz [8;9] = 3·9,5 = 28,5 kN/m
        let mut c = CortinaAtirantada::default();
        c.perfil.camadas[0].c = 0.0;
        c.cargas.q_uniforme = 20.0;
        c.empuxo.teoria = Teoria::Rankine;
        c.h = 8.0;
        c.ficha = 1.0;
        let (pp, ativo) = c.esforcos_ficha();
        assert!((ativo - 60.5).abs() < 0.05, "ativo = {ativo}");
        assert!((pp - 28.5).abs() < 0.05, "passivo = {pp}");
        // a verificação usa passivo reduzido − ativo
        let r = c.calcular(&Fatores::default());
        let v = r.verificacoes.iter().find(|v| v.nome.starts_with("Passivo reduzido")).unwrap();
        assert!((v.valor - (28.5 / 2.0 - 60.5)).abs() < 0.1, "{}", v.valor);
        assert!(!v.ok());
    }
}
