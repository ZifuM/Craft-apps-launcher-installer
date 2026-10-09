//! Display-only localization. Persisted IDs, paths and app names remain unchanged.
use std::{collections::HashMap, sync::{OnceLock, atomic::{AtomicUsize, Ordering}}};
use eframe::egui;
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);
static CATALOG: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
pub const LANGUAGES: [(&str, &str); 10] = [
    ("en", "English"), ("es", "Español"), ("fr", "Français"), ("de", "Deutsch"),
    ("pt", "Português"), ("ru", "Русский"), ("zh", "简体中文"), ("ja", "日本語"),
    ("ko", "한국어"), ("it", "Italiano"),
];
pub fn select(code: &str) { LANGUAGE.store(LANGUAGES.iter().position(|(id,_)| *id==code).unwrap_or(0),Ordering::Relaxed); }
pub fn name(code: &str) -> &'static str { LANGUAGES.iter().find(|(id,_)| *id==code).map(|(_,name)| *name).unwrap_or("English") }
pub fn tr(text: impl ToString) -> String {
    let owned=text.to_string();
    let text=owned.as_str();
    let language=LANGUAGE.load(Ordering::Relaxed);
    if language==0 { return text.to_owned(); }
    let catalog=CATALOG.get_or_init(|| {
        let mut rows: HashMap<String,Vec<String>>=serde_json::from_str(include_str!("../assets/translations.json")).expect("embedded translation catalog");
        let lower: Vec<_>=rows.iter().filter(|(key,_)| !key.contains("{0}")).map(|(key,row)|(key.to_lowercase(),row.clone())).collect();
        rows.extend(lower); rows
    });
    if let Some(row)=catalog.get(text) { return row[language-1].clone(); }
    let trimmed=text.trim();
    if let Some(row)=catalog.get(trimmed) { return row[language-1].clone(); }
    // Eyebrows use the same catalog as their title-case equivalents.
    if let Some(row)=catalog.get(&trimmed.to_lowercase()) { return row[language-1].clone(); }
    for separator in ["  /  ", "   /   "] {
        if text.contains(separator) { return text.split(separator).map(tr).collect::<Vec<_>>().join(separator); }
    }
    // Only explicitly catalogued display templates are interpolated.
    let mut templates: Vec<_> = catalog.iter().filter(|(key,_)| key.contains("{0}")).collect();
    templates.sort_by_key(|(key,_)| std::cmp::Reverse(key.len()));
    for (key,row) in templates {
        if let Some(values)=capture(key, text) {
            let mut result=row[language-1].clone();
            for (i,value) in values.iter().enumerate() { result=result.replace(&format!("{{{i}}}"),value); }
            return result;
        }
    }
    text.to_owned()
}
fn capture(template: &str, text: &str) -> Option<Vec<String>> {
    let mut rest=text; let mut pattern=template; let mut values=Vec::new();
    for i in 0..8 {
        let token=format!("{{{i}}}");
        let Some((prefix,tail))=pattern.split_once(&token) else { return if rest==pattern {Some(values)} else {None}; };
        rest=rest.strip_prefix(prefix)?;
        let next=format!("{{{}}}",i+1);
        let suffix=tail.split_once(&next).map(|(prefix,_)|prefix).unwrap_or(tail);
        if suffix.is_empty() {
            if !tail.is_empty() {return None;}
            values.push(rest.to_owned());return Some(values);
        }
        let end=rest.find(suffix)?;
        values.push(rest[..end].to_owned());rest=&rest[end..];pattern=tail;
    }
    None
}
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts=egui::FontDefinitions::default();
    let windows=std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(||"C:/Windows".into());
    // Use installed Windows fonts; no proprietary font files are redistributed.
    for (name,candidates) in [("chinese", &["msyh.ttc","simsun.ttc"][..]), ("japanese", &["YuGothM.ttc","msgothic.ttc"][..]), ("korean", &["malgun.ttf","gulim.ttc"][..])] {
        for candidate in candidates {
            if let Ok(bytes)=std::fs::read(windows.join("Fonts").join(candidate)) {
                fonts.font_data.insert(name.into(),egui::FontData::from_owned(bytes).into());
                fonts.families.entry(egui::FontFamily::Proportional).or_default().push(name.into());
                fonts.families.entry(egui::FontFamily::Monospace).or_default().push(name.into());
                break;
            }
        }
    }
    #[cfg(target_os="linux")]
    for (name,candidates) in [("noto-cjk",["/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc","/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc","/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc"])] {
        for path in candidates {if let Ok(bytes)=std::fs::read(path){fonts.font_data.insert(name.into(),egui::FontData::from_owned(bytes).into());fonts.families.entry(egui::FontFamily::Proportional).or_default().push(name.into());break;}}
    }
    #[cfg(target_os="macos")]
    for (name,path) in [("mac-chinese","/System/Library/Fonts/PingFang.ttc"),("mac-japanese","/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"),("mac-korean","/System/Library/Fonts/AppleSDGothicNeo.ttc")] {
        if let Ok(bytes)=std::fs::read(path){fonts.font_data.insert(name.into(),egui::FontData::from_owned(bytes).into());fonts.families.entry(egui::FontFamily::Proportional).or_default().push(name.into());}
    }
    ctx.set_fonts(fonts);
}
