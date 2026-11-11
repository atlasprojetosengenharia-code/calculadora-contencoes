# Calculadora de Contenções

Software desktop (Windows) para análise e dimensionamento de contenções:
muro de gravidade (pedra argamassada, concreto ciclópico, gabião…), muro de
flexão, cortina atirantada, estaca-prancha, parede de estacas e solo grampeado.

- Entrada de solos por sondagem **SPT** (tabela ou importação CSV/TXT) com correlações de γ, c' e φ'
- Empuxo por **Rankine** ou **Coulomb**, sobrecarga, carga em faixa, água e sismo (Mononobe-Okabe)
- Drenagem: barbacãs, filtro, dreno de pé, canaleta e DHP
- Verificações conforme **NBR 11682, NBR 6122, NBR 5629, NBR 16920-2, NBR 6118:2023 e NBR 6120:2019**
- Memorial de cálculo visual: fórmulas passo a passo, referências normativas e status de cada verificação; relatório HTML (imprimível em PDF) e texto
- Projetos salvos em `.ctc`

## Informações

| | |
|---|---|
| Linguagem | **Rust** (edição 2024) |
| Interface | egui / eframe (aplicativo desktop nativo, Windows) |
| Licença | **Proprietária** — todos os direitos reservados (ver [LICENSE](LICENSE)) |
| Titular | Atlas Projetos e Engenharia LTDA — https://www.atlasprojetos.eng.br |

## Download

Cada versão lançada (tag `vX.Y.Z`) é publicada no
repositório público **[calculadora-contencoes-releases](https://github.com/atlasprojetosengenharia-code/calculadora-contencoes-releases/releases)**
(somente executáveis — o código permanece neste repositório privado). Baixe o `.zip`, descompacte e execute
`CalculadoraContencoes.exe` — não requer instalação. O programa avisa quando
houver versão mais nova.

## Como publicar uma atualização

Enviar código para a `main` (`git push`) **não publica nada**: só roda os testes no
GitHub (workflow *Testes*). A publicação acontece apenas quando você envia uma tag de versão.

1. Trabalhe e teste localmente quantas vezes quiser:
   ```
   cargo test
   cargo run --release
   ```
   Pode fazer commits e `git push` normalmente durante o desenvolvimento.
2. Quando estiver pronto para lançar, aumente `version` em `Cargo.toml`
   (ex.: `0.2.0` → `0.3.0`) e faça o commit.
3. Rode o script de publicação:
   ```
   powershell -ExecutionPolicy Bypass -File .\publicar.ps1
   ```
   Ele confere se está na `main`, sem alterações pendentes, se a versão ainda não foi
   lançada, roda `cargo test`, pede confirmação e envia a tag `vX.Y.Z`.
4. O GitHub Actions compila e publica a release `vX.Y.Z-build.N` no repositório público
   (acompanhe na aba **Actions**). Se a tag não bater com a versão do `Cargo.toml`, nada é publicado.

Manualmente, sem o script: `git tag -a v0.3.0 -m "Versão 0.3.0"` e `git push origin v0.3.0`.

## Compilar localmente

Requer [Rust](https://rustup.rs) (toolchain MSVC no Windows).

```
cargo test
cargo run --release
```

## Aviso

Ferramenta de apoio ao cálculo. Parâmetros obtidos por correlação com o SPT são
estimativas. O projeto é de responsabilidade do engenheiro que emite a ART.
