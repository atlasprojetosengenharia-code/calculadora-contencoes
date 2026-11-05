# Calculadora de Contenções

Software desktop (Windows) para análise e dimensionamento de contenções:
muro de gravidade (pedra argamassada, concreto ciclópico, gabião…), muro de
flexão, cortina atirantada, estaca-prancha, parede de estacas e solo grampeado.

- Entrada de solos por sondagem **SPT** (tabela ou importação CSV/TXT) com correlações de γ, c' e φ'
- Empuxo por **Rankine** ou **Coulomb**, sobrecarga, carga em faixa, água e sismo (Mononobe-Okabe)
- Drenagem: barbacãs, filtro, dreno de pé, canaleta e DHP
- Verificações conforme **NBR 11682, NBR 6122, NBR 5629, NBR 16920-2, NBR 6118:2023 e NBR 6120:2019**
- Memorial de cálculo exportável e projetos salvos em `.ctc`

## Download

Cada envio para a branch `main` gera automaticamente uma nova versão em
**[Releases](../../releases)**. Baixe o `.zip`, descompacte e execute
`CalculadoraContencoes.exe` — não requer instalação. O programa avisa quando
houver versão mais nova.

## Como publicar uma atualização

1. Altere o código (opcional: aumente `version` em `Cargo.toml` para mudanças maiores).
2. Envie para o GitHub:
   ```
   git add -A
   git commit -m "descrição da alteração"
   git push
   ```
3. O GitHub Actions testa, compila e publica a release `vX.Y.Z-build.N` em poucos minutos
   (acompanhe na aba **Actions**).

## Compilar localmente

Requer [Rust](https://rustup.rs) (toolchain MSVC no Windows).

```
cargo test
cargo run --release
```

## Aviso

Ferramenta de apoio ao cálculo. Parâmetros obtidos por correlação com o SPT são
estimativas. O projeto é de responsabilidade do engenheiro que emite a ART.
