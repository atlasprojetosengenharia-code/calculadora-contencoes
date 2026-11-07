//! Relatório do memorial de cálculo em HTML autônomo (sem dependências
//! externas): seção transversal em SVG, resumo com etiquetas de status,
//! fórmulas passo a passo e caixas de referência normativa. Pronto para
//! impressão / "Salvar como PDF" no navegador.

use crate::resultado::{Bloco, Criterio, Resultado, Verificacao};

fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn badge(v: &Verificacao) -> String {
    let classe = match v.criterio {
        Criterio::Info => "info",
        _ if v.ok() => "ok",
        _ => "falha",
    };
    format!(r#"<span class="badge {classe}">{}</span>"#, v.status())
}

fn linha_verif(v: &Verificacao) -> String {
    format!(
        r#"<div class="verif"><span class="vnome">{}</span><span class="vexp">{}</span>{}</div>"#,
        esc(&v.nome),
        esc(&v.expressao()),
        badge(v)
    )
}

const CSS: &str = r#"
:root{--fundo:#f2f4f7;--cartao:#fff;--borda:#d6dce3;--texto:#1f2328;--fraco:#656d76;--destaque:#0969da;
--destaque-suave:#ddeafc;--ok:#1a7f37;--ok-suave:#dafbe1;--falha:#cf222e;--falha-suave:#ffebe9;--alerta:#9a6700;
--alerta-suave:#fff8c5;--norma:#8250df;--norma-suave:#f3eefc}
@media (prefers-color-scheme:dark){:root{--fundo:#0d1117;--cartao:#1a2029;--borda:#2c3440;--texto:#e6edf3;--fraco:#8b949e;
--destaque:#388bfd;--destaque-suave:#1c3456;--ok:#3fb950;--ok-suave:#12321b;--falha:#f85149;--falha-suave:#3d1416;
--alerta:#d29922;--alerta-suave:#3a2e0f;--norma:#a371f7;--norma-suave:#2a2142}}
*{box-sizing:border-box}
body{margin:0;background:var(--fundo);color:var(--texto);font:15px/1.5 "Segoe UI",system-ui,sans-serif}
main{max-width:1060px;margin:0 auto;padding:24px 16px 48px}
header{display:flex;flex-wrap:wrap;align-items:center;gap:12px 20px;margin-bottom:18px}
header h1{font-size:24px;margin:0}
header .sub{color:var(--fraco);font-size:13px}
.cartao{background:var(--cartao);border:1px solid var(--borda);border-radius:10px;padding:16px;margin-bottom:14px}
.grade{display:grid;grid-template-columns:1fr 1fr;gap:14px}
@media (max-width:800px){.grade{grid-template-columns:1fr}}
h2{font-size:17px;margin:26px 0 10px;padding-left:10px;border-left:4px solid var(--destaque)}
h3{font-size:15px;margin:0 0 10px}
.status{display:inline-block;font-weight:600;padding:6px 14px;border-radius:999px;font-size:14px}
.status.ok{background:var(--ok-suave);color:var(--ok);border:1px solid var(--ok)}
.status.falha{background:var(--falha-suave);color:var(--falha);border:1px solid var(--falha)}
.badge{display:inline-block;font-weight:700;font-size:11.5px;letter-spacing:.04em;padding:2px 9px;border-radius:999px;white-space:nowrap}
.badge.ok{background:var(--ok-suave);color:var(--ok);border:1px solid var(--ok)}
.badge.falha{background:var(--falha-suave);color:var(--falha);border:1px solid var(--falha)}
.badge.info{background:var(--fundo);color:var(--fraco);border:1px solid var(--borda)}
.verif{display:flex;flex-wrap:wrap;align-items:center;gap:6px 12px;padding:7px 10px;border-radius:8px;margin:6px 0;background:var(--fundo)}
.vnome{flex:1 1 260px}
.vexp{font-family:"Cambria Math","STIX Two Math",Cambria,serif;font-size:15.5px;font-variant-numeric:tabular-nums}
.formula{border:1px solid var(--borda);border-left:4px solid var(--destaque);border-radius:8px;margin:10px 0;overflow:hidden}
.formula .ftit{background:var(--destaque-suave);padding:6px 12px;font-weight:600;font-size:13.5px}
.formula .passo{display:grid;grid-template-columns:112px 1fr;gap:10px;padding:6px 12px;border-top:1px solid var(--borda);align-items:baseline}
.formula .rot{font-size:11.5px;font-weight:700;letter-spacing:.05em;text-transform:uppercase;color:var(--fraco)}
.formula .mat{font-family:"Cambria Math","STIX Two Math",Cambria,serif;font-size:16px;overflow-wrap:anywhere}
.formula .res .mat{font-weight:700;color:var(--destaque)}
blockquote.norma{margin:10px 0;padding:10px 14px;border-left:4px solid var(--norma);background:var(--norma-suave);border-radius:0 8px 8px 0}
blockquote.norma .ncab{font-weight:700;color:var(--norma);font-size:13.5px}
blockquote.norma p{margin:4px 0 0}
p.txt{margin:4px 0}
pre.tab{margin:4px 0;font:12.5px/1.45 Consolas,"Cascadia Mono",monospace;overflow-x:auto;white-space:pre}
.avisos li{margin:4px 0}
.avisos{border-left:4px solid var(--alerta);background:var(--alerta-suave)}
table.resumo{width:100%;border-collapse:collapse;font-size:13.5px}
table.resumo td{padding:5px 6px;border-bottom:1px solid var(--borda);vertical-align:middle}
table.resumo td.exp{font-family:"Cambria Math",Cambria,serif;white-space:nowrap}
footer{color:var(--fraco);font-size:12.5px;margin-top:26px}
@media print{body{background:#fff;font-size:12px}main{max-width:none;padding:0}.cartao,.formula,blockquote.norma,.verif{break-inside:avoid}
h2{break-after:avoid}}
"#;

/// Gera o relatório completo.
pub fn gerar(res: &Resultado, titulo: &str, versao: &str) -> String {
    let mut h = String::with_capacity(64 * 1024);
    h.push_str(&format!(
        r#"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Memorial de cálculo — {}</title><style>{CSS}</style></head><body><main>"#,
        esc(titulo)
    ));

    // Cabeçalho e situação geral
    let n_falha = res.verificacoes.iter().filter(|v| !v.ok()).count();
    let status = if n_falha == 0 {
        r#"<span class="status ok">ATENDE A TODAS AS VERIFICAÇÕES</span>"#.to_string()
    } else {
        format!(r#"<span class="status falha">NÃO ATENDE — {n_falha} verificação(ões)</span>"#)
    };
    h.push_str(&format!(
        r#"<header><div><h1>Memorial de cálculo</h1><div class="sub">{} · Calculadora de Contenções {}</div></div>{status}</header>"#,
        esc(titulo),
        esc(versao)
    ));

    // Seção + resumo
    h.push_str(r#"<div class="grade"><section class="cartao"><h3>Seção transversal</h3>"#);
    h.push_str(&crate::desenho::svg(&res.desenho, 520.0, 420.0));
    h.push_str(r#"</section><section class="cartao"><h3>Resumo das verificações</h3><table class="resumo">"#);
    for v in &res.verificacoes {
        h.push_str(&format!(
            r#"<tr><td>{}</td><td class="exp">{}</td><td>{}</td></tr>"#,
            esc(&v.nome),
            esc(&v.expressao()),
            badge(v)
        ));
    }
    h.push_str("</table></section></div>");

    if !res.avisos.is_empty() {
        h.push_str(r#"<section class="cartao avisos"><h3>⚠ Avisos</h3><ul>"#);
        for a in &res.avisos {
            h.push_str(&format!("<li>{}</li>", esc(a)));
        }
        h.push_str("</ul></section>");
    }

    // Memorial
    let mut aberto = false;
    let mut n = 0;
    for b in &res.blocos {
        match b {
            Bloco::Titulo(t) => {
                if aberto {
                    h.push_str("</section>");
                }
                n += 1;
                h.push_str(&format!(r#"<h2>{n}. {}</h2><section class="cartao">"#, esc(t)));
                aberto = true;
                continue;
            }
            _ if !aberto => {
                h.push_str(r#"<section class="cartao">"#);
                aberto = true;
            }
            _ => {}
        }
        match b {
            Bloco::Titulo(_) => {}
            Bloco::Texto(t) if t.contains("  ") => h.push_str(&format!(r#"<pre class="tab">{}</pre>"#, esc(t))),
            Bloco::Texto(t) if t.is_empty() => {}
            Bloco::Texto(t) => h.push_str(&format!(r#"<p class="txt">{}</p>"#, esc(t))),
            Bloco::Formula(f) => {
                h.push_str(&format!(r#"<div class="formula"><div class="ftit">{}</div>"#, esc(&f.titulo)));
                h.push_str(&format!(
                    r#"<div class="passo"><span class="rot">Equação</span><span class="mat">{}</span></div>"#,
                    esc(&f.equacao)
                ));
                if !f.substituicao.is_empty() {
                    h.push_str(&format!(
                        r#"<div class="passo"><span class="rot">Substituição</span><span class="mat">{}</span></div>"#,
                        esc(&f.substituicao)
                    ));
                }
                h.push_str(&format!(
                    r#"<div class="passo res"><span class="rot">Resultado</span><span class="mat">{}</span></div></div>"#,
                    esc(&f.resultado)
                ));
            }
            Bloco::Norma(r) => h.push_str(&format!(
                r#"<blockquote class="norma"><div class="ncab">§ {} — {}</div><p>{}</p></blockquote>"#,
                esc(r.norma),
                esc(r.item),
                esc(r.requisito)
            )),
            Bloco::Verif(i) => h.push_str(&linha_verif(&res.verificacoes[*i])),
        }
    }
    if aberto {
        h.push_str("</section>");
    }

    h.push_str(
        "<footer>As caixas de referência normativa apresentam sínteses dos requisitos e não substituem o texto \
         integral das normas ABNT. Ferramenta de apoio ao cálculo: o projeto é de responsabilidade do engenheiro \
         que emite a ART.</footer></main></body></html>",
    );
    h
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::resultado::RefNorma;

    #[test]
    fn relatorio_contem_blocos() {
        let mut r = Resultado::default();
        r.titulo("Estabilidade <externa>");
        r.norma(RefNorma { norma: "ABNT NBR 11682:2009", item: "Tabela 4", requisito: "FS ≥ 2,0" });
        r.formula("Tombamento", "FS = ΣM_est / ΣM_inst", "FS = 10 / 5", "FS = 2,00");
        r.verif(Verificacao::min("FS tombamento", 2.0, 2.0, ""));
        r.verif(Verificacao::min("FS deslizamento", 1.2, 1.5, ""));
        r.aviso("teste");
        let h = gerar(&r, "Projeto & Cia", "0.2.0");
        assert!(h.starts_with("<!doctype html>") && h.ends_with("</html>"));
        assert!(h.contains("Estabilidade &lt;externa&gt;") && h.contains("Projeto &amp; Cia"));
        assert!(h.contains(r#"<span class="badge ok">ATENDIDO</span>"#));
        assert!(h.contains(r#"<span class="badge falha">NÃO ATENDIDO</span>"#));
        assert!(h.contains("NÃO ATENDE — 1 verificação(ões)"));
        assert!(h.contains("1,20 ≥ 1,50") && h.contains("Substituição"));
        assert_eq!(h.matches("<section").count(), h.matches("</section>").count());
    }
}
