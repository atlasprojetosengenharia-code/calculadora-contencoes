//! Pesos específicos FIXOS dos materiais estruturais (não editáveis).
//!
//! Base: ABNT NBR 6120:2019, Tabela 1 — peso específico aparente dos
//! materiais de construção (valores médios quando a norma dá faixa).
//! Materiais compostos não tabelados na NBR 6120 (concreto ciclópico, pedra
//! argamassada, gabião, pedra seca) são obtidos dos constituintes tabelados,
//! adotando-se o LIMITE INFERIOR — a favor da segurança, pois o peso próprio
//! é a ação estabilizante dos muros de gravidade.

/// NBR 6120:2019 Tab. 1 — concreto simples.
pub const CONCRETO_SIMPLES: f64 = 24.0;
/// NBR 6120:2019 Tab. 1 — concreto armado.
pub const CONCRETO_ARMADO: f64 = 25.0;
/// NBR 6120:2019 Tab. 1 — argamassa de cimento e areia (19 a 23, médio 21).
pub const ARGAMASSA_CIMENTO_AREIA: f64 = 21.0;
/// NBR 6120:2019 Tab. 1 — granito, sienito, pórfiro (27 a 30, médio 28,5).
pub const GRANITO: f64 = 28.5;
/// Manual Técnico de Obras de Contenção (Maccaferri, 4.1): porosidade dos
/// gabiões normalmente entre 0,30 e 0,40 — adotado o valor máximo.
pub const POROSIDADE_GABIAO: f64 = 0.40;

/// Concreto ciclópico: concreto simples + pedra de mão (granito, 28,5).
/// Limite inferior dos constituintes = concreto simples.
pub const CONCRETO_CICLOPICO: f64 = CONCRETO_SIMPLES;
/// Pedra argamassada: pedra (granito, 28,5) + argamassa de cimento e areia (21).
/// Limite inferior dos constituintes = argamassa.
pub const PEDRA_ARGAMASSADA: f64 = ARGAMASSA_CIMENTO_AREIA;
/// Gabião: γg = γp·(1 − n) = 28,5 × (1 − 0,40) = 17,1 kN/m³.
pub const GABIAO: f64 = GRANITO * (1.0 - POROSIDADE_GABIAO);
/// Pedra seca arrumada: mesma regra do gabião (sem argamassa, porosidade 0,40).
pub const PEDRA_SECA: f64 = GRANITO * (1.0 - POROSIDADE_GABIAO);

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn valores_fixos() {
        assert_eq!(CONCRETO_SIMPLES, 24.0);
        assert_eq!(CONCRETO_ARMADO, 25.0);
        assert_eq!(PEDRA_ARGAMASSADA, 21.0);
        assert!((GABIAO - 17.1).abs() < 1e-9);
    }
}
