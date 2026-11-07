pub mod atirantada;
pub mod embutida;
pub mod flexao;
pub mod grampeado;
pub mod gravidade;

#[cfg(test)]
mod fumaca {
    use crate::fatores::Fatores;

    #[test]
    fn todos_os_tipos_com_padroes() {
        let f = Fatores::default();
        let casos = [
            ("GRAVIDADE", super::gravidade::MuroGravidade::default().calcular(&f)),
            ("FLEXAO", super::flexao::MuroFlexao::default().calcular(&f)),
            ("ATIRANTADA", super::atirantada::CortinaAtirantada::default().calcular(&f)),
            ("PRANCHA", super::embutida::EstacaPrancha::default().calcular(&f)),
            ("ESTACAS", super::embutida::ParedeEstacas::default().calcular(&f)),
            ("GRAMPEADO", super::grampeado::SoloGrampeado::default().calcular(&f)),
        ];
        for (nome, r) in casos {
            println!("\n##### {nome} — atende: {}", r.atende());
            for v in &r.verificacoes {
                println!("  {}", v.texto());
            }
            assert!(!r.desenho.poligonos.is_empty());
            assert!(r.verificacoes.iter().all(|v| !v.valor.is_nan()), "{nome}: NaN");
            // todo memorial tem fórmulas, normas e verificações
            use crate::resultado::Bloco;
            let conta = |f: fn(&Bloco) -> bool| r.blocos.iter().filter(|b| f(b)).count();
            let (nf, nn) = (conta(|b| matches!(b, Bloco::Formula(_))), conta(|b| matches!(b, Bloco::Norma(_))));
            println!("  blocos: {} fórmulas, {} referências normativas", nf, nn);
            assert!(nf >= 3 && nn >= 2, "{nome}: memorial sem fórmulas/normas suficientes");
            // amostras do relatório: MEMORIAL_AMOSTRA=<pasta> cargo test
            if let Ok(pasta) = std::env::var("MEMORIAL_AMOSTRA") {
                let html = crate::memorial_html::gerar(&r, nome, "teste");
                std::fs::write(format!("{pasta}/memorial_{}.html", nome.to_lowercase()), html).unwrap();
            }
        }
    }
}
