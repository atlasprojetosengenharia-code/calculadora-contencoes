//! Reaterro compactado do tardoz dos muros de gravidade e de flexão: material
//! especificado (tipo de solo e NSPT equivalente), grau de compactação e
//! espessura das camadas. Os parâmetros (γ, φ', c') seguem as mesmas
//! correlações com o SPT usadas na sondagem.

use crate::solo::{Camada, Perfil};
use crate::spt::{Sondagem, TipoSolo};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Reaterro {
    /// Usar o material especificado no tardoz (senão, vale o perfil da sondagem).
    pub usar: bool,
    pub tipo: TipoSolo,
    /// NSPT equivalente do solo compactado.
    pub nspt: f64,
    /// Grau de compactação mínimo (% do Proctor Normal).
    pub gc_min: f64,
    /// Espessura das camadas compactadas (cm).
    pub camada_cm: f64,
    /// Faixa junto à estrutura compactada com equipamento leve (m).
    pub faixa_leve_m: f64,
}

impl Default for Reaterro {
    fn default() -> Self {
        Self { usar: true, tipo: TipoSolo::AreiaArgilosa, nspt: 10.0, gc_min: 95.0, camada_cm: 20.0, faixa_leve_m: 1.0 }
    }
}

impl Reaterro {
    /// Projetos salvos antes da especificação do reaterro mantêm o perfil da sondagem.
    pub fn legado() -> Self {
        Self { usar: false, ..Self::default() }
    }

    /// (γ, γsat, c', φ') por correlação com o NSPT equivalente.
    pub fn parametros(&self, s: &Sondagem) -> (f64, f64, f64, f64) {
        s.parametros(self.tipo, self.nspt)
    }

    /// Perfil de cálculo do tardoz: camada única de solo compactado.
    pub fn perfil(&self, s: &Sondagem, na: Option<f64>) -> Perfil {
        let (g, gs, c, phi) = self.parametros(s);
        Perfil {
            camadas: vec![Camada::nova("Reaterro compactado", 100.0, g, gs, phi, c)],
            usar_na: na.is_some(),
            prof_na: na.unwrap_or(0.0),
        }
    }

    /// Especificação executiva do reaterro (memorial e notas do desenho).
    pub fn especificacao(&self, s: &Sondagem) -> Vec<String> {
        let (g, gs, c, phi) = self.parametros(s);
        let f = crate::resultado::fmt;
        vec![
            format!(
                "Reaterro: {} compactado, NSPT equivalente = {} → γ = {} kN/m³ (γsat = {}), φ' = {}°, c' = {} kPa (correlações da sondagem).",
                self.tipo.nome().to_lowercase(),
                f(self.nspt),
                f(g),
                f(gs),
                f(phi),
                f(c)
            ),
            format!(
                "Execução: camadas de {} cm (solta, antes de compactar), espalhadas em toda a largura, com umidade na faixa ótima ± 2 % e grau de compactação ≥ {} % do Proctor Normal (ensaio NBR 7182).",
                format!("{:.0}", self.camada_cm),
                format!("{:.0}", self.gc_min)
            ),
            format!(
                "Faixa de {} m junto à estrutura: compactadores manuais (sapo, placa vibratória), para não danificar o paramento; na parte restante, rolos e equipamentos convencionais.",
                f(self.faixa_leve_m)
            ),
            "Controle: grau de compactação e desvio de umidade por camada (frasco de areia, NBR 7185), controle visual e geométrico; solo isento de matéria orgânica, entulho e blocos; superfície do terreno natural inclinada acima de 1:3 (V:H) escalonada em degraus (Marchetti; Maccaferri, Manual Técnico).".into(),
            "O reaterro é lançado e compactado à medida que a estrutura sobe; o sistema de drenagem do tardoz é implantado junto, nunca depois.".into(),
        ]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn reaterro_gera_perfil_e_especificacao() {
        let s = Sondagem::default();
        let r = Reaterro::default();
        let p = r.perfil(&s, None);
        assert_eq!(p.camadas.len(), 1);
        let (g, _, _, phi) = r.parametros(&s);
        assert!((p.camadas[0].gamma - g).abs() < 1e-9 && (p.camadas[0].phi - phi).abs() < 1e-9);
        let t = r.especificacao(&s).join(" ");
        assert!(t.contains("20 cm") && t.contains("95 %") && t.contains("NBR 7182") && t.contains("NBR 7185"));
        assert!(!Reaterro::legado().usar && Reaterro::default().usar);
    }
}
