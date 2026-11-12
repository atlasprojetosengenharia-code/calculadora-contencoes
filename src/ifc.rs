//! Exportação do modelo 3D para IFC4 (ISO 16739, arquivo STEP-21).
//!
//! Cada peça vira um elemento IFC (IfcWall, IfcFooting, IfcPile, IfcMember,
//! IfcGeographicElement) com geometria triangulada (IfcTriangulatedFaceSet),
//! cor e material. Eixos IFC: X = seção, Y = ao longo da contenção, Z = vertical.

use crate::modelo3d::{Classe, Modelo3d};
use crate::projeto::Projeto;
use std::fmt::Write;

/// Texto IFC: apóstrofo duplicado, barra invertida e não-ASCII codificados (\X2\hhhh\X0\).
fn texto(s: &str) -> String {
    let mut o = String::from("'");
    for c in s.chars() {
        match c {
            '\'' => o.push_str("''"),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 32 => o.push(' '),
            c if c.is_ascii() => o.push(c),
            c => {
                let mut b = [0u16; 2];
                o.push_str("\\X2\\");
                for u in c.encode_utf16(&mut b) {
                    let _ = write!(o, "{:04X}", u);
                }
                o.push_str("\\X0\\");
            }
        }
    }
    o.push('\'');
    o
}

fn real(x: f64) -> String {
    let s = format!("{:.6}", x);
    let s = s.trim_end_matches('0');
    if s.ends_with('.') { format!("{s}0") } else { s.to_string() }
}

/// Gerador de GlobalId (128 bits em base 64 do IFC, 22 caracteres).
struct Guids(u64);

impl Guids {
    fn proximo(&mut self) -> String {
        let mut sm = || {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let n = ((sm() as u128) << 64) | sm() as u128;
        const B: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
        let mut s = String::with_capacity(24);
        s.push('\'');
        for i in 0..22 {
            let d = ((n >> (6 * (21 - i))) & 63) as usize;
            s.push(B[d] as char);
        }
        s.push('\'');
        s
    }
}

struct Arquivo {
    linhas: Vec<String>,
}

impl Arquivo {
    fn add(&mut self, e: String) -> usize {
        self.linhas.push(e);
        self.linhas.len()
    }
}

/// Gera o conteúdo do arquivo IFC.
pub fn gerar(m: &Modelo3d, proj: &Projeto) -> String {
    let semente = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
    let mut g = Guids(semente);
    let mut a = Arquivo { linhas: Vec::new() };

    let u1 = a.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)".into());
    let u2 = a.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)".into());
    let u3 = a.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)".into());
    let u4 = a.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)".into());
    let unidades = a.add(format!("IFCUNITASSIGNMENT((#{u1},#{u2},#{u3},#{u4}))"));
    let origem = a.add("IFCCARTESIANPOINT((0.,0.,0.))".into());
    let dz = a.add("IFCDIRECTION((0.,0.,1.))".into());
    let dx = a.add("IFCDIRECTION((1.,0.,0.))".into());
    let eixo = a.add(format!("IFCAXIS2PLACEMENT3D(#{origem},#{dz},#{dx})"));
    let ctx = a.add(format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{eixo},$)"));
    let corpo = a.add(format!("IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{ctx},$,.MODEL_VIEW.,$)"));
    let nome = if proj.nome.trim().is_empty() { "Contenção" } else { proj.nome.trim() };
    let projeto = a.add(format!(
        "IFCPROJECT({},$,{},{},$,$,$,(#{ctx}),#{unidades})",
        g.proximo(),
        texto(nome),
        texto(&format!("{} — Calculadora de Contenções", proj.tipo.nome()))
    ));
    let lp_site = a.add(format!("IFCLOCALPLACEMENT($,#{eixo})"));
    let local = if proj.local.trim().is_empty() { "Local da obra" } else { proj.local.trim() };
    let site = a.add(format!("IFCSITE({},$,{},$,$,#{lp_site},$,$,.ELEMENT.,$,$,$,$,$)", g.proximo(), texto(local)));
    a.add(format!("IFCRELAGGREGATES({},$,$,$,#{projeto},(#{site}))", g.proximo()));

    let mut elementos = Vec::new();
    let mut por_material: Vec<(&'static str, Vec<usize>)> = Vec::new();
    for pc in &m.pecas {
        let tris = pc.forma.triangulos(12);
        if tris.is_empty() {
            continue;
        }
        // vértices únicos (IFC: X = x, Y = z, Z = y)
        let mut pts: Vec<[f64; 3]> = Vec::new();
        let mut faces = Vec::new();
        for t in &tris {
            let mut f = [0usize; 3];
            for (k, v) in t.iter().enumerate() {
                let q = [v[0], v[2], v[1]];
                let i = pts.iter().position(|p| (p[0] - q[0]).abs() < 1e-7 && (p[1] - q[1]).abs() < 1e-7 && (p[2] - q[2]).abs() < 1e-7).unwrap_or_else(|| {
                    pts.push(q);
                    pts.len() - 1
                });
                f[k] = i + 1;
            }
            // (x,y,z) → (x,z,y) inverte a orientação: troca dois índices
            faces.push(format!("({},{},{})", f[0], f[2], f[1]));
        }
        let lista = pts.iter().map(|p| format!("({},{},{})", real(p[0]), real(p[1]), real(p[2]))).collect::<Vec<_>>().join(",");
        let pl = a.add(format!("IFCCARTESIANPOINTLIST3D(({lista}))"));
        let fechado = if pc.classe == Classe::Terreno { ".F." } else { ".T." };
        let fs = a.add(format!("IFCTRIANGULATEDFACESET(#{pl},$,{fechado},({}),$)", faces.join(",")));
        let ([r, gr, b], transp) = pc.material.cor();
        let cor = a.add(format!("IFCCOLOURRGB($,{},{},{})", real(r), real(gr), real(b)));
        let sh = a.add(format!("IFCSURFACESTYLESHADING(#{cor},{})", real(transp)));
        let st = a.add(format!("IFCSURFACESTYLE({},.BOTH.,(#{sh}))", texto(pc.material.nome())));
        a.add(format!("IFCSTYLEDITEM(#{fs},(#{st}),$)"));
        let sr = a.add(format!("IFCSHAPEREPRESENTATION(#{corpo},'Body','Tessellation',(#{fs}))"));
        let pds = a.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{sr}))"));
        let lp = a.add(format!("IFCLOCALPLACEMENT(#{lp_site},#{eixo})"));
        let (gid, n) = (g.proximo(), texto(&pc.nome));
        let el = a.add(match pc.classe {
            Classe::Muro => format!("IFCWALL({gid},$,{n},$,'MURO DE CONTENCAO',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Sapata => format!("IFCFOOTING({gid},$,{n},$,$,#{lp},#{pds},$,.STRIP_FOOTING.)"),
            Classe::Estaca => format!("IFCPILE({gid},$,{n},$,$,#{lp},#{pds},$,.BORED.,$)"),
            Classe::EstacaPrancha => format!("IFCPILE({gid},$,{n},$,'ESTACA-PRANCHA',#{lp},#{pds},$,.USERDEFINED.,$)"),
            Classe::Tirante => format!("IFCMEMBER({gid},$,{n},$,'TIRANTE',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Grampo => format!("IFCMEMBER({gid},$,{n},$,'GRAMPO',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Terreno => format!("IFCGEOGRAPHICELEMENT({gid},$,{n},$,$,#{lp},#{pds},$,.TERRAIN.)"),
            Classe::Placa => format!("IFCPLATE({gid},$,{n},$,'PLACA DE ANCORAGEM',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Cabeca => format!("IFCMECHANICALFASTENER({gid},$,{n},$,$,#{lp},#{pds},$,$,$,.ANCHORBOLT.)"),
            Classe::Tubo => format!("IFCPIPESEGMENT({gid},$,{n},$,$,#{lp},#{pds},$,.RIGIDSEGMENT.)"),
            Classe::Filtro => format!("IFCBUILDINGELEMENTPROXY({gid},$,{n},$,'FILTRO DRENANTE',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Canaleta => format!("IFCBUILDINGELEMENTPROXY({gid},$,{n},$,'CANALETA',#{lp},#{pds},$,.USERDEFINED.)"),
            Classe::Bloco => format!("IFCFOOTING({gid},$,{n},$,$,#{lp},#{pds},$,.PILE_CAP.)"),
            Classe::Pilar => format!("IFCCOLUMN({gid},$,{n},$,$,#{lp},#{pds},$,.COLUMN.)"),
            Classe::Viga => format!("IFCBEAM({gid},$,{n},$,$,#{lp},#{pds},$,.BEAM.)"),
        });
        elementos.push(el);
        let mn = pc.material.nome();
        match por_material.iter_mut().find(|(k, _)| *k == mn) {
            Some((_, v)) => v.push(el),
            None => por_material.push((mn, vec![el])),
        }
    }
    let refs = |v: &[usize]| v.iter().map(|i| format!("#{i}")).collect::<Vec<_>>().join(",");
    if !elementos.is_empty() {
        a.add(format!("IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,({}),#{site})", g.proximo(), refs(&elementos)));
    }
    for (mn, els) in &por_material {
        let mat = a.add(format!("IFCMATERIAL({},$,$)", texto(mn)));
        a.add(format!("IFCRELASSOCIATESMATERIAL({},$,$,$,({}),#{mat})", g.proximo(), refs(els)));
    }

    let agora = data_iso();
    let mut s = String::new();
    s.push_str("ISO-10303-21;\nHEADER;\n");
    s.push_str("FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n");
    let _ = writeln!(
        s,
        "FILE_NAME({},'{}',(''),(''),'Calculadora de Contencoes {}','Calculadora de Contencoes','');",
        texto(&format!("{nome}.ifc")),
        agora,
        env!("CARGO_PKG_VERSION")
    );
    s.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    for (i, l) in a.linhas.iter().enumerate() {
        let _ = writeln!(s, "#{}={};", i + 1, l);
    }
    s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    s
}

/// Data e hora UTC no formato ISO 8601 (sem dependências externas).
fn data_iso() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (dias, seg) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    // algoritmo de Howard Hinnant (civil_from_days)
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + if mes <= 2 { 1 } else { 0 };
    format!("{ano:04}-{mes:02}-{d:02}T{:02}:{:02}:{:02}", seg / 3600, seg % 3600 / 60, seg % 60)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::projeto::{TipoContencao, TIPOS_CONTENCAO};

    #[test]
    fn texto_codifica_acentos() {
        assert_eq!(texto("Contenção d'água"), "'Conten\\X2\\00E7\\X0\\\\X2\\00E3\\X0\\o d''\\X2\\00E1\\X0\\gua'");
    }

    #[test]
    fn guid_tem_22_caracteres_e_primeiro_ate_3() {
        let mut g = Guids(42);
        let a = g.proximo();
        let b = g.proximo();
        assert_eq!(a.len(), 24);
        assert_ne!(a, b);
        assert!("0123".contains(a.chars().nth(1).unwrap()));
    }

    #[test]
    fn ifc_valido_para_todos_os_tipos() {
        for tipo in TIPOS_CONTENCAO {
            let mut p = Projeto::default();
            p.tipo = tipo;
            p.geometria.h = 4.0;
            p.pre_dimensionar();
            let r = p.calcular();
            let m = Modelo3d::montar(&p, &r, 6.0, false);
            let s = gerar(&m, &p);
            assert!(s.starts_with("ISO-10303-21;") && s.trim_end().ends_with("END-ISO-10303-21;"));
            assert!(s.contains("FILE_SCHEMA(('IFC4'))"));
            assert!(s.contains("IFCPROJECT(") && s.contains("IFCSITE(") && s.contains("IFCRELCONTAINEDINSPATIALSTRUCTURE("));
            assert!(s.is_ascii(), "STEP-21 é ASCII");
            // toda referência #n aponta para uma entidade existente
            let n = s.lines().filter(|l| l.starts_with('#')).count();
            for l in s.lines().filter(|l| l.starts_with('#')) {
                let corpo = &l[l.find('=').unwrap()..];
                for parte in corpo.split('#').skip(1) {
                    let k: usize = parte.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap();
                    assert!(k >= 1 && k <= n, "{}: referência #{k} inexistente", tipo.nome());
                }
            }
            match tipo {
                TipoContencao::Estacas => assert!(s.contains("IFCPILE(")),
                TipoContencao::Flexao | TipoContencao::Gravidade => assert!(s.contains("IFCFOOTING(") && s.contains("IFCWALL(")),
                TipoContencao::Atirantada => assert!(s.contains("'TIRANTE'") && s.contains("IFCPLATE(") && s.contains("IFCMECHANICALFASTENER(") && s.contains("IFCPIPESEGMENT(")),
                TipoContencao::Grampeado => assert!(s.contains("'GRAMPO'")),
                _ => {}
            }
        }
    }
}
