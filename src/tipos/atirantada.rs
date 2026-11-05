//! Cortina atirantada — vários níveis de tirantes (NBR 5629 para tirantes,
//! NBR 6118 para o painel). Cargas nos tirantes por áreas de influência.

use crate::concreto::{flexao_retangular, Concreto};
use crate::empuxo::{integrar, rad, ParamEmpuxo, Teoria};
use crate::fatores::Fatores;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CortinaAtirantada {
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
            "Tirante {} ; fixação por {} ; fyk·As = {} kN ; carga admissível do aço 0,9·fyk·As/{} = {} kN (4.5.2.1)",
            if perm { "permanente" } else { "provisório" },
            match fat.tirante_fixacao {
                crate::fatores::Fixacao::Rosca => "rosca",
                crate::fatores::Fixacao::Clavete => "clavetes",
            },
            fmt(rt), fmt(fs_aco), fmt(t_adm)
        ));
        r.linha(format!(
            "Bulbo: L = {} · Ft / (π · D · qs) (B.2.4.1) ; trecho livre ≥ {} m (B.2.7)",
            fmt(fat.fs_tirante_bulbo()), fmt(fat.tirante_livre_min())
        ));
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
            r.verif(Verificacao::max(format!("Tirante {}: Ft ≤ 0,9·fyk·As/{}", i + 1, fmt(fs_aco)), carga, t_adm, "kN"));
            tirantes.push(Tirante { z, f_m: f_m[i], carga, l_livre, l_bulbo });
        }
        let t_max = tirantes.iter().map(|t| t.carga).fold(0.0, f64::max);
        r.verif(Verificacao::info("Carga de trabalho máxima Ft", t_max, "kN"));
        // Ensaios de recebimento — NBR 5629 Tabela D.1
        let (fa, fb, ta, tb) = if perm { (1.75, 1.40, "A", "B") } else { (1.50, 1.20, "C", "D") };
        r.linha(format!(
            "Ensaios de recebimento (Tab. D.1): tipo {ta} (≥10% dos tirantes) até {fa}·Ft = {} kN ; tipo {tb} (demais) até {fb}·Ft = {} kN",
            fmt(fa * t_max), fmt(fb * t_max)
        ));
        r.verif(Verificacao::max(
            format!("Carga máx. de ensaio ({fa}·Ft) ≤ 0,9·fyk·As (D.3.4)"),
            fa * t_max,
            0.9 * rt,
            "kN",
        ));
        let v_tir: f64 = tirantes.iter().map(|t| t.f_m * incl.tan()).sum();
        r.linha(format!("Componente vertical dos tirantes transmitida à cortina = {} kN/m", fmt(v_tir)));

        // Ficha
        r.titulo("Ficha / apoio na base");
        if self.ficha > 0.0 {
            let fs_p = fat.fs_passivo.max(1.0);
            let dp = if self.empuxo.teoria == Teoria::Coulomb { self.delta_passivo } else { 0.0 };
            let pp = integrar(
                |z| {
                    let c = self.perfil.camada_em(z);
                    let kp = self.empuxo.coef_passivo(c.phi, dp);
                    let sv = self.perfil.sigma_v_ef_desde(h, z, self.perfil.na().map(|n| n.max(h)));
                    (kp * sv + 2.0 * c.c * kp.sqrt()) * rad(dp).cos()
                },
                h,
                h + self.ficha,
                200,
            )
            .0;
            r.linha(format!("Passivo na ficha = {} kN/m ; reduzido (÷{}) = {} kN/m", fmt(pp), fmt(fs_p), fmt(pp / fs_p)));
            r.verif(Verificacao::min("Passivo reduzido ≥ reação na base", pp / fs_p, r_base, "kN/m"));
        } else {
            r.aviso("Sem ficha: a reação na base deve ser resistida por apoio em rocha/viga de coroamento.");
        }

        // Painel
        self.painel(&mut r, fat, &niveis, p_max, t_max);

        self.cargas.avisar_minima(&mut r, fat.sobrecarga_minima);
        r.aviso("Comprimentos de bulbo e qs devem ser confirmados por ensaios de tirantes (NBR 5629, Anexo D).");
        r.aviso(format!(
            "Estabilidade global ainda não implementada — FS mínimo exigido = {} (NBR 5629 Tab. B.3 / NBR 11682 Tab. 3{}).",
            fmt(if fat.tirante_permanente { fat.fs_global() } else { 1.2 }),
            if fat.tirante_permanente { "" } else { "; obra provisória: mínimo 1,2" }
        ));
        r.aviso("Folga do trecho livre além da cunha ativa e bulbo mínimo são critérios de projeto (editáveis em Fatores), não exigências da NBR 5629.");
        r.desenho = self.desenho(&tirantes, rho);
        r
    }

    fn painel(&self, r: &mut Resultado, fat: &Fatores, niveis: &[f64], p_max: f64, t_max: f64) {
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
        r.verif(Verificacao::max("Punção na placa: τSd ≤ τRd1", tsd, trd1, "MPa"));
    }

    fn desenho(&self, tirantes: &[Tirante], rho: f64) -> Desenho {
        let mut d = Desenho::default();
        let h = self.h;
        let lt = h + self.ficha;
        let fundo = -(lt + 0.15 * h);
        let l_max = tirantes.iter().map(|t| t.l_livre + t.l_bulbo).fold(h, f64::max);
        let xe = l_max * 1.05;
        let xi = -0.5 * h;
        d.retangulo(Estilo::Solo, 0.0, fundo, xe, 0.0);
        d.retangulo(Estilo::Solo, xi, fundo, 0.0, -h);
        d.retangulo(Estilo::Concreto, -self.espessura, -lt, 0.0, 0.0);
        if let Some(na) = self.perfil.na() {
            d.tracejada(Estilo::Agua, vec![[0.0, -na], [xe, -na]]);
            d.texto([xe * 0.9, -na + 0.04 * h], "NA");
        }
        if self.cargas.q_uniforme > 0.0 {
            d.sobrecarga(0.2, xe, 0.0, 0.06 * h);
            d.texto([xe * 0.6, 0.11 * h], format!("q = {} kPa", fmt(self.cargas.q_uniforme)));
        }
        if rho > 0.0 {
            d.tracejada(Estilo::Ruptura, vec![[0.0, -h], [h / rho.tan(), 0.0]]);
        }
        let i = rad(self.inclinacao);
        for (k, t) in tirantes.iter().enumerate() {
            let p0 = [0.0, -t.z];
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
