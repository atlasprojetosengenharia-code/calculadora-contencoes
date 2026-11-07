//! Referências normativas exibidas nas caixas do memorial de cálculo.
//!
//! Os textos são SÍNTESES dos requisitos (não transcrições literais das
//! normas ABNT, que são protegidas por direitos autorais). Os números de item
//! seguem as referências já usadas nos módulos de cálculo; confira sempre o
//! texto integral da edição vigente de cada norma.

use crate::resultado::RefNorma;

const fn r(norma: &'static str, item: &'static str, requisito: &'static str) -> RefNorma {
    RefNorma { norma, item, requisito }
}

// --- ABNT NBR 11682:2009 — Estabilidade de encostas -------------------------

pub const NBR11682_TAB3: RefNorma = r(
    "ABNT NBR 11682:2009",
    "Tabela 3",
    "Fator de segurança mínimo de estabilidade em função dos níveis de segurança contra danos a vidas humanas \
     e contra danos materiais e ambientais: de 1,2 (baixo/baixo) a 1,5 (nível alto). Em caso de grande \
     variabilidade dos resultados geotécnicos, majorar os valores em 10 % (Nota 1).",
);
pub const NBR11682_TOMBAMENTO: RefNorma = r(
    "ABNT NBR 11682:2009",
    "Tabela 4",
    "Muros de contenção — fator de segurança mínimo ao tombamento: 2,0.",
);
pub const NBR11682_DESLIZAMENTO: RefNorma = r(
    "ABNT NBR 11682:2009",
    "Tabela 4",
    "Muros de contenção — fator de segurança mínimo ao deslizamento na base: 1,5.",
);
pub const NBR11682_CAPACIDADE: RefNorma = r(
    "ABNT NBR 11682:2009",
    "Tabela 4 (e Nota)",
    "Muros de contenção — fator de segurança mínimo à capacidade de carga da fundação: 3,0, ou os critérios \
     da ABNT NBR 6122.",
);
pub const NBR11682_TERCO_CENTRAL: RefNorma = r(
    "ABNT NBR 11682:2009",
    "7.3.3.1 e 7.3.3.2",
    "Muros de gravidade e de flexão: a linha de ação da resultante dos esforços envolvidos deve interceptar o \
     terço central da base (e ≤ B/6); casos contrários devem ser justificados.",
);
pub const NBR11682_SOBRECARGA: RefNorma = r(
    "ABNT NBR 11682:2009",
    "7.3.3",
    "Considerar no terrapleno uma sobrecarga acidental mínima de 20 kPa; valores menores devem ser justificados.",
);

// --- ABNT NBR 6122:2019 — Fundações ----------------------------------------

pub const NBR6122_FS_GLOBAL: RefNorma = r(
    "ABNT NBR 6122:2019",
    "Tabela 1",
    "Fundações superficiais — fator de segurança global mínimo de 3,0 sobre a capacidade de carga obtida por \
     métodos analíticos.",
);
pub const NBR6122_AREA_COMPRIMIDA: RefNorma = r(
    "ABNT NBR 6122:2019",
    "7.6.2",
    "Fundação rasa sob carga excêntrica, solo sem resistência à tração: área comprimida de no mínimo 2/3 da \
     área total com as solicitações características (ou 50 % com as de cálculo), e tensão máxima de borda \
     atendendo à Seção 6.",
);
pub const NBR6122_PASSIVO: RefNorma = r(
    "ABNT NBR 6122:2019",
    "7.6.3",
    "Além do atrito solo-fundação, pode-se contar com o empuxo passivo, desde que o solo não possa ser removido \
     durante a vida útil; o valor calculado deve ser reduzido por coeficiente de no mínimo 2,0.",
);

// --- ABNT NBR 5629:2018 — Tirantes ancorados no terreno ---------------------

pub const NBR5629_ACO: RefNorma = r(
    "ABNT NBR 5629:2018",
    "4.5.2.1",
    "Carga de trabalho do tirante limitada pela resistência do aço: Ft ≤ 0,9·fyk·As / FS, com FS = 1,75 \
     (tirante permanente) ou 1,50 (provisório).",
);
pub const NBR5629_BULBO: RefNorma = r(
    "ABNT NBR 5629:2018",
    "B.2.4.1",
    "Comprimento do bulbo dimensionado com fator de segurança 1,75 (permanente) ou 1,50 (provisório) sobre a \
     resistência ao arrancamento no contato bulbo-solo.",
);
pub const NBR5629_TRECHO_LIVRE: RefNorma = r(
    "ABNT NBR 5629:2018",
    "B.2.7",
    "Comprimento livre mínimo do tirante: 3,0 m (fixação por rosca) ou 5,0 m (fixação por clavetes).",
);
pub const NBR5629_ENSAIOS: RefNorma = r(
    "ABNT NBR 5629:2018",
    "Tabela D.1 e D.3.4",
    "Ensaios de recebimento: tipos A (≥ 10 % dos tirantes) e B até 1,75·Ft e 1,40·Ft (permanentes); tipos C e D \
     até 1,50·Ft e 1,20·Ft (provisórios). A carga máxima de ensaio não pode exceder 0,9·fyk·As.",
);

// --- ABNT NBR 16920-2:2021 — Solo grampeado -------------------------------

pub const NBR16920_ESTABILIDADE: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "Anexo B (Tabela B.3)",
    "Fator de segurança de estabilidade do maciço grampeado conforme os níveis de segurança da ABNT NBR 11682 \
     (Tabela 3).",
);
pub const NBR16920_BARRA: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "9.1",
    "Resistência de cálculo da armação do grampo: Rtd = As·fyk / γm, com γm = 1,15.",
);
pub const NBR16920_INTERNA: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "9.8",
    "Estabilidade interna: fator de segurança mínimo de 1,5 ao arrancamento e à ruptura por tração dos grampos.",
);

// --- ABNT NBR 6118:2023 — Estruturas de concreto ---------------------------

pub const NBR6118_FLEXAO: RefNorma = r(
    "ABNT NBR 6118:2023",
    "17.2.2 e 14.6.4.3",
    "Flexão no ELU com diagrama retangular (y = 0,8·x; σc = 0,85·ηc·fcd). Ductilidade: x/d ≤ 0,45 para \
     fck ≤ 50 MPa.",
);
pub const NBR6118_AS_MIN: RefNorma = r(
    "ABNT NBR 6118:2023",
    "Tabela 17.3",
    "Taxa mínima de armadura de flexão ρmin = As,min/Ac: 0,150 % até C30, crescendo até 0,208 % em C50.",
);
pub const NBR6118_CORTANTE: RefNorma = r(
    "ABNT NBR 6118:2023",
    "19.4.1",
    "Lajes sem armadura transversal: VSd ≤ VRd1 = τRd·k·(1,2 + 40·ρ1)·bw·d, com τRd = 0,25·fctd e \
     k = |1,6 − d| ≥ 1 (d em metros).",
);
pub const NBR6118_PUNCAO: RefNorma = r(
    "ABNT NBR 6118:2023",
    "19.5.3.2",
    "Punção no contorno C' (a 2d da face da área carregada): τSd ≤ τRd1 = 0,13·(1 + √(20/d))·(100·ρ·fck)^(1/3), \
     d em centímetros.",
);
pub const NBR6118_ARM_LONGITUDINAL: RefNorma = r(
    "ABNT NBR 6118:2023",
    "17.3.5.3",
    "Armadura longitudinal de peças comprimidas/fletidas de seção cheia: mínimo de 0,4 % de Ac; máximo de 8 % \
     de Ac incluindo a região de emendas (4 % fora dela).",
);

// --- ABNT NBR 8800:2008 e NBR 6120:2019 ------------------------------------

pub const NBR8800_FLEXAO: RefNorma = r(
    "ABNT NBR 8800:2008",
    "γa1 = 1,10",
    "Resistência de cálculo à flexão em regime elástico: MRd = W·fy / γa1. Verificar também a classe da seção \
     (flambagem local) e os deslocamentos.",
);
pub const NBR6120_PESOS: RefNorma = r(
    "ABNT NBR 6120:2019",
    "Tabela 1",
    "Pesos específicos aparentes dos materiais de construção; para materiais compostos, adotado o limite \
     inferior dos constituintes (a favor da segurança).",
);

// --- Drenagem ---------------------------------------------------------------

pub const NBR11682_DRENAGEM: RefNorma = r(
    "ABNT NBR 11682:2009",
    "7.3.1 e)",
    "O projeto deve detalhar os elementos de drenagem (dimensões, materiais, declividades, selagem e pontos de \
     saída d'água) e a sua compatibilidade com a solução de estabilização.",
);
pub const NBR11682_MANUTENCAO: RefNorma = r(
    "ABNT NBR 11682:2009",
    "Seção 10",
    "Obras de estabilização exigem plano de manutenção, incluindo inspeção e limpeza periódica dos dispositivos \
     de drenagem; sem ela, a hipótese de maciço drenado deixa de valer.",
);
pub const NBR16920_DRENAGEM: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "9.7.1 e Seção 12",
    "Contenções sem drenagem interna eficiente devem considerar a pressão d'água no dimensionamento.",
);

// --- Alvenarias (muro de gravidade) ----------------------------------------

pub const NBR6118_CONCRETO_SIMPLES: RefNorma = r(
    "ABNT NBR 6118:2023",
    "24.3, 24.4 e 24.5.2",
    "Concreto simples estrutural C15 a C40; γc = 1,2 × 1,4 = 1,68; σcRd = 0,85·fcd ; σctRd = 0,85·fctd ;      τwRd = 0,30·fctd na flexão simples e 0,30·fctd·(1 + 3·σcmd/fck) na flexo-compressão, com o fator ≤ 2.      Juntas de dilatação pelo menos a cada 15 m.",
);
pub const NBR16868_JUNTAS: RefNorma = r(
    "ABNT NBR 16868-1:2020",
    "Tabelas 2 e 4 (por analogia)",
    "Resistência característica ao cisalhamento em juntas horizontais fvk = f0 + 0,5·σ (f0 = 0,10 / 0,15 / 0,35 MPa      conforme a argamassa; σ = 0,9 × compressão permanente), com γm = 2,0. A norma trata de blocos e tijolos: para      pedra argamassada, uso por analogia.",
);
pub const MACCAFERRI_GABIAO: RefNorma = r(
    "Manual técnico Maccaferri",
    "Muros de gabiões",
    "Tensão admissível à compressão σadm = 50·γg − 30 (tf/m²) e ângulo de atrito entre camadas φ* = 25·γg − 10 (°),      com γg em tf/m³; o gabião não resiste à tração. Referência técnica, não norma ABNT.",
);

// --- Detalhamento do aço ------------------------------------------------------

pub const NBR6118_ANCORAGEM: RefNorma = r(
    "ABNT NBR 6118:2023",
    "9.4.2 e 9.5.2",
    "Ancoragem por aderência: lb = (Ø/4)·(fyd/fbd) ≥ 25Ø, fbd = η1·η2·η3·fctd (η1 = 2,25 barras nervuradas;      η2 = 1,0 boa aderência / 0,7 má aderência); lb,nec = α·lb·As,calc/As,ef ≥ lb,mín. Traspasse de barras      tracionadas l0t = α0t·lb,nec.",
);
pub const NBR6118_DISTRIBUICAO: RefNorma = r(
    "ABNT NBR 6118:2023",
    "Tabela 19.1 e 20.1",
    "Armadura secundária (distribuição) ≥ 20 % da principal, ≥ 0,9 cm²/m e ≥ 0,5·ρmín. Espaçamento da      armadura principal ≤ 2h e ≤ 20 cm; da secundária ≤ 33 cm.",
);

// --- Detalhamento por tipo de contenção --------------------------------------

pub const NBR5629_INICIO_BULBO: RefNorma = r(
    "ABNT NBR 5629:2018",
    "5.5.1 a)",
    "Em nenhum caso o início do bulbo deve distar menos de 3,0 m da superfície do terreno de início da perfuração \
     ou da face interna do paramento.",
);
pub const NBR6118_PILAR_DET: RefNorma = r(
    "ABNT NBR 6118:2023",
    "18.4.2 e 18.4.3",
    "Barras longitudinais com Ø ≥ 10 mm e ≤ 1/8 da menor dimensão; seção circular com no mínimo 6 barras; \
     espaçamento entre eixos ≤ 2× a menor dimensão e ≤ 400 mm, livre ≥ 20 mm. Estribos com Ø ≥ 5 mm e ≥ Ø/4, \
     espaçamento ≤ 200 mm, ≤ menor dimensão e ≤ 12Ø (CA-50).",
);
pub const NBR16920_PROTECAO: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "10.2.3 e 10.3",
    "Armação do grampo com cobrimento de calda ≥ 20 mm (inclusive emendas) e espaçadores a cada 2 m no máximo; \
     calda de preenchimento com a/c ≤ 0,5 e ≥ 25 MPa. Cabeça protegida até 30 cm para dentro do terreno, com \
     cobrimento ≥ 30 mm.",
);
pub const NBR16920_PARAMENTO: RefNorma = r(
    "ABNT NBR 16920-2:2021",
    "9.3",
    "Paramento de concreto com função estrutural dimensionado pela ABNT NBR 6118, verificando punção, \
     cisalhamento e flexão; para tela metálica estrutural, FS global mínimo de 1,5 na conexão grampo-placa-tela.",
);
