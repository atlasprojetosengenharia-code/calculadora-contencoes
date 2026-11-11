//! Referências bibliográficas do memorial de cálculo (formato ABNT NBR 6023).
//!
//! A lista é montada a partir do que o memorial efetivamente usa: as normas
//! citadas no texto, nas caixas normativas e nos avisos, e os métodos e autores
//! mencionados (Rankine, Coulomb, Bishop, Godoy, Teixeira, Marchetti…).

use crate::resultado::{Bloco, Resultado};

/// Título e ano padrão de cada norma (quando o memorial cita só o número).
const NORMAS: [(&str, &str, &str); 14] = [
    ("5629", "Tirantes ancorados no terreno — Projeto e execução", "2018"),
    ("6118", "Projeto de estruturas de concreto", "2023"),
    ("6120", "Ações para o cálculo de estruturas de edificações", "2019"),
    ("6122", "Projeto e execução de fundações", "2019"),
    ("6484", "Solo — Sondagem de simples reconhecimento com SPT — Método de ensaio", "2020"),
    ("6502", "Solos e rochas — Terminologia", "2022"),
    ("7182", "Solo — Ensaio de compactação", "2016"),
    ("7185", "Solo — Determinação da massa específica aparente, in situ, com emprego do frasco de areia", "2016"),
    ("7211", "Agregados para concreto — Requisitos", "2022"),
    ("8800", "Projeto de estruturas de aço e de estruturas mistas de aço e concreto de edifícios", "2008"),
    ("11682", "Estabilidade de encostas", "2009"),
    ("16868-1", "Alvenaria estrutural — Parte 1: Projeto", "2020"),
    ("16920-1", "Muros e taludes em solos reforçados — Parte 1: Solos reforçados em aterros", "2021"),
    ("16920-2", "Muros e taludes em solos reforçados — Parte 2: Solos grampeados", "2021"),
];

/// Obras técnicas: (palavras que indicam o uso no memorial, referência ABNT).
const OBRAS: [(&[&str], &str); 17] = [
    (&["Bishop"], "BISHOP, A. W. The use of the slip circle in the stability analysis of slopes. Géotechnique, London, v. 5, n. 1, p. 7-17, 1955."),
    (&["Blum"], "BLUM, H. Einspannungsverhältnisse bei Bohlwerken. Berlin: Wilhelm Ernst & Sohn, 1931."),
    (&["Boussinesq"], "BOUSSINESQ, J. Application des potentiels à l'étude de l'équilibre et du mouvement des solides élastiques. Paris: Gauthier-Villars, 1885."),
    (&["Clouterre"], "CLOUTERRE. Recommendations Clouterre 1991: soil nailing recommendations for designing, calculating, constructing and inspecting earth support systems using soil nailing. Washington, DC: Federal Highway Administration, 1993. (FHWA-SA-93-026)."),
    (&["Coulomb"], "COULOMB, C. A. Essai sur une application des règles de maximis et minimis à quelques problèmes de statique relatifs à l'architecture. Mémoires de Mathématique et de Physique présentés à l'Académie Royale des Sciences, Paris, v. 7, p. 343-382, 1776."),
    (&["Godoy"], "GODOY, N. S. Fundações: notas de aula do curso de graduação. São Carlos: Escola de Engenharia de São Carlos, Universidade de São Paulo, 1972."),
    (&["Maccaferri"], "MACCAFERRI DO BRASIL. Obras de contenção: manual técnico. Jundiaí: Maccaferri do Brasil, [s. d.]."),
    (&["Marchetti"], "MARCHETTI, O. Muros de arrimo. São Paulo: Blucher, 2007."),
    (&["Meyerhof"], "MEYERHOF, G. G. Some recent research on the bearing capacity of foundations. Canadian Geotechnical Journal, Ottawa, v. 1, n. 1, p. 16-26, 1963."),
    (&["Mononobe"], "MONONOBE, N.; MATSUO, H. On the determination of earth pressures during earthquakes. In: WORLD ENGINEERING CONGRESS, 1929, Tokyo. Proceedings [...]. Tokyo, 1929. v. 9, p. 177-185."),
    (&["Mononobe", "Okabe"], "OKABE, S. General theory of earth pressure. Journal of the Japanese Society of Civil Engineers, Tokyo, v. 12, n. 1, 1926."),
    (&["Rankine"], "RANKINE, W. J. M. On the stability of loose earth. Philosophical Transactions of the Royal Society of London, London, v. 147, p. 9-27, 1857."),
    (&["Teixeira"], "TEIXEIRA, A. H. Projeto e execução de fundações. In: SEMINÁRIO DE ENGENHARIA DE FUNDAÇÕES ESPECIAIS E GEOTECNIA, 3., 1996, São Paulo. Anais [...]. São Paulo: ABMS/ABEF, 1996. v. 1, p. 33-50."),
    (&["Terzaghi"], "TERZAGHI, K.; PECK, R. B.; MESRI, G. Soil mechanics in engineering practice. 3. ed. New York: John Wiley & Sons, 1996."),
    (&["Vesic"], "VESIC, A. S. Analysis of ultimate loads of shallow foundations. Journal of the Soil Mechanics and Foundations Division, New York, v. 99, n. SM1, p. 45-73, 1973."),
    (&["suporte livre", "Suporte livre", "free earth"], "PADFIELD, C. J.; MAIR, R. J. Design of retaining walls embedded in stiff clay. London: CIRIA, 1984. (CIRIA Report 104)."),
    (&["VPA", "VANIL"], "VPA INFRA. Estaca prancha metálica; Estaca prancha VANIL® (PVC): catálogo técnico — tabela de modelos. [S. l.]: VPA Infra, [s. d.]. Disponível em: https://abms.com.br/assets/upload/associado/corporativo/portfolio/27_2.pdf."),
];

/// Todo o texto do memorial (blocos, verificações e avisos).
fn texto(r: &Resultado) -> String {
    let mut s = String::new();
    for b in &r.blocos {
        match b {
            Bloco::Titulo(t) | Bloco::Texto(t) => s += t,
            Bloco::Formula(f) => {
                s += &f.titulo;
                s += " ";
                s += &f.equacao;
                s += " ";
                s += &f.resultado;
            }
            Bloco::Norma(n) => {
                s += n.norma;
                s += " ";
                s += n.requisito;
            }
            Bloco::Verif(_) => {}
        }
        s.push('\n');
    }
    for v in &r.verificacoes {
        s += &v.nome;
        s.push('\n');
    }
    for a in &r.avisos {
        s += a;
        s.push('\n');
    }
    s
}

/// Normas citadas: número → ano (o primeiro ano explícito encontrado, ou o padrão).
fn normas_citadas(txt: &str) -> Vec<(String, String)> {
    let mut achadas: Vec<(String, Option<String>)> = Vec::new();
    let mut resto = txt;
    while let Some(i) = resto.find("NBR ") {
        resto = &resto[i + 4..];
        let num: String = resto.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
        let num = num.trim_end_matches('-').to_string();
        if num.is_empty() {
            continue;
        }
        let depois = &resto[num.len()..];
        let ano = depois
            .strip_prefix(':')
            .map(|d| d.chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
            .filter(|a| a.len() == 4);
        match achadas.iter_mut().find(|(n, _)| *n == num) {
            Some((_, a)) if a.is_none() => *a = ano,
            Some(_) => {}
            None => achadas.push((num, ano)),
        }
    }
    achadas
        .into_iter()
        .filter_map(|(n, a)| {
            let (_, _, padrao) = NORMAS.iter().find(|(k, _, _)| *k == n)?;
            Some((n, a.unwrap_or_else(|| padrao.to_string())))
        })
        .collect()
}

/// Lista de referências (normas ABNT primeiro, depois as obras técnicas, em ordem alfabética).
pub fn gerar(r: &Resultado) -> Vec<String> {
    let txt = texto(r);
    let mut normas = normas_citadas(&txt);
    // a NBR 6120 (pesos específicos) e a NBR 6484 (SPT) fundamentam os dados de entrada de todo memorial
    for n in ["6120", "6484"] {
        if !normas.iter().any(|(k, _)| k == n) {
            let (_, _, a) = NORMAS.iter().find(|(k, _, _)| *k == n).unwrap();
            normas.push((n.to_string(), a.to_string()));
        }
    }
    let chave = |n: &str| -> (u32, u32) {
        let mut it = n.split('-');
        (it.next().and_then(|x| x.parse().ok()).unwrap_or(0), it.next().and_then(|x| x.parse().ok()).unwrap_or(0))
    };
    normas.sort_by_key(|(n, _)| chave(n));
    let mut v: Vec<String> = normas
        .iter()
        .map(|(n, ano)| {
            let (_, titulo, _) = NORMAS.iter().find(|(k, _, _)| k == n).unwrap();
            format!("ASSOCIAÇÃO BRASILEIRA DE NORMAS TÉCNICAS. NBR {n}: {titulo}. Rio de Janeiro: ABNT, {ano}.")
        })
        .collect();
    let mut obras: Vec<&str> = OBRAS.iter().filter(|(chaves, _)| chaves.iter().any(|k| txt.contains(k))).map(|(_, r)| *r).collect();
    obras.sort();
    obras.dedup();
    v.extend(obras.into_iter().map(String::from));
    v
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::projeto::{Projeto, TipoContencao, TIPOS_CONTENCAO};

    #[test]
    fn normas_com_ano_e_padrao() {
        let n = normas_citadas("conforme ABNT NBR 11682:2009, Tabela 4; NBR 6122 7.6.3; NBR 16920-2 9.8; NBR 9999");
        assert_eq!(n, vec![("11682".into(), "2009".into()), ("6122".into(), "2019".into()), ("16920-2".into(), "2021".into())]);
    }

    #[test]
    fn todo_memorial_termina_com_referencias() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            let r = p.calcular();
            assert!(!r.referencias.is_empty(), "{}", tipo.nome());
            assert!(r.referencias.iter().any(|x| x.contains("NBR 11682: Estabilidade de encostas")), "{}", tipo.nome());
            assert!(r.referencias.iter().any(|x| x.starts_with("BISHOP")), "{}: estabilidade global por Bishop", tipo.nome());
            // normas antes das obras, sem repetição
            let primeira_obra = r.referencias.iter().position(|x| !x.starts_with("ASSOCIAÇÃO")).unwrap_or(r.referencias.len());
            assert!(r.referencias[primeira_obra..].iter().all(|x| !x.starts_with("ASSOCIAÇÃO")));
            let mut u = r.referencias.clone();
            u.dedup();
            assert_eq!(u.len(), r.referencias.len());
            if tipo == TipoContencao::Prancha {
                assert!(r.referencias.iter().any(|x| x.starts_with("VPA INFRA")), "catálogo VPA");
                assert!(r.referencias.iter().any(|x| x.contains("NBR 8800")));
            }
            if tipo == TipoContencao::Grampeado {
                assert!(r.referencias.iter().any(|x| x.contains("NBR 16920-2")));
            }
        }
    }
}
