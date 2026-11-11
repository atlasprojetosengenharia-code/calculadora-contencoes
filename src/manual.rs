//! Manual do usuário em PDF, gerado pelo próprio programa (sem arquivos
//! externos): capa, sumário, passo a passo de uso e interpretação dos
//! resultados.

use crate::pdf::{self, Cor, Pagina, ALTURA, AZUL, BRANCO, CINZA, CLARO, LARGURA, PRETO};
use crate::sobre;

/// Blocos de conteúdo do manual.
enum B {
    /// Capítulo (inicia nova página e entra no sumário).
    Cap(&'static str),
    /// Seção dentro do capítulo.
    Sec(&'static str),
    /// Parágrafo.
    P(&'static str),
    /// Item com marcador.
    I(&'static str),
    /// Passo numerado.
    N(u32, &'static str),
    /// Nota destacada em caixa.
    Nota(&'static str),
    /// Linha de formato/código (monoespaçado não disponível: recuo e cor).
    C(&'static str),
}
use B::*;

fn conteudo() -> Vec<B> {
    vec![
        Cap("1. Apresentação"),
        P("A Calculadora de Contenções analisa e dimensiona estruturas de contenção de solo a partir de uma sondagem SPT, \
           com memorial de cálculo passo a passo, verificações normativas, detalhamento das armaduras e exportação \
           do desenho para CAD."),
        Sec("Tipos de contenção"),
        I("Muro de gravidade — concreto simples, concreto ciclópico, pedra argamassada, gabião e pedra seca."),
        I("Muro de flexão — concreto armado (parede, ponta e talão)."),
        I("Cortina atirantada — paramento de concreto com tirantes (NBR 5629)."),
        I("Estaca-prancha — perfis metálicos em balanço ou apoiados."),
        I("Parede de estacas — estacas de concreto armado justapostas ou espaçadas."),
        I("Solo grampeado — grampos, tela e concreto projetado (NBR 16920-2)."),
        Sec("Normas adotadas"),
        I("ABNT NBR 11682:2009 — Estabilidade de encostas (muros, fatores de segurança, sobrecarga mínima)."),
        I("ABNT NBR 6122:2019 — Projeto e execução de fundações (capacidade de carga, área comprimida, passivo)."),
        I("ABNT NBR 6118:2023 — Projeto de estruturas de concreto (flexão, cisalhamento, ancoragem, concreto simples)."),
        I("ABNT NBR 5629:2018 — Tirantes ancorados no terreno."),
        I("ABNT NBR 16920-2:2021 — Muros e taludes em solos reforçados: solos grampeados."),
        Nota("O programa é uma ferramenta de apoio ao projeto. Os resultados devem ser conferidos e assumidos por \
              engenheiro legalmente habilitado, com a respectiva ART. Parâmetros de solo obtidos por correlação \
              com o SPT são estimativas e devem ser confirmados por ensaios."),

        Cap("2. Primeiros passos"),
        Sec("Tela inicial"),
        I("Iniciar novo projeto — abre um projeto em branco na etapa 1 (Sondagem SPT)."),
        I("Carregar projeto existente — abre um arquivo .ctc salvo anteriormente."),
        I("Continuar último projeto — retoma a sessão anterior, quando houver."),
        I("Manual do usuário (PDF) — abre este documento."),
        I("Central de normas — referências das normas ABNT usadas nos cálculos."),
        P("Quando há uma versão mais nova publicada, a tela inicial mostra um aviso com o link para download."),
        Sec("Ambiente de projeto"),
        P("A barra superior traz o nome do projeto e os comandos Novo, Abrir, Salvar, Salvar como e Início, além da \
           troca entre tema claro e escuro. A navegação lateral organiza o trabalho em etapas:"),
        N(1, "Sondagem SPT — camadas, nível d'água e parâmetros do solo."),
        N(2, "Contenção e geometria — tipo, dimensões, drenagem e cargas, com pré-visualização em tempo real."),
        N(3, "Resultados — verificações, seção transversal, tabela de armação, memorial e exportações."),
        I("Normas e fatores (ícone de engrenagem) — coeficientes de segurança e critérios normativos editáveis."),
        P("Ainda na lateral, preencha Local / obra e Responsável técnico: esses dados entram no cabeçalho do memorial. \
           A barra inferior mostra mensagens, o arquivo aberto, a versão e os direitos autorais."),
        Sec("Arquivos de projeto"),
        P("Os projetos são salvos com extensão .ctc (texto JSON). Projetos de versões anteriores abrem normalmente; \
           campos novos recebem os valores padrão."),

        Cap("3. Sondagem SPT"),
        Sec("Modo de entrada do solo"),
        I("SPT completo — perfil estratificado, camada a camada. Use quando houver boletim de sondagem."),
        I("Entrada simplificada na base — informe o NSPT representativo e o tipo de solo predominante; γ, φ' e c' são \
           estimados por correlação e o mesmo solo é usado no maciço contido e na fundação (solo homogêneo)."),
        Sec("Digitando as camadas"),
        N(1, "Clique em “+ Camada” para cada estrato."),
        N(2, "Preencha Até (m) — profundidade final da camada (a inicial é a final da camada anterior)."),
        N(3, "Informe o NSPT e a descrição do solo (ex.: Areia fina argilosa). Descrição vazia repete a anterior."),
        N(4, "γ, c' e φ' aparecem em modo Auto (correlação). Clique no indicador para alternar para Manual e digitar \
              valores de ensaio."),
        N(5, "Em “Nível d'água e correlações”, informe o NA e escolha as correlações."),
        P("“⇊ Agrupar camadas iguais” une camadas consecutivas de mesma descrição; “🗑 Limpar” remove todas."),
        Sec("Importando de arquivo (CSV / TXT)"),
        P("Use “📂 Importar CSV / TXT”. Uma linha por registro, separador ponto e vírgula (aceita vírgula decimal), \
           tabulação ou vírgula. Formatos aceitos:"),
        C("prof. inicial ; prof. final ; NSPT ; descrição"),
        C("profundidade ; NSPT ; descrição        (ensaio metro a metro)"),
        C("NA ; 3,50                             (nível d'água)"),
        P("Cabeçalhos e linhas iniciadas por # são ignorados."),
        Sec("Correlações disponíveis"),
        I("Peso específico γ — Godoy (1972), em função do tipo de solo e do NSPT."),
        I("φ' de solos granulares — Teixeira (1996): φ' = √(20·N) + 15 ; ou Godoy (1983): φ' = 28 + 0,4·N."),
        I("Solos coesivos — Drenado (longo prazo), com c' e φ' adotados ; ou Não drenado: Su = 10·N e φ = 0."),
        Nota("Correlações são estimativas. Confirme os parâmetros com ensaios de laboratório ou de campo \
              (NBR 11682, Anexo D), principalmente em obras de maior porte."),

        Cap("4. Contenção e geometria"),
        Sec("Geometria essencial"),
        N(1, "Escolha o tipo de contenção no seletor do topo."),
        N(2, "Informe a altura contida H e a inclinação do talude de montante β."),
        N(3, "Informe a sobrecarga na crista q. A NBR 11682 (7.3.3) exige no mínimo 20 kPa; abaixo disso o programa \
              mostra um alerta."),
        N(4, "Informe a profundidade de engastamento (muros) ou acompanhe a ficha calculada (contenções embutidas)."),
        P("O cartão de pré-visualização desenha a seção e mostra o selo ATENDE / NÃO ATENDE em tempo real; \
           “Ver resultados completos” leva à etapa 3."),
        Sec("Muro de gravidade"),
        I("Material — define o peso específico e a origem das tensões admissíveis."),
        I("Dimensões A a F — ressalto frontal, projeção do paramento frontal, largura da crista, projeção do tardoz, \
           ressalto posterior e altura da base (croqui ao lado dos campos)."),
        I("Alvenaria — tensões admissíveis: fck do concreto (C15 a C40), classe da argamassa ou valores do projetista."),
        I("Dente de base (chave de cisalhamento), empuxo passivo frontal e subpressão na base."),
        I("Detalhamento construtivo — espaçamento das juntas de dilatação e viga de coroamento."),
        Sec("Muro de flexão"),
        I("Espessura da parede no topo e na base, espessura da sapata, comprimento da ponta e do talão."),
        I("Concreto e aço (NBR 6118:2023) — fck, fyk e cobrimento."),
        I("Dente de base, passivo frontal e subpressão, como no muro de gravidade."),
        Sec("Cortina atirantada"),
        I("Espessura da cortina, níveis de tirantes, espaçamento horizontal, inclinação e diâmetro do furo."),
        I("qs (atrito lateral do bulbo), fyk e área de aço do tirante, placa de ancoragem e tipo de tirante \
           (permanente ou provisório, rosca ou clavete) — critérios da NBR 5629."),
        Sec("Estaca-prancha e parede de estacas"),
        I("Condição de apoio (balanço ou apoiada), profundidade e inclinação do apoio, δ no lado passivo."),
        I("Estaca-prancha: perfil escolhido no catálogo VPA (aço U, Z e W perfilados a frio, com W, inércia e peso; ou PVC VANIL®, \
           verificado pelo momento admissível do fabricante), aço do perfil (fy) e acréscimo construtivo de ficha. \
           Parede de estacas: diâmetro e espaçamento eixo a eixo."),
        I("Otimizar — avalia as alternativas usuais do tipo de contenção (base, dente, embutimento, níveis de tirantes, \
           malha e barras dos grampos, apoio, ficha e perfil), ajusta espessuras e placas e aplica a mais econômica que \
           atende a todas as verificações, inclusive a estabilidade global. Roda em segundo plano."),
        I("Altura da viga de travamento / chapéu e NA no lado escavado, quando houver."),
        Sec("Solo grampeado"),
        I("Inclinação da face, espaçamentos horizontal Sh e vertical Sv, profundidade do 1º grampo."),
        I("Comprimento L, inclinação abaixo da horizontal, diâmetro do furo, qs de arrancamento, diâmetro e fyk da barra."),
        I("Espessura da face em concreto projetado e placa da cabeça."),
        Sec("Drenagem, cargas e empuxo"),
        I("Drenagem — barbacãs (diâmetro, espaçamentos, altura da 1ª fila, declividade), filtro no tardoz, dreno de pé, \
           canaleta de crista e DHP. A condição de drenagem altera o cálculo: 'drenada' despreza a água (e exige camada            drenante + barbacãs, ou DHP); 'parcial' e 'sem drenagem' adotam uma lâmina d'água (fração de H) que entra no            empuxo hidrostático, no peso submerso do solo, na subpressão sob a base e, no solo grampeado, na poropressão do            plano de ruptura (NBR 16920-2, 12)."),
        I("Reaterro do tardoz (muros) — tipo de solo e NSPT equivalente do material compactado, grau de compactação mínimo            (Proctor Normal) e camadas de 20 cm; o memorial traz o SPT do solo de apoio e do tardoz. Materiais da drenagem:            areia (grossa ou média), brita nº 1, 2 ou 3 e gramatura do geotêxtil."),
        I("Dimensionamento pela tração — tirantes (barra ST 85/105 ou cordoalhas) e grampos (barra CA-50 e comprimento) são            dimensionados pela carga de cada elemento; desligue a opção para informar o aço manualmente."),
        I("Cargas adicionais — carga em faixa (Boussinesq) e ação sísmica (Mononobe-Okabe, kh e kv)."),
        I("Teoria de empuxo — Rankine ou Coulomb (com atrito solo-estrutura δ)."),
        I("Contato base-solo de fundação — atrito tan(k·φ') ou k·tg φ' (Marchetti), com predefinições ⅔φ, 0,67·tg φ \
           e 0,9·tg φ, e adesão como fração de c'."),

        Cap("5. Normas e fatores"),
        P("A etapa Normas e fatores reúne os coeficientes usados nas verificações. Os valores padrão seguem as normas; altere-os \
           apenas com justificativa técnica."),
        Sec("Estabilidade — NBR 11682 Tabela 3"),
        P("Escolha os níveis de segurança contra perda de vidas humanas e contra danos materiais e ambientais; o FS \
           mínimo de estabilidade é obtido da tabela (com acréscimo de 10 % para grande variabilidade dos ensaios, \
           se marcado)."),
        Sec("Muros — NBR 11682 Tabela 4"),
        I("FS deslizamento 1,5 ; FS tombamento 2,0 ; FS capacidade de carga 3,0 (NBR 6122)."),
        I("Excentricidade máxima B/6 (terço central, NBR 11682 7.3.3.1) e área comprimida mínima 2/3 (NBR 6122 7.6.2)."),
        I("Sobrecarga mínima de 20 kPa (NBR 11682 7.3.3)."),
        Sec("Exceções justificadas (desligadas por padrão)"),
        I("Terço central só com ações permanentes (sem sobrecarga)."),
        I("Critério prático (Marchetti 3.3): aceitar e > B/6 com redistribuição de tensões sem tração (σmáx = 2N/3c), \
           mantidas a área comprimida ≥ 2/3 e a capacidade de carga."),
        I("Muro de flexão: sobrecarga sobre o talão como força estabilizante (Marchetti 4.2)."),
        Nota("Ao ligar uma exceção, o memorial passa a registrar um aviso. Documente a justificativa técnica no projeto."),
        Sec("Demais grupos"),
        I("Caso sísmico, contenções embutidas (redução do passivo, acréscimo de ficha), tirantes (NBR 5629), \
           grampos (NBR 16920-2) e coeficientes γf, γc e γs (NBR 6118)."),

        Cap("6. Interpretando os resultados"),
        Sec("Visão geral"),
        P("O topo da etapa 3 mostra “ATENDE A TODAS AS VERIFICAÇÕES” ou “NÃO ATENDE — n verificação(ões)”. Abaixo \
           ficam a seção transversal, a lista de verificações, a tabela de armação, os avisos e o memorial completo."),
        Sec("Status das verificações"),
        I("ATENDIDO — o valor calculado cumpre o limite (ex.: “1,65 ≥ 1,50”)."),
        I("NÃO ATENDIDO — o limite não é cumprido; ajuste a geometria ou os parâmetros."),
        I("INFORMATIVO — valor apresentado sem critério de aceitação (ex.: excentricidade com o critério prático)."),
        Sec("Estabilidade externa de muros"),
        I("Tombamento — FS = ΣM estabilizante / ΣM instabilizante em torno da ponta. Se falhar: aumente a base ou a \
           ponta, ou desloque o peso para trás."),
        I("Deslizamento — FS = (N·tan δb + adesão·B + Ep) / ΣH. Se falhar: aumente a base, use dente de base, reveja o \
           atrito na base. O memorial informa quanto do empuxo vem da sobrecarga."),
        I("Excentricidade — e = B/2 − x_R; o terço central exige e ≤ B/6."),
        I("Área comprimida — fração da base em compressão; mínimo de 2/3 (NBR 6122)."),
        I("Capacidade de carga — Meyerhof/Vesic com largura efetiva B' = B − 2e e inclinação da carga."),
        Sec("Verificações estruturais"),
        I("Gravidade: compressão, tração e cisalhamento nas seções horizontais do corpo, com as tensões admissíveis do material."),
        I("Concreto armado: flexão (As necessária e mínima), cisalhamento (VRd1) e detalhamento (NBR 6118)."),
        I("Embutidas: ficha, momento máximo e capacidade do perfil ou da estaca."),
        I("Tirantes: carga de trabalho do aço, bulbo e comprimentos livre e ancorado (NBR 5629)."),
        I("Grampos: arrancamento, tração na barra e estabilidade interna (NBR 16920-2)."),
        Sec("Memorial de cálculo"),
        P("Cada verificação aparece com a fórmula, os valores substituídos e o resultado, acompanhada da caixa com a \
           referência normativa (norma e item). Os avisos destacam hipóteses, exceções e pontos que exigem atenção."),
        Nota("A estabilidade global (ruptura geral do talude) ainda não é calculada pelo programa: verifique-a em \
              software de equilíbrio limite, conforme a NBR 11682."),

        Cap("7. Detalhamento e tabela de armação"),
        P("Para cada tipo, a seção “Detalhamento construtivo” do memorial descreve juntas, coroamento, armaduras \
           principais e de distribuição, esperas, viga de travamento, tela e demais elementos. A tabela de armação \
           traz as colunas:"),
        C("Pos. | Elemento | Bitola | s (cm) | Quant. | C (m) | As,nec | As,ef | lb (cm)"),
        I("Bitola e espaçamento s escolhidos para atender As necessária, As mínima e as regras de detalhamento."),
        I("lb — comprimento de ancoragem básico pela NBR 6118 (boa aderência, CA-50); traspasse com α0t = 2,0."),
        I("C — comprimento aproximado de cada barra."),
        Nota("Os comprimentos são aproximados: confira dobras, ganchos, cobrimentos e emendas no projeto executivo \
              antes de liberar a armação para corte e dobra."),

        I("Estabilidade global — verificação por Bishop simplificado (superfícies circulares) com o FS mínimo da Tabela 3 da            NBR 11682, incluindo reaterro, água e a tração dos tirantes e grampos; o arco crítico aparece na seção."),
        I("Detalhes construtivos e barras de aço isoladas — ao lado da seção: barbacã, canaleta, cabeça e seção do grampo,            tirante e cabeça de ancoragem; e cada posição de armação com dobras, ganchos e pinos (NBR 6118, Tabelas 9.1 e 9.2)."),
        Cap("8. Exportações"),
        I("📐 Exportar DXF (AutoCAD) — seção, armaduras, tabela de armação, notas e carimbo em DXF R12 (metros), uma \
           camada por elemento. No AutoCAD, abra o arquivo e use “Salvar como” para gerar o .dwg."),
        I("🌐 Exportar relatório (.html) — relatório visual com seção, fórmulas, normas e status; imprima ou salve em \
           PDF pelo navegador."),
        I("📄 Exportar memorial (.docx) — documento do Word com a seção, fórmulas, normas, verificações e tabela de armação.            A margem superior é folgada e o cabeçalho vem vazio: aplique o papel timbrado do escritório no Word."),
        I("📋 Copiar memorial — texto puro para colar em outro documento."),

        Cap("9. Central de normas"),
        P("A Central de normas (tela inicial ou barra superior) lista as normas usadas nos cálculos, com os itens \
           aplicados pelo programa, links para o Catálogo ABNT e a busca da norma."),
        P("As normas ABNT são protegidas por direitos autorais e não acompanham o programa. Se você possui as normas \
           em PDF, escolha a pasta onde estão: o programa localiza cada arquivo pelo número da norma e oferece o botão \
           para abri-lo."),

        Cap("10. Exemplo passo a passo"),
        P("Roteiro para um muro de flexão de 3 m em solo arenoso:"),
        N(1, "Na tela inicial, clique em “Iniciar novo projeto” e dê um nome ao projeto na barra superior."),
        N(2, "Etapa 1: importe o boletim SPT ou digite as camadas; informe o NA. Confira γ, φ' e c' estimados."),
        N(3, "Etapa 2: selecione Muro de flexão; H = 3,0 m, β = 0°, q = 20 kPa; ajuste o engastamento."),
        N(4, "Comece o pré-dimensionamento com base de 50 % a 70 % de H, parede de 20 cm no topo e 8 % a 10 % de H na \
              base, sapata de 8 % a 10 % de H."),
        N(5, "Configure drenagem (barbacãs e filtro) e o contato base-solo."),
        N(6, "Observe o selo da pré-visualização; ajuste talão e ponta até ATENDE."),
        N(7, "Etapa 3: revise as verificações, os avisos e o memorial; confira a tabela de armação."),
        N(8, "Exporte o DXF para o desenho e o relatório HTML para o memorial; salve o projeto (.ctc)."),

        Cap("11. Limitações e boas práticas"),
        I("Estabilidade global por Bishop simplificado (circular): casos com estratigrafia complexa, sismo ou carga em faixa pedem conferência com métodos rigorosos."),
        I("Parâmetros por correlação com o SPT são estimativas; prefira ensaios."),
        I("Comprimentos de armadura aproximados — detalhar no projeto executivo."),
        I("Critérios não normativos (ex.: inclinação de grampos de 10° a 15°, espaçamento de barbacãs) aparecem no \
           memorial como “critério de projeto”."),
        I("Mantenha o programa atualizado: novas versões corrigem e ampliam as verificações."),

        Cap("12. Créditos e suporte"),
        P("Criação: Eng. Civil Kaique Ferreira."),
        P("Atlas Projetos e Engenharia LTDA — www.atlasprojetos.eng.br"),
        P("Copyright © 2026 Atlas Projetos e Engenharia LTDA. Todos os direitos reservados."),
    ]
}

const MARGEM: f64 = 56.0;
const TOPO: f64 = ALTURA - 78.0;
const BASE: f64 = 70.0;
const CORPO: f64 = 10.5;
const ENTRELINHA: f64 = 14.5;

struct Diagramador {
    paginas: Vec<Pagina>,
    y: f64,
    sumario: Vec<(&'static str, usize)>,
}

impl Diagramador {
    fn pagina(&mut self) -> &mut Pagina {
        self.paginas.last_mut().unwrap()
    }
    fn nova(&mut self) {
        self.paginas.push(Pagina::default());
        self.y = TOPO;
    }
    /// Garante espaço vertical `h`; senão, nova página.
    fn espaco(&mut self, h: f64) {
        if self.y - h < BASE {
            self.nova();
        }
    }
    fn linhas(&mut self, linhas: &[String], x: f64, tamanho: f64, negrito: bool, cor: Cor) {
        for l in linhas {
            self.espaco(ENTRELINHA);
            let y = self.y;
            self.pagina().texto(x, y, tamanho, negrito, cor, l);
            self.y -= ENTRELINHA;
        }
    }
    fn bloco(&mut self, b: &B) {
        let larg = LARGURA - 2.0 * MARGEM;
        match b {
            Cap(t) => {
                self.nova();
                let indice = self.paginas.len();
                self.sumario.push((t, indice));
                let y = self.y;
                let p = self.pagina();
                p.texto(MARGEM, y, 18.0, true, AZUL, t);
                p.linha(MARGEM, y - 8.0, LARGURA - MARGEM, y - 8.0, 1.2, AZUL);
                self.y -= 34.0;
            }
            Sec(t) => {
                self.espaco(ENTRELINHA * 3.0);
                self.y -= 6.0;
                let y = self.y;
                self.pagina().texto(MARGEM, y, 12.5, true, PRETO, t);
                self.y -= ENTRELINHA + 3.0;
            }
            P(t) => {
                let l = pdf::quebrar(t, CORPO, false, larg);
                self.linhas(&l, MARGEM, CORPO, false, PRETO);
                self.y -= 5.0;
            }
            I(t) => {
                let l = pdf::quebrar(t, CORPO, false, larg - 16.0);
                self.espaco(ENTRELINHA);
                let y = self.y;
                self.pagina().texto(MARGEM + 4.0, y, CORPO, true, AZUL, "•");
                self.linhas(&l, MARGEM + 16.0, CORPO, false, PRETO);
                self.y -= 2.0;
            }
            N(n, t) => {
                let l = pdf::quebrar(t, CORPO, false, larg - 22.0);
                self.espaco(ENTRELINHA);
                let y = self.y;
                self.pagina().texto(MARGEM + 2.0, y, CORPO, true, AZUL, &format!("{n}."));
                self.linhas(&l, MARGEM + 22.0, CORPO, false, PRETO);
                self.y -= 2.0;
            }
            C(t) => {
                self.espaco(ENTRELINHA + 4.0);
                let y = self.y;
                let p = self.pagina();
                p.retangulo(MARGEM + 10.0, y - 4.5, larg - 10.0, ENTRELINHA + 1.0, CLARO);
                p.texto(MARGEM + 18.0, y, 9.5, false, AZUL, t);
                self.y -= ENTRELINHA + 4.0;
            }
            Nota(t) => {
                let l = pdf::quebrar(t, 9.8, false, larg - 28.0);
                let h = l.len() as f64 * 13.5 + 14.0;
                self.espaco(h + 6.0);
                let y = self.y + 10.0;
                let p = self.pagina();
                p.retangulo(MARGEM, y - h, larg, h, CLARO);
                p.retangulo(MARGEM, y - h, 3.0, h, AZUL);
                let mut yy = y - 17.0;
                for s in &l {
                    p.texto(MARGEM + 14.0, yy, 9.8, false, PRETO, s);
                    yy -= 13.5;
                }
                self.y -= h + 8.0;
            }
        }
    }
}

/// Gera o manual do usuário (bytes do arquivo PDF).
pub fn gerar() -> Vec<u8> {
    let versao = sobre::versao_completa();

    // 1) corpo (a partir da página 3: capa e sumário vêm antes)
    let mut d = Diagramador { paginas: Vec::new(), y: TOPO, sumario: Vec::new() };
    for b in conteudo() {
        d.bloco(&b);
    }
    let corpo = d.paginas;
    let total = corpo.len() + 2;

    // 2) capa
    let mut capa = Pagina::default();
    let logo = crate::marca::logotipo();
    let simbolo = crate::marca::simbolo();
    capa.retangulo(0.0, ALTURA - 420.0, LARGURA, 420.0, AZUL);
    let lh = 120.0;
    let lw = lh * logo.largura as f64 / logo.altura as f64;
    capa.imagem("Logo", MARGEM, ALTURA - 60.0 - lh, lw, lh);
    capa.texto(MARGEM, ALTURA - 250.0, 13.0, true, BRANCO, "MANUAL DO USUÁRIO");
    capa.texto(MARGEM, ALTURA - 292.0, 30.0, true, BRANCO, sobre::NOME);
    capa.texto(MARGEM, ALTURA - 318.0, 14.0, false, BRANCO, sobre::SUBTITULO);
    capa.texto(MARGEM, ALTURA - 380.0, 11.0, false, BRANCO, &format!("Versão {versao}"));
    let mut y = ALTURA - 470.0;
    for (t, negrito) in [
        ("Tutorial de uso: sondagem SPT, geometria, verificações de estabilidade,", false),
        ("detalhamento das armaduras e exportação para CAD.", false),
    ] {
        capa.texto(MARGEM, y, 12.0, negrito, PRETO, t);
        y -= 18.0;
    }
    capa.texto(MARGEM, 200.0, 10.0, false, CINZA, "Criação");
    capa.texto(MARGEM, 184.0, 13.0, true, PRETO, sobre::CRIADOR);
    capa.texto(MARGEM, 150.0, 10.0, false, CINZA, "Empresa");
    capa.texto(MARGEM, 134.0, 13.0, true, PRETO, sobre::EMPRESA);
    capa.texto(MARGEM, 118.0, 11.0, false, AZUL, sobre::site_curto());
    capa.linha(MARGEM, 80.0, LARGURA - MARGEM, 80.0, 0.6, CINZA);
    capa.texto(MARGEM, 62.0, 9.0, false, CINZA, &sobre::copyright());

    // 3) sumário
    let mut sum = Pagina::default();
    sum.texto(MARGEM, TOPO, 18.0, true, AZUL, "Sumário");
    sum.linha(MARGEM, TOPO - 8.0, LARGURA - MARGEM, TOPO - 8.0, 1.2, AZUL);
    let mut y = TOPO - 40.0;
    for (t, pag) in &d.sumario {
        let num = format!("{}", pag + 2);
        sum.texto(MARGEM, y, 11.5, false, PRETO, t);
        let x0 = MARGEM + pdf::largura(t, 11.5, false) + 6.0;
        let x1 = LARGURA - MARGEM - pdf::largura(&num, 11.5, false) - 6.0;
        if x1 > x0 {
            sum.linha(x0, y + 2.0, x1, y + 2.0, 0.4, CINZA);
        }
        sum.texto_direita(LARGURA - MARGEM, y, 11.5, false, PRETO, &num);
        y -= 22.0;
    }

    // 4) cabeçalho e rodapé nas páginas internas
    let mut paginas = vec![capa, sum];
    paginas.extend(corpo);
    for (i, p) in paginas.iter_mut().enumerate().skip(1) {
        let yc = ALTURA - 44.0;
        let sh = 16.0;
        p.imagem("Simbolo", MARGEM, yc - 3.0, sh * simbolo.largura as f64 / simbolo.altura as f64, sh);
        p.texto(MARGEM + 28.0, yc, 8.5, false, CINZA, &format!("{} — Manual do usuário", sobre::NOME));
        p.texto_direita(LARGURA - MARGEM, yc, 8.5, false, CINZA, &format!("v{versao}"));
        p.linha(MARGEM, yc - 6.0, LARGURA - MARGEM, yc - 6.0, 0.5, CINZA);
        p.linha(MARGEM, 48.0, LARGURA - MARGEM, 48.0, 0.5, CINZA);
        p.texto(MARGEM, 34.0, 8.0, false, CINZA, &format!("{} · {}", sobre::copyright(), sobre::site_curto()));
        p.texto_direita(LARGURA - MARGEM, 34.0, 8.5, false, CINZA, &format!("{} / {total}", i + 1));
    }
    let claro = Cor(crate::marca::CLARO[0] as f64 / 255.0, crate::marca::CLARO[1] as f64 / 255.0, crate::marca::CLARO[2] as f64 / 255.0);
    let imagens = [
        pdf::Imagem { nome: "Logo", largura: logo.largura, altura: logo.altura, alfa: logo.alfa, cor: claro },
        pdf::Imagem { nome: "Simbolo", largura: simbolo.largura, altura: simbolo.altura, alfa: simbolo.alfa, cor: AZUL },
    ];
    pdf::montar(&paginas, &imagens, &format!("{} — Manual do usuário", sobre::NOME), &format!("{} — {}", sobre::CRIADOR, sobre::EMPRESA))
}

/// Grava o manual na pasta temporária e o abre no leitor de PDF padrão.
pub fn abrir() -> Result<std::path::PathBuf, String> {
    let caminho = std::env::temp_dir().join("Manual_Calculadora_Contencoes.pdf");
    std::fs::write(&caminho, gerar()).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer").arg(&caminho).spawn().map_err(|e| e.to_string())?;
    Ok(caminho)
}

/// Pergunta onde salvar uma cópia do manual.
pub fn salvar_copia() -> Option<Result<std::path::PathBuf, String>> {
    let arq = rfd::FileDialog::new()
        .set_title("Salvar manual do usuário")
        .set_file_name("Manual_Calculadora_Contencoes.pdf")
        .add_filter("PDF", &["pdf"])
        .save_file()?;
    Some(std::fs::write(&arq, gerar()).map(|_| arq).map_err(|e| e.to_string()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn manual_gerado() {
        let b = super::gerar();
        assert!(b.starts_with(b"%PDF-1.4") && b.ends_with(b"%%EOF\n"));
        let s = String::from_utf8_lossy(&b);
        let paginas = s.matches("/Type /Page ").count();
        assert!(paginas >= 12, "{paginas} páginas");
        // nenhum caractere do texto sem equivalente no Windows-1252
        for b in super::conteudo() {
            let t = match b {
                super::B::Cap(t) | super::B::Sec(t) | super::B::P(t) | super::B::I(t) | super::B::Nota(t) | super::B::C(t) => t,
                super::B::N(_, t) => t,
            };
            let n = |s: &str| s.matches('?').count();
            assert_eq!(n(&crate::pdf::normalizar(t)), n(t), "caractere sem equivalente: {t}");
        }
        if let Ok(dir) = std::env::var("MEMORIAL_AMOSTRA") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("manual.pdf"), &b);
        }
    }
}
