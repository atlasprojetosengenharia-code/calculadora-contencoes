//! Observações exibidas ao parar o mouse sobre campos e tópicos: o que cada
//! valor representa e como interfere no cálculo e no dimensionamento.

/// Texto de observação para um rótulo da interface (busca exata e, depois,
/// sem os espaços de recuo).
pub fn dica(rotulo: &str) -> Option<&'static str> {
    texto(rotulo).or_else(|| texto(rotulo.trim()))
}

fn texto(r: &str) -> Option<&'static str> {
    Some(match r {
        // ---------------- Geometria essencial ----------------
        "Geometria essencial" => "Dados que definem o problema: altura contida, talude de montante, sobrecarga e embutimento. \
            Alterar H (com o pré-dimensionamento ligado) recalcula as proporções iniciais da seção.",
        "Altura do corpo H" | "Altura total H" | "Altura do talude H" | "Altura escavada H" => "Altura de solo contida. É o parâmetro \
            que mais pesa: o empuxo cresce com H² (solo) e com H (sobrecarga), e os momentos com H³. Com o pré-dimensionamento \
            ligado, a geometria é preenchida pelas proporções do Marchetti (2007).",
        "Inclinação do talude de montante β" => "Inclinação do terreno acima da crista. Aumenta o coeficiente de empuxo ativo \
            (Rankine/Coulomb) e a componente vertical do empuxo; β não pode superar φ' do aterro. Aparece no desenho de todos os             tipos; no solo grampeado entra no peso da cunha de ruptura (o plano pelo pé precisa ter ω > β).",
        "Sobrecarga na crista q" => "Carga distribuída sobre o terreno contido (tráfego, edificações, estoque). Gera empuxo \
            uniforme Ka·q em toda a altura — em muros baixos costuma ser a maior parcela. A NBR 11682 (7.3.3) exige no mínimo 20 kPa.",
        "Embutimento (solo à frente) Df" => "Profundidade da base abaixo do terreno à frente do muro. Entra na capacidade de carga \
            (sobrecarga q = γ·Df) e no empuxo passivo, quando considerado. Pré-dimensionamento: máx(espessura da base + 30 cm \
            (Marchetti); 0,10·H; 0,40 m) (NBR 16920-1 D.6.1, por analogia; 0,1 a 0,2·H se o terreno à frente for inclinado). \
            Na divisa com terreno vizinho, a NBR 6122 (7.7.2) exige apoio a pelo menos 1,5 m, salvo em rocha.",
        "Ficha abaixo da escavação" => "Comprimento da cortina abaixo do fundo da escavação. Define o apoio inferior e o \
            empuxo passivo disponível; ficha curta reduz o FS do pé da cortina. O pré-dimensionamento adota a menor ficha \
            que equilibra a reação na base (passivo reduzido − ativo abaixo da escavação).",
        "Profundidade de engastamento" => "Nas contenções embutidas a ficha é calculada pelo programa (equilíbrio de momentos \
            com o passivo reduzido), acrescida do fator de ficha definido em Normas e fatores.",
        "Pré-dimensionar ao alterar H" => "Quando ligado, ao mudar a altura H o programa preenche a geometria pelas proporções \
            do livro (gravidade e flexão) ou da prática corrente (solo grampeado). Desligue para manter valores digitados.",
        "Drenagem" => "Dispositivos que aliviam a pressão d'água no tardoz. A lâmina d'água adotada define o empuxo \
            hidrostático; uma drenagem eficiente permite considerar o maciço sem água.",
        "Cargas adicionais" => "Ações além da sobrecarga uniforme: carga em faixa (Boussinesq) e ação sísmica (Mononobe-Okabe).",
        "Teoria de empuxo:" => "Rankine: parede lisa, sem atrito solo-muro (mais conservador). Coulomb: considera o atrito δ \
            entre solo e parede, reduzindo o empuxo horizontal e gerando componente vertical estabilizante.",
        "Atrito solo-estrutura δ" => "Ângulo de atrito entre o aterro e o paramento (Coulomb). Usual 2/3·φ'. Inclina o empuxo, \
            reduz a componente horizontal e aumenta a vertical — melhora tombamento e deslizamento.",

        // ---------------- Contato base-solo ----------------
        "Contato base-solo de fundação" => "Parâmetros do atrito e da adesão entre a base e o solo de fundação, que resistem ao \
            deslizamento. γ, φ' e c' vêm da camada da sondagem na cota da base.",
        "Atrito na base:" => "tan(k·φ'): forma usual com δb = k·φ'. k·tg φ': forma do Marchetti (f = 0,67 a 0,9·tg φ'). \
            O coeficiente multiplica a força normal na resistência ao deslizamento.",
        "k = δ base / φ'" => "Fração de φ' adotada como atrito solo-base (usual 2/3 para concreto moldado no local). \
            Maior k → maior resistência ao deslizamento.",
        "k (f = k·tg φ')" => "Fator sobre tg φ' (Marchetti 3.1: 0,67 a 1,0; 0,9 em base compacta). Multiplica diretamente \
            a resistência ao deslizamento.",
        "Adesão / c'" => "Fração da coesão mobilizada como adesão na base (Marchetti: 0,5 a 0,67). Soma adesão·B à resistência \
            ao deslizamento — relevante em solos coesivos.",
        "Predefinições:" => "Atalhos para os critérios de atrito: ⅔φ (usual), 0,67·tg φ e 0,9·tg φ (Marchetti 3.1).",

        // ---------------- Concreto ----------------
        "Concreto e aço (NBR 6118:2023)" => "Materiais do concreto armado: resistência do concreto, do aço e cobrimento. \
            Definem As necessária, cisalhamento resistente (VRd1) e ancoragens.",
        "fck" => "Resistência característica do concreto à compressão. Aumenta a capacidade à flexão e ao cisalhamento e reduz \
            o comprimento de ancoragem lb; o mínimo por durabilidade depende da classe de agressividade (NBR 6118 Tab. 7.1).",
        "fyk" => "Resistência característica do aço (CA-50 = 500 MPa). Entra no cálculo de As (fyd = fyk/γs) e de lb.",
        "Cobrimento" => "Distância da face do concreto à armadura. Reduz a altura útil d (aumenta As) e garante durabilidade; \
            mínimo pela classe de agressividade (NBR 6118 Tab. 7.2) — em contato com o solo, usual 30 a 50 mm.",
        "Peso específico" => "Peso do concreto armado: 25 kN/m³ (NBR 6120). Compõe o peso próprio estabilizante.",

        // ---------------- Passivo, subpressão, dente ----------------
        "Considerar empuxo passivo frontal" => "Soma o empuxo passivo do solo à frente à resistência ao deslizamento. Só é válido \
            se esse solo nunca for removido (NBR 6122 7.6.3); Marchetti recomenda desprezá-lo por risco de erosão.",
        "Fração mobilizada (≤ 1/redução)" => "Parcela do passivo considerada. A NBR 6122 exige redução por no mínimo 2,0 \
            (fração ≤ 0,5), pois o passivo só se mobiliza com deslocamentos grandes.",
        "Considerar subpressão na base" => "Inclui a pressão d'água sob a base (triangular), que reduz a força normal efetiva — \
            diminui a resistência ao deslizamento e o FS ao tombamento. Desprezar só com drenagem eficiente.",
        "Dente de base (chave de cisalhamento)" => "Saliência sob a base que leva o passivo até a cota do dente, aumentando a \
            resistência ao deslizamento sem alargar a base. Passivo reduzido pela NBR 6122 7.6.3.",
        "Profundidade do dente" => "Altura do dente abaixo da base. Aumenta a profundidade do passivo mobilizado (cresce com D²).",
        "Largura do dente" => "Largura do dente. Deve resistir ao cisalhamento e à flexão gerados pelo passivo; acrescenta peso próprio.",

        // ---------------- Gravidade ----------------
        "Material:" => "Material do corpo do muro. Define o peso específico (estabilizante) e as tensões admissíveis de \
            compressão, tração e cisalhamento nas juntas.",
        "Alvenaria — tensões admissíveis" => "Limites para as tensões nas seções horizontais do corpo: compressão, tração e \
            cisalhamento nas juntas. Vêm do banco de materiais ou dos valores do projetista.",
        "fck do concreto (C15 a C40)" => "Resistência do concreto simples/ciclópico. Define σadm = 0,85·fcd/1,4 e a tração e o \
            cisalhamento admissíveis (NBR 6118 seção 24, γc = 1,68).",
        "Argamassa (resist. média):" => "Classe da argamassa da pedra argamassada. Define o cisalhamento nas juntas (fvk = f0 + 0,5σ) \
            por analogia com a NBR 16868-1 Tab. 4.",
        "Informar valores do projetista" => "Substitui o banco de materiais por valores próprios de tensão admissível \
            (ensaios ou especificação do projeto).",
        "σ admissível (compressão)" => "Tensão máxima de compressão no corpo. Comparada à maior tensão nas seções horizontais.",
        "Tração admissível nas juntas" => "Tração máxima aceita no corpo. Excentricidades grandes nas seções geram tração no paramento interno.",
        "Cisalhamento admissível" => "Parcela de coesão do cisalhamento nas juntas (τadm = c0 + μ·σ).",
        "A — ressalto frontal" => "Ponta da base à frente do paramento. Afasta a aresta de tombamento: aumenta o braço do \
            peso e reduz a excentricidade. Marchetti: 0,5 a 1,0 vez a altura da base.",
        "B — projeção do paramento frontal" => "Recuo horizontal do paramento frontal (inclinação da face). Marchetti: 1:10 a 1:15. \
            Inclinar a face desloca o peso para trás e melhora a excentricidade.",
        "C — largura da crista" => "Largura do topo do muro. Marchetti: 30 cm ou 8 a 15 % de H. Aumenta o peso próprio.",
        "D — projeção do tardoz" => "Inclinação do tardoz (face interna). Aumenta a base e o peso; com tardoz inclinado, \
            parte do solo apoia sobre o muro e soma peso estabilizante.",
        "E — ressalto posterior" => "Calcanhar da base atrás do tardoz. O solo sobre ele soma peso estabilizante e aumenta a base.",
        "F — altura da base" => "Altura do bloco de base. Marchetti: 12 a 15 % de H. Peso próprio e rigidez da sapata.",
        "Detalhamento construtivo (juntas e coroamento)" => "Juntas de dilatação e viga de coroamento do muro de gravidade.",
        "Espaçamento das juntas de dilatação" => "Distância entre juntas. A NBR 6118 (24.4) limita a 15 m em concreto simples \
            para controlar a fissuração por retração e temperatura.",
        "Viga de coroamento em concreto armado" => "Viga no topo que distribui cargas, alinha a crista e protege a alvenaria. \
            Gera armaduras na tabela de armação.",
        "Altura da viga de coroamento" => "Altura da viga do topo; define sua armadura construtiva.",

        // ---------------- Flexão ----------------
        "Espessura da parede (topo)" => "Espessura da parede no topo. Marchetti: 20 cm mínimo (concretagem e cobrimento).",
        "Espessura da parede (base)" => "Espessura no engaste com a sapata, onde o momento é máximo. Marchetti: 8 a 10 % de H. \
            Define a altura útil d e, com ela, a armadura principal N1.",
        "Espessura da sapata" => "Altura da laje de base. Marchetti: 8 a 10 % de H. Resiste aos momentos na ponta e no talão.",
        "Comprimento da ponta" => "Laje à frente da parede. Marchetti: 10 a 12 % de H. Aumenta o braço estabilizante e reduz a \
            tensão máxima na borda da base.",
        "Comprimento do talão" => "Laje atrás da parede. O solo sobre o talão é o principal peso estabilizante — aumentar o talão \
            melhora deslizamento e tombamento.",

        // ---------------- Cortina atirantada ----------------
        "Espessura da cortina" => "Espessura do paramento de concreto. Resiste à flexão entre tirantes e à punção na placa de ancoragem.",
        "Placa de ancoragem (lado)" => "Lado da placa de apoio da cabeça do tirante. Define o perímetro crítico na verificação de punção.",
        "Espaçamento horizontal" => "Distância entre tirantes na mesma linha. Maior espaçamento → maior carga por tirante e \
            maiores momentos na cortina.",
        "Inclinação" => "Inclinação do tirante abaixo da horizontal. Aumenta o comprimento livre necessário e gera componente \
            vertical sobre a cortina.",
        "Diâmetro do furo" => "Diâmetro da perfuração (bulbo). A capacidade do bulbo cresce com o perímetro: T = π·D·L·qs.",
        "qs (atrito lateral do bulbo)" => "Atrito unitário calda–solo no bulbo (ensaios ou correlações). Define o comprimento \
            ancorado necessário (NBR 5629).",
        "fyk do aço do tirante" => "Resistência do aço do tirante. Carga de trabalho ≤ 0,9·fyk·As/FS (NBR 5629 4.5.2.1).",
        "Área de aço do tirante" => "Seção de aço do tirante (barra ou cordoalhas). Limita a carga de trabalho pelo aço.",
        "Tirante:" => "Permanente ou provisório: muda os fatores de segurança do aço e do bulbo (1,75 ou 1,50 — NBR 5629).",
        "Fixação da cabeça:" => "Rosca ou clavetes: define o comprimento livre mínimo (3 m ou 5 m — NBR 5629 B.2.7).",
        "Diagrama:" => "Forma da envoltória de empuxo aparente usada nas cortinas com vários níveis de tirantes.",

        // ---------------- Embutidas ----------------
        "Profundidade do apoio" => "Profundidade do apoio (tirante ou estronca) a partir do topo. Altera o vão, o momento máximo e a ficha.",
        "Inclinação do tirante" => "Inclinação do apoio. A componente vertical carrega a parede; a horizontal define a reação de apoio.",
        "NA no lado escavado a" => "Nível d'água no lado escavado. Reduz o passivo (peso submerso) e define o empuxo hidrostático líquido.",
        "δ no lado passivo" => "Atrito solo-parede no passivo. Aumenta Kp — use valores moderados (o passivo com atrito alto é contra a segurança).",
        "Perfil:" => "Perfil da estaca-prancha. Define o módulo resistente W e a capacidade à flexão.",
        "Módulo resistente W" => "Módulo de resistência do perfil por metro. Tensão = M/W, comparada a fy/γa1.",
        "Altura da viga de travamento" => "Altura da viga de coroamento/travamento das estacas. Distribui cargas e solidariza a parede.",
        "fy do aço" => "Tensão de escoamento do perfil metálico (ex.: 270 a 355 MPa).",
        "Diâmetro" => "Diâmetro das estacas. Aumenta a inércia e a capacidade à flexão; define a armadura longitudinal mínima.",
        "Espaçamento (eixo a eixo)" => "Distância entre estacas. Maior espaçamento → mais carga por estaca; vãos grandes \
            podem exigir revestimento entre estacas.",
        "Nº de barras longitudinais" => "Quantidade de barras da armadura longitudinal da estaca (mínimo 6, NBR 6118 18.4.2).",

        // ---------------- Solo grampeado ----------------
        "Inclinação da face (c/ vertical)" => "Inclinação do talude grampeado. Faces mais inclinadas reduzem o empuxo e a carga nos grampos.",
        "Espaçamento horizontal Sh" => "Distância horizontal entre grampos. A carga por grampo cresce com Sh·Sv. Usual 1,0 a 2,0 m.",
        "Espaçamento vertical Sv" => "Distância vertical entre linhas de grampos. Usual 1,0 a 2,0 m.",
        "Profundidade do 1º grampo" => "Cota da primeira linha abaixo da crista. Usual Sv/2.",
        "Comprimento L" => "Comprimento do grampo. Define o trecho além da superfície de ruptura que resiste ao arrancamento. \
            Prática corrente: 0,7 a 1,0·H.",
        "Inclinação (abaixo da horizontal)" => "Inclinação do grampo. Usual 10° a 15° (facilita a injeção); maiores inclinações \
            reduzem a contribuição à tração.",
        "qs (arrancamento)" => "Resistência unitária ao arrancamento calda–solo (ensaios de arrancamento). Define o comprimento ancorado.",
        "Diâmetro da barra" => "Bitola da barra do grampo. Resistência à tração = As·fyk/γm (NBR 16920-2 9.1).",
        "fyk da barra" => "Resistência característica do aço do grampo.",
        "Espessura da face" => "Espessura do concreto projetado da face. Verificada à flexão e à punção na placa (NBR 16920-2 9.3).",
        "Placa da cabeça (lado)" => "Lado da placa de apoio da cabeça do grampo; define o perímetro de punção na face.",

        // ---------------- Drenagem ----------------
        "Lâmina d'água / altura contida" => "Fração da altura contida com água atrás da estrutura. Define o empuxo hidrostático (γw·hw²/2) e reduz o peso do solo             (γ'). Em 'parcial' é a lâmina residual de um sistema que funciona mal; em 'sem drenagem', 1,0 = aterro saturado até a             crista (pior caso). No solo grampeado entra como poropressão no plano de ruptura.",
        "Barbacãs" => "Tubos que atravessam o paramento para escoar a água do tardoz. Reduzem a pressão hidrostática.",
        "Diâmetro (tubo PVC)" => "Diâmetro dos barbacãs (usual 75 a 100 mm).",
        "   Espaçamento horizontal" => "Distância horizontal entre barbacãs (usual 1,5 a 2,0 m).",
        "Espaçamento vertical" => "Distância vertical entre filas de barbacãs.",
        "Altura da 1ª fila" => "Altura da primeira fila acima do terreno à frente — deve ficar acima do nível de enchentes/acúmulo.",
        "Declividade para fora" => "Inclinação dos barbacãs para fora, garantindo o escoamento por gravidade.",
        "Filtro / camada drenante no tardoz" => "Camada de brita/areia ou geocomposto atrás do muro que conduz a água aos \
            drenos e impede o carreamento de finos.",
        "Espessura" => "Espessura da camada drenante no tardoz.",
        "Dreno de pé (tubo perfurado) Ø" => "Tubo perfurado na base do tardoz que coleta a água e a leva para fora do muro.",
        "Canaleta de crista" => "Canaleta no topo que intercepta a água superficial antes de infiltrar no aterro.",
        "Drenos sub-horizontais profundos (DHP)" => "Drenos perfurados no maciço que rebaixam o nível d'água.",
        "Comprimento" => "Comprimento dos drenos sub-horizontais.",
        "   Material:" => "Material da camada drenante.",

        // ---------------- Cargas ----------------
        "Carga em faixa (Boussinesq)" => "Carga distribuída numa faixa paralela à contenção (via, sapata). O acréscimo de \
            empuxo é calculado pela teoria da elasticidade e depende da distância à contenção.",
        "Intensidade" => "Pressão da carga em faixa (kPa).",
        "Distância à contenção" => "Afastamento da faixa em relação à contenção. Quanto mais próxima, maior o empuxo no topo.",
        "Largura da faixa" => "Largura da carga em faixa.",
        "Ação sísmica (Mononobe-Okabe)" => "Acréscimo de empuxo por aceleração sísmica (M-O) e forças de inércia no muro. \
            Verificado com os FS do caso sísmico.",
        "kh (ah/g)" => "Coeficiente sísmico horizontal (aceleração horizontal / g). Aumenta o empuxo e as forças de inércia.",
        "kv" => "Coeficiente sísmico vertical. Reduz (ou aumenta) o peso efetivo.",

        // ---------------- SPT ----------------
        "Identificação:" => "Nome do furo de sondagem; aparece no memorial.",
        "Modo de entrada do solo:" => "SPT completo: perfil estratificado camada a camada. Entrada simplificada: solo homogêneo \
            com NSPT representativo — mais rápida, menos precisa.",
        "Nível d'água encontrado a" => "Profundidade do NA na sondagem. Abaixo dele o solo usa peso submerso e há empuxo \
            hidrostático (salvo drenagem).",
        "Nível d'água e correlações" => "NA e escolha das correlações SPT que estimam γ, φ' e c' de cada camada.",
        "φ' de solos granulares:" => "Correlação para φ' de areias: Teixeira (√(20N) + 15) ou Godoy (28 + 0,4N). φ' controla Ka e Kp.",
        "Solos coesivos:" => "Drenado (longo prazo, c' e φ') ou não drenado (Su = 10·N, φ = 0). Muros permanentes: em geral, drenado.",
        "c' adotado" => "Coesão efetiva adotada para argilas em condição drenada. Reduz o empuxo ativo (−2c√Ka).",
        "φ' adotado" => "Ângulo de atrito efetivo adotado para argilas em condição drenada.",
        "NSPT" => "Número de golpes do SPT. Base das correlações de γ, φ' e c' (ou Su).",
        "Tipo de solo" => "Classe do solo predominante, que escolhe a correlação adequada.",
        "Camadas" => "Estratos da sondagem. Cada camada tem NSPT, descrição e parâmetros (Auto por correlação ou Manual).",
        "Até (m)" => "Profundidade final da camada.",
        "De (m)" => "Profundidade inicial (igual ao fim da camada anterior).",
        "Descrição" => "Descrição do solo; define o tipo e o estado usados nas correlações.",
        "γ" => "Peso específico natural (kN/m³). Entra no empuxo e no peso do solo sobre a base.",
        "γsat" => "Peso específico saturado, usado abaixo do NA (peso submerso = γsat − γw).",
        "c' (kPa)" => "Coesão efetiva. Reduz o empuxo ativo e aumenta a capacidade de carga.",
        "φ' (°)" => "Ângulo de atrito efetivo. Define Ka, Kp e a capacidade de carga.",
        "Entrada simplificada na base" => "Solo homogêneo a partir de um NSPT e do tipo de solo.",

        // ---------------- Normas e fatores ----------------
        "NBR 11682 — Nível de segurança (Tab. 1, 2 e 3)" => "Níveis de segurança contra perda de vidas e danos materiais; \
            definem o FS mínimo de estabilidade (Tab. 3).",
        "Grande variabilidade dos ensaios (+10 %, Nota 1)" => "Majora em 10 % o FS mínimo quando os parâmetros do solo são muito variáveis.",
        "Muros — NBR 11682 Tabela 4" => "Fatores de segurança e critérios de muros de gravidade e de flexão.",
        "FS deslizamento" => "FS mínimo ao deslizamento na base (NBR 11682: 1,5). Maior FS → base maior ou dente.",
        "FS tombamento" => "FS mínimo ao tombamento em torno da ponta (NBR 11682: 2,0; Marchetti usa 1,5 em solo não coesivo).",
        "FS capacidade de carga (NBR 6122)" | "FS capacidade de carga" => "FS global à ruptura do solo de fundação (NBR 6122: 3,0).",
        "Excentricidade máx. / B" => "Limite da excentricidade da resultante (terço central: B/6). Garante base toda comprimida.",
        "Área comprimida mín. / B" => "Fração mínima da base em compressão (NBR 6122 7.6.2: 2/3).",
        "Sobrecarga mínima (7.3.3)" => "Menor sobrecarga aceita na crista (NBR 11682: 20 kPa). Abaixo disso, o programa alerta.",
        "Empuxo passivo e contenções embutidas" => "Reduções do passivo e acréscimos de ficha.",
        "Redução do passivo — muros (NBR 6122 ≥ 2,0)" => "Divisor do passivo à frente de muros (mínimo 2,0).",
        "FS do passivo — embutidas (projeto)" => "Fator sobre o passivo na ficha de paredes embutidas (critério de projeto, usual 1,5 a 2,0).",
        "Acréscimo de ficha — balanço" => "Multiplicador da ficha teórica em paredes em balanço (usual 1,2).",
        "Acréscimo de ficha — ancorada" => "Multiplicador da ficha teórica em paredes ancoradas.",
        "NBR 5629:2018 — Tirantes" => "Fatores e comprimentos mínimos de tirantes.",
        "Aço permanente: 0,9·fyk /" | "Aço provisório: 0,9·fyk /" => "Fator de segurança do aço do tirante (NBR 5629 4.5.2.1).",
        "Bulbo permanente (B.2.4.1)" | "Bulbo provisório (B.2.4.1)" => "Fator de segurança do bulbo ao arrancamento.",
        "Trecho livre mín. — rosca" | "Trecho livre mín. — clavetes" => "Comprimento livre mínimo do tirante (NBR 5629 B.2.7).",
        "Folga além da cunha (× H)" => "Folga adicional do trecho livre além da cunha ativa (critério de projeto).",
        "Bulbo mínimo (projeto)" => "Comprimento mínimo de bulbo adotado (critério de projeto).",
        "NBR 16920-2:2021 — Solo grampeado" => "Fatores da estabilidade interna e da armação dos grampos.",
        "FS estabilidade interna (9.8)" => "FS mínimo ao arrancamento e à tração dos grampos.",
        "γm da armação (9.1)" => "Coeficiente de minoração do aço do grampo.",
        "NBR 6118:2023 / NBR 8800 — Ponderação" => "Coeficientes de ponderação das ações e dos materiais.",
        "γf (ações)" => "Majorador das ações no dimensionamento estrutural (1,4).",
        "γc (concreto)" => "Minorador do concreto (1,4; concreto simples usa 1,68).",
        "γs (aço CA)" => "Minorador do aço de armadura (1,15).",
        "γa1 (aço estrutural)" => "Minorador do aço estrutural dos perfis (1,10).",

        // ---------------- Resultados / memorial ----------------
        "Pré-visualização" => "Seção desenhada em tempo real com o selo ATENDE / NÃO ATENDE das verificações.",
        "Seção transversal" => "Desenho da seção com dimensões, armaduras e diagrama de empuxo.",
        "Verificações" => "Todas as verificações com valor, limite e status (ATENDIDO / NÃO ATENDIDO / INFORMATIVO).",
        "Detalhamento do aço — tabela de armação" => "Posições, bitolas, espaçamentos e comprimentos aproximados das armaduras.",
        "Fórmulas passo a passo" => "Mostra no memorial cada fórmula com os valores substituídos.",
        "Referências normativas" => "Mostra as caixas com a norma e o item de cada verificação.",
        "Avisos" => "Hipóteses, exceções e pontos que exigem atenção do projetista.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    /// Todo campo numérico e caixa de marcar das telas tem observação.
    #[test]
    fn todos_os_campos_tem_observacao() {
        let mut faltando = Vec::new();
        for (nome, fonte) in [
            ("contencao", include_str!("contencao.rs")),
            ("fatores", include_str!("fatores.rs")),
            ("spt", include_str!("spt.rs")),
        ] {
            for marcador in ["num(ui, \"", "caixa(ui, "] {
                for trecho in fonte.split(marcador).skip(1) {
                    let t = if marcador.starts_with("caixa") {
                        match trecho.split_once('"') {
                            Some((_, resto)) => resto,
                            None => continue,
                        }
                    } else {
                        trecho
                    };
                    let rotulo = t.split('"').next().unwrap_or("");
                    if super::dica(rotulo).is_none() {
                        faltando.push(format!("{nome}: {rotulo}"));
                    }
                }
            }
        }
        for r in ["Altura do corpo H", "Altura total H", "Altura do talude H", "Altura escavada H", "Embutimento (solo à frente) Df"] {
            if super::dica(r).is_none() {
                faltando.push(r.to_string());
            }
        }
        assert!(faltando.is_empty(), "sem observação: {faltando:#?}");
    }
}
