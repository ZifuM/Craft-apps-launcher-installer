//! Additional saved-content previews. Never executes document scripts or follows external URLs.
use std::{fs::File, io::{Read, Cursor}, path::Path, sync::{Arc, OnceLock}};
use image::RgbaImage;
use base64::Engine;
const LIMIT: u64 = 32 * 1024 * 1024;
pub type Preview = (RgbaImage, &'static str);
fn read(path: &Path) -> Option<Vec<u8>> {
    let mut bytes = Vec::new(); File::open(path).ok()?.take(LIMIT + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= LIMIT).then_some(bytes)
}
fn raster(bytes: &[u8]) -> Option<RgbaImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?;
    let mut limits = image::Limits::default(); limits.max_image_width = Some(30_000); limits.max_image_height = Some(30_000); limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().ok().map(|im| im.thumbnail(640,400).to_rgba8())
}
fn svg(bytes: &[u8]) -> Option<RgbaImage> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    let fonts = FONTS.get_or_init(|| { let mut db = resvg::usvg::fontdb::Database::new(); db.load_system_fonts(); Arc::new(db) });
    let mut opts = resvg::usvg::Options::default(); opts.fontdb = fonts.clone();
    opts.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let tree = resvg::usvg::Tree::from_data(bytes, &opts).ok()?;
    let size = tree.size(); let scale = (640.0 / size.width()).min(400.0 / size.height()).min(1.0);
    let mut pixmap = resvg::tiny_skia::Pixmap::new((size.width()*scale).ceil().max(1.0) as u32, (size.height()*scale).ceil().max(1.0) as u32)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let mut result = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())?;
    for p in result.pixels_mut() { if p[3] > 0 { for i in 0..3 { p[i] = ((p[i] as u32 * 255 / p[3] as u32).min(255)) as u8; } } }
    Some(result)
}
fn escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }
fn text_page(lines: &[String], table: bool) -> Option<RgbaImage> {
    if lines.is_empty() { return None; }
    let mut xml = String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"><rect width="640" height="400" fill="#f9fafc"/>"##);
    for (i, line) in lines.iter().take(16).enumerate() {
        let y = 32 + i*23;
        if table { xml.push_str(&format!(r##"<path d="M16 {}H624" stroke="#dce1e8"/>"##,y+7)); }
        xml.push_str(&format!(r##"<text x="24" y="{y}" font-family="Segoe UI" font-size="14" fill="#263041">{}</text>"##,escape(&line.chars().take(78).collect::<String>())));
    }
    xml.push_str("</svg>"); svg(xml.as_bytes())
}
fn zip_entry(zip: &mut zip::ZipArchive<File>, name: &str) -> Option<Vec<u8>> {
    let mut entry = zip.by_name(name).ok()?; if entry.size() > LIMIT { return None; }
    let mut bytes = Vec::new(); entry.by_ref().take(LIMIT+1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= LIMIT).then_some(bytes)
}
fn xml_text(bytes: &[u8], tags: &[&str]) -> Option<Vec<String>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let doc = roxmltree::Document::parse(text).ok()?;
    let lines = doc.descendants().filter(|n|n.is_element() && tags.contains(&n.tag_name().name()))
        .map(|n|n.descendants().filter(|n|n.is_text()).filter_map(|n|n.text()).collect::<String>())
        .filter(|s| !s.trim().is_empty()).take(16).collect(); Some(lines)
}
fn packaged(path: &Path, ext: &str) -> Option<Preview> {
    let mut zip = zip::ZipArchive::new(File::open(path).ok()?).ok()?;
    let mut candidates: Vec<_> = zip.file_names().take(10_000).filter(|n| { let n=n.to_lowercase(); n.contains("thumbnail") || n.contains("preview") || n.contains("composite") }).map(str::to_owned).collect();
    candidates.sort_by_key(|n| if n.contains("composite") {0} else if n.contains("preview"){1}else{2});
    for name in candidates { if let Some(im)=zip_entry(&mut zip,&name).and_then(|b|raster(&b)){return Some((im,"Saved document preview"));} }
    let lines = match ext {
        "docx" => xml_text(&zip_entry(&mut zip,"word/document.xml")?, &["p"]),
        "odt" => xml_text(&zip_entry(&mut zip,"content.xml")?, &["p", "h"]),
        "pptx" => xml_text(&zip_entry(&mut zip,"ppt/slides/slide1.xml")?, &["p"]),
        "xlsx" => {
            let shared = zip_entry(&mut zip,"xl/sharedStrings.xml").and_then(|b|xml_text_all(&b,"si")).unwrap_or_default();
            let bytes=zip_entry(&mut zip,"xl/worksheets/sheet1.xml")?;
            let doc=roxmltree::Document::parse(std::str::from_utf8(&bytes).ok()?).ok()?;
            let rows=doc.descendants().filter(|n|n.has_tag_name("row")).take(16).map(|row|{
                row.children().filter(|n|n.has_tag_name("c")).take(8).map(|cell|{
                    let value=cell.descendants().find(|n|n.has_tag_name("v")||n.has_tag_name("t")).and_then(|n|n.text()).unwrap_or("");
                    if cell.attribute("t")==Some("s") {value.parse::<usize>().ok().and_then(|i|shared.get(i)).cloned().unwrap_or_default()} else {value.to_owned()}
                }).collect::<Vec<_>>().join("  |  ")
            }).collect();Some(rows)
        }
        _ => None,
    }?;
    Some((text_page(&lines,ext=="xlsx")?,"Content preview (simplified formatting)"))
}
fn xml_text_all(bytes:&[u8],tag:&str)->Option<Vec<String>>{
    let doc=roxmltree::Document::parse(std::str::from_utf8(bytes).ok()?).ok()?;
    Some(doc.descendants().filter(|n|n.has_tag_name(tag)).take(100_000).map(|n|n.descendants().filter(|n|n.is_text()).filter_map(|n|n.text()).collect()).collect())
}
fn vector(path: &Path) -> Option<Preview> {
    let mut bytes=read(path)?;
    if bytes.starts_with(&[0x1f,0x8b]) { let mut out=Vec::new(); flate2::read::GzDecoder::new(bytes.as_slice()).take(LIMIT+1).read_to_end(&mut out).ok()?; if out.len() as u64>LIMIT{return None;} bytes=out; }
    let mut stream=serde_json::Deserializer::from_slice(&bytes).into_iter::<serde_json::Value>();
    let value=stream.next()?.ok()?; let offset=stream.byte_offset();
    let Some(preview)=value.get("preview") else {
        // Empty artboards are valid previews, distinct from unsupported artwork.
        fn empty(node:&serde_json::Value, depth:usize)->bool {
            if depth>64{return false;}
            let kind=&node["kind"];
            matches!(kind["type"].as_str(),Some("layer"|"group")) && kind["children"].as_array().is_some_and(|c|c.iter().all(|n|empty(n,depth+1)))
        }
        let doc=&value["document"];
        if !doc["layers"].as_array()?.iter().all(|n|empty(n,0)){return None;}
        let rect=&doc["artboards"].as_array()?.first()?["rect"];
        let w=rect["x1"].as_f64()?-rect["x0"].as_f64()?;let h=rect["y1"].as_f64()?-rect["y0"].as_f64()?;
        if w<=0.0||h<=0.0{return None;}let scale=(640.0/w).min(400.0/h);
        return Some((RgbaImage::from_pixel((w*scale).max(1.0) as u32,(h*scale).max(1.0) as u32,image::Rgba([255,255,255,255])),"Empty saved artboard"));
    };
    let data=if let Some(data)=preview.get("data").and_then(|n|n.as_str()) { base64::engine::general_purpose::STANDARD.decode(data).ok()? }
    else { let tail=bytes.get(offset..)?.strip_prefix(b"\n\0VCBLOBS\0")?; let b=preview.get("blob")?.as_array()?;let start=usize::try_from(b.first()?.as_u64()?).ok()?;let len=usize::try_from(b.get(1)?.as_u64()?).ok()?;tail.get(start..start.checked_add(len)?)?.to_vec() };
    Some((raster(&data)?,"Saved artboard preview"))
}
fn raw_photo(path: &Path) -> Option<Preview> {
    // RAW containers commonly carry a camera-generated JPEG. Read a bounded prefix
    // and choose the largest decodable candidate rather than a tiny EXIF icon.
    let mut bytes=Vec::new();File::open(path).ok()?.take(64*1024*1024).read_to_end(&mut bytes).ok()?;
    let mut best:Option<RgbaImage>=None;let mut pos=0;let mut attempts=0;
    while pos+3<bytes.len() && attempts<32 {
        let Some(rel)=bytes[pos..].windows(3).position(|b|b==[255,216,255]) else{break;};let start=pos+rel;pos=start+3;attempts+=1;
        if let Some(im)=raster(&bytes[start..]) { if best.as_ref().is_none_or(|b|im.width()*im.height()>b.width()*b.height()){best=Some(im);} }
    }
    Some((best?,"Embedded camera preview (may not include edits)"))
}
fn waveform(path:&Path)->Option<Preview>{
    use symphonia::core::{io::MediaSourceStream,probe::Hint,audio::SampleBuffer};
    let mss=MediaSourceStream::new(Box::new(File::open(path).ok()?),Default::default());
    let mut hint=Hint::new();if let Some(ext)=path.extension().and_then(|s|s.to_str()){hint.with_extension(ext);}
    let mut format=symphonia::default::get_probe().format(&hint,mss,&Default::default(),&Default::default()).ok()?.format;
    let track=format.default_track()?;let id=track.id;let frames=track.codec_params.n_frames?;let rate=track.codec_params.sample_rate? as u64;
    if frames==0 || frames>rate*60*30{return None;}
    let mut decoder=symphonia::default::get_codecs().make(&track.codec_params,&Default::default()).ok()?;
    let mut peaks=[0.0f32;600];let mut frame=0u64;
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
    while let Ok(packet)=format.next_packet(){
        if std::time::Instant::now()>deadline{return None;}
        if packet.track_id()!=id {continue;}
        let decoded=decoder.decode(&packet).ok()?;let channels=decoded.spec().channels.count();if channels==0{return None;}
        let mut samples=SampleBuffer::<f32>::new(decoded.capacity() as u64,*decoded.spec());samples.copy_interleaved_ref(decoded);
        for values in samples.samples().chunks(channels){let bin=(frame.saturating_mul(600)/frames).min(599) as usize;for v in values{peaks[bin]=peaks[bin].max(v.abs().min(1.0));}frame+=1;}
    }
    if frame==0{return None;}
    let mut im=RgbaImage::from_pixel(640,240,image::Rgba([24,30,37,255]));
    for (i,p) in peaks.iter().enumerate(){let half=(p*98.0).round().max(1.0) as u32;for y in 120-half..=120+half{im.put_pixel(i as u32+20,y,image::Rgba([55,188,176,255]));}}
    Some((im,"Audio waveform"))
}
#[cfg(target_os="linux")]
fn linux_pdf(path:&Path)->Option<RgbaImage>{
    use std::{fs,process::{Command,Stdio},time::{Instant,Duration}};
    let root=crate::platform::data_dir()?.join("preview-work");fs::create_dir_all(&root).ok()?;
    let token=format!("pdf-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos());
    let directory=root.join(token);fs::create_dir(&directory).ok()?;
    let output=directory.join("page");let image_path=directory.join("page.png");
    let result=(||{
        let mut command=Command::new("pdftoppm");command.args(["-f","1","-l","1","-singlefile","-scale-to","512","-png"]).arg(path).arg(&output).stdout(Stdio::null()).stderr(Stdio::null());
        let mut process=crate::platform::spawn(&mut command).ok()?;let started=Instant::now();
        loop{if let Some(status)=process.try_wait().ok()?{if !status.success(){return None;}break;}if started.elapsed()>Duration::from_secs(10){let _=process.kill();let _=process.wait();return None;}std::thread::sleep(Duration::from_millis(25));}
        raster(&read(&image_path)?)
    })();
    let _=fs::remove_file(image_path);let _=fs::remove_dir(directory);result
}
pub fn load(path:&Path,ext:&str)->Option<Preview>{
    let result=match ext {
        "svg"=>read(path).and_then(|b|svg(&b)).map(|p|(p,"SVG preview (embedded assets only)")),
        "vectorcraft"=>vector(path),
        "dxf"=>dxf(path),
        "wav"|"aif"|"aiff"|"flac"=>waveform(path),
        "dng"|"cr2"|"cr3"|"nef"|"arw"|"raf"|"orf"|"rw2"|"pef"=>raw_photo(path),
        "pcraft"|"designcraft"|"deckcraft"|"dcbook"|"idml"|"docx"|"odt"|"pptx"|"xlsx"|"fcproj"|"ecproj"=>packaged(path,ext),
        "txt"|"md"|"csv"|"tsv"=>read(path).and_then(|b|String::from_utf8(b).ok()).and_then(|s|text_page(&s.lines().take(16).map(|s|s.replace('\t',"   |   ")).collect::<Vec<_>>(),matches!(ext,"csv"|"tsv"))).map(|p|(p,"Content preview (simplified formatting)")),
        _=>None,
    };
    if result.is_some(){return result;}
    #[cfg(target_os="linux")]
    if matches!(ext,"pdf"|"ai"){if let Some(image)=linux_pdf(path){return Some((image,"First page preview"));}}
    #[cfg(target_os="windows")]
    {
        if matches!(ext,"pdf"|"ai") {if let Some(im)=crate::preview_windows::pdf(path){return Some((im,"First page preview"));}}
        if let Some(im)=crate::preview_windows::shell(path){return Some((im,"Windows document thumbnail"));}
    }
    None
}
// A conservative 2D DXF reader. Unsupported entities cause a fallback to the
// Windows handler rather than silently disappearing from the preview.
fn dxf(path:&Path)->Option<Preview>{
    let bytes=read(path)?;let text=std::str::from_utf8(&bytes).ok()?;
    let lines:Vec<_>=text.lines().collect();if lines.len()%2!=0{return None;}
    let pairs:Vec<(i32,&str)>=lines.chunks_exact(2).map(|p|Some((p[0].trim().parse().ok()?,p[1].trim()))).collect::<Option<_>>()?;
    let start=pairs.windows(2).position(|p|p[0]==(0,"SECTION")&&p[1]==(2,"ENTITIES"))?+2;
    let mut shapes=Vec::new();let mut bounds=[f64::INFINITY,f64::INFINITY,f64::NEG_INFINITY,f64::NEG_INFINITY];let mut at=start;
    while at<pairs.len(){
        if pairs[at]==(0,"ENDSEC"){break;}
        if pairs[at].0!=0 {at+=1;continue;}
        let kind=pairs[at].1;let end=(at+1..pairs.len()).find(|i|pairs[*i].0==0).unwrap_or(pairs.len());let fields=&pairs[at+1..end];
        let number=|code|fields.iter().find(|(c,_)|*c==code).and_then(|(_,v)|v.parse::<f64>().ok()).filter(|v|v.is_finite()&&v.abs()<1e12);
        let mut point=|x:f64,y:f64|{bounds[0]=bounds[0].min(x);bounds[1]=bounds[1].min(y);bounds[2]=bounds[2].max(x);bounds[3]=bounds[3].max(y);};
        match kind{
            "LINE"=>{let (x,y,u,v)=(number(10)?,number(20)?,number(11)?,number(21)?);point(x,y);point(u,v);shapes.push(format!(r#"<path d="M{x} {y}L{u} {v}"/>"#));}
            "CIRCLE"=>{let (x,y,r)=(number(10)?,number(20)?,number(40)?);if r<=0.0{return None;}point(x-r,y-r);point(x+r,y+r);shapes.push(format!(r#"<circle cx="{x}" cy="{y}" r="{r}"/>"#));}
            "LWPOLYLINE"=>{
                if fields.iter().any(|(c,v)|*c==42 && v.parse::<f64>().unwrap_or(1.0)!=0.0){return None;}
                let mut d=String::new();let mut x=None;
                for (code,value) in fields{if *code==10{x=Some(value.parse::<f64>().ok()?);}if *code==20{let (x,y)=(x.take()?,value.parse::<f64>().ok()?);if !x.is_finite()||!y.is_finite(){return None;}point(x,y);d.push_str(&format!("{}{} {} ",if d.is_empty(){"M"}else{"L"},x,y));}}
                if number(70).unwrap_or(0.0) as u32 & 1 != 0{d.push('Z');}shapes.push(format!(r#"<path d="{d}"/>"#));
            }
            _=>return None,
        }
        if shapes.len()>20_000{return None;}at=end;
    }
    if shapes.is_empty(){return None;}
    let w=(bounds[2]-bounds[0]).max(1.0);let h=(bounds[3]-bounds[1]).max(1.0);let pad=w.max(h)*0.04;
    let xml=format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400" viewBox="{} {} {} {}"><rect x="{}" y="{}" width="{}" height="{}" fill="#fafbfd"/><g transform="translate(0,{}) scale(1,-1)" fill="none" stroke="#26627b" stroke-width="{}">{}</g></svg>"##,bounds[0]-pad,bounds[1]-pad,w+2.0*pad,h+2.0*pad,bounds[0]-pad,bounds[1]-pad,w+2.0*pad,h+2.0*pad,bounds[1]+bounds[3],w.max(h)/450.0,shapes.join(""));
    Some((svg(xml.as_bytes())?,"2D drawing preview"))
}
