//! Identificação do software — edite aqui nome do criador, site e ano.

pub const NOME: &str = "Calculadora de Contenções";
pub const SUBTITULO: &str = "Software de Análise e Dimensionamento de Contenções";
pub const VERSAO: &str = env!("CARGO_PKG_VERSION");
/// Número do build, injetado pelo GitHub Actions (variável BUILD_NUMBER).
pub const BUILD: Option<&str> = option_env!("BUILD_NUMBER");
/// Repositório "usuario/nome" usado para verificar atualizações.
pub const REPOSITORIO: &str = "atlasprojetosengenharia-code/calculadora-contencoes-releases";

pub fn versao_completa() -> String {
    match BUILD {
        Some(b) => format!("{VERSAO} (build {b})"),
        None => format!("{VERSAO} (local)"),
    }
}
/// Autoria do software.
pub const CRIADOR: &str = "Eng. Civil Kaique Ferreira";
pub const EMPRESA: &str = "Atlas Projetos e Engenharia LTDA";
pub const SITE: &str = "https://www.atlasprojetos.eng.br";
pub const ANO: &str = "2026";

/// Endereço do site sem o protocolo (para exibição).
pub fn site_curto() -> &'static str {
    SITE.trim_start_matches("https://")
}

pub const NORMAS: [&str; 6] = [
    "ABNT NBR 11682:2009",
    "ABNT NBR 6122:2019",
    "ABNT NBR 5629:2018",
    "ABNT NBR 16920-2:2021",
    "ABNT NBR 6118:2023",
    "ABNT NBR 8800",
];

pub fn copyright() -> String {
    format!("Copyright © {ANO} {EMPRESA}. Todos os direitos reservados.")
}
