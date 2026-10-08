//! Windows-owned document rendering; invoked only by the background project scan.
use std::path::Path;
use image::RgbaImage;
use windows::{core::HSTRING, Win32::{System::Com::*, Graphics::Gdi::*, UI::Shell::*, Foundation::SIZE}};
struct Apartment;
impl Apartment {
    fn new() -> Option<Self> { unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok()?; } Some(Self) }
}
impl Drop for Apartment { fn drop(&mut self) { unsafe { CoUninitialize(); } } }
struct Bitmap(HBITMAP);
impl Drop for Bitmap { fn drop(&mut self) { unsafe { let _ = DeleteObject(self.0); } } }

pub fn shell(path: &Path) -> Option<RgbaImage> {
    let _com = Apartment::new()?;
    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).ok()?;
        let bitmap = Bitmap(factory.GetImage(SIZE { cx: 640, cy: 400 }, SIIGBF_THUMBNAILONLY).ok()?);
        let mut info = BITMAP::default();
        if GetObjectW(bitmap.0, std::mem::size_of::<BITMAP>() as i32, Some((&mut info as *mut BITMAP).cast())) == 0 { return None; }
        let w = info.bmWidth; let h = info.bmHeight.abs();
        if w <= 0 || h <= 0 || w > 4096 || h > 4096 { return None; }
        let mut header = BITMAPINFO::default();
        header.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() };
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() { return None; }
        let mut pixels = vec![0u8; w as usize * h as usize * 4];
        let rows = GetDIBits(dc, bitmap.0, 0, h as u32, Some(pixels.as_mut_ptr().cast()), &mut header, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        if rows != h { return None; }
        let opaque = pixels.chunks_exact(4).all(|p| p[3] == 0);
        for p in pixels.chunks_exact_mut(4) {
            p.swap(0, 2);
            if opaque { p[3] = 255; }
            else if p[3] > 0 { for i in 0..3 { p[i] = ((p[i] as u32 * 255 / p[3] as u32).min(255)) as u8; } }
        }
        Some(image::DynamicImage::ImageRgba8(RgbaImage::from_raw(w as u32, h as u32, pixels)?).thumbnail(640,400).to_rgba8())
    }
}

pub fn pdf(path: &Path) -> Option<RgbaImage> {
    use windows::{Data::Pdf::{PdfDocument, PdfPageRenderOptions}, Storage::{StorageFile, Streams::{InMemoryRandomAccessStream, DataReader}}};
    let _com = Apartment::new()?;
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str())).ok()?.get().ok()?;
    let doc = PdfDocument::LoadFromFileAsync(&file).ok()?.get().ok()?;
    if doc.PageCount().ok()? == 0 { return None; }
    let page = doc.GetPage(0).ok()?;
    let size = page.Size().ok()?;
    let scale = (640.0 / size.Width).min(400.0 / size.Height);
    if !scale.is_finite() || scale <= 0.0 { return None; }
    let opts = PdfPageRenderOptions::new().ok()?;
    opts.SetDestinationWidth((size.Width * scale).max(1.0) as u32).ok()?;
    opts.SetDestinationHeight((size.Height * scale).max(1.0) as u32).ok()?;
    let stream = InMemoryRandomAccessStream::new().ok()?;
    page.RenderWithOptionsToStreamAsync(&stream, &opts).ok()?.get().ok()?;
    let len = stream.Size().ok()?;
    if len > 10 * 1024 * 1024 { return None; }
    let input = stream.GetInputStreamAt(0).ok()?;
    let reader = DataReader::CreateDataReader(&input).ok()?;
    reader.LoadAsync(len as u32).ok()?.get().ok()?;
    let mut bytes = vec![0; len as usize]; reader.ReadBytes(&mut bytes).ok()?;
    let _ = page.Close();
    image::load_from_memory(&bytes).ok().map(|p|p.to_rgba8())
}
