//! Verificação de nova versão publicada nas Releases do GitHub.
//! A consulta roda em segundo plano ao abrir o programa e não bloqueia a
//! interface. Falhas (sem internet, repositório privado) são silenciosas.

use crate::sobre;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum Estado {
    Verificando,
    Atualizado,
    NovaVersao { tag: String, url: String },
    Indisponivel(String),
}

pub struct Verificador {
    estado: Arc<Mutex<Estado>>,
}

impl Verificador {
    pub fn iniciar(ctx: egui::Context) -> Self {
        let estado = Arc::new(Mutex::new(Estado::Verificando));
        let e2 = estado.clone();
        std::thread::spawn(move || {
            let r = consultar();
            if let Ok(mut e) = e2.lock() {
                *e = r;
            }
            ctx.request_repaint();
        });
        Self { estado }
    }

    pub fn estado(&self) -> Estado {
        self.estado.lock().map(|e| e.clone()).unwrap_or(Estado::Indisponivel("erro interno".into()))
    }
}

/// Converte "v0.2.0-build.12" → (0, 2, 0, 12). Sem build → build 0.
pub fn ler_versao(tag: &str) -> Option<(u64, u64, u64, u64)> {
    let t = tag.trim().trim_start_matches(['v', 'V']);
    let (ver, build) = match t.split_once("-build.") {
        Some((v, b)) => (v, b.parse().ok()?),
        None => (t, 0),
    };
    let mut p = ver.split('.').map(|x| x.parse::<u64>());
    Some((p.next()?.ok()?, p.next()?.ok()?, p.next().unwrap_or(Ok(0)).ok()?, build))
}

pub fn versao_atual() -> (u64, u64, u64, u64) {
    let tag = match sobre::BUILD {
        Some(b) => format!("{}-build.{b}", sobre::VERSAO),
        None => sobre::VERSAO.to_string(),
    };
    ler_versao(&tag).unwrap_or((0, 0, 0, 0))
}

fn consultar() -> Estado {
    if sobre::REPOSITORIO.is_empty() {
        return Estado::Indisponivel("repositório não configurado".into());
    }
    let url = format!("https://api.github.com/repos/{}/releases/latest", sobre::REPOSITORIO);
    let resp = ureq::get(&url)
        .set("User-Agent", "CalculadoraContencoes-atualizador")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(8))
        .call();
    match resp {
        Ok(r) => match r.into_json::<serde_json::Value>() {
            Ok(v) => {
                let tag = v["tag_name"].as_str().unwrap_or_default().to_string();
                let pagina = v["html_url"].as_str().unwrap_or_default().to_string();
                match ler_versao(&tag) {
                    Some(nova) if nova > versao_atual() => Estado::NovaVersao { tag, url: pagina },
                    Some(_) => Estado::Atualizado,
                    None => Estado::Indisponivel(format!("tag não reconhecida: {tag}")),
                }
            }
            Err(e) => Estado::Indisponivel(e.to_string()),
        },
        Err(ureq::Error::Status(404, _)) => Estado::Indisponivel("repositório privado ou sem releases".into()),
        Err(e) => Estado::Indisponivel(e.to_string()),
    }
}

/// Aviso clicável de nova versão (ou nada).
pub fn aviso(ui: &mut egui::Ui, estado: &Estado) {
    if let Estado::NovaVersao { tag, url } = estado {
        let p = crate::ui::tema::de(ui);
        let b = egui::Button::new(egui::RichText::new(format!("⬇  Nova versão {tag} disponível — baixar")).color(egui::Color32::WHITE))
            .fill(p.ok)
            .corner_radius(6.0);
        if ui.add(b).on_hover_text(url.as_str()).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn comparacao_de_versoes() {
        assert_eq!(ler_versao("v0.2.0-build.12"), Some((0, 2, 0, 12)));
        assert_eq!(ler_versao("v1.0.0"), Some((1, 0, 0, 0)));
        assert!(ler_versao("v0.2.0-build.13") > ler_versao("v0.2.0-build.12"));
        assert!(ler_versao("v0.3.0-build.1") > ler_versao("v0.2.0-build.99"));
        assert_eq!(ler_versao("lixo"), None);
    }
}
