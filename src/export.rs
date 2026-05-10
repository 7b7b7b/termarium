use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, OnceLock,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
    time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use directories::UserDirs;
use image::{Delay, Frame as ImageFrame, codecs::gif::GifEncoder};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};
use resvg::{tiny_skia, usvg};
use unicode_width::UnicodeWidthStr;

use crate::{app::ViewMode, config::Location};

pub const IMAGE_TERMINAL_WIDTH: u16 = 160;
pub const IMAGE_PIXEL_WIDTH: u32 = 2160;
pub const IMAGE_PIXEL_HEIGHT: u32 = 2160;
pub const ANIMATION_PIXEL_WIDTH: u32 = 1080;
pub const ANIMATION_PIXEL_HEIGHT: u32 = 1080;
pub const ANIMATION_FPS: u32 = 15;
pub const ANIMATION_MAX_DURATION: StdDuration = StdDuration::from_secs(60);
pub const GIF_MAX_DURATION: StdDuration = StdDuration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSize {
    Standard,
    Wide,
    UltraWide,
    Portrait,
    Tall,
    LargeSquare,
}

impl ExportSize {
    pub const ALL: [Self; 6] = [
        Self::Standard,
        Self::Wide,
        Self::UltraWide,
        Self::Portrait,
        Self::Tall,
        Self::LargeSquare,
    ];

    pub fn next(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|size| *size == self).unwrap_or(0);
        let next = if forward {
            (index + 1) % Self::ALL.len()
        } else {
            (index + Self::ALL.len() - 1) % Self::ALL.len()
        };
        Self::ALL[next]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "square",
            Self::Wide => "wide",
            Self::UltraWide => "cinema",
            Self::Portrait => "portrait",
            Self::Tall => "story",
            Self::LargeSquare => "large square",
        }
    }

    pub fn terminal_size(self) -> (u16, u16) {
        match self {
            Self::Standard => (IMAGE_TERMINAL_WIDTH, 80),
            Self::Wide => (192, 54),
            Self::UltraWide => (240, 51),
            Self::Portrait => (108, 96),
            Self::Tall => (96, 128),
            Self::LargeSquare => (220, 110),
        }
    }

    pub fn image_pixels(self) -> (u32, u32) {
        match self {
            Self::Standard => (IMAGE_PIXEL_WIDTH, IMAGE_PIXEL_HEIGHT),
            Self::Wide => (2400, 1350),
            Self::UltraWide => (2560, 1080),
            Self::Portrait => (1350, 2400),
            Self::Tall => (1080, 1920),
            Self::LargeSquare => (2880, 2880),
        }
    }

    pub fn animation_pixels(self) -> (u32, u32) {
        match self {
            Self::Standard => (ANIMATION_PIXEL_WIDTH, ANIMATION_PIXEL_HEIGHT),
            Self::Wide => (1280, 720),
            Self::UltraWide => (1280, 540),
            Self::Portrait => (720, 1280),
            Self::Tall => (608, 1080),
            Self::LargeSquare => (1440, 1440),
        }
    }

    pub fn gif_pixels(self) -> (u32, u32) {
        match self {
            Self::Standard | Self::LargeSquare => (1080, 1080),
            Self::Wide => (1080, 608),
            Self::UltraWide => (1080, 456),
            Self::Portrait => (608, 1080),
            Self::Tall => (540, 960),
        }
    }

    pub fn output_pixels(self, format: ExportFormat) -> (u32, u32) {
        match format {
            ExportFormat::Png | ExportFormat::Svg => self.image_pixels(),
            ExportFormat::Gif => self.gif_pixels(),
            ExportFormat::Mp4 | ExportFormat::Webm => self.animation_pixels(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Svg,
    Gif,
    Mp4,
    Webm,
}

impl ExportFormat {
    pub const ALL: [Self; 5] = [Self::Png, Self::Svg, Self::Gif, Self::Mp4, Self::Webm];

    pub fn next(self, forward: bool) -> Self {
        let index = Self::ALL
            .iter()
            .position(|format| *format == self)
            .unwrap_or(0);
        let next = if forward {
            (index + 1) % Self::ALL.len()
        } else {
            (index + Self::ALL.len() - 1) % Self::ALL.len()
        };
        Self::ALL[next]
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Gif => "gif",
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Svg => "SVG",
            Self::Gif => "GIF",
            Self::Mp4 => "MP4",
            Self::Webm => "WebM",
        }
    }

    pub fn is_image(self) -> bool {
        matches!(self, Self::Png | Self::Svg)
    }

    pub fn needs_ffmpeg(self) -> bool {
        matches!(self, Self::Mp4 | Self::Webm)
    }
}

#[derive(Debug)]
pub struct ExportState {
    pub panel_open: bool,
    pub format: ExportFormat,
    pub size: ExportSize,
    pub path_input: String,
    pub message: String,
    pub recording: Option<RecordingState>,
    pub render_job: Option<ExportJob>,
    pub rendering_export: bool,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            panel_open: false,
            format: ExportFormat::Png,
            size: ExportSize::Standard,
            path_input: String::new(),
            message: String::new(),
            recording: None,
            render_job: None,
            rendering_export: false,
        }
    }
}

#[derive(Debug)]
pub struct RecordingState {
    pub format: ExportFormat,
    pub size: ExportSize,
    pub output_path: PathBuf,
    pub started: Instant,
    pub captured_frames: u32,
    pub frames: Vec<Buffer>,
}

impl RecordingState {
    pub fn new(format: ExportFormat, size: ExportSize, output_path: PathBuf) -> Self {
        Self {
            format,
            size,
            output_path,
            started: Instant::now(),
            captured_frames: 0,
            frames: Vec::new(),
        }
    }

    pub fn elapsed(&self) -> StdDuration {
        self.started.elapsed().min(self.max_duration())
    }

    pub fn should_capture(&self) -> bool {
        self.captured_frames < self.target_frame_count()
    }

    pub fn mark_captured(&mut self, frame: Buffer) {
        self.frames.push(frame);
        self.captured_frames += 1;
    }

    pub fn is_expired(&self) -> bool {
        self.started.elapsed() >= self.max_duration()
    }

    fn target_frame_count(&self) -> u32 {
        let elapsed = self.started.elapsed().min(self.max_duration());
        let frame_time = elapsed.as_secs_f64() * ANIMATION_FPS as f64;
        frame_time.floor() as u32 + 1
    }

    pub fn max_duration(&self) -> StdDuration {
        match self.format {
            ExportFormat::Gif => GIF_MAX_DURATION,
            _ => ANIMATION_MAX_DURATION,
        }
    }
}

#[derive(Debug)]
pub struct ExportJob {
    pub format: ExportFormat,
    pub frame_count: usize,
    receiver: Receiver<io::Result<PathBuf>>,
}

impl ExportJob {
    pub fn spawn(recording: RecordingState) -> Self {
        let format = recording.format;
        let size = recording.size;
        let path = recording.output_path;
        let frames = recording.frames;
        let frame_count = frames.len();
        let thread_path = path.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = match format {
                ExportFormat::Gif => write_gif(&frames, &thread_path, size),
                ExportFormat::Mp4 | ExportFormat::Webm => {
                    write_video(&frames, &thread_path, format, size)
                }
                _ => Ok(()),
            }
            .map(|_| thread_path);
            let _ = sender.send(result);
        });
        Self {
            format,
            frame_count,
            receiver,
        }
    }

    pub fn try_finish(&self) -> Option<io::Result<PathBuf>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(io::Error::other("export worker stopped"))),
        }
    }
}

pub fn default_output_path(
    format: ExportFormat,
    location: &Location,
    target: Option<&str>,
    view_mode: ViewMode,
    now: DateTime<Utc>,
) -> PathBuf {
    let base_dir = default_output_dir(format);
    let view = match view_mode {
        ViewMode::Sky => "sky",
        ViewMode::Ground => "globe",
    };
    let target = target.unwrap_or(view);
    let timestamp = now.format("%Y%m%d-%H%M%S");
    let filename = format!(
        "termarium-{}-{}-{}.{}",
        slug(&location.name),
        slug(target),
        timestamp,
        format.extension()
    );
    base_dir.join(filename)
}

pub fn resolve_output_path(input: &str, default_path: &Path, format: ExportFormat) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return default_path.to_path_buf();
    }
    let expanded = expand_home(trimmed);
    let path = PathBuf::from(expanded);
    let looks_like_dir = trimmed.ends_with('/') || path.extension().is_none() || path.is_dir();
    if looks_like_dir {
        let filename = default_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("termarium-export.png"));
        return path.join(filename);
    }
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(format.extension()))
    {
        path
    } else {
        path.with_extension(format.extension())
    }
}

pub fn write_svg(
    buffer: &Buffer,
    path: &Path,
    pixel_width: u32,
    pixel_height: u32,
) -> io::Result<()> {
    let svg = buffer_to_svg(buffer, pixel_width, pixel_height);
    write_text(path, &svg)
}

pub fn write_png(
    buffer: &Buffer,
    path: &Path,
    pixel_width: u32,
    pixel_height: u32,
) -> io::Result<()> {
    let svg = buffer_to_svg(buffer, pixel_width, pixel_height);
    let png = svg_to_png_bytes(&svg)?;
    write_bytes(path, &png)
}

pub fn render_png_frame(
    buffer: &Buffer,
    pixel_width: u32,
    pixel_height: u32,
) -> io::Result<Vec<u8>> {
    let svg = buffer_to_svg(buffer, pixel_width, pixel_height);
    svg_to_png_bytes(&svg)
}

pub fn write_gif(frames: &[Buffer], path: &Path, size: ExportSize) -> io::Result<()> {
    if frames.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no animation frames were captured",
        ));
    }
    let (pixel_width, pixel_height) = size.gif_pixels();
    ensure_parent(path)?;
    let file = fs::File::create(path)?;
    let mut encoder = GifEncoder::new(file);
    encoder
        .set_repeat(image::codecs::gif::Repeat::Infinite)
        .map_err(image_error)?;
    let delay = Delay::from_numer_denom_ms(1000 / ANIMATION_FPS, 1);
    for frame in frames {
        let png = render_png_frame(frame, pixel_width, pixel_height)?;
        let image = image::load_from_memory(&png)
            .map_err(image_error)?
            .into_rgba8();
        encoder
            .encode_frame(ImageFrame::from_parts(image, 0, 0, delay))
            .map_err(image_error)?;
    }
    Ok(())
}

pub fn write_video(
    frames: &[Buffer],
    path: &Path,
    format: ExportFormat,
    size: ExportSize,
) -> io::Result<()> {
    if frames.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no animation frames were captured",
        ));
    }
    if !ffmpeg_available() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            ffmpeg_install_hint(),
        ));
    }
    ensure_parent(path)?;
    let (pixel_width, pixel_height) = size.animation_pixels();
    let temp_dir = std::env::temp_dir().join(format!("termarium-export-{}", unique_id()));
    fs::create_dir_all(&temp_dir)?;
    let result = (|| {
        for (index, frame) in frames.iter().enumerate() {
            let png = render_png_frame(frame, pixel_width, pixel_height)?;
            fs::write(temp_dir.join(format!("frame-{index:04}.png")), png)?;
        }
        let input = temp_dir.join("frame-%04d.png");
        let mut command = Command::new("ffmpeg");
        command
            .arg("-y")
            .arg("-framerate")
            .arg(ANIMATION_FPS.to_string())
            .arg("-i")
            .arg(&input);
        match format {
            ExportFormat::Mp4 => {
                command
                    .arg("-c:v")
                    .arg("libx264")
                    .arg("-pix_fmt")
                    .arg("yuv420p")
                    .arg("-movflags")
                    .arg("+faststart");
            }
            ExportFormat::Webm => {
                command
                    .arg("-c:v")
                    .arg("libvpx-vp9")
                    .arg("-pix_fmt")
                    .arg("yuv420p")
                    .arg("-b:v")
                    .arg("0")
                    .arg("-crf")
                    .arg("32");
            }
            _ => {}
        }
        let output = command.arg(path).output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ))
        }
    })();
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn ffmpeg_install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "ffmpeg is required for MP4/WebM export. Install it with: brew install ffmpeg"
    } else if cfg!(target_os = "windows") {
        "ffmpeg is required for MP4/WebM export. Install it with: winget install Gyan.FFmpeg"
    } else {
        "ffmpeg is required for MP4/WebM export. Install it with: sudo apt install ffmpeg"
    }
}

pub fn buffer_to_svg(buffer: &Buffer, pixel_width: u32, pixel_height: u32) -> String {
    let area = buffer.area;
    let width = area.width.max(1);
    let height = area.height.max(1);
    let cell_width = pixel_width as f64 / width as f64;
    let cell_height = pixel_height as f64 / height as f64;
    let font_size = cell_height * 0.62;
    let baseline = cell_height * 0.72;
    let font_family = "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Noto Sans Mono CJK SC', 'Noto Sans CJK SC', monospace";

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{pixel_width}" height="{pixel_height}" viewBox="0 0 {pixel_width} {pixel_height}" shape-rendering="crispEdges">"#
    ));
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{}"/>"#,
        color_css(Color::Black)
    ));

    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let bg = color_css(cell.bg);
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                x as f64 * cell_width,
                y as f64 * cell_height,
                cell_width + 0.6,
                cell_height + 0.6,
                bg
            ));
        }
    }
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            let symbol_width = UnicodeWidthStr::width(symbol);
            if symbol.trim().is_empty() || symbol_width <= 1 {
                continue;
            }
            let bg = color_css(cell.bg);
            let covered_cells = symbol_width.min(width.saturating_sub(x) as usize) as f64;
            svg.push_str(&format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                x as f64 * cell_width,
                y as f64 * cell_height,
                cell_width * covered_cells + 0.6,
                cell_height + 0.6,
                bg
            ));
        }
    }

    svg.push_str(r#"<g shape-rendering="geometricPrecision" text-rendering="optimizeLegibility">"#);
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.trim().is_empty() {
                continue;
            }
            let text_width = UnicodeWidthStr::width(symbol)
                .max(1)
                .min(width.saturating_sub(x) as usize) as f64
                * cell_width;
            let weight = if cell.modifier.contains(Modifier::BOLD) {
                "700"
            } else {
                "500"
            };
            svg.push_str(&format!(
                r#"<text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{:.2}" font-weight="{}" textLength="{:.2}" lengthAdjust="spacingAndGlyphs">{}</text>"#,
                x as f64 * cell_width,
                y as f64 * cell_height + baseline,
                color_css(cell.fg),
                font_family,
                font_size,
                weight,
                text_width,
                escape_xml(symbol)
            ));
        }
    }
    svg.push_str("</g></svg>");
    svg
}

fn svg_to_png_bytes(svg: &str) -> io::Result<Vec<u8>> {
    let mut options = usvg::Options::default();
    options.fontdb = cached_fontdb().clone();
    let tree = usvg::Tree::from_data(svg.as_bytes(), &options).map_err(io::Error::other)?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| io::Error::other("could not allocate PNG pixmap"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(io::Error::other)
}

fn cached_fontdb() -> &'static Arc<usvg::fontdb::Database> {
    static FONTDB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTDB.get_or_init(|| {
        let mut database = usvg::fontdb::Database::new();
        database.load_system_fonts();
        Arc::new(database)
    })
}

fn write_text(path: &Path, text: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, text)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, bytes)
}

fn ensure_parent(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn default_output_dir(format: ExportFormat) -> PathBuf {
    let user_dirs = UserDirs::new();
    let base = if matches!(format, ExportFormat::Png | ExportFormat::Svg) {
        user_dirs
            .as_ref()
            .and_then(UserDirs::picture_dir)
            .map(Path::to_path_buf)
    } else {
        user_dirs
            .as_ref()
            .and_then(UserDirs::video_dir)
            .map(Path::to_path_buf)
    };
    base.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("Termarium")
}

fn expand_home(value: &str) -> String {
    if value == "~" || value.starts_with("~/") {
        if let Some(home) = UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home
                .join(value.strip_prefix("~/").unwrap_or(""))
                .to_string_lossy()
                .into_owned();
        }
    }
    value.to_string()
}

fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '/' | '\\' | ':' | '.') {
                Some('-')
            } else if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
                Some(ch)
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        "termarium".to_string()
    } else {
        slug
    }
}

fn color_css(color: Color) -> String {
    let (r, g, b) = color_rgb(color);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn color_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Black | Color::Reset => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray | Color::White => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(index) => indexed_color(index),
    }
}

fn indexed_color(index: u8) -> (u8, u8, u8) {
    const BASIC: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if index < 16 {
        return BASIC[index as usize];
    }
    if index <= 231 {
        let value = index - 16;
        let r = value / 36;
        let g = (value % 36) / 6;
        let b = value % 6;
        return (cube_color(r), cube_color(g), cube_color(b));
    }
    let gray = 8 + (index - 232) * 10;
    (gray, gray, gray)
}

fn cube_color(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn image_error(error: image::ImageError) -> io::Error {
    io::Error::other(error)
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect, style::Style};

    #[test]
    fn output_path_uses_directory_or_file() {
        let default = PathBuf::from("/tmp/Termarium/termarium-shanghai-sky-20260509.png");
        assert_eq!(
            resolve_output_path("", &default, ExportFormat::Png),
            default
        );
        assert_eq!(
            resolve_output_path("/tmp/share", &default, ExportFormat::Png),
            PathBuf::from("/tmp/share/termarium-shanghai-sky-20260509.png")
        );
        assert_eq!(
            resolve_output_path("/tmp/share/custom.svg", &default, ExportFormat::Svg),
            PathBuf::from("/tmp/share/custom.svg")
        );
        assert_eq!(
            resolve_output_path("/tmp/share/custom.png", &default, ExportFormat::Gif),
            PathBuf::from("/tmp/share/custom.gif")
        );
    }

    #[test]
    fn svg_contains_dimensions_and_wide_text() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 2));
        buffer[(0, 0)].set_symbol("北").set_style(
            Style::default()
                .fg(Color::Rgb(1, 2, 3))
                .bg(Color::Rgb(4, 5, 6)),
        );
        buffer[(1, 0)].set_style(Style::default().bg(Color::Black));
        let svg = buffer_to_svg(&buffer, 400, 200);
        assert!(svg.contains(r#"width="400""#));
        assert!(svg.contains("北"));
        assert!(svg.contains("#040506"));
        assert!(svg.contains("#010203"));
        assert!(svg.contains(r##"width="200.60" height="100.60" fill="#040506""##));
    }

    #[test]
    fn png_export_writes_requested_dimensions() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 4));
        buffer[(0, 0)]
            .set_symbol("T")
            .set_style(Style::default().fg(Color::White).bg(Color::Black));
        let path = std::env::temp_dir().join(format!("termarium-test-{}.png", unique_id()));
        write_png(&buffer, &path, 320, 240).unwrap();
        assert_eq!(image::image_dimensions(&path).unwrap(), (320, 240));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn gif_export_writes_readable_file() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 4));
        buffer[(0, 0)]
            .set_symbol("A")
            .set_style(Style::default().fg(Color::White).bg(Color::Black));
        let frame_a = buffer.clone();
        buffer[(1, 0)].set_symbol("B").set_style(
            Style::default()
                .fg(Color::Rgb(255, 200, 100))
                .bg(Color::Black),
        );
        let frame_b = buffer.clone();
        let path = std::env::temp_dir().join(format!("termarium-test-{}.gif", unique_id()));
        write_gif(&[frame_a, frame_b], &path, ExportSize::Standard).unwrap();
        assert!(fs::metadata(&path).unwrap().len() > 0);
        let _ = fs::remove_file(path);
    }
}
