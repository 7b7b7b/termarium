use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, Write},
    time::{Duration as StdDuration, Instant},
};

use chrono_tz::Tz;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, MouseButton, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthChar;

use crate::{
    app::{self, App, PRESETS, Screen, Target, ViewMode},
    astro,
    config::{LandscapeMode, Language, Location, SkyCulture, SkyOrientation, Theme},
    constellations, i18n, planets, solar, star_aliases,
};

#[derive(Debug, Clone, Copy)]
struct Palette {
    bg: Color,
    panel: Color,
    cyan: Color,
    silver: Color,
    muted: Color,
    veil: Color,
    moon: Color,
    warm: Color,
    line: Color,
    dim_line: Color,
    selected: Color,
    deep: Color,
}

#[derive(Debug, Clone, Copy)]
struct SkyCell {
    ch: char,
    style: Style,
}

#[derive(Debug, Clone, Copy)]
struct MotionVector {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct RgbColor {
    r: f64,
    g: f64,
    b: f64,
}

impl RgbColor {
    fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    fn mix(from: Self, to: Self, progress: f64) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        Self {
            r: lerp_f64(from.r, to.r, progress),
            g: lerp_f64(from.g, to.g, progress),
            b: lerp_f64(from.b, to.b, progress),
        }
    }

    fn scale(self, brightness: f64) -> Self {
        Self {
            r: self.r * brightness,
            g: self.g * brightness,
            b: self.b * brightness,
        }
    }

    fn to_color(self) -> Color {
        Color::Rgb(
            self.r.round().clamp(0.0, 255.0) as u8,
            self.g.round().clamp(0.0, 255.0) as u8,
            self.b.round().clamp(0.0, 255.0) as u8,
        )
    }
}

const EARTH_TEXTURE_WIDTH: usize = 720;
const EARTH_TEXTURE_HEIGHT: usize = 360;
const EARTH_TEXTURE: &[u8] = include_bytes!("../data/earth_720x360.rgb");

pub fn run(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    execute!(terminal.backend_mut(), EnableMouseCapture, Hide)?;
    terminal.backend_mut().flush()?;

    let result = app_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        Show,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    result
}

fn app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let tick = StdDuration::from_millis(33);
    let mut last_draw = Instant::now();

    while !app.should_quit {
        terminal.draw(|frame| draw(frame, app))?;
        let timeout = tick.saturating_sub(last_draw.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                CEvent::Key(key) => app.handle_key(key)?,
                CEvent::Mouse(mouse) => {
                    let area = terminal.size()?;
                    handle_mouse_event(app, mouse, area.into())?;
                }
                _ => {}
            }
        }
        if last_draw.elapsed() >= tick {
            app.tick();
            last_draw = Instant::now();
        }
    }

    Ok(())
}

const SKY_MOUSE_PICK_RADIUS: usize = 8;

fn handle_mouse_event(app: &mut App, mouse: MouseEvent, area: Rect) -> io::Result<()> {
    match mouse.kind {
        MouseEventKind::Up(MouseButton::Left) => {
            handle_mouse_click(app, mouse.column, mouse.row, area)
        }
        MouseEventKind::ScrollUp => handle_mouse_scroll(app, false),
        MouseEventKind::ScrollDown => handle_mouse_scroll(app, true),
        _ => Ok(()),
    }
}

fn handle_mouse_scroll(app: &mut App, down: bool) -> io::Result<()> {
    if app.help {
        return Ok(());
    }
    match app.screen {
        Screen::Search => {
            if down {
                if !app.search.results.is_empty() {
                    app.select_search_result(app.search.selected.saturating_add(1));
                }
            } else {
                app.select_search_result(app.search.selected.saturating_sub(1));
            }
        }
        Screen::Settings => {
            if app.settings.theme_picker {
                let index = if down {
                    app.settings.theme_selected.saturating_add(1)
                } else {
                    app.settings.theme_selected.saturating_sub(1)
                };
                app.select_theme(index);
            } else {
                let index = if down {
                    app.settings.selected.saturating_add(1)
                } else {
                    app.settings.selected.saturating_sub(1)
                };
                app.select_setting(index);
            }
        }
        Screen::Setup => {
            app.move_setup_field(if down { 1 } else { 0 });
        }
        Screen::Sky => {}
    }
    Ok(())
}

fn handle_mouse_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    if app.help {
        app.help = false;
        return Ok(());
    }

    match app.screen {
        Screen::Search => handle_search_click(app, column, row, area),
        Screen::Settings => handle_settings_click(app, column, row, area),
        Screen::Setup => handle_setup_click(app, column, row, area),
        Screen::Sky => handle_canvas_click(app, column, row, area),
    }
}

fn handle_search_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    let modal = search_modal_inner(area);
    if !rect_contains(modal, column, row) {
        return Ok(());
    }
    let local_row = row.saturating_sub(modal.y);
    if local_row >= 2 {
        let index = local_row.saturating_sub(2) as usize;
        if index < app.search.results.len().min(10) {
            app.activate_search_result(index)?;
        }
    }
    Ok(())
}

fn handle_settings_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    if app.settings.theme_picker {
        return handle_theme_picker_click(app, column, row, area);
    }

    let modal = settings_modal_inner(area);
    if !rect_contains(modal, column, row) {
        return Ok(());
    }
    let index = row.saturating_sub(modal.y) as usize;
    if index < app::settings_count() {
        app.select_setting(index);
        let forward = column >= modal.x.saturating_add(modal.width / 2);
        app.adjust_setting(forward)?;
    }
    Ok(())
}

fn handle_theme_picker_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    let modal = theme_picker_modal_inner(area);
    if !rect_contains(modal, column, row) {
        app.settings.theme_picker = false;
        return Ok(());
    }

    let index = row.saturating_sub(modal.y) as usize;
    if index < Theme::ALL.len() {
        app.select_theme(index);
        app.apply_selected_theme()?;
    }
    Ok(())
}

fn handle_setup_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    let modal = setup_modal_inner(area);
    if !rect_contains(modal, column, row) {
        return Ok(());
    }
    match row.saturating_sub(modal.y) {
        2 => {
            app.move_setup_field(0);
            let forward = column >= modal.x.saturating_add(modal.width / 2);
            app.cycle_preset(forward);
        }
        9 => {
            app.move_setup_field(1);
            app.commit_setup()?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_canvas_click(app: &mut App, column: u16, row: u16, area: Rect) -> io::Result<()> {
    let layout = sky_layout(area, app);
    if rect_contains(layout.footer, column, row) {
        return handle_footer_click(app, column, layout.footer);
    }
    let Some((x, y)) = rect_local(layout.canvas_inner, column, row) else {
        return Ok(());
    };
    if app.horizon_transition().is_some() {
        return Ok(());
    }

    match app.view_mode {
        ViewMode::Sky => {
            if let Some(code) = sky_constellation_label_hit(
                app,
                layout.canvas_inner.width as usize,
                layout.canvas_inner.height as usize,
                x,
                y,
            ) {
                app.select_constellation_code(code);
            } else {
                app.activate_pointer_at(x, y, SKY_MOUSE_PICK_RADIUS);
            }
        }
        ViewMode::Ground => {
            if let Some(index) = ground_city_hit(
                app,
                layout.canvas_inner.width as usize,
                layout.canvas_inner.height as usize,
                x,
                y,
            ) {
                app.apply_ground_city(index)?;
            } else if let Some(globe) = globe_projection(
                app,
                layout.canvas_inner.width as usize,
                layout.canvas_inner.height as usize,
            ) && let Some((lon, lat)) = globe.lon_lat_at(x, y)
            {
                app.preview_ground_location(lat, lon);
            }
        }
    }
    Ok(())
}

fn handle_footer_click(app: &mut App, column: u16, footer: Rect) -> io::Result<()> {
    if footer.width < 3 {
        return Ok(());
    }
    let local_x = column.saturating_sub(footer.x);
    let third = footer.width / 3;
    if local_x < third {
        match app.view_mode {
            ViewMode::Sky => app.open_sky_search(),
            ViewMode::Ground => app.open_city_search(),
        }
    } else if local_x < third.saturating_mul(2) {
        app.toggle_ground_sky();
    } else {
        app.screen = Screen::Settings;
        app.message.clear();
    }
    Ok(())
}

fn search_modal_inner(area: Rect) -> Rect {
    centered_rect(area, 58, 18).inner(Margin {
        horizontal: 2,
        vertical: 2,
    })
}

fn settings_modal_inner(area: Rect) -> Rect {
    centered_rect(area, 60, 19).inner(Margin {
        horizontal: 2,
        vertical: 2,
    })
}

fn theme_picker_modal(area: Rect) -> Rect {
    centered_rect(area, 40, Theme::ALL.len() as u16 + 4)
}

fn theme_picker_modal_inner(area: Rect) -> Rect {
    theme_picker_modal(area).inner(Margin {
        horizontal: 2,
        vertical: 1,
    })
}

fn setup_modal_inner(area: Rect) -> Rect {
    Block::default()
        .borders(Borders::ALL)
        .inner(centered_rect(area, 78, 22))
}

fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.left() && column < rect.right() && row >= rect.top() && row < rect.bottom()
}

fn rect_local(rect: Rect, column: u16, row: u16) -> Option<(usize, usize)> {
    rect_contains(rect, column, row).then(|| {
        (
            column.saturating_sub(rect.x) as usize,
            row.saturating_sub(rect.y) as usize,
        )
    })
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let palette = palette(app.config.display.theme);
    match app.screen {
        Screen::Sky | Screen::Search | Screen::Settings | Screen::Setup => {
            draw_sky(frame, app, palette)
        }
    }

    if app.opening_ticks > 0 {
        draw_opening(frame, app, palette);
    }
    if app.help {
        draw_help(frame, app, palette);
    }
    if matches!(app.screen, Screen::Search) {
        draw_search(frame, app, palette);
    }
    if matches!(app.screen, Screen::Settings) {
        draw_settings(frame, app, palette);
    }
    if matches!(app.screen, Screen::Setup) {
        draw_setup(frame, app, palette);
    }
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Midnight => Palette {
            bg: Color::Rgb(4, 8, 18),
            panel: Color::Rgb(10, 18, 32),
            cyan: Color::Rgb(94, 224, 232),
            silver: Color::Rgb(226, 232, 240),
            muted: Color::Rgb(106, 126, 150),
            veil: Color::Rgb(54, 63, 77),
            moon: Color::Rgb(246, 220, 154),
            warm: Color::Rgb(255, 232, 184),
            line: Color::Rgb(95, 210, 235),
            dim_line: Color::Rgb(46, 86, 111),
            selected: Color::Rgb(255, 238, 167),
            deep: Color::Rgb(176, 132, 255),
        },
        Theme::Aurora => Palette {
            bg: Color::Rgb(3, 13, 16),
            panel: Color::Rgb(7, 27, 31),
            cyan: Color::Rgb(100, 255, 206),
            silver: Color::Rgb(224, 245, 239),
            muted: Color::Rgb(108, 151, 146),
            veil: Color::Rgb(52, 68, 68),
            moon: Color::Rgb(231, 221, 167),
            warm: Color::Rgb(255, 205, 152),
            line: Color::Rgb(116, 225, 198),
            dim_line: Color::Rgb(42, 92, 88),
            selected: Color::Rgb(252, 240, 160),
            deep: Color::Rgb(151, 190, 255),
        },
        Theme::Amber => Palette {
            bg: Color::Rgb(12, 8, 5),
            panel: Color::Rgb(31, 21, 11),
            cyan: Color::Rgb(108, 206, 195),
            silver: Color::Rgb(239, 226, 203),
            muted: Color::Rgb(147, 123, 91),
            veil: Color::Rgb(76, 70, 64),
            moon: Color::Rgb(255, 213, 123),
            warm: Color::Rgb(255, 188, 112),
            line: Color::Rgb(110, 195, 180),
            dim_line: Color::Rgb(86, 68, 48),
            selected: Color::Rgb(255, 235, 153),
            deep: Color::Rgb(222, 155, 255),
        },
        Theme::Dusk => Palette {
            bg: Color::Rgb(15, 12, 31),
            panel: Color::Rgb(27, 22, 48),
            cyan: Color::Rgb(130, 219, 216),
            silver: Color::Rgb(231, 226, 241),
            muted: Color::Rgb(141, 132, 164),
            veil: Color::Rgb(69, 61, 88),
            moon: Color::Rgb(248, 207, 142),
            warm: Color::Rgb(244, 166, 139),
            line: Color::Rgb(125, 202, 210),
            dim_line: Color::Rgb(73, 74, 103),
            selected: Color::Rgb(255, 202, 156),
            deep: Color::Rgb(214, 146, 255),
        },
        Theme::Forest => Palette {
            bg: Color::Rgb(3, 15, 10),
            panel: Color::Rgb(9, 28, 19),
            cyan: Color::Rgb(117, 223, 180),
            silver: Color::Rgb(218, 235, 218),
            muted: Color::Rgb(109, 139, 119),
            veil: Color::Rgb(50, 74, 58),
            moon: Color::Rgb(238, 214, 151),
            warm: Color::Rgb(247, 176, 99),
            line: Color::Rgb(100, 191, 145),
            dim_line: Color::Rgb(38, 87, 65),
            selected: Color::Rgb(249, 226, 130),
            deep: Color::Rgb(142, 172, 255),
        },
        Theme::Dracula => Palette {
            bg: Color::Rgb(40, 42, 54),
            panel: Color::Rgb(52, 55, 70),
            cyan: Color::Rgb(139, 233, 253),
            silver: Color::Rgb(248, 248, 242),
            muted: Color::Rgb(98, 114, 164),
            veil: Color::Rgb(80, 83, 104),
            moon: Color::Rgb(241, 250, 140),
            warm: Color::Rgb(255, 184, 108),
            line: Color::Rgb(80, 250, 123),
            dim_line: Color::Rgb(72, 104, 112),
            selected: Color::Rgb(255, 121, 198),
            deep: Color::Rgb(189, 147, 249),
        },
        Theme::Nord => Palette {
            bg: Color::Rgb(46, 52, 64),
            panel: Color::Rgb(59, 66, 82),
            cyan: Color::Rgb(136, 192, 208),
            silver: Color::Rgb(236, 239, 244),
            muted: Color::Rgb(129, 161, 193),
            veil: Color::Rgb(76, 86, 106),
            moon: Color::Rgb(235, 203, 139),
            warm: Color::Rgb(208, 135, 112),
            line: Color::Rgb(143, 188, 187),
            dim_line: Color::Rgb(81, 100, 122),
            selected: Color::Rgb(180, 142, 173),
            deep: Color::Rgb(163, 190, 140),
        },
        Theme::Gruvbox => Palette {
            bg: Color::Rgb(40, 40, 40),
            panel: Color::Rgb(50, 48, 47),
            cyan: Color::Rgb(142, 192, 124),
            silver: Color::Rgb(235, 219, 178),
            muted: Color::Rgb(168, 153, 132),
            veil: Color::Rgb(80, 73, 69),
            moon: Color::Rgb(250, 189, 47),
            warm: Color::Rgb(254, 128, 25),
            line: Color::Rgb(131, 165, 152),
            dim_line: Color::Rgb(102, 92, 84),
            selected: Color::Rgb(251, 73, 52),
            deep: Color::Rgb(211, 134, 155),
        },
        Theme::SolarizedDark => Palette {
            bg: Color::Rgb(0, 43, 54),
            panel: Color::Rgb(7, 54, 66),
            cyan: Color::Rgb(42, 161, 152),
            silver: Color::Rgb(238, 232, 213),
            muted: Color::Rgb(101, 123, 131),
            veil: Color::Rgb(88, 110, 117),
            moon: Color::Rgb(181, 137, 0),
            warm: Color::Rgb(203, 75, 22),
            line: Color::Rgb(38, 139, 210),
            dim_line: Color::Rgb(63, 93, 101),
            selected: Color::Rgb(220, 50, 47),
            deep: Color::Rgb(108, 113, 196),
        },
        Theme::TokyoNight => Palette {
            bg: Color::Rgb(26, 27, 38),
            panel: Color::Rgb(36, 40, 59),
            cyan: Color::Rgb(125, 207, 255),
            silver: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(86, 95, 137),
            veil: Color::Rgb(65, 72, 104),
            moon: Color::Rgb(224, 175, 104),
            warm: Color::Rgb(255, 158, 100),
            line: Color::Rgb(115, 218, 202),
            dim_line: Color::Rgb(62, 88, 120),
            selected: Color::Rgb(187, 154, 247),
            deep: Color::Rgb(247, 118, 142),
        },
        Theme::Mono => Palette {
            bg: Color::Black,
            panel: Color::Rgb(12, 12, 12),
            cyan: Color::White,
            silver: Color::White,
            muted: Color::DarkGray,
            veil: Color::DarkGray,
            moon: Color::Gray,
            warm: Color::White,
            line: Color::Gray,
            dim_line: Color::DarkGray,
            selected: Color::White,
            deep: Color::Gray,
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct SkyLayout {
    header: Rect,
    canvas_outer: Rect,
    canvas_inner: Rect,
    footer: Rect,
    side_panel: Option<Rect>,
}

fn sky_layout(area: Rect, app: &App) -> SkyLayout {
    let wants_panel = app.config.display.side_panel;
    let (sky_column, side_panel) = if wants_panel && area.width >= 94 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(54), Constraint::Length(36)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else if wants_panel && area.height >= 28 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(16), Constraint::Length(10)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(sky_column);
    let canvas_inner = Block::default().borders(Borders::ALL).inner(rows[1]);
    SkyLayout {
        header: rows[0],
        canvas_outer: rows[1],
        canvas_inner,
        footer: rows[2],
        side_panel,
    }
}

fn draw_sky(frame: &mut Frame, app: &mut App, palette: Palette) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.bg)),
        area,
    );

    let layout = sky_layout(area, app);
    draw_sky_column_parts(
        frame,
        app,
        layout.header,
        layout.canvas_outer,
        layout.footer,
        palette,
    );
    if let Some(side_panel) = layout.side_panel {
        draw_side_panel(frame, app, side_panel, palette);
    }
}

fn draw_sky_column_parts(
    frame: &mut Frame,
    app: &mut App,
    header: Rect,
    canvas: Rect,
    footer: Rect,
    palette: Palette,
) {
    draw_header(frame, app, header, palette);
    if app.horizon_transition().is_some() {
        draw_horizon_canvas(frame, app, canvas, palette);
    } else if app.view_mode == ViewMode::Ground {
        draw_ground_canvas(frame, app, canvas, palette);
    } else {
        draw_star_canvas(frame, app, canvas, palette);
    }
    draw_footer(frame, app, footer, palette);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let now = app.now();
    let timezone = app
        .active_location()
        .timezone
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let local_time = now.with_timezone(&timezone).format("%Y-%m-%d %H:%M:%S %Z");
    let location = app.active_location();
    let mode = if app.paused {
        i18n::tr(language, "paused")
    } else {
        i18n::tr(language, "live")
    };
    let title = Line::from(vec![
        Span::styled(
            format!(" {} ", i18n::tr(language, "title")),
            Style::default()
                .fg(palette.cyan)
                .bg(palette.panel)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}", i18n::tr(language, "subtitle")),
            Style::default().fg(palette.silver),
        ),
    ]);
    let meta = Line::from(vec![
        Span::styled(
            format!("{}: ", i18n::tr(language, "observer")),
            Style::default().fg(palette.muted),
        ),
        Span::styled(
            format!(
                "{}  {:+.4} {:+.4}",
                location.name, location.latitude, location.longitude
            ),
            Style::default().fg(palette.silver),
        ),
        Span::styled("   //   ", Style::default().fg(palette.muted)),
        Span::styled(local_time.to_string(), Style::default().fg(palette.warm)),
        Span::styled("   //   ", Style::default().fg(palette.muted)),
        Span::styled(mode, Style::default().fg(palette.cyan)),
    ]);
    frame.render_widget(Paragraph::new(vec![title, meta]), area);
}

fn draw_star_canvas(frame: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let base_block = Block::default().borders(Borders::ALL);
    let inner = base_block.inner(area);

    if inner.width == 0 || inner.height == 0 {
        frame.render_widget(
            base_block
                .title(sky_title_line(language, palette))
                .border_style(Style::default().fg(palette.dim_line))
                .style(Style::default().bg(palette.bg)),
            area,
        );
        return;
    }

    let sky_location = app.render_location();
    let visible = visible_stars_for_canvas(
        app,
        &sky_location,
        inner.width as usize,
        inner.height as usize,
        app.config.display.sky_orientation,
    );
    let stat_title = sky_stat_title(app, language, visible.len(), palette);
    let block = base_block
        .title(sky_title_line(language, palette))
        .title_top(stat_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line))
        .style(Style::default().bg(palette.bg));
    frame.render_widget(block, area);

    app.set_pointer_canvas(inner.width as usize, inner.height as usize);

    let lines = sky_canvas_lines(
        app,
        &visible,
        &sky_location,
        inner.width as usize,
        inner.height as usize,
        palette,
    );
    frame.render_widget(Paragraph::new(lines), inner);
}

fn sky_canvas_lines(
    app: &App,
    visible: &[astro::VisibleStar],
    sky_location: &Location,
    width: usize,
    height: usize,
    palette: Palette,
) -> Vec<Line<'static>> {
    if app.should_render_zoomed_sky() && app.zoom_render_code().is_some() {
        zoomed_sky_lines(app, width, height, palette)
            .unwrap_or_else(|| sky_lines(app, visible, sky_location, width, height, palette))
    } else {
        sky_lines(app, visible, sky_location, width, height, palette)
    }
}

fn draw_ground_canvas(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let base_block = Block::default().borders(Borders::ALL);
    let inner = base_block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        frame.render_widget(
            base_block
                .title(ground_title_line(language, palette))
                .border_style(Style::default().fg(palette.dim_line))
                .style(Style::default().bg(palette.bg)),
            area,
        );
        return;
    }

    let block = base_block
        .title(ground_title_line(language, palette))
        .title_top(ground_stat_title(app, language, palette))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line))
        .style(Style::default().bg(palette.bg));
    frame.render_widget(block, area);

    let grid = ground_grid(app, inner.width as usize, inner.height as usize, palette);
    frame.render_widget(Paragraph::new(grid_to_lines(grid)), inner);
}

fn draw_horizon_canvas(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let base_block = Block::default().borders(Borders::ALL);
    let inner = base_block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        frame.render_widget(
            base_block
                .title(horizon_title_line(language, palette))
                .border_style(Style::default().fg(palette.dim_line))
                .style(Style::default().bg(palette.bg)),
            area,
        );
        return;
    }

    let block = base_block
        .title(horizon_title_line(language, palette))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line))
        .style(Style::default().bg(palette.bg));
    frame.render_widget(block, area);

    let grid = horizon_transition_grid(app, inner.width as usize, inner.height as usize, palette);
    frame.render_widget(Paragraph::new(grid_to_lines(grid)), inner);
}

fn sky_title_line(language: Language, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {} ", i18n::tr(language, "sky")),
        Style::default()
            .fg(palette.cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn ground_title_line(language: Language, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {} ", i18n::tr(language, "ground")),
        Style::default()
            .fg(palette.cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn horizon_title_line(language: Language, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {} ", i18n::tr(language, "horizon")),
        Style::default()
            .fg(palette.cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn sky_stat_title(
    app: &App,
    language: Language,
    visible_count: usize,
    palette: Palette,
) -> Line<'static> {
    let mut stat = format!(
        " {} {} {} · {} {:.1} · {} {} · {} {} ",
        i18n::tr(language, "visible"),
        visible_count,
        i18n::tr(language, "stars"),
        i18n::tr(language, "limit"),
        app.config.display.limiting_magnitude,
        i18n::tr(language, "lines"),
        on_off(language, app.config.display.constellations),
        i18n::tr(language, "theme"),
        format!("{:?}", app.config.display.theme).to_lowercase()
    );
    if app.constellation_zoom {
        if let Some(code) = app.selected_constellation_code() {
            stat.push_str(&format!(
                " · {} {} · Tab ",
                i18n::tr(language, "constellation_zoom"),
                code
            ));
        }
    }
    Line::from(Span::styled(
        stat,
        Style::default().fg(palette.muted).bg(palette.bg),
    ))
    .right_aligned()
}

fn ground_stat_title(app: &App, language: Language, palette: Palette) -> Line<'static> {
    let preview = &app.ground.preview_location;
    Line::from(Span::styled(
        format!(
            " {} {:+.1} {:+.1} · {} {} · s {} · g {} ",
            i18n::tr(language, "ground_cursor"),
            preview.latitude,
            preview.longitude,
            i18n::tr(language, "timezone"),
            preview.timezone,
            i18n::tr(language, "save"),
            i18n::tr(language, "sky")
        ),
        Style::default().fg(palette.muted).bg(palette.bg),
    ))
    .right_aligned()
}

fn horizon_transition_grid(
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
) -> Vec<Vec<SkyCell>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let Some((from, to, progress)) = app.horizon_transition() else {
        return match app.view_mode {
            ViewMode::Sky => sky_canvas_grid(app, width, height, palette),
            ViewMode::Ground => ground_grid(app, width, height, palette),
        };
    };
    let final_sky = sky_canvas_grid(app, width, height, palette);
    let globe = globe_layer_grid(app, width, height, palette);
    let globe_projection = globe_projection(app, width, height);
    let ground = ground_grid(app, width, height, palette);
    let look_up = match (from, to) {
        (ViewMode::Sky, ViewMode::Ground) => 1.0 - progress,
        (ViewMode::Ground, ViewMode::Sky) => progress,
        _ => progress,
    }
    .clamp(0.0, 1.0);

    if look_up <= 0.02 {
        return ground;
    }
    if look_up >= 0.999 {
        return final_sky;
    }

    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };

    let (location_progress, shape_progress) = staged_horizon_sky_phase(from, to, progress, look_up);
    let mut grid = horizon_sky_grid(
        app,
        width,
        height,
        palette,
        location_progress,
        shape_progress,
    );
    let motion = horizon_path_motion_vector(app);
    let (globe_offset_x, globe_offset_y) =
        globe_motion_offset(from, to, progress, width, height, motion);
    for y in 0..height {
        for x in 0..width {
            let ground_cell = shifted_globe_cell(
                &globe,
                globe_projection,
                x,
                y,
                globe_offset_x,
                globe_offset_y,
                empty,
            )
            .unwrap_or(empty);
            if ground_cell.ch != ' ' || ground_cell.style.bg != Some(palette.bg) {
                grid[y][x] = ground_cell;
            }
        }
    }

    grid
}

fn sky_canvas_grid(app: &App, width: usize, height: usize, palette: Palette) -> Vec<Vec<SkyCell>> {
    let sky_location = app.render_location();
    sky_canvas_grid_for_location(app, &sky_location, width, height, palette)
}

fn sky_canvas_grid_for_location(
    app: &App,
    sky_location: &Location,
    width: usize,
    height: usize,
    palette: Palette,
) -> Vec<Vec<SkyCell>> {
    let visible = visible_stars_for_canvas(
        app,
        sky_location,
        width,
        height,
        app.config.display.sky_orientation,
    );
    let lines = sky_canvas_lines(app, &visible, sky_location, width, height, palette);
    lines_to_grid(lines, width, height, palette)
}

fn horizon_sky_grid(
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
    location_progress: f64,
    shape_progress: f64,
) -> Vec<Vec<SkyCell>> {
    let location = horizon_observer_location(app, location_progress);
    let time = app.now();
    let mut visible = app
        .catalog
        .stars
        .iter()
        .copied()
        .filter(|star| star.magnitude <= app.config.display.limiting_magnitude)
        .filter_map(|star| {
            let horizontal =
                astro::horizontal_position(star.ra_hours, star.dec_degrees, &location, time);
            let (x, y) = horizon_sky_project(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                shape_progress,
            )?;
            Some(astro::VisibleStar { star, x, y })
        })
        .collect::<Vec<_>>();
    visible.sort_by(|a, b| {
        a.star
            .magnitude
            .partial_cmp(&b.star.magnitude)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    let unicode = app.config.display.charset.canvas_unicode();

    if !matches!(app.config.display.landscape, LandscapeMode::Off) {
        draw_local_ground_ring(
            &mut grid,
            app,
            &location,
            width,
            height,
            SkyOrientation::Observer,
            horizon_sky_scale(shape_progress),
            1.0,
            palette,
        );
    }

    for star in visible.iter().rev() {
        set_cell(
            &mut grid,
            star.x,
            star.y,
            star_symbol(star.star.magnitude, unicode, app.animation_tick),
            star_style(star.star.magnitude, star.star.color_index, palette),
        );
    }

    if app.config.display.constellations {
        if shape_progress < 0.96 {
            draw_backdrop_constellation_lines(&mut grid, &visible, app, width, palette, unicode);
        } else {
            draw_constellation_lines(&mut grid, &visible, app, width, height, palette, unicode);
        }
    }

    draw_deep_sky(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            horizon_sky_project(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                shape_progress,
            )
        },
        palette,
        unicode,
    );
    draw_planets(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            horizon_sky_project(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                shape_progress,
            )
        },
        palette,
        unicode,
    );

    if location_progress > 0.98 && shape_progress > 0.92 {
        if app.config.display.constellations {
            draw_constellation_labels(&mut grid, &visible, app, width, height, palette, unicode);
        }
        if app.config.display.labels {
            for star in visible
                .iter()
                .filter(|star| !star.star.proper.is_empty() && star.star.magnitude <= 1.65)
                .take(9)
            {
                draw_label(
                    &mut grid,
                    star.x,
                    star.y,
                    star.star.proper,
                    palette,
                    unicode,
                );
            }
        }
        draw_planet_labels(
            &mut grid,
            app,
            |ra, dec| {
                let horizontal = astro::horizontal_position(ra, dec, &location, time);
                horizon_sky_project(
                    horizontal.altitude,
                    horizontal.azimuth,
                    width,
                    height,
                    shape_progress,
                )
            },
            palette,
            unicode,
        );
        if !app.pointer.active {
            draw_selected_label(
                &mut grid,
                app,
                |ra, dec| {
                    let horizontal = astro::horizontal_position(ra, dec, &location, time);
                    horizon_sky_project(
                        horizontal.altitude,
                        horizontal.azimuth,
                        width,
                        height,
                        shape_progress,
                    )
                },
                palette,
                unicode,
            );
        }
    }

    grid
}

fn horizon_sky_project(
    altitude: f64,
    azimuth: f64,
    width: usize,
    height: usize,
    shape_progress: f64,
) -> Option<(usize, usize)> {
    if altitude < 0.0 || width == 0 || height == 0 {
        return None;
    }

    let radius = ((90.0 - altitude) / 90.0).clamp(0.0, 1.0);
    let center_x = width.saturating_sub(1) as f64 / 2.0;
    let center_y = height.saturating_sub(1) as f64 / 2.0;
    let x_radius = center_x.max(1.0);
    let y_radius = center_y.max(1.0);
    let scale = horizon_sky_scale(shape_progress);
    let azimuth = azimuth.to_radians();
    let x = center_x - azimuth.sin() * radius * x_radius * scale;
    let y = center_y - azimuth.cos() * radius * y_radius * scale;

    if !x.is_finite() || !y.is_finite() {
        return None;
    }

    if !(0.0..=width.saturating_sub(1) as f64).contains(&x)
        || !(0.0..=height.saturating_sub(1) as f64).contains(&y)
    {
        return None;
    }

    Some((x.round() as usize, y.round() as usize))
}

fn horizon_sky_scale(shape_progress: f64) -> f64 {
    lerp_f64(1.58, 1.0, shape_progress)
}

fn horizon_observer_location(app: &App, progress: f64) -> Location {
    let from = antipode_location(app);
    let to = app.render_location();
    let t = progress.clamp(0.0, 1.0);
    let (_, center_lon) = app.render_ground_center();
    let longitude_direction = if center_lon >= 0.0 { 1.0 } else { -1.0 };
    Location {
        name: format!("{} -> {}", from.name, to.name),
        latitude: lerp_f64(from.latitude, to.latitude, t),
        longitude: normalize_degrees(from.longitude + 180.0 * longitude_direction * t),
        timezone: to.timezone,
    }
}

fn antipode_location(app: &App) -> Location {
    let (center_lat, center_lon) = app.render_ground_center();
    Location {
        name: "Antipode".to_string(),
        latitude: -center_lat,
        longitude: normalize_degrees(center_lon + 180.0),
        timezone: "UTC".to_string(),
    }
}

fn ground_grid(app: &App, width: usize, height: usize, palette: Palette) -> Vec<Vec<SkyCell>> {
    let mut grid = horizon_sky_grid(app, width, height, palette, 0.0, 0.0);
    let unicode = app.config.display.charset.canvas_unicode();

    draw_globe_layer(&mut grid, app, width, height, palette, unicode);
    draw_ground_overlay(&mut grid, app, palette, unicode);

    grid
}

fn globe_layer_grid(app: &App, width: usize, height: usize, palette: Palette) -> Vec<Vec<SkyCell>> {
    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    let unicode = app.config.display.charset.canvas_unicode();

    draw_globe_layer(&mut grid, app, width, height, palette, unicode);

    grid
}

fn draw_globe_layer(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
    unicode: bool,
) {
    if let Some(globe) = globe_projection(app, width, height) {
        draw_globe_fill(grid, app, globe);
        draw_globe_graticule(grid, globe, palette, unicode);
        draw_preset_city_markers(grid, app, globe, palette, unicode);
        draw_globe_markers(grid, app, globe, palette, unicode);
    }
}

fn globe_projection(app: &App, width: usize, height: usize) -> Option<GlobeProjection> {
    let (center_lat, center_lon) = app.render_ground_center();
    GlobeProjection::new(width, height, center_lat, center_lon)
}

#[derive(Debug, Clone, Copy)]
struct GlobeProjection {
    width: usize,
    height: usize,
    center_x: f64,
    center_y: f64,
    radius_x: f64,
    radius_y: f64,
    center_lat_rad: f64,
    center_lon_deg: f64,
    center_lon_rad: f64,
}

impl GlobeProjection {
    fn new(width: usize, height: usize, center_lat: f64, center_lon: f64) -> Option<Self> {
        if width < 8 || height < 5 {
            return None;
        }
        let radius_y = ((height as f64 - 2.0) / 2.0)
            .min((width as f64 - 4.0) / 4.0)
            .max(1.0);
        Some(Self {
            width,
            height,
            center_x: width.saturating_sub(1) as f64 / 2.0,
            center_y: height.saturating_sub(1) as f64 / 2.0,
            radius_x: radius_y * 2.0,
            radius_y,
            center_lat_rad: center_lat.clamp(-89.5, 89.5).to_radians(),
            center_lon_deg: normalize_degrees(center_lon),
            center_lon_rad: normalize_degrees(center_lon).to_radians(),
        })
    }

    fn project(self, lon: f64, lat: f64) -> Option<(usize, usize)> {
        let lat_rad = lat.to_radians();
        let delta_lon = normalize_degrees(lon - self.center_lon_deg).to_radians();
        let (sin_lat, cos_lat) = lat_rad.sin_cos();
        let (sin_center, cos_center) = self.center_lat_rad.sin_cos();
        let (sin_delta, cos_delta) = delta_lon.sin_cos();
        let visible = sin_center * sin_lat + cos_center * cos_lat * cos_delta;
        if visible < -0.01 {
            return None;
        }

        let x = self.center_x + self.radius_x * cos_lat * sin_delta;
        let y = self.center_y
            - self.radius_y * (cos_center * sin_lat - sin_center * cos_lat * cos_delta);
        if !(0.0..=self.width.saturating_sub(1) as f64).contains(&x)
            || !(0.0..=self.height.saturating_sub(1) as f64).contains(&y)
        {
            return None;
        }
        Some((x.round() as usize, y.round() as usize))
    }

    fn lon_lat_at(self, x: usize, y: usize) -> Option<(f64, f64)> {
        let normalized_x = (x as f64 - self.center_x) / self.radius_x;
        let normalized_y = (self.center_y - y as f64) / self.radius_y;
        let rho_squared = normalized_x * normalized_x + normalized_y * normalized_y;
        if rho_squared > 1.0 {
            return None;
        }
        let rho = rho_squared.sqrt();
        if rho <= f64::EPSILON {
            return Some((self.center_lon_deg, self.center_lat_rad.to_degrees()));
        }

        let cos_c = (1.0 - rho_squared).sqrt();
        let (sin_center, cos_center) = self.center_lat_rad.sin_cos();
        let lat = (cos_c * sin_center + normalized_y * cos_center)
            .clamp(-1.0, 1.0)
            .asin();
        let lon = self.center_lon_rad
            + normalized_x.atan2(cos_center * cos_c - normalized_y * sin_center);
        Some((normalize_degrees(lon.to_degrees()), lat.to_degrees()))
    }
}

fn draw_globe_fill(grid: &mut [Vec<SkyCell>], app: &App, globe: GlobeProjection) {
    let height = grid.len();
    let width = grid.first().map_or(0, Vec::len);
    if width == 0 || height == 0 {
        return;
    }
    let sun = solar::sun_position(app.now());
    let gmst_hours = astro::local_sidereal_time_hours(app.now(), 0.0);
    let subsolar_lon = normalize_degrees((sun.ra_hours - gmst_hours) * 15.0);
    let sun_lat = sun.dec_degrees.to_radians();

    for y in 0..height {
        for x in 0..width {
            let Some((lon, lat)) = globe.lon_lat_at(x, y) else {
                continue;
            };
            let altitude = solar_altitude(lat.to_radians(), lon, sun_lat, subsolar_lon);
            let color = globe_surface_color(lon, lat, altitude).to_color();
            set_cell(grid, x, y, ' ', Style::default().fg(color).bg(color));
        }
    }
}

fn globe_surface_color(lon: f64, lat: f64, solar_altitude: f64) -> RgbColor {
    let base = earth_texture_color(lon, lat);
    sunlight_color(base, solar_altitude)
}

fn earth_texture_color(lon: f64, lat: f64) -> RgbColor {
    let x = ((normalize_degrees(lon) + 180.0) / 360.0) * EARTH_TEXTURE_WIDTH as f64;
    let y =
        ((90.0 - lat.clamp(-90.0, 90.0)) / 180.0) * EARTH_TEXTURE_HEIGHT.saturating_sub(1) as f64;
    let x0 = x.floor() as usize % EARTH_TEXTURE_WIDTH;
    let x1 = (x0 + 1) % EARTH_TEXTURE_WIDTH;
    let y0 = y
        .floor()
        .clamp(0.0, EARTH_TEXTURE_HEIGHT.saturating_sub(1) as f64) as usize;
    let y1 = (y0 + 1).min(EARTH_TEXTURE_HEIGHT - 1);
    let tx = x.fract();
    let ty = y.fract();
    let top = RgbColor::mix(earth_texture_pixel(x0, y0), earth_texture_pixel(x1, y0), tx);
    let bottom = RgbColor::mix(earth_texture_pixel(x0, y1), earth_texture_pixel(x1, y1), tx);
    RgbColor::mix(top, bottom, ty)
}

fn earth_texture_pixel(x: usize, y: usize) -> RgbColor {
    let offset = (y * EARTH_TEXTURE_WIDTH + x) * 3;
    RgbColor::new(
        EARTH_TEXTURE[offset] as f64,
        EARTH_TEXTURE[offset + 1] as f64,
        EARTH_TEXTURE[offset + 2] as f64,
    )
}

fn sunlight_color(base: RgbColor, altitude: f64) -> RgbColor {
    let daylight = smootherstep01((altitude + 8.0) / 30.0);
    let brightness = lerp_f64(0.38, 1.24, daylight);
    base.scale(brightness)
}

fn draw_globe_graticule(
    grid: &mut [Vec<SkyCell>],
    globe: GlobeProjection,
    palette: Palette,
    unicode: bool,
) {
    let style = Style::default().fg(palette.veil);
    let ch = if unicode { '·' } else { '.' };

    for lat in (-60..=60).step_by(30) {
        draw_globe_polyline(
            grid,
            (-180..=180).step_by(2).map(|lon| (lon as f64, lat as f64)),
            globe,
            style,
            ch,
            unicode,
        );
    }
    for lon in (-150..=150).step_by(30) {
        draw_globe_polyline(
            grid,
            (-90..=90).step_by(2).map(|lat| (lon as f64, lat as f64)),
            globe,
            style,
            ch,
            unicode,
        );
    }
}

fn draw_globe_polyline<I>(
    grid: &mut [Vec<SkyCell>],
    points: I,
    globe: GlobeProjection,
    style: Style,
    point: char,
    unicode: bool,
) where
    I: IntoIterator<Item = (f64, f64)>,
{
    let mut previous = None;
    for (lon, lat) in points {
        let Some((x, y)) = globe.project(lon, lat) else {
            previous = None;
            continue;
        };
        if let Some((previous_x, previous_y)) = previous {
            if x.abs_diff(previous_x) as f64 <= globe.radius_x * 0.65
                && y.abs_diff(previous_y) as f64 <= globe.radius_y * 0.65
            {
                draw_line_overlay(grid, previous_x, previous_y, x, y, style, unicode);
            }
        }
        set_cell_overlay(grid, x, y, point, style);
        previous = Some((x, y));
    }
}

fn draw_globe_markers(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    globe: GlobeProjection,
    palette: Palette,
    unicode: bool,
) {
    let saved = &app.config.location;
    if (saved.latitude - app.ground.preview_location.latitude).abs() > 0.0001
        || (saved.longitude - app.ground.preview_location.longitude).abs() > 0.0001
    {
        if let Some((x, y)) = globe.project(saved.longitude, saved.latitude) {
            let style = Style::default().fg(palette.warm);
            set_cell_overlay(grid, x, y, '+', style);
            let label = location_display_label(saved, app.config.language, unicode);
            draw_text_overlay(grid, x.saturating_add(2), y, &label, style, unicode);
        }
    }

    if let Some((x, y)) = globe.project(app.ground.cursor_lon, app.ground.cursor_lat) {
        let style = Style::default()
            .fg(palette.selected)
            .add_modifier(Modifier::BOLD);
        set_cell_overlay(grid, x, y, if unicode { '◎' } else { '@' }, style);
        set_cell_offset_overlay(grid, x, y, -2, 0, if unicode { '─' } else { '-' }, style);
        set_cell_offset_overlay(grid, x, y, -1, 0, if unicode { '─' } else { '-' }, style);
        set_cell_offset_overlay(grid, x, y, 1, 0, if unicode { '─' } else { '-' }, style);
        set_cell_offset_overlay(grid, x, y, 2, 0, if unicode { '─' } else { '-' }, style);
        set_cell_offset_overlay(grid, x, y, 0, -1, if unicode { '│' } else { '|' }, style);
        set_cell_offset_overlay(grid, x, y, 0, 1, if unicode { '│' } else { '|' }, style);
        let label =
            location_display_label(&app.ground.preview_location, app.config.language, unicode);
        if !label.starts_with("Globe ") {
            draw_text_overlay(grid, x.saturating_add(3), y, &label, style, unicode);
        }
    }
}

fn draw_preset_city_markers(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    globe: GlobeProjection,
    palette: Palette,
    unicode: bool,
) {
    let style = Style::default().fg(palette.silver);
    let marker = if unicode { '·' } else { '.' };
    let mut occupied = label_occupancy_grid(grid);
    reserve_globe_focus_labels(&mut occupied, app, globe, unicode);
    let visible_cities = PRESETS
        .iter()
        .filter(|preset| !preset.custom)
        .filter_map(|preset| {
            globe
                .project(preset.longitude, preset.latitude)
                .map(|(x, y)| (preset, x, y))
        })
        .collect::<Vec<_>>();

    for (_, x, y) in &visible_cities {
        reserve_marker_cells(&mut occupied, *x, *y, 0);
        set_cell_overlay(grid, *x, *y, marker, style);
    }

    for (preset, x, y) in visible_cities {
        set_cell_overlay(grid, x, y, marker, style);

        let is_preview = preset_matches_location(preset, &app.ground.preview_location);
        let is_saved = preset_matches_location(preset, &app.config.location);
        if is_preview || is_saved {
            continue;
        }

        let label = preset_city_label(preset, app.config.language, unicode);
        let label_width = canvas_label_width(label, unicode);
        if label_width == 0 {
            continue;
        }

        let Some((label_x, label_y)) = city_label_position(&occupied, x, y, label_width) else {
            continue;
        };
        reserve_label_cells(&mut occupied, label_x, label_y, label_width);
        draw_text_overlay(grid, label_x, label_y, label, style, unicode);
    }
}

fn ground_city_hit(app: &App, width: usize, height: usize, x: usize, y: usize) -> Option<usize> {
    let globe = globe_projection(app, width, height)?;
    let unicode = app.config.display.charset.canvas_unicode();
    let empty = SkyCell {
        ch: ' ',
        style: Style::default(),
    };
    let grid = vec![vec![empty; width]; height];
    let mut occupied = label_occupancy_grid(&grid);
    reserve_globe_focus_labels(&mut occupied, app, globe, unicode);

    let visible_cities = PRESETS
        .iter()
        .enumerate()
        .filter(|(_, preset)| !preset.custom)
        .filter_map(|(index, preset)| {
            globe
                .project(preset.longitude, preset.latitude)
                .map(|(marker_x, marker_y)| (index, preset, marker_x, marker_y))
        })
        .collect::<Vec<_>>();

    for (_, _, marker_x, marker_y) in &visible_cities {
        reserve_marker_cells(&mut occupied, *marker_x, *marker_y, 0);
    }

    let mut nearest_marker = None;
    for (index, _, marker_x, marker_y) in &visible_cities {
        let distance = marker_x.abs_diff(x).pow(2) + marker_y.abs_diff(y).pow(2);
        if distance <= 2 {
            nearest_marker = match nearest_marker {
                Some((best_distance, best_index)) if best_distance <= distance => {
                    Some((best_distance, best_index))
                }
                _ => Some((distance, *index)),
            };
        }
    }
    if let Some((_, index)) = nearest_marker {
        return Some(index);
    }

    for (index, preset, marker_x, marker_y) in visible_cities {
        let is_preview = preset_matches_location(preset, &app.ground.preview_location);
        let is_saved = preset_matches_location(preset, &app.config.location);
        if is_preview || is_saved {
            continue;
        }

        let label = preset_city_label(preset, app.config.language, unicode);
        let label_width = canvas_label_width(label, unicode);
        if label_width == 0 {
            continue;
        }
        let Some((label_x, label_y)) =
            city_label_position(&occupied, marker_x, marker_y, label_width)
        else {
            continue;
        };
        if y == label_y && x >= label_x && x < label_x.saturating_add(label_width) {
            return Some(index);
        }
        reserve_label_cells(&mut occupied, label_x, label_y, label_width);
    }

    None
}

fn label_occupancy_grid(grid: &[Vec<SkyCell>]) -> Vec<Vec<bool>> {
    let width = grid.first().map_or(0, Vec::len);
    vec![vec![false; width]; grid.len()]
}

fn reserve_globe_focus_labels(
    occupied: &mut [Vec<bool>],
    app: &App,
    globe: GlobeProjection,
    unicode: bool,
) {
    let saved = &app.config.location;
    if ((saved.latitude - app.ground.preview_location.latitude).abs() > 0.0001
        || (saved.longitude - app.ground.preview_location.longitude).abs() > 0.0001)
        && let Some((x, y)) = globe.project(saved.longitude, saved.latitude)
    {
        reserve_marker_cells(occupied, x, y, 1);
        let label = location_display_label(saved, app.config.language, unicode);
        reserve_label_cells(
            occupied,
            x.saturating_add(2),
            y,
            canvas_label_width(&label, unicode),
        );
    }

    if let Some((x, y)) = globe.project(app.ground.cursor_lon, app.ground.cursor_lat) {
        reserve_marker_cells(occupied, x, y, 2);
        let label =
            location_display_label(&app.ground.preview_location, app.config.language, unicode);
        if !label.starts_with("Globe ") {
            reserve_label_cells(
                occupied,
                x.saturating_add(3),
                y,
                canvas_label_width(&label, unicode),
            );
        }
    }
}

fn city_label_position(
    occupied: &[Vec<bool>],
    marker_x: usize,
    marker_y: usize,
    label_width: usize,
) -> Option<(usize, usize)> {
    if label_width == 0 {
        return None;
    }
    let grid_width = occupied.first().map_or(0, Vec::len) as isize;
    let grid_height = occupied.len() as isize;
    let x = marker_x as isize;
    let y = marker_y as isize;
    let width = label_width as isize;
    let candidates = [
        (x + 2, y),
        (x - width - 2, y),
        (x + 2, y - 1),
        (x + 2, y + 1),
        (x - width - 2, y - 1),
        (x - width - 2, y + 1),
        (x - width / 2, y - 2),
        (x - width / 2, y + 2),
    ];

    candidates
        .into_iter()
        .filter(|(candidate_x, candidate_y)| {
            *candidate_x >= 0
                && *candidate_y >= 0
                && candidate_x.saturating_add(width) <= grid_width
                && *candidate_y < grid_height
        })
        .map(|(candidate_x, candidate_y)| (candidate_x as usize, candidate_y as usize))
        .find(|(candidate_x, candidate_y)| {
            label_cells_are_free(occupied, *candidate_x, *candidate_y, label_width)
        })
}

fn label_cells_are_free(occupied: &[Vec<bool>], x: usize, y: usize, label_width: usize) -> bool {
    let Some(row) = occupied.get(y) else {
        return false;
    };
    if x >= row.len() || x.saturating_add(label_width) > row.len() {
        return false;
    }
    let start = x.saturating_sub(1);
    let end = x.saturating_add(label_width + 1).min(row.len());
    row[start..end].iter().all(|cell| !*cell)
}

fn reserve_label_cells(occupied: &mut [Vec<bool>], x: usize, y: usize, label_width: usize) {
    let Some(row) = occupied.get_mut(y) else {
        return;
    };
    if x >= row.len() || label_width == 0 {
        return;
    }
    let start = x.saturating_sub(1);
    let end = x.saturating_add(label_width + 1).min(row.len());
    for cell in &mut row[start..end] {
        *cell = true;
    }
}

fn reserve_marker_cells(occupied: &mut [Vec<bool>], x: usize, y: usize, radius: usize) {
    let y_start = y.saturating_sub(radius);
    let y_end = y
        .saturating_add(radius)
        .min(occupied.len().saturating_sub(1));
    for row in occupied.iter_mut().take(y_end + 1).skip(y_start) {
        if row.is_empty() {
            continue;
        }
        let x_start = x.saturating_sub(radius);
        let x_end = x.saturating_add(radius).min(row.len().saturating_sub(1));
        for cell in &mut row[x_start..=x_end] {
            *cell = true;
        }
    }
}

fn preset_city_label(preset: &app::Preset, language: Language, unicode: bool) -> &'static str {
    match (language, unicode) {
        (Language::Zh, true) => preset.zh,
        _ => preset.en,
    }
}

fn location_display_label<'a>(
    location: &'a Location,
    language: Language,
    unicode: bool,
) -> Cow<'a, str> {
    if let Some(preset) = PRESETS
        .iter()
        .find(|preset| !preset.custom && preset_matches_location(preset, location))
    {
        return Cow::Borrowed(preset_city_label(preset, language, unicode));
    }
    Cow::Borrowed(location.name.as_str())
}

fn preset_matches_location(preset: &app::Preset, location: &Location) -> bool {
    (preset.latitude - location.latitude).abs() < 0.0001
        && (preset.longitude - location.longitude).abs() < 0.0001
}

fn draw_ground_overlay(grid: &mut [Vec<SkyCell>], app: &App, palette: Palette, unicode: bool) {
    if grid.is_empty() || grid[0].is_empty() {
        return;
    }
    let preview = &app.ground.preview_location;
    let text = format!(
        "{} {:+.1} {:+.1} · {}",
        i18n::tr(app.config.language, "ground_cursor"),
        preview.latitude,
        preview.longitude,
        preview.timezone
    );
    draw_text(
        grid,
        1,
        0,
        &text,
        Style::default().fg(palette.silver).bg(palette.bg),
        unicode,
    );
}

fn normalize_degrees(degrees: f64) -> f64 {
    ((degrees + 180.0).rem_euclid(360.0)) - 180.0
}

fn lerp_f64(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress.clamp(0.0, 1.0)
}

fn smootherstep01(progress: f64) -> f64 {
    let t = progress.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn staged_horizon_sky_phase(
    from: ViewMode,
    to: ViewMode,
    progress: f64,
    look_up: f64,
) -> (f64, f64) {
    match (from, to) {
        (ViewMode::Ground, ViewMode::Sky) => {
            let p = progress.clamp(0.0, 1.0);
            let shape = smootherstep01((p / 0.56).clamp(0.0, 1.0));
            let location = smootherstep01((p / 0.88).clamp(0.0, 1.0));
            (location, shape)
        }
        (ViewMode::Sky, ViewMode::Ground) => {
            let p = progress.clamp(0.0, 1.0);
            let shape = 1.0 - smootherstep01((p / 0.56).clamp(0.0, 1.0));
            let location = 1.0 - smootherstep01((p / 0.88).clamp(0.0, 1.0));
            (location, shape)
        }
        _ => (look_up, look_up),
    }
}

fn horizon_path_motion_vector(app: &App) -> MotionVector {
    let before = horizon_observer_location(app, 0.92);
    let after = horizon_observer_location(app, 1.0);
    let mean_lat = ((before.latitude + after.latitude) * 0.5).to_radians();
    let lon_delta = normalize_degrees(after.longitude - before.longitude).to_radians();
    let lat_delta = (after.latitude - before.latitude).to_radians();
    let x = lon_delta * mean_lat.cos().abs().max(0.25);
    let y = -lat_delta;
    let length = (x * x + y * y).sqrt();
    if length <= f64::EPSILON {
        MotionVector { x: 0.0, y: 1.0 }
    } else {
        MotionVector {
            x: x / length,
            y: -y / length,
        }
    }
}

fn globe_motion_offset(
    from: ViewMode,
    to: ViewMode,
    eased_progress: f64,
    width: usize,
    height: usize,
    motion: MotionVector,
) -> (f64, f64) {
    let travel = globe_motion_travel(width, height, motion);
    let amount = match (from, to) {
        (ViewMode::Ground, ViewMode::Sky) => {
            smootherstep01((eased_progress / 0.42).clamp(0.0, 1.0))
        }
        (ViewMode::Sky, ViewMode::Ground) => {
            1.0 - smootherstep01(((eased_progress - 0.42) / 0.42).clamp(0.0, 1.0))
        }
        _ => 0.0,
    };
    (motion.x * travel * amount, motion.y * travel * amount)
}

fn globe_motion_travel(width: usize, height: usize, motion: MotionVector) -> f64 {
    if width == 0 || height == 0 {
        return 0.0;
    }
    let radius_y = ((height as f64 - 2.0) / 2.0)
        .min((width as f64 - 4.0) / 4.0)
        .max(1.0);
    let radius_x = radius_y * 2.0;
    let half_width = width.saturating_sub(1) as f64 / 2.0;
    let half_height = height.saturating_sub(1) as f64 / 2.0;
    let screen_support = motion.x.abs() * half_width + motion.y.abs() * half_height;
    let globe_support = ((motion.x * radius_x).powi(2) + (motion.y * radius_y).powi(2)).sqrt();
    let label_margin = 36.0;
    screen_support + globe_support + label_margin
}

fn shifted_globe_cell(
    globe_layer: &[Vec<SkyCell>],
    globe: Option<GlobeProjection>,
    x: usize,
    y: usize,
    offset_x: f64,
    offset_y: f64,
    fallback: SkyCell,
) -> Option<SkyCell> {
    let source_x = x as f64 - offset_x;
    let source_y = y as f64 - offset_y;
    if source_x < 0.0 || source_y < 0.0 {
        return None;
    }
    let sx = source_x.round() as usize;
    let sy = source_y.round() as usize;
    let cell = globe_layer
        .get(sy)
        .and_then(|row| row.get(sx))
        .copied()
        .unwrap_or(fallback);
    let contains_globe = globe.is_some_and(|projection| projection.lon_lat_at(sx, sy).is_some());
    (contains_globe || cell.ch != ' ').then_some(cell)
}

fn solar_altitude(lat: f64, lon: f64, sun_lat: f64, subsolar_lon: f64) -> f64 {
    let delta_lon = (lon - subsolar_lon).to_radians();
    (lat.sin() * sun_lat.sin() + lat.cos() * sun_lat.cos() * delta_lon.cos())
        .clamp(-1.0, 1.0)
        .asin()
        .to_degrees()
}

fn lines_to_grid(
    lines: Vec<Line<'static>>,
    width: usize,
    height: usize,
    palette: Palette,
) -> Vec<Vec<SkyCell>> {
    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    for (y, line) in lines.into_iter().take(height).enumerate() {
        let mut x = 0usize;
        for span in line.spans {
            for ch in span.content.chars() {
                if x >= width {
                    break;
                }
                grid[y][x] = SkyCell {
                    ch,
                    style: span.style,
                };
                x += 1;
            }
            if x >= width {
                break;
            }
        }
    }
    grid
}

fn zoomed_sky_lines(
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
) -> Option<Vec<Line<'static>>> {
    let code = app.zoom_render_code()?;
    let view = app.zoom_render_view(width, height)?;
    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    let unicode = app.config.display.charset.canvas_unicode();
    let location = app.render_location();
    let time = app.now();
    let moon = moon_render_context(&location, time);
    let mut visible = view.stars.clone();
    visible.retain(|star| {
        star_survives_moonlight(
            star.star,
            &location,
            time,
            app.config.display.limiting_magnitude,
            moon,
        )
    });
    let visible = &visible;
    let horizon_style = Style::default().fg(palette.dim_line).bg(palette.bg);

    for azimuth in (0..360).step_by(2) {
        if let Some((x, y)) = view.project_horizontal(0.0, azimuth as f64) {
            set_cell(&mut grid, x, y, '.', horizon_style);
        }
    }

    if let Some((x, y)) = view.project_horizontal(90.0, 0.0) {
        set_cell(
            &mut grid,
            x,
            y,
            '+',
            Style::default().fg(palette.dim_line).bg(palette.bg),
        );
    }

    for star in visible.iter().rev() {
        let in_constellation = app.figure_contains_hip(code, star.star.hip)
            || (app.config.display.sky_culture == SkyCulture::Western
                && star.star.constellation.eq_ignore_ascii_case(code));
        let selected =
            matches!(app.selected_target, Some(Target::Star(hip)) if hip == star.star.hip);
        let symbol = if selected {
            selected_symbol(unicode)
        } else {
            star_symbol(star.star.magnitude, unicode, app.animation_tick)
        };
        let mut style = star_style(star.star.magnitude, star.star.color_index, palette);
        if selected {
            style = Style::default()
                .fg(palette.selected)
                .bg(palette.bg)
                .add_modifier(Modifier::BOLD);
        } else if in_constellation {
            style = style.add_modifier(Modifier::BOLD);
        }
        set_cell(&mut grid, star.x, star.y, symbol, style);
    }

    if app.config.display.constellations {
        draw_constellation_lines(&mut grid, visible, app, width, height, palette, unicode);
    }

    draw_deep_sky(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            view.project_horizontal(horizontal.altitude, horizontal.azimuth)
        },
        palette,
        unicode,
    );
    draw_planets(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            view.project_horizontal(horizontal.altitude, horizontal.azimuth)
        },
        palette,
        unicode,
    );

    if app.config.display.constellations {
        draw_constellation_labels(&mut grid, visible, app, width, height, palette, unicode);
    }

    label_cardinal_projected(&mut grid, 0.0, 'N', palette, |altitude, azimuth| {
        view.project_horizontal(altitude, azimuth)
    });
    label_cardinal_projected(&mut grid, 90.0, 'E', palette, |altitude, azimuth| {
        view.project_horizontal(altitude, azimuth)
    });
    label_cardinal_projected(&mut grid, 180.0, 'S', palette, |altitude, azimuth| {
        view.project_horizontal(altitude, azimuth)
    });
    label_cardinal_projected(&mut grid, 270.0, 'W', palette, |altitude, azimuth| {
        view.project_horizontal(altitude, azimuth)
    });

    if app.config.display.labels {
        let label_limit = if app.constellation_zoom { 4.0 } else { 1.65 };
        let label_count = if app.constellation_zoom { 16 } else { 9 };
        for star in visible
            .iter()
            .filter(|star| !star.star.proper.is_empty() && star.star.magnitude <= label_limit)
            .take(label_count)
        {
            draw_label(
                &mut grid,
                star.x,
                star.y,
                star.star.proper,
                palette,
                unicode,
            );
        }
    }

    draw_planet_labels(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            view.project_horizontal(horizontal.altitude, horizontal.azimuth)
        },
        palette,
        unicode,
    );

    draw_moon(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, &location, time);
            view.project_horizontal(horizontal.altitude, horizontal.azimuth)
        },
        palette,
        unicode,
    );

    if !app.pointer.active {
        draw_selected_label(
            &mut grid,
            app,
            |ra, dec| {
                let horizontal = astro::horizontal_position(ra, dec, &location, time);
                view.project_horizontal(horizontal.altitude, horizontal.azimuth)
            },
            palette,
            unicode,
        );
    }

    let meta = app.figure_meta(code);
    let title = match app.config.display.sky_culture {
        SkyCulture::Western => format!("{} · {}", meta.code, meta.en),
        SkyCulture::Chinese => format!("{} · {}", meta.zh, meta.en),
    };
    draw_text(
        &mut grid,
        1,
        0,
        &title,
        Style::default()
            .fg(palette.selected)
            .bg(palette.bg)
            .add_modifier(Modifier::BOLD),
        unicode,
    );

    draw_pointer(&mut grid, app, palette, unicode);
    Some(grid_to_lines(grid))
}

fn sky_lines(
    app: &App,
    visible: &[astro::VisibleStar],
    location: &Location,
    width: usize,
    height: usize,
    palette: Palette,
) -> Vec<Line<'static>> {
    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    let unicode = app.config.display.charset.canvas_unicode();
    let orientation = app.config.display.sky_orientation;
    let horizon_style = Style::default().fg(palette.dim_line).bg(palette.bg);
    let time = app.now();

    for azimuth in (0..360).step_by(2) {
        if let Some((x, y)) =
            astro::project_dome_for_orientation(0.0, azimuth as f64, width, height, orientation)
        {
            set_cell(&mut grid, x, y, '.', horizon_style);
        }
    }

    if let Some((x, y)) = astro::project_dome_for_orientation(90.0, 0.0, width, height, orientation)
    {
        set_cell(
            &mut grid,
            x,
            y,
            '+',
            Style::default().fg(palette.dim_line).bg(palette.bg),
        );
    }

    draw_landscape(
        &mut grid,
        app,
        location,
        width,
        height,
        app.config.display.landscape,
        orientation,
        palette,
    );

    for star in visible.iter().rev() {
        let selected =
            matches!(app.selected_target, Some(Target::Star(hip)) if hip == star.star.hip);
        let symbol = if selected {
            selected_symbol(unicode)
        } else {
            star_symbol(star.star.magnitude, unicode, app.animation_tick)
        };
        let style = if selected {
            Style::default()
                .fg(palette.selected)
                .bg(palette.bg)
                .add_modifier(Modifier::BOLD)
        } else {
            star_style(star.star.magnitude, star.star.color_index, palette)
        };
        set_cell(&mut grid, star.x, star.y, symbol, style);
    }

    if app.config.display.constellations {
        draw_constellation_lines(&mut grid, visible, app, width, height, palette, unicode);
    }

    draw_deep_sky(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome_for_orientation(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                orientation,
            )
        },
        palette,
        unicode,
    );
    draw_planets(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome_for_orientation(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                orientation,
            )
        },
        palette,
        unicode,
    );

    if app.config.display.constellations {
        draw_constellation_labels(&mut grid, visible, app, width, height, palette, unicode);
    }

    label_cardinal(&mut grid, width, height, 0.0, 'N', orientation, palette);
    label_cardinal(&mut grid, width, height, 90.0, 'E', orientation, palette);
    label_cardinal(&mut grid, width, height, 180.0, 'S', orientation, palette);
    label_cardinal(&mut grid, width, height, 270.0, 'W', orientation, palette);

    if app.config.display.labels {
        for star in visible
            .iter()
            .filter(|star| !star.star.proper.is_empty() && star.star.magnitude <= 1.65)
            .take(9)
        {
            draw_label(
                &mut grid,
                star.x,
                star.y,
                star.star.proper,
                palette,
                unicode,
            );
        }
    }

    draw_planet_labels(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome_for_orientation(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                orientation,
            )
        },
        palette,
        unicode,
    );

    draw_moon(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome_for_orientation(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                orientation,
            )
        },
        palette,
        unicode,
    );

    if !app.pointer.active {
        draw_selected_label(
            &mut grid,
            app,
            |ra, dec| {
                let horizontal = astro::horizontal_position(ra, dec, location, time);
                astro::project_dome_for_orientation(
                    horizontal.altitude,
                    horizontal.azimuth,
                    width,
                    height,
                    orientation,
                )
            },
            palette,
            unicode,
        );
    }
    draw_pointer(&mut grid, app, palette, unicode);

    grid_to_lines(grid)
}

fn star_symbol(magnitude: f64, unicode: bool, tick: u64) -> char {
    if unicode {
        if magnitude <= 0.0 {
            if tick % 60 < 30 { '✦' } else { '✧' }
        } else if magnitude <= 1.4 {
            '✶'
        } else if magnitude <= 2.7 {
            '•'
        } else {
            '·'
        }
    } else if magnitude <= 1.4 {
        '*'
    } else {
        '.'
    }
}

fn selected_symbol(unicode: bool) -> char {
    if unicode { '◎' } else { '@' }
}

fn draw_landscape(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    location: &Location,
    width: usize,
    height: usize,
    mode: LandscapeMode,
    orientation: SkyOrientation,
    palette: Palette,
) {
    if matches!(mode, LandscapeMode::Off) || width == 0 || height == 0 {
        return;
    }

    draw_local_ground_ring(
        grid,
        app,
        location,
        width,
        height,
        orientation,
        1.0,
        1.0,
        palette,
    );

    if !matches!(mode, LandscapeMode::Bearings) {
        return;
    }

    let tick_style = Style::default()
        .fg(palette.cyan)
        .bg(palette.bg)
        .add_modifier(Modifier::BOLD);
    for azimuth in (0..360).step_by(45) {
        if let Some((x, y)) =
            astro::project_dome_for_orientation(0.0, azimuth as f64, width, height, orientation)
        {
            let tick_len = if azimuth % 90 == 0 { 3 } else { 2 };
            for dy in 0..tick_len {
                set_cell(grid, x, (y + dy).min(height - 1), '|', tick_style);
            }
        }
    }
}

fn draw_local_ground_ring(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    location: &Location,
    width: usize,
    height: usize,
    orientation: SkyOrientation,
    dome_scale: f64,
    opacity: f64,
    palette: Palette,
) {
    if width < 4 || height < 4 {
        return;
    }

    let center_x = width.saturating_sub(1) as f64 / 2.0;
    let center_y = height.saturating_sub(1) as f64 / 2.0;
    let radius_x = center_x.max(1.0) * dome_scale.max(0.01);
    let radius_y = center_y.max(1.0) * dome_scale.max(0.01);
    let opacity = opacity.clamp(0.0, 1.0);
    let outer_radius = [
        (0.0, 0.0),
        (width.saturating_sub(1) as f64, 0.0),
        (0.0, height.saturating_sub(1) as f64),
        (
            width.saturating_sub(1) as f64,
            height.saturating_sub(1) as f64,
        ),
    ]
    .into_iter()
    .map(|(x, y)| {
        let dx = (x - center_x) / radius_x;
        let dy = (center_y - y) / radius_y;
        (dx * dx + dy * dy).sqrt()
    })
    .fold(1.0, f64::max);
    let ring_depth = (outer_radius - 1.0).max(0.001);

    let sun = solar::sun_position(app.now());
    let gmst_hours = astro::local_sidereal_time_hours(app.now(), 0.0);
    let subsolar_lon = normalize_degrees((sun.ra_hours - gmst_hours) * 15.0);
    let sun_lat = sun.dec_degrees.to_radians();
    let bg = color_to_rgb(palette.bg);
    let east_sign = match orientation {
        SkyOrientation::Observer => -1.0,
        SkyOrientation::Map => 1.0,
    };

    for y in 0..height {
        for x in 0..width {
            let dx = (x as f64 - center_x) / radius_x;
            let dy = (center_y - y as f64) / radius_y;
            let radius = (dx * dx + dy * dy).sqrt();
            if radius <= 1.0 {
                continue;
            }

            let azimuth = normalize_degrees((dx / east_sign).atan2(dy).to_degrees());
            let ring_t = ((radius - 1.0) / ring_depth).clamp(0.0, 1.0);
            let distance_km = lerp_f64(25.0, 2_400.0, smootherstep01(ring_t));
            let (lon, lat) =
                destination_point(location.latitude, location.longitude, azimuth, distance_km);
            let altitude = solar_altitude(lat.to_radians(), lon, sun_lat, subsolar_lon);
            let earth = globe_surface_color(lon, lat, altitude);
            let edge_fade = smootherstep01((radius - 1.0) / 0.055);
            let strength = opacity * edge_fade * lerp_f64(0.16, 0.52, smootherstep01(ring_t));
            let color = RgbColor::mix(bg, earth, strength).to_color();
            set_cell(grid, x, y, ' ', Style::default().fg(color).bg(color));
        }
    }
}

fn destination_point(lat: f64, lon: f64, bearing: f64, distance_km: f64) -> (f64, f64) {
    const EARTH_RADIUS_KM: f64 = 6_371.0;
    let angular_distance = distance_km / EARTH_RADIUS_KM;
    let lat1 = lat.to_radians();
    let lon1 = lon.to_radians();
    let bearing = bearing.to_radians();
    let (sin_lat1, cos_lat1) = lat1.sin_cos();
    let (sin_distance, cos_distance) = angular_distance.sin_cos();
    let (sin_bearing, cos_bearing) = bearing.sin_cos();

    let lat2 = (sin_lat1 * cos_distance + cos_lat1 * sin_distance * cos_bearing)
        .clamp(-1.0, 1.0)
        .asin();
    let lon2 =
        lon1 + (sin_bearing * sin_distance * cos_lat1).atan2(cos_distance - sin_lat1 * lat2.sin());

    (normalize_degrees(lon2.to_degrees()), lat2.to_degrees())
}

fn color_to_rgb(color: Color) -> RgbColor {
    match color {
        Color::Rgb(r, g, b) => RgbColor::new(r as f64, g as f64, b as f64),
        Color::Black => RgbColor::new(0.0, 0.0, 0.0),
        Color::DarkGray => RgbColor::new(64.0, 64.0, 64.0),
        Color::Gray => RgbColor::new(128.0, 128.0, 128.0),
        Color::White => RgbColor::new(255.0, 255.0, 255.0),
        _ => RgbColor::new(4.0, 8.0, 18.0),
    }
}

fn draw_pointer(grid: &mut [Vec<SkyCell>], app: &App, palette: Palette, unicode: bool) {
    if !app.pointer.active || grid.is_empty() {
        return;
    }
    let height = grid.len();
    let width = grid.first().map_or(0, Vec::len);
    if width == 0 || height == 0 {
        return;
    }

    let x = app.pointer.x.min(width - 1);
    let y = app.pointer.y.min(height - 1);
    let style = Style::default()
        .fg(palette.selected)
        .bg(palette.bg)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(palette.cyan).bg(palette.bg);
    let (tl, tr, bl, br, h, v, center) = if unicode {
        ('┌', '┐', '└', '┘', '─', '│', '+')
    } else {
        ('+', '+', '+', '+', '-', '|', '+')
    };

    for dx in -2isize..=2 {
        set_cell_offset(grid, x, y, dx, -2, h, dim);
        set_cell_offset(grid, x, y, dx, 2, h, dim);
    }
    for dy in -1isize..=1 {
        set_cell_offset(grid, x, y, -3, dy, v, dim);
        set_cell_offset(grid, x, y, 3, dy, v, dim);
    }
    set_cell_offset(grid, x, y, -3, -2, tl, dim);
    set_cell_offset(grid, x, y, 3, -2, tr, dim);
    set_cell_offset(grid, x, y, -3, 2, bl, dim);
    set_cell_offset(grid, x, y, 3, 2, br, dim);
    set_cell(grid, x, y, center, style);
}

fn set_cell_offset(
    grid: &mut [Vec<SkyCell>],
    x: usize,
    y: usize,
    dx: isize,
    dy: isize,
    ch: char,
    style: Style,
) {
    let Some(next_x) = x.checked_add_signed(dx) else {
        return;
    };
    let Some(next_y) = y.checked_add_signed(dy) else {
        return;
    };
    set_cell(grid, next_x, next_y, ch, style);
}

fn set_cell_offset_overlay(
    grid: &mut [Vec<SkyCell>],
    x: usize,
    y: usize,
    dx: isize,
    dy: isize,
    ch: char,
    style: Style,
) {
    let Some(next_x) = x.checked_add_signed(dx) else {
        return;
    };
    let Some(next_y) = y.checked_add_signed(dy) else {
        return;
    };
    set_cell_overlay(grid, next_x, next_y, ch, style);
}

fn star_style(magnitude: f64, color_index: Option<f64>, palette: Palette) -> Style {
    let color = match color_index {
        Some(ci) if ci < 0.0 => Color::Rgb(190, 220, 255),
        Some(ci) if ci < 0.55 => palette.silver,
        Some(ci) if ci < 1.1 => palette.warm,
        Some(_) => Color::Rgb(255, 198, 145),
        None => palette.silver,
    };
    let style = Style::default().fg(color).bg(palette.bg);
    if magnitude <= 1.4 {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn label_cardinal(
    grid: &mut [Vec<SkyCell>],
    width: usize,
    height: usize,
    azimuth: f64,
    label: char,
    orientation: SkyOrientation,
    palette: Palette,
) {
    if let Some((x, y)) =
        astro::project_dome_for_orientation(0.0, azimuth, width, height, orientation)
    {
        set_cell(
            grid,
            x,
            y,
            label,
            Style::default()
                .fg(palette.cyan)
                .bg(palette.bg)
                .add_modifier(Modifier::BOLD),
        );
    }
}

fn label_cardinal_projected<F>(
    grid: &mut [Vec<SkyCell>],
    azimuth: f64,
    label: char,
    palette: Palette,
    mut project: F,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    if let Some((x, y)) = project(0.0, azimuth) {
        set_cell(
            grid,
            x,
            y,
            label,
            Style::default()
                .fg(palette.cyan)
                .bg(palette.bg)
                .add_modifier(Modifier::BOLD),
        );
    }
}

fn draw_constellation_lines(
    grid: &mut [Vec<SkyCell>],
    visible: &[astro::VisibleStar],
    app: &App,
    _width: usize,
    _height: usize,
    palette: Palette,
    unicode: bool,
) {
    let points = visible
        .iter()
        .map(|star| (star.star.hip, (star.x, star.y)))
        .collect::<HashMap<_, _>>();
    let selected = app.selected_constellation_code();

    for constellation in app.active_constellation_lines() {
        let highlighted =
            selected.is_some_and(|code| code.eq_ignore_ascii_case(constellation.code));
        let dimmed = selected.is_some() && !highlighted;
        let line_color = if highlighted {
            palette.selected
        } else if dimmed {
            palette.dim_line
        } else {
            palette.line
        };
        let line_style = Style::default().fg(line_color).bg(palette.bg);
        let joint_style = if highlighted {
            line_style.add_modifier(Modifier::BOLD)
        } else {
            line_style
        };
        let mut endpoints = Vec::new();

        for pair in constellation.hips.windows(2) {
            let Some(&(x0, y0)) = points.get(&pair[0]) else {
                continue;
            };
            let Some(&(x1, y1)) = points.get(&pair[1]) else {
                continue;
            };
            draw_line(grid, x0, y0, x1, y1, line_style, unicode);
            endpoints.push((x0, y0));
            endpoints.push((x1, y1));
        }

        endpoints.sort_unstable();
        endpoints.dedup();
        for &(x, y) in &endpoints {
            set_cell(grid, x, y, if unicode { '○' } else { 'o' }, joint_style);
        }
    }
}

fn draw_backdrop_constellation_lines(
    grid: &mut [Vec<SkyCell>],
    visible: &[astro::VisibleStar],
    app: &App,
    width: usize,
    palette: Palette,
    unicode: bool,
) {
    let points = visible
        .iter()
        .map(|star| (star.star.hip, (star.x, star.y)))
        .collect::<HashMap<_, _>>();
    let selected = app.selected_constellation_code();

    for constellation in app.active_constellation_lines() {
        let highlighted =
            selected.is_some_and(|code| code.eq_ignore_ascii_case(constellation.code));
        let dimmed = selected.is_some() && !highlighted;
        let line_color = if highlighted {
            palette.selected
        } else if dimmed {
            palette.dim_line
        } else {
            palette.line
        };
        let line_style = Style::default().fg(line_color).bg(palette.bg);
        let joint_style = if highlighted {
            line_style.add_modifier(Modifier::BOLD)
        } else {
            line_style
        };
        let mut endpoints = Vec::new();

        for pair in constellation.hips.windows(2) {
            let Some(&(x0, y0)) = points.get(&pair[0]) else {
                continue;
            };
            let Some(&(x1, y1)) = points.get(&pair[1]) else {
                continue;
            };
            if width > 0 && x0.abs_diff(x1) > width / 2 {
                continue;
            }
            draw_line(grid, x0, y0, x1, y1, line_style, unicode);
            endpoints.push((x0, y0));
            endpoints.push((x1, y1));
        }

        endpoints.sort_unstable();
        endpoints.dedup();
        for &(x, y) in &endpoints {
            set_cell(grid, x, y, if unicode { '○' } else { 'o' }, joint_style);
        }
    }
}

fn draw_constellation_labels(
    grid: &mut [Vec<SkyCell>],
    visible: &[astro::VisibleStar],
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
    unicode: bool,
) {
    if !app.config.display.labels {
        return;
    }

    let selected = app.selected_constellation_code();
    for hit in constellation_label_hits(visible, app, width, height, unicode) {
        let highlighted = selected.is_some_and(|code| code.eq_ignore_ascii_case(&hit.code));
        let label_style = Style::default()
            .fg(if highlighted {
                palette.selected
            } else {
                palette.line
            })
            .bg(palette.bg)
            .add_modifier(Modifier::BOLD);
        draw_text(grid, hit.x, hit.y, &hit.label, label_style, unicode);
    }
}

#[derive(Debug, Clone)]
struct ConstellationLabelHit {
    code: String,
    label: String,
    x: usize,
    y: usize,
    width: usize,
}

fn constellation_label_hits(
    visible: &[astro::VisibleStar],
    app: &App,
    width: usize,
    height: usize,
    unicode: bool,
) -> Vec<ConstellationLabelHit> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let points = visible
        .iter()
        .map(|star| (star.star.hip, (star.x, star.y)))
        .collect::<HashMap<_, _>>();

    let mut groups: Vec<(String, Vec<(usize, usize)>)> = Vec::new();
    let mut group_index = HashMap::new();
    for constellation in app.active_constellation_lines() {
        let mut endpoints = Vec::new();
        for pair in constellation.hips.windows(2) {
            let Some(&(x0, y0)) = points.get(&pair[0]) else {
                continue;
            };
            let Some(&(x1, y1)) = points.get(&pair[1]) else {
                continue;
            };
            endpoints.push((x0, y0));
            endpoints.push((x1, y1));
        }

        if endpoints.is_empty() {
            continue;
        }

        let index = *group_index
            .entry(constellation.code.to_string())
            .or_insert_with(|| {
                groups.push((constellation.code.to_string(), Vec::new()));
                groups.len() - 1
            });
        groups[index].1.extend(endpoints);
    }

    groups
        .into_iter()
        .filter_map(|(code, mut endpoints)| {
            endpoints.sort_unstable();
            endpoints.dedup();
            if endpoints.is_empty() {
                return None;
            }

            let sum_x = endpoints.iter().map(|(x, _)| *x).sum::<usize>();
            let sum_y = endpoints.iter().map(|(_, y)| *y).sum::<usize>();
            let x = (sum_x / endpoints.len()).min(width - 1);
            let y = (sum_y / endpoints.len()).min(height - 1);
            let label_x = x.saturating_add(1);
            if label_x >= width {
                return None;
            }
            let label = app.figure_label(&code);
            let label_width = canvas_label_width(&label, unicode).min(width - label_x);
            (label_width > 0).then_some(ConstellationLabelHit {
                code,
                label,
                x: label_x,
                y,
                width: label_width,
            })
        })
        .collect()
}

fn sky_constellation_label_hit(
    app: &App,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) -> Option<String> {
    if !app.config.display.constellations || !app.config.display.labels {
        return None;
    }

    let unicode = app.config.display.charset.canvas_unicode();
    let visible = sky_visible_for_hit(app, width, height);
    constellation_label_hits(&visible, app, width, height, unicode)
        .into_iter()
        .find(|hit| {
            let start = hit.x.saturating_sub(1);
            let end = hit.x.saturating_add(hit.width).saturating_add(1).min(width);
            y == hit.y && x >= start && x < end
        })
        .map(|hit| hit.code)
}

fn sky_visible_for_hit(app: &App, width: usize, height: usize) -> Vec<astro::VisibleStar> {
    if app.should_render_zoomed_sky() && app.zoom_render_code().is_some() {
        if let Some(view) = app.zoom_render_view(width, height) {
            return view.stars;
        }
    }

    let sky_location = app.render_location();
    visible_stars_for_canvas(
        app,
        &sky_location,
        width,
        height,
        app.config.display.sky_orientation,
    )
}

#[derive(Debug, Clone, Copy)]
struct MoonRenderContext {
    position: astro::MoonPosition,
    horizontal: astro::HorizontalPosition,
    phase: astro::MoonPhase,
    angular_radius: f64,
}

fn moon_render_context(
    location: &Location,
    time: chrono::DateTime<chrono::Utc>,
) -> MoonRenderContext {
    let position = astro::moon_position(time);
    let horizontal =
        astro::horizontal_position(position.ra_hours, position.dec_degrees, location, time);
    MoonRenderContext {
        position,
        horizontal,
        phase: astro::moon_phase(time),
        angular_radius: astro::moon_angular_radius_degrees(position.distance_earth_radii),
    }
}

fn visible_stars_for_canvas(
    app: &App,
    location: &Location,
    width: usize,
    height: usize,
    orientation: SkyOrientation,
) -> Vec<astro::VisibleStar> {
    let time = app.now();
    let limiting_magnitude = app.config.display.limiting_magnitude;
    let moon = moon_render_context(location, time);
    astro::visible_stars_for_orientation(
        &app.catalog.stars,
        location,
        time,
        limiting_magnitude,
        width,
        height,
        orientation,
    )
    .into_iter()
    .filter(|visible| {
        star_survives_moonlight(visible.star, location, time, limiting_magnitude, moon)
    })
    .collect()
}

fn star_survives_moonlight(
    star: crate::catalog::Star,
    location: &Location,
    time: chrono::DateTime<chrono::Utc>,
    limiting_magnitude: f64,
    moon: MoonRenderContext,
) -> bool {
    if moon.horizontal.altitude <= 0.0 {
        return true;
    }
    let separation = astro::angular_separation_degrees(
        star.ra_hours,
        star.dec_degrees,
        moon.position.ra_hours,
        moon.position.dec_degrees,
    );
    if separation <= moon.angular_radius {
        return false;
    }
    let horizontal = astro::horizontal_position(star.ra_hours, star.dec_degrees, location, time);
    let loss = astro::moonlight_limiting_magnitude_loss(
        moon.horizontal.altitude,
        horizontal.altitude,
        separation,
        moon.phase,
    );
    star.magnitude <= limiting_magnitude - loss
}

fn draw_deep_sky<F>(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    mut project: F,
    palette: Palette,
    unicode: bool,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    if !app.config.display.deep_sky {
        return;
    }
    for object in &app.deep_sky {
        if object.magnitude.unwrap_or(99.0) > 9.5 {
            continue;
        }
        let Some((x, y)) = project(object.ra_hours, object.dec_degrees) else {
            continue;
        };
        let selected =
            matches!(app.selected_target, Some(Target::DeepSky(name)) if name == object.name);
        set_cell(
            grid,
            x,
            y,
            if selected {
                selected_symbol(unicode)
            } else if unicode {
                '◇'
            } else {
                'x'
            },
            Style::default()
                .fg(if selected {
                    palette.selected
                } else {
                    palette.deep
                })
                .bg(palette.bg)
                .add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        );
    }
}

fn draw_planets<F>(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    mut project: F,
    palette: Palette,
    unicode: bool,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    if !app.config.display.planets {
        return;
    }
    for planet in planets::visible_planets(app.now()) {
        let Some((x, y)) = project(planet.ra_hours, planet.dec_degrees) else {
            continue;
        };
        let selected =
            matches!(app.selected_target, Some(Target::Planet(name)) if name == planet.name);
        set_cell(
            grid,
            x,
            y,
            if selected {
                selected_symbol(unicode)
            } else {
                planet.symbol
            },
            Style::default()
                .fg(if selected {
                    palette.selected
                } else {
                    palette.moon
                })
                .bg(palette.bg)
                .add_modifier(Modifier::BOLD),
        );
    }
}

fn draw_moon<F>(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    mut project: F,
    palette: Palette,
    unicode: bool,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    let location = app.render_location();
    let time = app.now();
    let moon = moon_render_context(&location, time);
    let Some((x, y)) = project(moon.position.ra_hours, moon.position.dec_degrees) else {
        return;
    };
    let selected = matches!(app.selected_target, Some(Target::Moon));
    let phase = moon.phase;
    let symbol = if selected {
        selected_symbol(unicode)
    } else if unicode {
        i18n::moon_symbol(phase).chars().next().unwrap_or('●')
    } else {
        'M'
    };
    let style = Style::default()
        .fg(if selected {
            palette.selected
        } else {
            palette.moon
        })
        .bg(palette.bg)
        .add_modifier(Modifier::BOLD);
    set_cell(grid, x, y, symbol, style);
    if app.config.display.labels {
        draw_text(
            grid,
            x.saturating_add(2),
            y,
            i18n::tr(app.config.language, "moon"),
            Style::default()
                .fg(if selected {
                    palette.selected
                } else {
                    palette.moon
                })
                .bg(palette.bg)
                .add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            unicode,
        );
    }
}

fn draw_planet_labels<F>(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    mut project: F,
    palette: Palette,
    unicode: bool,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    if !app.config.display.planets || !app.config.display.labels {
        return;
    }
    for planet in planets::visible_planets(app.now()) {
        let Some((x, y)) = project(planet.ra_hours, planet.dec_degrees) else {
            continue;
        };
        draw_text(
            grid,
            x.saturating_add(2),
            y,
            planet.name,
            Style::default().fg(palette.moon).bg(palette.bg),
            unicode,
        );
    }
}

fn draw_selected_label<F>(
    grid: &mut [Vec<SkyCell>],
    app: &App,
    mut project: F,
    palette: Palette,
    unicode: bool,
) where
    F: FnMut(f64, f64) -> Option<(usize, usize)>,
{
    let Some((label, ra, dec)) = selected_coordinates(app) else {
        return;
    };
    let Some((x, y)) = project(ra, dec) else {
        return;
    };
    draw_text(
        grid,
        x.saturating_add(2),
        y,
        &label,
        Style::default()
            .fg(palette.selected)
            .bg(palette.bg)
            .add_modifier(Modifier::BOLD),
        unicode,
    );
}

fn selected_coordinates(app: &App) -> Option<(String, f64, f64)> {
    match app.selected_target.as_ref()? {
        Target::Star(hip) => app.star_by_hip(*hip).map(|star| {
            (
                star_aliases::display_name(star),
                star.ra_hours,
                star.dec_degrees,
            )
        }),
        Target::Planet(name) => app
            .planet_by_name(name)
            .map(|planet| (planet.name.to_string(), planet.ra_hours, planet.dec_degrees)),
        Target::Moon => {
            let moon = astro::moon_position(app.now());
            Some((
                i18n::tr(app.config.language, "moon").to_string(),
                moon.ra_hours,
                moon.dec_degrees,
            ))
        }
        Target::DeepSky(name) => app
            .deep_sky_by_name(name)
            .map(|object| (object.name.to_string(), object.ra_hours, object.dec_degrees)),
        Target::Constellation(_) | Target::City(_) => None,
    }
}

fn draw_line(
    grid: &mut [Vec<SkyCell>],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    style: Style,
    unicode: bool,
) {
    let dx = x1 as isize - x0 as isize;
    let dy = y1 as isize - y0 as isize;
    let steps = dx.abs().max(dy.abs()) as usize;
    if steps == 0 {
        return;
    }

    let mut last_x = x0 as isize;
    let mut last_y = y0 as isize;
    for step in 1..steps {
        let t = step as f64 / steps as f64;
        let x = (x0 as f64 + dx as f64 * t).round() as isize;
        let y = (y0 as f64 + dy as f64 * t).round() as isize;
        if x == last_x && y == last_y {
            continue;
        }
        let ch = line_char(x - last_x, y - last_y, unicode);
        set_cell(grid, x as usize, y as usize, ch, style);
        last_x = x;
        last_y = y;
    }
}

fn draw_line_overlay(
    grid: &mut [Vec<SkyCell>],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    style: Style,
    unicode: bool,
) {
    let dx = x1 as isize - x0 as isize;
    let dy = y1 as isize - y0 as isize;
    let steps = dx.abs().max(dy.abs()) as usize;
    if steps == 0 {
        return;
    }

    let mut last_x = x0 as isize;
    let mut last_y = y0 as isize;
    for step in 1..steps {
        let t = step as f64 / steps as f64;
        let x = (x0 as f64 + dx as f64 * t).round() as isize;
        let y = (y0 as f64 + dy as f64 * t).round() as isize;
        if x == last_x && y == last_y {
            continue;
        }
        let ch = line_char(x - last_x, y - last_y, unicode);
        set_cell_overlay(grid, x as usize, y as usize, ch, style);
        last_x = x;
        last_y = y;
    }
}

fn line_char(dx: isize, dy: isize, unicode: bool) -> char {
    if unicode {
        if dx == 0 {
            '│'
        } else if dy == 0 {
            '─'
        } else if dx.signum() == dy.signum() {
            '╲'
        } else {
            '╱'
        }
    } else if dx == 0 {
        '|'
    } else if dy == 0 {
        '-'
    } else if dx.signum() == dy.signum() {
        '\\'
    } else {
        '/'
    }
}

fn draw_label(
    grid: &mut [Vec<SkyCell>],
    x: usize,
    y: usize,
    label: &str,
    palette: Palette,
    unicode: bool,
) {
    if grid.is_empty() || y >= grid.len() {
        return;
    }
    let width = grid[y].len();
    let start = x.saturating_add(2);
    if start >= width {
        return;
    }
    let style = Style::default().fg(palette.silver).bg(palette.bg);
    draw_text(grid, start, y, label, style, unicode);
}

fn draw_text(
    grid: &mut [Vec<SkyCell>],
    x: usize,
    y: usize,
    text: &str,
    style: Style,
    unicode: bool,
) {
    if grid.is_empty() || y >= grid.len() {
        return;
    }
    let width = grid[y].len();
    if x >= width {
        return;
    }
    let mut cursor = x;
    for ch in text.chars().filter_map(|ch| canvas_label_char(ch, unicode)) {
        let char_width = canvas_label_char_width(ch, unicode);
        if cursor.saturating_add(char_width) > width {
            break;
        }
        set_cell(grid, cursor, y, ch, style);
        for continuation in 1..char_width {
            set_cell(grid, cursor + continuation, y, ' ', style);
        }
        cursor += char_width;
    }
}

fn draw_text_overlay(
    grid: &mut [Vec<SkyCell>],
    x: usize,
    y: usize,
    text: &str,
    style: Style,
    unicode: bool,
) {
    if grid.is_empty() || y >= grid.len() {
        return;
    }
    let width = grid[y].len();
    if x >= width {
        return;
    }
    let mut cursor = x;
    for ch in text.chars().filter_map(|ch| canvas_label_char(ch, unicode)) {
        let char_width = canvas_label_char_width(ch, unicode);
        if cursor.saturating_add(char_width) > width {
            break;
        }
        set_cell_overlay(grid, cursor, y, ch, style);
        for continuation in 1..char_width {
            set_cell_overlay(grid, cursor + continuation, y, ' ', style);
        }
        cursor += char_width;
    }
}

fn canvas_label_char(ch: char, unicode: bool) -> Option<char> {
    if ch.is_ascii() || (unicode && !ch.is_control()) {
        Some(ch)
    } else {
        None
    }
}

fn canvas_label_width(text: &str, unicode: bool) -> usize {
    text.chars()
        .filter_map(|ch| canvas_label_char(ch, unicode))
        .map(|ch| canvas_label_char_width(ch, unicode))
        .sum()
}

fn canvas_label_char_width(ch: char, unicode: bool) -> usize {
    if unicode {
        UnicodeWidthChar::width(ch).unwrap_or(1).max(1)
    } else {
        1
    }
}

fn set_cell(grid: &mut [Vec<SkyCell>], x: usize, y: usize, ch: char, style: Style) {
    if let Some(row) = grid.get_mut(y) {
        if let Some(cell) = row.get_mut(x) {
            *cell = SkyCell { ch, style };
        }
    }
}

fn set_cell_overlay(grid: &mut [Vec<SkyCell>], x: usize, y: usize, ch: char, style: Style) {
    if let Some(row) = grid.get_mut(y) {
        if let Some(cell) = row.get_mut(x) {
            let style = if let Some(bg) = cell.style.bg {
                style.bg(bg)
            } else {
                style
            };
            *cell = SkyCell { ch, style };
        }
    }
}

fn grid_to_lines(grid: Vec<Vec<SkyCell>>) -> Vec<Line<'static>> {
    grid.into_iter()
        .map(|row| {
            let mut skip_continuation_cells = 0usize;
            Line::from(
                row.into_iter()
                    .filter_map(|cell| {
                        if skip_continuation_cells > 0 {
                            skip_continuation_cells -= 1;
                            return None;
                        }
                        skip_continuation_cells = UnicodeWidthChar::width(cell.ch)
                            .unwrap_or(1)
                            .saturating_sub(1);
                        Some(Span::styled(cell.ch.to_string(), cell.style))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn draw_side_panel(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "observatory")))
        .title_style(
            Style::default()
                .fg(palette.moon)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    frame.render_widget(block, area);

    let mut lines = Vec::new();
    if app.config.display.moon_panel {
        lines.extend(moon_lines(app, palette));
        lines.push(Line::from(""));
    }
    lines.extend(sunlight_lines(app, palette));
    if app.pointer.active {
        lines.push(Line::from(""));
        lines.extend(pointer_lines(app, palette));
    }
    if app.selected_target.is_some() {
        lines.push(Line::from(""));
        lines.extend(target_lines(app, palette));
    }
    if app.show_recommendations {
        lines.push(Line::from(""));
        lines.extend(recommendation_lines(app, palette));
    }
    lines.push(Line::from(""));
    lines.extend(legend_lines(app, palette));

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver).bg(palette.panel))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn moon_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let phase = astro::moon_phase(app.now());
    let mut lines = vec![Line::from(Span::styled(
        format!(
            "{} {}",
            i18n::moon_symbol(phase),
            i18n::tr(language, "moon")
        ),
        Style::default()
            .fg(palette.moon)
            .add_modifier(Modifier::BOLD),
    ))];
    for row in moon_art(
        phase.phase_fraction,
        app.config.display.charset.canvas_unicode(),
    ) {
        lines.push(Line::from(Span::styled(
            row,
            Style::default().fg(palette.moon),
        )));
    }
    lines.extend([
        Line::from(format!(
            "{}: {}",
            i18n::tr(language, "phase"),
            i18n::moon_phase_name(language, phase)
        )),
        Line::from(format!(
            "{}: {:.0}%",
            i18n::tr(language, "illumination"),
            phase.illumination * 100.0
        )),
        Line::from(format!(
            "{}: {:.1} {}",
            i18n::tr(language, "age"),
            phase.age_days,
            i18n::tr(language, "days")
        )),
    ]);
    lines
}

fn sunlight_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let location = app.active_location();
    let light = solar::skylight(location, app.now());
    let events = solar::tonight_events(location, app.now());
    let timezone = app
        .active_location()
        .timezone
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let light_name = match language {
        Language::En => light.name_en(),
        Language::Zh => light.name_zh(),
    };
    let sun = solar::sun_position(app.now());
    let sun_horizontal =
        astro::horizontal_position(sun.ra_hours, sun.dec_degrees, location, app.now());
    let sunset = events
        .sunset
        .map(|time| time.with_timezone(&timezone).format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string());
    let sunrise = events
        .sunrise
        .map(|time| time.with_timezone(&timezone).format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string());
    vec![
        Line::from(Span::styled(
            i18n::tr(language, "sunlight"),
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            sun_altitude_art(sun_horizontal.altitude),
            Style::default().fg(palette.warm),
        )),
        Line::from(format!(
            "{}: {}",
            i18n::tr(language, "sky_light"),
            light_name
        )),
        Line::from(format!(
            "Alt {:+.1}  Az {:.1}",
            sun_horizontal.altitude, sun_horizontal.azimuth
        )),
        Line::from(format!(
            "{}: {}  {}: {}",
            i18n::tr(language, "sunset"),
            sunset,
            i18n::tr(language, "sunrise"),
            sunrise
        )),
    ]
}

fn pointer_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let hits = app.targets_at_pointer();
    let mut lines = vec![
        Line::from(Span::styled(
            i18n::tr(language, "pointer"),
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(i18n::tr(language, "pointer_hint").to_string()),
    ];

    if hits.is_empty() {
        lines.push(Line::from(
            i18n::tr(language, "pointer_no_star").to_string(),
        ));
        lines.push(Line::from(format!(
            "x {:>3}  y {:>3}",
            app.pointer.x, app.pointer.y
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("{} ({})", i18n::tr(language, "pointer_hits"), hits.len()),
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for (index, target) in hits.iter().take(4).enumerate() {
            if index > 0 {
                lines.push(Line::from(""));
            }
            push_target_summary(&mut lines, app, target);
        }
        if hits.len() > 4 {
            lines.push(Line::from(format!("+{} more", hits.len() - 4)));
        }
    }
    lines
}

fn push_star_summary(lines: &mut Vec<Line<'static>>, app: &App, star: crate::catalog::Star) {
    let language = app.config.language;
    let horizontal = astro::horizontal_position(
        star.ra_hours,
        star.dec_degrees,
        app.active_location(),
        app.now(),
    );
    let meta = constellations::meta_for(star.constellation);
    lines.push(Line::from(star_aliases::display_name_for(star, language)));
    lines.push(Line::from(format!(
        "HIP {} · mag {:.1} · {}",
        star.hip, star.magnitude, meta.en
    )));
    lines.push(Line::from(format!(
        "Alt {:+.1}  Az {:.1}",
        horizontal.altitude, horizontal.azimuth
    )));
    lines.push(Line::from(format!(
        "RA {}  Dec {:+.1}",
        format_ra(star.ra_hours),
        star.dec_degrees
    )));
    if let Some(aliases) = star_aliases::summary_aliases_for(star, language) {
        lines.push(Line::from(format!("aka {aliases}")));
    }
    lines.push(Line::from(format!(
        "{}: {}",
        i18n::tr(language, "visible_state"),
        on_off(language, horizontal.altitude > 0.0)
    )));
}

fn push_target_summary(lines: &mut Vec<Line<'static>>, app: &App, target: &Target) {
    match target {
        Target::Star(hip) => {
            if let Some(star) = app.star_by_hip(*hip) {
                push_star_summary(lines, app, star);
            }
        }
        Target::Planet(name) => {
            if let Some(planet) = app.planet_by_name(name) {
                push_planet_summary(lines, app, planet);
            }
        }
        Target::Moon => push_moon_summary(lines, app),
        Target::DeepSky(name) => {
            if let Some(object) = app.deep_sky_by_name(name) {
                push_deep_sky_summary(lines, app, object);
            }
        }
        Target::Constellation(code) => {
            let meta = app.figure_meta(code);
            match app.config.display.sky_culture {
                SkyCulture::Western => {
                    lines.push(Line::from(format!("{} · {}", meta.code, meta.en)));
                    lines.push(Line::from(meta.zh.to_string()));
                }
                SkyCulture::Chinese => {
                    lines.push(Line::from(format!("{} · {}", meta.zh, meta.en)));
                    if let Some(pinyin) = constellations::pinyin_for(code) {
                        lines.push(Line::from(pinyin.to_string()));
                    }
                }
            }
        }
        Target::City(_) => {}
    }
}

fn push_planet_summary(lines: &mut Vec<Line<'static>>, app: &App, planet: planets::Planet) {
    push_position(lines, app, planet.name, planet.ra_hours, planet.dec_degrees);
    lines.push(Line::from(format!("planet · mag {:.1}", planet.magnitude)));
}

fn push_moon_summary(lines: &mut Vec<Line<'static>>, app: &App) {
    let moon = astro::moon_position(app.now());
    let phase = astro::moon_phase(app.now());
    push_position(
        lines,
        app,
        i18n::tr(app.config.language, "moon"),
        moon.ra_hours,
        moon.dec_degrees,
    );
    lines.push(Line::from(i18n::moon_phase_name(
        app.config.language,
        phase,
    )));
    lines.push(Line::from(format!(
        "{}: {:.0}%",
        i18n::tr(app.config.language, "illumination"),
        phase.illumination * 100.0
    )));
}

fn push_deep_sky_summary(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    object: crate::deep_sky::DeepSkyObject,
) {
    let language = app.config.language;
    let label = if object.common.is_empty() {
        object.name.to_string()
    } else {
        format!("{} · {}", object.name, object.common)
    };
    push_position(lines, app, &label, object.ra_hours, object.dec_degrees);
    lines.push(Line::from(format!(
        "{} · mag {}",
        object.kind,
        object
            .magnitude
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "?".to_string())
    )));
    let meta = constellations::meta_for(object.constellation);
    lines.push(Line::from(format!(
        "{}: {}",
        i18n::tr(language, "constellation"),
        meta.en
    )));
}

fn target_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let mut lines = vec![Line::from(Span::styled(
        i18n::tr(language, "target"),
        Style::default()
            .fg(palette.selected)
            .add_modifier(Modifier::BOLD),
    ))];
    match app.selected_target.as_ref() {
        Some(Target::Star(hip)) => {
            if let Some(star) = app.star_by_hip(*hip) {
                push_position(
                    &mut lines,
                    app,
                    &star_aliases::display_name_for(star, language),
                    star.ra_hours,
                    star.dec_degrees,
                );
                lines.push(Line::from(format!(
                    "HIP {} · mag {:.1}",
                    star.hip, star.magnitude
                )));
                if let Some(aliases) = star_aliases::summary_aliases_for(star, language) {
                    lines.push(Line::from(format!("aka {aliases}")));
                }
                let meta = constellations::meta_for(star.constellation);
                lines.push(Line::from(format!(
                    "{}: {}",
                    i18n::tr(language, "constellation"),
                    meta.en
                )));
            }
        }
        Some(Target::Constellation(code)) => {
            let meta = app.figure_meta(code);
            match app.config.display.sky_culture {
                SkyCulture::Western => {
                    lines.push(Line::from(format!("{} · {}", meta.code, meta.en)));
                    lines.push(Line::from(meta.zh.to_string()));
                }
                SkyCulture::Chinese => {
                    lines.push(Line::from(format!("{} · {}", meta.zh, meta.en)));
                    if let Some(pinyin) = constellations::pinyin_for(code) {
                        lines.push(Line::from(pinyin.to_string()));
                    }
                }
            }
            let visible = app.constellation_visible(code);
            if !visible {
                lines.push(Line::from(Span::styled(
                    i18n::tr(language, "constellation_not_visible_here"),
                    Style::default()
                        .fg(palette.warm)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(i18n::tr(language, "constellation_pulled_back")));
            }
            lines.push(Line::from(Span::styled(
                if !visible {
                    i18n::tr(language, "constellation_hidden_hint")
                } else if app.constellation_zoom {
                    i18n::tr(language, "constellation_zoom_exit_hint")
                } else {
                    i18n::tr(language, "constellation_zoom_hint")
                },
                Style::default().fg(palette.warm),
            )));
            lines.push(Line::from(format!(
                "{}: {}",
                i18n::tr(language, "visible_state"),
                on_off(language, visible)
            )));
            let segments = app
                .active_constellation_lines()
                .iter()
                .filter(|line| line.code.eq_ignore_ascii_case(code))
                .map(crate::constellations::ConstellationLine::segment_count)
                .sum::<usize>();
            lines.push(Line::from(format!("segments: {segments}")));
        }
        Some(Target::Planet(name)) => {
            if let Some(planet) = app.planet_by_name(name) {
                push_position(
                    &mut lines,
                    app,
                    planet.name,
                    planet.ra_hours,
                    planet.dec_degrees,
                );
                lines.push(Line::from(format!("planet · mag {:.1}", planet.magnitude)));
                lines.push(Line::from(format!(
                    "RA {}  Dec {:+.1}",
                    format_ra(planet.ra_hours),
                    planet.dec_degrees
                )));
            }
        }
        Some(Target::Moon) => push_moon_summary(&mut lines, app),
        Some(Target::DeepSky(name)) => {
            if let Some(object) = app.deep_sky_by_name(name) {
                let label = if object.common.is_empty() {
                    object.name
                } else {
                    object.common
                };
                push_position(&mut lines, app, label, object.ra_hours, object.dec_degrees);
                lines.push(Line::from(format!(
                    "{} · mag {}",
                    object.kind,
                    object
                        .magnitude
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "?".to_string())
                )));
                let meta = constellations::meta_for(object.constellation);
                lines.push(Line::from(format!(
                    "{}: {}",
                    i18n::tr(language, "constellation"),
                    meta.en
                )));
                lines.push(Line::from(format!(
                    "RA {}  Dec {:+.1}",
                    format_ra(object.ra_hours),
                    object.dec_degrees
                )));
            }
        }
        Some(Target::City(_)) => {}
        None => {}
    }
    lines
}

fn push_position(
    lines: &mut Vec<Line<'static>>,
    app: &App,
    label: &str,
    ra_hours: f64,
    dec_degrees: f64,
) {
    let horizontal =
        astro::horizontal_position(ra_hours, dec_degrees, app.active_location(), app.now());
    lines.push(Line::from(label.to_string()));
    lines.push(Line::from(format!(
        "Alt {:+.1}  Az {:.1}",
        horizontal.altitude, horizontal.azimuth
    )));
    lines.push(Line::from(format!(
        "RA {}  Dec {:+.1}",
        format_ra(ra_hours),
        dec_degrees
    )));
    lines.push(Line::from(format!(
        "{}: {}",
        i18n::tr(app.config.language, "visible_state"),
        on_off(app.config.language, horizontal.altitude > 0.0)
    )));
}

fn recommendation_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let mut lines = vec![Line::from(Span::styled(
        i18n::tr(language, "tonight"),
        Style::default()
            .fg(palette.cyan)
            .add_modifier(Modifier::BOLD),
    ))];

    if let Some(star) = brightest_visible_star(app) {
        let label = star_aliases::display_name_for(star, language);
        lines.push(Line::from(format!(
            "{}: {} ({:.1})",
            i18n::tr(language, "bright_star"),
            label,
            star.magnitude
        )));
    }

    let moon = astro::moon_phase(app.now());
    lines.push(Line::from(format!(
        "{}: {:.0}%",
        i18n::tr(language, "moonlight"),
        moon.illumination * 100.0
    )));

    if let Some(planet) = planets::visible_planets(app.now())
        .into_iter()
        .find(|planet| {
            astro::horizontal_position(
                planet.ra_hours,
                planet.dec_degrees,
                app.active_location(),
                app.now(),
            )
            .altitude
                > 0.0
        })
    {
        lines.push(Line::from(format!(
            "{}: {}",
            i18n::tr(language, "planet"),
            planet.name
        )));
    }

    if let Some(object) = app.deep_sky.iter().find(|object| {
        object.magnitude.unwrap_or(99.0) <= 7.0
            && astro::horizontal_position(
                object.ra_hours,
                object.dec_degrees,
                app.active_location(),
                app.now(),
            )
            .altitude
                > 20.0
    }) {
        lines.push(Line::from(format!(
            "{}: {} {}",
            i18n::tr(language, "deep_sky"),
            object.name,
            object.common
        )));
    }

    let visible = app.visible_constellation_codes();
    if let Some(code) = visible.first() {
        let meta = app.figure_meta(code);
        lines.push(Line::from(format!(
            "{}: {}",
            i18n::tr(language, app.figure_kind_key()),
            constellations::display_name_for_culture(app.config.display.sky_culture, meta.code)
        )));
    }
    lines
}

fn legend_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    if app.view_mode == ViewMode::Ground {
        return globe_legend_lines(app, palette);
    }
    sky_legend_lines(app, palette)
}

fn sky_legend_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let unicode = app.config.display.charset.canvas_unicode();
    vec![
        Line::from(Span::styled(
            i18n::tr(language, "legend_sky"),
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "{} {}",
            star_symbol(-1.0, unicode, 0),
            i18n::tr(language, "legend_bright_star")
        )),
        Line::from(format!(
            "{} {}",
            star_symbol(4.0, unicode, 0),
            i18n::tr(language, "legend_dim_star")
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '○' } else { 'o' },
            i18n::tr(language, app.figure_node_legend_key())
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '◇' } else { 'x' },
            i18n::tr(language, "legend_deep_sky")
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '●' } else { 'M' },
            i18n::tr(language, "moon")
        )),
        Line::from(format!("M/S {}", i18n::tr(language, "legend_planets"))),
    ]
}

fn globe_legend_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let unicode = app.config.display.charset.canvas_unicode();
    vec![
        Line::from(Span::styled(
            i18n::tr(language, "legend_globe"),
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("██", Style::default().fg(Color::Rgb(116, 164, 104))),
            Span::raw(format!(" {}", i18n::tr(language, "legend_globe_day"))),
        ]),
        Line::from(vec![
            Span::styled("██", Style::default().fg(Color::Rgb(37, 62, 86))),
            Span::raw(format!(" {}", i18n::tr(language, "legend_globe_night"))),
        ]),
        Line::from(format!(
            "{} {}",
            if unicode { '·' } else { '.' },
            i18n::tr(language, "legend_globe_cities")
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '·' } else { '.' },
            i18n::tr(language, "legend_globe_graticule")
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '◎' } else { '@' },
            i18n::tr(language, "legend_globe_cursor")
        )),
        Line::from(format!("+ {}", i18n::tr(language, "legend_globe_saved"))),
        Line::from(format!(
            "{} {}",
            star_symbol(3.0, unicode, 0),
            i18n::tr(language, "legend_globe_backdrop")
        )),
    ]
}

fn moon_art(phase_fraction: f64, unicode: bool) -> Vec<String> {
    let row_count = 9i32;
    let columns = 16i32;
    let radius = 1.0;
    let lit = if phase_fraction <= 0.5 {
        phase_fraction * 2.0
    } else {
        (1.0 - phase_fraction) * 2.0
    };
    let waxing = phase_fraction <= 0.5;
    let threshold = -1.0 + lit * 2.0;
    let mut rows = Vec::new();
    for y in 0..row_count {
        let mut row = String::new();
        for cell in 0..columns {
            let left_x = ((cell * 2) as f64 / (columns * 2 - 1) as f64) * 2.0 - 1.0;
            let right_x = (((cell * 2 + 1) as f64) / (columns * 2 - 1) as f64) * 2.0 - 1.0;
            let y_norm = (y as f64 / (row_count - 1) as f64) * 2.0 - 1.0;
            let left = moon_sample(left_x, y_norm, threshold, waxing, radius);
            let right = moon_sample(right_x, y_norm, threshold, waxing, radius);
            row.push(moon_cell(left, right, unicode));
        }
        rows.push(row.trim_end().to_string());
    }
    rows
}

fn moon_sample(x: f64, y: f64, threshold: f64, waxing: bool, radius: f64) -> Option<bool> {
    if x * x + y * y > radius * radius {
        return None;
    }
    Some(if waxing {
        x >= threshold
    } else {
        x <= -threshold
    })
}

fn moon_cell(left: Option<bool>, right: Option<bool>, unicode: bool) -> char {
    match (unicode, left, right) {
        (_, None, None) => ' ',
        (true, Some(true), Some(true)) => '█',
        (true, Some(false), Some(false)) => '░',
        (true, Some(true), Some(false)) => '▌',
        (true, Some(false), Some(true)) => '▐',
        (true, Some(true), None) => '▌',
        (true, None, Some(true)) => '▐',
        (true, Some(false), None) => '░',
        (true, None, Some(false)) => '░',
        (false, Some(true), Some(true)) => '#',
        (false, Some(false), Some(false)) => '.',
        (false, Some(true), Some(false)) => '|',
        (false, Some(false), Some(true)) => '|',
        (false, Some(true), None) => '/',
        (false, None, Some(true)) => '\\',
        (false, Some(false), None) => '.',
        (false, None, Some(false)) => '.',
    }
}

fn sun_altitude_art(altitude: f64) -> String {
    let width = 21usize;
    let position = ((altitude + 18.0) / 108.0 * (width as f64 - 1.0))
        .round()
        .clamp(0.0, width as f64 - 1.0) as usize;
    let mut line = String::from("[-18 ");
    for index in 0..width {
        if index == position {
            line.push('*');
        } else if index == 3 {
            line.push('|');
        } else {
            line.push('-');
        }
    }
    line.push_str(" +90]");
    line
}

fn format_ra(ra_hours: f64) -> String {
    let hours = ra_hours.floor() as u32;
    let minutes = ((ra_hours - hours as f64) * 60.0).round() as u32;
    format!("{hours:02}h{minutes:02}m")
}

fn brightest_visible_star(app: &App) -> Option<crate::catalog::Star> {
    app.catalog
        .stars
        .iter()
        .copied()
        .filter(|star| star.magnitude <= app.config.display.limiting_magnitude)
        .filter(|star| {
            astro::horizontal_position(
                star.ra_hours,
                star.dec_degrees,
                app.active_location(),
                app.now(),
            )
            .altitude
                > 0.0
        })
        .min_by(|a, b| {
            a.magnitude
                .partial_cmp(&b.magnitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let compact = area.width < 118;
    let footer_key = match (app.view_mode, compact) {
        (ViewMode::Ground, true) => "ground_footer_compact",
        (ViewMode::Ground, false) => "ground_footer",
        (ViewMode::Sky, true) => "footer_compact",
        (ViewMode::Sky, false) => "footer",
    };
    let text = if app.message.is_empty() {
        i18n::tr(language, footer_key).to_string()
    } else {
        format!("{}   ·   {}", app.message, i18n::tr(language, footer_key))
    };
    let mouse = footer_mouse_line(language, compact).to_string();
    frame.render_widget(
        Paragraph::new(vec![Line::from(text), Line::from(mouse)])
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette.muted).bg(palette.bg)),
        area,
    );
}

fn footer_mouse_line(language: Language, compact: bool) -> &'static str {
    match (language, compact) {
        (Language::Zh, true) => "鼠标: 左搜索 · 中切换 · 右设置",
        (Language::Zh, false) => "鼠标: 左搜索 · 中切换视图 · 右设置",
        (Language::En, true) => "mouse: left search · center flip · right settings",
        (Language::En, false) => "mouse: left search · center flip · right settings",
    }
}

fn draw_setup(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 78, 22);
    dim_modal_background(frame, area, palette);
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "setup_title")))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                i18n::tr(language, "title"),
                Style::default()
                    .fg(palette.cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                i18n::tr(language, "subtitle"),
                Style::default().fg(palette.silver),
            ),
        ]),
        Line::from(""),
    ];

    lines.push(setup_action_line(
        setup_preset_row(app, language),
        app.setup.field == 0,
        palette,
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        i18n::tr(language, "observer"),
        Style::default()
            .fg(palette.muted)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(setup_info_line(
        format!("{}: {}", i18n::tr(language, "name"), app.setup.name),
        palette,
    ));
    lines.push(setup_info_line(
        format!(
            "{}: {}   {}: {}",
            i18n::tr(language, "latitude"),
            app.setup.latitude,
            i18n::tr(language, "longitude"),
            app.setup.longitude
        ),
        palette,
    ));
    lines.push(setup_info_line(
        format!("{}: {}", i18n::tr(language, "timezone"), app.setup.timezone),
        palette,
    ));
    lines.push(Line::from(""));
    lines.push(setup_action_line(
        i18n::tr(language, "save").to_string(),
        app.setup.field == 1,
        palette,
    ));

    lines.push(Line::from(""));
    let hint_key = if app.setup.field == 0 {
        "setup_preset_hint"
    } else {
        "setup_hint"
    };
    lines.push(Line::from(Span::styled(
        i18n::tr(language, hint_key),
        Style::default().fg(palette.muted),
    )));
    if !app.message.is_empty() {
        lines.push(Line::from(Span::styled(
            app.message.clone(),
            Style::default().fg(palette.moon),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn setup_action_line(text: String, selected: bool, palette: Palette) -> Line<'static> {
    let prefix = if selected { "> " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(palette.warm)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.silver)
    };
    Line::from(vec![
        Span::styled(prefix, Style::default().fg(palette.cyan)),
        Span::styled(text, style),
    ])
}

fn setup_info_line(text: String, palette: Palette) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(text, Style::default().fg(palette.muted)),
    ])
}

fn setup_preset_row(app: &App, language: Language) -> String {
    let label = app.setup.preset_label(language);
    if app.setup.preset_query.is_empty() {
        return if app.setup.field == 0 {
            format!(
                "{}: {}  ({})",
                i18n::tr(language, "preset"),
                label,
                i18n::tr(language, "type_to_search")
            )
        } else {
            format!("{}: {}", i18n::tr(language, "preset"), label)
        };
    }

    let query = app.setup.preset_query.as_str();
    if app.setup_preset_match_count() == 0 {
        format!(
            "{}: {}  {}: {}",
            i18n::tr(language, "preset"),
            i18n::tr(language, "city_no_match"),
            i18n::tr(language, "search"),
            query
        )
    } else {
        format!(
            "{}: {}  {}: {}",
            i18n::tr(language, "preset"),
            label,
            i18n::tr(language, "search"),
            query
        )
    }
}

fn draw_search(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 58, 18);
    dim_modal_background(frame, area, palette);
    let title_key = match app.search.mode {
        app::SearchMode::Sky => "search",
        app::SearchMode::City => "city_search",
    };
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, title_key)))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.cyan));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("/", Style::default().fg(palette.cyan)),
            Span::styled(&app.search.query, Style::default().fg(palette.warm)),
        ]),
        Line::from(""),
    ];
    if app.search.results.is_empty() {
        let empty_key = match app.search.mode {
            app::SearchMode::Sky if app.config.display.sky_culture == SkyCulture::Chinese => {
                "search_empty_chinese_sky"
            }
            app::SearchMode::Sky => "search_empty",
            app::SearchMode::City => "city_search_empty",
        };
        lines.push(Line::from(Span::styled(
            i18n::tr(language, empty_key),
            Style::default().fg(palette.muted),
        )));
    } else {
        for (index, result) in app.search.results.iter().take(10).enumerate() {
            let selected = index == app.search.selected;
            let prefix = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(palette.selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.silver)
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(palette.cyan)),
                Span::styled(result.label.clone(), style),
                Span::styled(
                    format!("  {}", result.detail),
                    Style::default().fg(palette.muted),
                ),
            ]));
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_settings(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 60, 21);
    dim_modal_background(frame, area, palette);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "settings")))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.cyan))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    frame.render_widget(block, area);

    let rows = settings_rows(app);
    let mut lines = Vec::new();
    for (index, (name, value)) in rows.into_iter().enumerate() {
        let selected = index == app.settings.selected;
        let style = if selected {
            Style::default()
                .fg(palette.selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.silver)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "> " } else { "  " },
                Style::default().fg(palette.cyan),
            ),
            Span::styled(format!("{name:<20}"), style),
            Span::styled(value, Style::default().fg(palette.warm)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        i18n::tr(language, "settings_hint"),
        Style::default().fg(palette.muted),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver).bg(palette.panel))
            .wrap(Wrap { trim: false }),
        inner,
    );

    if app.settings.theme_picker {
        draw_theme_picker(frame, app, palette);
    }
}

fn settings_rows(app: &App) -> Vec<(String, String)> {
    let language = app.config.language;
    let display = &app.config.display;
    debug_assert_eq!(app::settings_count(), 14);
    vec![
        (
            i18n::tr(language, "language").to_string(),
            app.config.language.to_string(),
        ),
        (
            i18n::tr(language, "theme").to_string(),
            format!("{} >", display.theme.label()),
        ),
        (
            i18n::tr(language, "charset").to_string(),
            format!("{:?}", display.charset).to_lowercase(),
        ),
        (
            i18n::tr(language, "animations").to_string(),
            on_off(language, display.animations).to_string(),
        ),
        (
            i18n::tr(language, "planets").to_string(),
            on_off(language, display.planets).to_string(),
        ),
        (
            i18n::tr(language, "deep_sky").to_string(),
            on_off(language, display.deep_sky).to_string(),
        ),
        (
            i18n::tr(language, "moon").to_string(),
            on_off(language, display.moon_panel).to_string(),
        ),
        (
            i18n::tr(language, "labels").to_string(),
            on_off(language, display.labels).to_string(),
        ),
        (
            i18n::tr(language, "lines").to_string(),
            on_off(language, display.constellations).to_string(),
        ),
        (
            format!("{} (y)", i18n::tr(language, "sky_culture")),
            sky_culture_label(language, display.sky_culture).to_string(),
        ),
        (
            i18n::tr(language, "side_panel").to_string(),
            on_off(language, display.side_panel).to_string(),
        ),
        (
            i18n::tr(language, "landscape").to_string(),
            landscape_label(language, display.landscape).to_string(),
        ),
        (
            i18n::tr(language, "sky_orientation").to_string(),
            sky_orientation_label(language, display.sky_orientation).to_string(),
        ),
        (
            i18n::tr(language, "limit").to_string(),
            format!("{:.1}", display.limiting_magnitude),
        ),
    ]
}

fn draw_theme_picker(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = theme_picker_modal(frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "theme_picker")))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.cyan))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(area).inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    frame.render_widget(block, area);

    let current = app.config.display.theme;
    let mut lines = Vec::new();
    for (index, theme) in Theme::ALL.into_iter().enumerate() {
        let selected = index == app.settings.theme_selected;
        let marker = if theme == current { "*" } else { " " };
        let style = if selected {
            Style::default()
                .fg(palette.selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.silver)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "> " } else { "  " },
                Style::default().fg(palette.cyan),
            ),
            Span::styled(marker, Style::default().fg(palette.warm)),
            Span::styled(" ", Style::default()),
            Span::styled(theme.label(), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        i18n::tr(language, "theme_picker_hint"),
        Style::default().fg(palette.muted),
    )));

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver).bg(palette.panel))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_opening(frame: &mut Frame, app: &App, palette: Palette) {
    let area = frame.area();
    if area.width < 60 || area.height < 20 {
        return;
    }
    let popup = centered_rect(area, 44, 7);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.cyan))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let dots = ".".repeat((app.animation_tick as usize % 4) + 1);
    let lines = vec![
        Line::from("").alignment(Alignment::Center),
        Line::from(Span::styled(
            "TERMARIUM",
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(format!("calibrating dome{dots}")).alignment(Alignment::Center),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(palette.silver).bg(palette.panel)),
        inner,
    );
}

fn draw_help(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 88, 24);
    dim_help_background(frame, area, palette);
    let block = Block::default()
        .title(i18n::tr(language, "help"))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.cyan));
    let inner = block.inner(area).inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    frame.render_widget(block, area);

    let (left, right) = help_columns_for_view(
        language,
        app.view_mode,
        app.config.display.sky_culture,
        palette,
    );
    if inner.width >= 62 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);
        render_help_lines(frame, columns[0], &left);
        render_help_lines(frame, columns[1], &right);
    } else {
        let mut lines = left;
        lines.push(Line::from(""));
        lines.extend(right);
        render_help_lines(frame, inner, &lines);
    }
}

fn dim_help_background(frame: &mut Frame, area: Rect, palette: Palette) {
    let buffer = frame.buffer_mut();
    let area = buffer.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buffer[(x, y)];
            if cell.symbol() != " " {
                cell.set_fg(palette.veil);
            }
        }
    }
}

fn dim_modal_background(frame: &mut Frame, area: Rect, palette: Palette) {
    let buffer = frame.buffer_mut();
    let area = buffer.area.intersection(area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buffer[(x, y)];
            cell.set_fg(dim_color(cell.fg, palette.veil, 0.42));
            cell.set_bg(dim_color(cell.bg, palette.bg, 0.28));
        }
    }
}

fn dim_color(color: Color, fallback: Color, amount: f64) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * amount).round().clamp(0.0, 255.0) as u8,
            (g as f64 * amount).round().clamp(0.0, 255.0) as u8,
            (b as f64 * amount).round().clamp(0.0, 255.0) as u8,
        ),
        Color::Black => Color::Black,
        Color::DarkGray => Color::Rgb(18, 20, 24),
        Color::Gray => Color::Rgb(34, 38, 44),
        Color::White => Color::Rgb(72, 78, 88),
        Color::Reset => fallback,
        _ => fallback,
    }
}

fn render_help_lines(frame: &mut Frame, area: Rect, lines: &[Line<'static>]) {
    for (row, line) in lines.iter().enumerate() {
        let y = area.y.saturating_add(row as u16);
        if y >= area.bottom() {
            break;
        }

        let mut x = area.x;
        for span in &line.spans {
            x = draw_transparent_text(
                frame,
                x,
                y,
                span.content.as_ref(),
                line.style.patch(span.style),
                area.right(),
            );
            if x >= area.right() {
                break;
            }
        }
    }
}

fn draw_transparent_text(
    frame: &mut Frame,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    right: u16,
) -> u16 {
    let mut cursor = x;
    let mut run = String::new();

    for character in text.chars() {
        if cursor >= right {
            break;
        }
        if character.is_whitespace() {
            if !run.is_empty() {
                let max_width = right.saturating_sub(cursor) as usize;
                cursor = frame
                    .buffer_mut()
                    .set_stringn(cursor, y, &run, max_width, style)
                    .0;
                run.clear();
            }
            cursor = cursor.saturating_add(1).min(right);
        } else {
            run.push(character);
        }
    }

    if !run.is_empty() && cursor < right {
        let max_width = right.saturating_sub(cursor) as usize;
        cursor = frame
            .buffer_mut()
            .set_stringn(cursor, y, &run, max_width, style)
            .0;
    }

    cursor
}

#[cfg(test)]
fn help_columns(language: Language, palette: Palette) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    help_columns_for_view(language, ViewMode::Sky, SkyCulture::Western, palette)
}

fn help_columns_for_view(
    language: Language,
    view_mode: ViewMode,
    sky_culture: SkyCulture,
    palette: Palette,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let (zoom_label_zh, next_label_zh, prev_label_zh, line_label_zh) = match sky_culture {
        SkyCulture::Western => ("放大星座", "下个星座", "上个星座", "星座连线"),
        SkyCulture::Chinese => ("放大星官", "下个星官", "上个星官", "星官连线"),
    };
    let (zoom_label_en, next_label_en, prev_label_en, line_label_en) = match sky_culture {
        SkyCulture::Western => (
            "zoom constellation",
            "next constellation",
            "previous constellation",
            "constellation lines",
        ),
        SkyCulture::Chinese => (
            "zoom asterism",
            "next asterism",
            "previous asterism",
            "asterism lines",
        ),
    };
    match (language, view_mode) {
        (Language::Zh, ViewMode::Sky) => (
            vec![
                help_section("常用", palette),
                help_item("/", "搜索天体", palette),
                help_item("g", "地球/星空", palette),
                help_item("s", "位置/城市", palette),
                help_item("o", "设置", palette),
                help_item("q", "退出", palette),
                help_item("Esc", "取消/退出", palette),
                help_item("?", "帮助", palette),
                Line::from(""),
                help_section("时间", palette),
                help_item("space", "暂停/实时", palette),
                help_item("[ / ]", "前后一小时", palette),
                help_item("{ / }", "前后一天", palette),
                help_item("r", "回到实时", palette),
            ],
            vec![
                help_section("星空进阶", palette),
                help_item("x", "指针", palette),
                help_item("z", zoom_label_zh, palette),
                help_item("Tab", next_label_zh, palette),
                help_item("S-Tab", prev_label_zh, palette),
                help_item("h", "今晚推荐", palette),
                help_item("v", "城市巡游", palette),
                help_item("鼠标", "点天体/标签", palette),
                Line::from(""),
                help_section("显示/设置", palette),
                help_item("m", "月相面板", palette),
                help_item("l", "星名标签", palette),
                help_item("c", line_label_zh, palette),
                help_item("p", "行星", palette),
                help_item("d", "深空天体", palette),
                help_item("a", "动效", palette),
                help_item("+ / -", "星等", palette),
                help_item("t", "中英文", palette),
                help_item("y", "星空体系", palette),
                help_item("T", "主题", palette),
                help_item("u", "字符集", palette),
            ],
        ),
        (Language::Zh, ViewMode::Ground) => (
            vec![
                help_section("常用", palette),
                help_item("/", "搜索城市", palette),
                help_item("g", "回到星空", palette),
                help_item("s", "保存预览点", palette),
                help_item("o", "设置", palette),
                help_item("q/Esc", "退出", palette),
                help_item("?", "帮助", palette),
                Line::from(""),
                help_section("时间", palette),
                help_item("space", "暂停/实时", palette),
                help_item("[ / ]", "前后一小时", palette),
                help_item("{ / }", "前后一天", palette),
                help_item("r", "回到实时", palette),
            ],
            vec![
                help_section("地球进阶", palette),
                help_item("←/→", "旋转经度", palette),
                help_item("↑/↓", "调整纬度", palette),
                help_item("鼠标", "点城市/定位", palette),
                help_item("准星", "当前预览点", palette),
                help_item("明暗", "太阳照明", palette),
                help_item("星幕", "背面星空", palette),
                Line::from(""),
                help_section("显示/设置", palette),
                help_item("t", "中英文", palette),
                help_item("y", "星空体系", palette),
                help_item("T", "主题", palette),
                help_item("u", "字符集", palette),
                help_item("a", "动效", palette),
            ],
        ),
        (Language::En, ViewMode::Sky) => (
            vec![
                help_section("Common", palette),
                help_item("/", "search object", palette),
                help_item("g", "globe/sky", palette),
                help_item("s", "location/city", palette),
                help_item("o", "settings", palette),
                help_item("q", "quit", palette),
                help_item("Esc", "clear/quit", palette),
                help_item("?", "help", palette),
                Line::from(""),
                help_section("Time", palette),
                help_item("space", "pause/live", palette),
                help_item("[ / ]", "one hour", palette),
                help_item("{ / }", "one day", palette),
                help_item("r", "return live", palette),
            ],
            vec![
                help_section("Sky Tools", palette),
                help_item("x", "pointer", palette),
                help_item("z", zoom_label_en, palette),
                help_item("Tab", next_label_en, palette),
                help_item("S-Tab", prev_label_en, palette),
                help_item("h", "tonight", palette),
                help_item("v", "city tour", palette),
                help_item("mouse", "pick object/label", palette),
                Line::from(""),
                help_section("Display", palette),
                help_item("m", "moon panel", palette),
                help_item("l", "star labels", palette),
                help_item("c", line_label_en, palette),
                help_item("p", "planets", palette),
                help_item("d", "deep sky", palette),
                help_item("a", "animations", palette),
                help_item("+ / -", "magnitude", palette),
                help_item("t", "language", palette),
                help_item("y", "sky culture", palette),
                help_item("T", "theme", palette),
                help_item("u", "charset", palette),
            ],
        ),
        (Language::En, ViewMode::Ground) => (
            vec![
                help_section("Common", palette),
                help_item("/", "search city", palette),
                help_item("g", "return to sky", palette),
                help_item("s", "save preview", palette),
                help_item("o", "settings", palette),
                help_item("q/Esc", "quit", palette),
                help_item("?", "help", palette),
                Line::from(""),
                help_section("Time", palette),
                help_item("space", "pause/live", palette),
                help_item("[ / ]", "one hour", palette),
                help_item("{ / }", "one day", palette),
                help_item("r", "return live", palette),
            ],
            vec![
                help_section("Globe Tools", palette),
                help_item("←/→", "rotate longitude", palette),
                help_item("↑/↓", "adjust latitude", palette),
                help_item("mouse", "pick city/place cursor", palette),
                help_item("cross", "preview observer", palette),
                help_item("shade", "sunlight brightness", palette),
                help_item("stars", "opposite sky", palette),
                Line::from(""),
                help_section("Display", palette),
                help_item("t", "language", palette),
                help_item("y", "sky culture", palette),
                help_item("T", "theme", palette),
                help_item("u", "charset", palette),
                help_item("a", "animations", palette),
            ],
        ),
    }
}

fn help_section(title: &'static str, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(palette.cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

fn help_item(key: &'static str, description: &'static str, palette: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<8}"),
            Style::default()
                .fg(palette.warm)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(description, Style::default().fg(palette.silver)),
    ])
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width
        .min(area.width.saturating_sub(2))
        .max(20)
        .min(area.width);
    let height = height
        .min(area.height.saturating_sub(2))
        .max(10)
        .min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn on_off(language: Language, value: bool) -> &'static str {
    if value {
        i18n::tr(language, "on")
    } else {
        i18n::tr(language, "off")
    }
}

fn landscape_label(language: Language, mode: LandscapeMode) -> &'static str {
    match (language, mode) {
        (Language::Zh, LandscapeMode::Off) => "关",
        (Language::Zh, LandscapeMode::Horizon) => "地表环",
        (Language::Zh, LandscapeMode::Bearings) => "方位",
        (_, LandscapeMode::Off) => "off",
        (_, LandscapeMode::Horizon) => "ground ring",
        (_, LandscapeMode::Bearings) => "bearings",
    }
}

fn sky_orientation_label(language: Language, orientation: SkyOrientation) -> &'static str {
    match (language, orientation) {
        (Language::Zh, SkyOrientation::Observer) => "仰望",
        (Language::Zh, SkyOrientation::Map) => "地图",
        (_, SkyOrientation::Observer) => "observer",
        (_, SkyOrientation::Map) => "map",
    }
}

fn sky_culture_label(language: Language, culture: SkyCulture) -> &'static str {
    match (language, culture) {
        (Language::Zh, SkyCulture::Western) => i18n::tr(language, "western"),
        (Language::Zh, SkyCulture::Chinese) => i18n::tr(language, "chinese"),
        (_, SkyCulture::Western) => "western",
        (_, SkyCulture::Chinese) => "chinese",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::TimeZone;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};
    use unicode_width::UnicodeWidthStr;

    use super::*;
    use crate::{
        catalog::Catalog,
        config::{Charset, Config, LandscapeMode, SkyCulture, SkyOrientation},
    };

    fn app_for_test(start_setup: bool) -> App {
        App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-config.json"),
            Catalog::load(),
            start_setup,
            true,
            Some(chrono::Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap()),
        )
    }

    fn visible_stars_for_figure(app: &App, code: &str) -> Vec<astro::VisibleStar> {
        let mut hips = Vec::new();
        for line in app
            .active_constellation_lines()
            .iter()
            .filter(|line| line.code.eq_ignore_ascii_case(code))
        {
            for hip in &line.hips {
                if !hips.contains(hip) {
                    hips.push(*hip);
                }
            }
        }

        hips.into_iter()
            .enumerate()
            .map(|(index, hip)| astro::VisibleStar {
                star: app
                    .catalog
                    .stars
                    .iter()
                    .copied()
                    .find(|star| star.hip == hip)
                    .unwrap(),
                x: 2 + (index * 5) % 90,
                y: 2 + (index * 3) % 28,
            })
            .collect()
    }

    #[test]
    fn renders_sky_at_common_sizes() {
        for (width, height) in [(80, 24), (120, 36), (44, 18)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = app_for_test(false);
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }

    #[test]
    fn renders_chinese_sky_at_common_sizes() {
        for (width, height) in [(80, 24), (120, 36), (44, 18)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = app_for_test(false);
            app.config.display.sky_culture = SkyCulture::Chinese;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }

    #[test]
    fn renders_ground_at_common_sizes() {
        for (width, height) in [(80, 24), (120, 36), (44, 18)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = app_for_test(false);
            app.config.display.animations = false;
            app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }

    #[test]
    fn renders_horizon_transition_at_common_sizes() {
        for (width, height) in [(80, 24), (120, 36), (44, 18)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = app_for_test(false);
            app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }

    #[test]
    fn side_panel_is_global_in_ground_and_transition() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.display.side_panel = true;
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(text.contains(i18n::tr(Language::En, "observatory")));

        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.display.side_panel = true;
        app.config.display.animations = false;
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(text.contains(i18n::tr(Language::En, "observatory")));

        app.config.display.side_panel = false;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(!text.contains(i18n::tr(Language::En, "observatory")));
    }

    #[test]
    fn renders_constellation_zoom_screen() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        let code = app.visible_constellation_codes().first().cloned().unwrap();
        app.selected_target = Some(Target::Constellation(code));
        app.constellation_zoom = true;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }

    #[test]
    fn sky_status_stays_out_of_canvas_area() {
        let backend = TestBackend::new(84, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        let test_palette = palette(Theme::Midnight);
        terminal
            .draw(|frame| {
                draw_star_canvas(frame, &mut app, Rect::new(0, 0, 84, 16), test_palette);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let top = (0..84).map(|x| buffer[(x, 0)].symbol()).collect::<String>();
        let first_canvas_row = (0..84).map(|x| buffer[(x, 1)].symbol()).collect::<String>();
        assert!(top.contains("Visible"));
        assert!(!first_canvas_row.contains("Visible"));
        assert!(!first_canvas_row.contains("limit"));
    }

    #[test]
    fn renders_setup_and_search_screen() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(true);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        app.screen = Screen::Search;
        app.search.query = "Vega".to_string();
        app.search.results.push(crate::app::SearchResult {
            label: "Vega".to_string(),
            detail: "star".to_string(),
            target: Target::Star(91262),
        });
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    }

    #[test]
    fn renders_settings_screen() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.screen = Screen::Settings;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        let text = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains(i18n::tr(Language::En, "landscape")));
        assert!(text.contains("ground ring"));
        assert!(text.contains(i18n::tr(Language::En, "sky_orientation")));
        assert!(text.contains("observer"));
        assert!(text.contains(i18n::tr(Language::En, "sky_culture")));
        assert!(text.contains("(y)"));
        assert!(text.contains("western"));
    }

    #[test]
    fn renders_theme_picker_with_programmer_themes() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.screen = Screen::Settings;
        app.settings.selected = 1;
        app.settings.theme_picker = true;

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let text = buffer_text(&terminal, 100, 30);
        for expected in [
            "dracula",
            "nord",
            "gruvbox",
            "solarized-dark",
            "tokyo-night",
        ] {
            assert!(
                text.contains(expected),
                "{expected} missing from theme picker"
            );
        }
    }

    #[test]
    fn settings_modal_uses_opaque_panel_background() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.screen = Screen::Settings;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let modal = centered_rect(Rect::new(0, 0, 80, 24), 60, 21);
        let test_palette = palette(app.config.display.theme);
        for y in modal.top()..modal.bottom() {
            for x in modal.left()..modal.right() {
                assert_eq!(
                    buffer[(x, y)].bg,
                    test_palette.panel,
                    "settings modal cell at ({x}, {y}) should use panel background"
                );
            }
        }
    }

    #[test]
    fn footer_prioritizes_primary_actions() {
        let zh_footer = i18n::tr(Language::Zh, "footer");
        for expected in ["/ 搜索", "s 位置", "g 地球/星空", "o 设置", "? 帮助"] {
            assert!(zh_footer.contains(expected), "{expected} missing");
        }
        for advanced in [
            "space 暂停",
            "x 指针",
            "z 放大",
            "+/- 星等",
            "[/] 小时",
            "Tab 星座",
            "c 连线",
            "h 今晚",
            "y 星空体系",
        ] {
            assert!(
                !zh_footer.contains(advanced),
                "{advanced} should live in help, not the footer"
            );
        }

        let en_compact = i18n::tr(Language::En, "footer_compact");
        for expected in [
            "/ search",
            "s location",
            "g globe/sky",
            "o settings",
            "? help",
        ] {
            assert!(en_compact.contains(expected), "{expected} missing");
        }
        assert!(!en_compact.contains("x pointer"));
        assert!(!en_compact.contains("+/- mag"));
        assert!(!en_compact.contains("c lines"));
        assert!(!en_compact.contains("y sky culture"));

        let ground_footer = i18n::tr(Language::En, "ground_footer");
        assert!(ground_footer.contains("/ city"));
        assert!(ground_footer.contains("g sky"));
        assert!(ground_footer.contains("s save"));
        assert!(!ground_footer.contains("arrows rotate"));
        assert!(!ground_footer.contains("z zoom"));
        assert!(!i18n::tr(Language::Zh, "ground_footer").contains("c 连线"));
    }

    #[test]
    fn footer_renders_spatial_mouse_hint_line() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(
            text.contains("mouse: left search · center flip · right settings"),
            "footer should expose mouse interaction hints\n{text}"
        );
    }

    #[test]
    fn sky_label_click_selects_constellation() {
        let mut app = app_for_test(false);
        let area = Rect::new(0, 0, 120, 36);
        let layout = sky_layout(area, &app);
        let width = layout.canvas_inner.width as usize;
        let height = layout.canvas_inner.height as usize;
        let unicode = app.config.display.charset.canvas_unicode();
        let visible = sky_visible_for_hit(&app, width, height);
        let hit = constellation_label_hits(&visible, &app, width, height, unicode)
            .into_iter()
            .next()
            .expect("test sky should render at least one constellation label");
        let code = hit.code.clone();

        handle_mouse_click(
            &mut app,
            layout.canvas_inner.x + hit.x as u16,
            layout.canvas_inner.y + hit.y as u16,
            area,
        )
        .unwrap();

        assert_eq!(app.selected_constellation_code(), Some(code.as_str()));
        assert!(!app.pointer.active);
    }

    #[test]
    fn multi_path_figures_render_one_label() {
        let app = app_for_test(false);
        let visible = visible_stars_for_figure(&app, "UMa");
        let hits = constellation_label_hits(&visible, &app, 120, 36, true);
        assert_eq!(
            hits.iter().filter(|hit| hit.code == "UMa").count(),
            1,
            "western multi-path figure should only render one label"
        );

        let mut app = app_for_test(false);
        app.config.display.sky_culture = SkyCulture::Chinese;
        let visible = visible_stars_for_figure(&app, "CN290");
        let hits = constellation_label_hits(&visible, &app, 120, 36, true);
        assert_eq!(
            hits.iter().filter(|hit| hit.code == "CN290").count(),
            1,
            "Chinese multi-path figure should only render one label"
        );
    }

    #[test]
    fn ground_footer_and_legend_use_globe_text() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.display.animations = false;
        app.config.display.side_panel = true;
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(text.contains("/ city"));
        assert!(text.contains("g sky"));
        assert!(!text.contains("arrows rotate"));
        assert!(text.contains("Globe legend"));
        assert!(text.contains("preview center"));
        assert!(!text.contains("constellation node"));
    }

    #[test]
    fn ground_globe_labels_visible_city_presets() {
        let backend = TestBackend::new(140, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.display.side_panel = false;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 30.0,
            cursor_lon: 110.0,
            preview_location: Location {
                name: "East Asia".to_string(),
                latitude: 30.0,
                longitude: 110.0,
                timezone: "Asia/Shanghai".to_string(),
            },
        };
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 140, 40);
        let present = [
            "Shanghai",
            "Beijing",
            "Guangzhou",
            "Chengdu",
            "Tokyo",
            "Seoul",
            "Singapore",
            "Delhi",
        ]
        .into_iter()
        .filter(|city| text.contains(city))
        .collect::<Vec<_>>();
        assert!(
            present.len() >= 3,
            "expected several visible city labels, found {present:?}\n{text}"
        );
    }

    #[test]
    fn ground_current_city_label_stays_localized_after_roundtrip() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.language = Language::Zh;
        app.config.display.charset = Charset::Unicode;
        app.config.display.animations = false;
        app.config.location = Location {
            name: "London".to_string(),
            latitude: 51.5072,
            longitude: -0.1276,
            timezone: "Europe/London".to_string(),
        };
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(text.contains("伦"));
        assert!(text.contains("敦"));
    }

    #[test]
    fn city_label_positions_declutter_dense_rows() {
        let mut occupied = vec![vec![false; 24]; 7];
        let first = city_label_position(&occupied, 10, 3, 4).unwrap();
        reserve_label_cells(&mut occupied, first.0, first.1, 4);
        let second = city_label_position(&occupied, 10, 3, 4).unwrap();
        reserve_label_cells(&mut occupied, second.0, second.1, 4);

        assert_ne!(first, second);
        let overlap_on_same_row = first.1 == second.1
            && first.0 < second.0.saturating_add(4)
            && second.0 < first.0.saturating_add(4);
        assert!(!overlap_on_same_row);
        assert!(!label_cells_are_free(&occupied, first.0, first.1, 4));
        assert!(!label_cells_are_free(&occupied, second.0, second.1, 4));
    }

    #[test]
    fn ground_city_hit_selects_visible_city_marker() {
        let mut app = app_for_test(false);
        app.config.display.animations = false;
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.ground = app::GroundState {
            cursor_lat: 35.0,
            cursor_lon: 105.0,
            preview_location: Location {
                name: "Asia".to_string(),
                latitude: 35.0,
                longitude: 105.0,
                timezone: "UTC".to_string(),
            },
        };
        let width = 120usize;
        let height = 32usize;
        let beijing = PRESETS
            .iter()
            .position(|preset| preset.en == "Beijing")
            .unwrap();
        let globe = globe_projection(&app, width, height).unwrap();
        let (x, y) = globe
            .project(PRESETS[beijing].longitude, PRESETS[beijing].latitude)
            .unwrap();

        assert_eq!(ground_city_hit(&app, width, height, x, y), Some(beijing));
    }

    #[test]
    fn ground_canvas_click_rotates_to_clicked_point() {
        let mut app = app_for_test(false);
        app.config.display.animations = false;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 0.0,
            cursor_lon: 0.0,
            preview_location: Location {
                name: "Equator".to_string(),
                latitude: 0.0,
                longitude: 0.0,
                timezone: "UTC".to_string(),
            },
        };
        let area = Rect::new(0, 0, 120, 36);
        let layout = sky_layout(area, &app);
        let globe = globe_projection(
            &app,
            layout.canvas_inner.width as usize,
            layout.canvas_inner.height as usize,
        )
        .unwrap();
        let (x, y) = globe.project(60.0, 0.0).unwrap();
        let (clicked_lon, clicked_lat) = globe.lon_lat_at(x, y).unwrap();

        handle_mouse_click(
            &mut app,
            layout.canvas_inner.x + x as u16,
            layout.canvas_inner.y + y as u16,
            area,
        )
        .unwrap();

        assert!((app.ground.cursor_lat - clicked_lat).abs() < 1.0);
        assert!((app.ground.cursor_lon - clicked_lon).abs() < 1.5);
    }

    #[test]
    fn ground_city_click_animates_to_city() {
        let mut app = app_for_test(false);
        app.config.display.animations = true;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 35.0,
            cursor_lon: 105.0,
            preview_location: Location {
                name: "Asia".to_string(),
                latitude: 35.0,
                longitude: 105.0,
                timezone: "UTC".to_string(),
            },
        };
        let area = Rect::new(0, 0, 120, 36);
        let layout = sky_layout(area, &app);
        let beijing = PRESETS
            .iter()
            .position(|preset| preset.en == "Beijing")
            .unwrap();
        let globe = globe_projection(
            &app,
            layout.canvas_inner.width as usize,
            layout.canvas_inner.height as usize,
        )
        .unwrap();
        let (x, y) = globe
            .project(PRESETS[beijing].longitude, PRESETS[beijing].latitude)
            .unwrap();

        handle_mouse_click(
            &mut app,
            layout.canvas_inner.x + x as u16,
            layout.canvas_inner.y + y as u16,
            area,
        )
        .unwrap();

        assert_eq!(app.ground.preview_location.name, "Beijing");
        assert!((app.ground.cursor_lon - PRESETS[beijing].longitude).abs() < 0.001);
        let (render_lat, render_lon) = app.render_ground_center();
        assert!((render_lat - app.ground.cursor_lat).abs() > 0.1);
        assert!((render_lon - app.ground.cursor_lon).abs() > 0.1);
    }

    #[test]
    fn ground_mouse_down_up_rotates_once() {
        let mut app = app_for_test(false);
        app.config.display.animations = false;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 0.0,
            cursor_lon: 0.0,
            preview_location: Location {
                name: "Equator".to_string(),
                latitude: 0.0,
                longitude: 0.0,
                timezone: "UTC".to_string(),
            },
        };
        let area = Rect::new(0, 0, 120, 36);
        let layout = sky_layout(area, &app);
        let globe = globe_projection(
            &app,
            layout.canvas_inner.width as usize,
            layout.canvas_inner.height as usize,
        )
        .unwrap();
        let (x, y) = globe.project(60.0, 0.0).unwrap();
        let column = layout.canvas_inner.x + x as u16;
        let row = layout.canvas_inner.y + y as u16;

        handle_mouse_event(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            area,
        )
        .unwrap();
        assert_eq!(app.ground.cursor_lon, 0.0);

        handle_mouse_event(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            area,
        )
        .unwrap();
        let lon_after_first_click = app.ground.cursor_lon;

        handle_mouse_event(
            &mut app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            area,
        )
        .unwrap();

        assert_eq!(app.ground.cursor_lon, lon_after_first_click);
    }

    #[test]
    fn preview_ground_location_animates_render_center() {
        let mut app = app_for_test(false);
        app.config.display.animations = true;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 0.0,
            cursor_lon: 0.0,
            preview_location: Location {
                name: "Equator".to_string(),
                latitude: 0.0,
                longitude: 0.0,
                timezone: "UTC".to_string(),
            },
        };

        app.preview_ground_location(0.0, 80.0);

        let (_, render_lon) = app.render_ground_center();
        assert_eq!(app.ground.cursor_lon, 80.0);
        assert!(render_lon.abs() < 80.0);
    }

    #[test]
    fn ground_backdrop_uses_animated_render_center() {
        let mut app = app_for_test(false);
        app.config.display.animations = true;
        app.view_mode = ViewMode::Ground;
        app.ground = app::GroundState {
            cursor_lat: 0.0,
            cursor_lon: 0.0,
            preview_location: Location {
                name: "Equator".to_string(),
                latitude: 0.0,
                longitude: 0.0,
                timezone: "UTC".to_string(),
            },
        };

        app.preview_ground_location(0.0, 80.0);

        let (render_lat, render_lon) = app.render_ground_center();
        let backdrop = antipode_location(&app);
        assert!((backdrop.latitude + render_lat).abs() < 0.001);
        assert!(
            normalize_degrees(backdrop.longitude - normalize_degrees(render_lon + 180.0)).abs()
                < 0.001
        );
        assert!(
            normalize_degrees(
                backdrop.longitude - normalize_degrees(app.ground.cursor_lon + 180.0)
            )
            .abs()
                > 10.0
        );
    }

    #[test]
    fn search_result_click_activates_city() {
        let mut app = app_for_test(true);
        let area = Rect::new(0, 0, 120, 36);
        let modal = search_modal_inner(area);

        handle_mouse_click(&mut app, modal.x + 2, modal.y + 3, area).unwrap();

        assert_eq!(app.screen, Screen::Sky);
        assert_eq!(app.view_mode, ViewMode::Ground);
        assert_eq!(app.ground.preview_location.name, PRESETS[1].en);
    }

    #[test]
    fn horizon_globe_motion_uses_opposite_side_of_sky_path() {
        let mut app = app_for_test(false);
        app.ground = app::GroundState {
            cursor_lat: 20.0,
            cursor_lon: 100.0,
            preview_location: Location {
                name: "East".to_string(),
                latitude: 20.0,
                longitude: 100.0,
                timezone: "UTC".to_string(),
            },
        };
        let east = horizon_path_motion_vector(&app);
        assert!(east.x > 0.0);

        app.ground.cursor_lon = -100.0;
        app.ground.preview_location.longitude = -100.0;
        let west = horizon_path_motion_vector(&app);
        assert!(west.x < 0.0);
    }

    #[test]
    fn horizon_projection_uses_look_up_handedness() {
        let width = 81;
        let height = 25;
        let center_x = width / 2;
        let (east_x, _) = horizon_sky_project(0.0, 90.0, width, height, 1.0).unwrap();
        let (west_x, _) = horizon_sky_project(0.0, 270.0, width, height, 1.0).unwrap();
        assert!(east_x < center_x);
        assert!(west_x > center_x);
    }

    #[test]
    fn ground_to_sky_globe_offset_mirrors_sky_to_ground_timing() {
        let motion = MotionVector { x: 1.0, y: 0.0 };
        let width = 120;
        let height = 36;
        let (ground_start, _) =
            globe_motion_offset(ViewMode::Ground, ViewMode::Sky, 0.0, width, height, motion);
        let (ground_mid, _) =
            globe_motion_offset(ViewMode::Ground, ViewMode::Sky, 0.21, width, height, motion);
        let (ground_gone, _) =
            globe_motion_offset(ViewMode::Ground, ViewMode::Sky, 0.42, width, height, motion);
        let (sky_waiting, _) =
            globe_motion_offset(ViewMode::Sky, ViewMode::Ground, 0.42, width, height, motion);
        let (sky_mid, _) =
            globe_motion_offset(ViewMode::Sky, ViewMode::Ground, 0.63, width, height, motion);
        let (sky_arrived, _) =
            globe_motion_offset(ViewMode::Sky, ViewMode::Ground, 0.84, width, height, motion);

        assert!(ground_start < ground_mid && ground_mid < ground_gone);
        assert!((ground_gone - sky_waiting).abs() < 1.0);
        assert!(sky_waiting > sky_mid && sky_mid > sky_arrived);
    }

    #[test]
    fn horizon_globe_offset_fully_clears_canvas_edges() {
        let mut app = app_for_test(false);
        app.view_mode = ViewMode::Ground;
        let directions = [
            MotionVector { x: 1.0, y: 0.0 },
            MotionVector { x: -1.0, y: 0.0 },
            MotionVector { x: 0.0, y: 1.0 },
            MotionVector { x: 0.0, y: -1.0 },
            MotionVector {
                x: std::f64::consts::FRAC_1_SQRT_2,
                y: std::f64::consts::FRAC_1_SQRT_2,
            },
            MotionVector {
                x: -std::f64::consts::FRAC_1_SQRT_2,
                y: std::f64::consts::FRAC_1_SQRT_2,
            },
        ];

        for (width, height) in [(120usize, 36usize), (200, 60), (80, 24)] {
            let palette = palette(Theme::Midnight);
            let globe_layer = globe_layer_grid(&app, width, height, palette);
            let globe = globe_projection(&app, width, height);
            let empty = SkyCell {
                ch: ' ',
                style: Style::default().fg(palette.muted).bg(palette.bg),
            };

            for motion in directions {
                let (offset_x, offset_y) = globe_motion_offset(
                    ViewMode::Ground,
                    ViewMode::Sky,
                    0.42,
                    width,
                    height,
                    motion,
                );
                let mut visible_cells = 0usize;
                for y in 0..height {
                    for x in 0..width {
                        let cell = shifted_globe_cell(
                            &globe_layer,
                            globe,
                            x,
                            y,
                            offset_x,
                            offset_y,
                            empty,
                        )
                        .unwrap_or(empty);
                        if cell.ch != ' ' || cell.style.bg != Some(palette.bg) {
                            visible_cells += 1;
                        }
                    }
                }

                assert_eq!(
                    visible_cells, 0,
                    "globe should fully clear {width}x{height} toward {motion:?}"
                );
            }
        }
    }

    #[test]
    fn ground_city_search_overlay_uses_city_text() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = app_for_test(false);
        app.config.display.animations = false;
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('/'))).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text = buffer_text(&terminal, 120, 36);
        assert!(text.contains("City Search"));
        assert!(text.contains("Shanghai"));
    }

    #[test]
    fn help_columns_group_common_and_advanced_shortcuts() {
        let (left, right) = help_columns(Language::Zh, palette(Theme::Midnight));
        let help_text = format!("{}{}", lines_text(&left), lines_text(&right));
        let compact_help = help_text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        for expected in [
            "常用",
            "/搜索天体",
            "g地球/星空",
            "s位置/城市",
            "o设置",
            "时间",
            "space暂停/实时",
            "星空进阶",
            "x指针",
            "z放大星座",
            "Tab下个星座",
            "+/-星等",
            "显示/设置",
            "c星座连线",
            "t中英文",
            "T主题",
            "u字符集",
            "y星空体系",
        ] {
            let compact_expected = expected
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            assert!(
                compact_help.contains(&compact_expected),
                "{expected} missing from help"
            );
        }
    }

    #[test]
    fn chinese_sky_help_uses_asterism_copy() {
        let (left, right) = help_columns_for_view(
            Language::Zh,
            ViewMode::Sky,
            SkyCulture::Chinese,
            palette(Theme::Midnight),
        );
        let compact_help = format!("{}{}", lines_text(&left), lines_text(&right))
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact_help.contains("z放大星官"));
        assert!(compact_help.contains("Tab下个星官"));
        assert!(compact_help.contains("c星官连线"));
        assert!(!compact_help.contains("z放大星座"));
    }

    #[test]
    fn ground_help_uses_ground_shortcuts() {
        let (left, right) = help_columns_for_view(
            Language::En,
            ViewMode::Ground,
            SkyCulture::Western,
            palette(Theme::Midnight),
        );
        let help_text = format!("{}{}", lines_text(&left), lines_text(&right));
        assert!(help_text.contains("Common"));
        assert!(help_text.contains("Globe Tools"));
        assert!(help_text.contains("Display"));
        assert!(help_text.contains("rotate longitude"));
        assert!(help_text.contains("save preview"));
        assert!(help_text.contains("opposite sky"));
        assert!(!help_text.contains("zoom constellation"));
        assert!(!help_text.contains("constellation lines"));
    }

    #[test]
    fn landscape_modes_change_canvas_ground_cues() {
        let mut app = app_for_test(false);
        app.config.display.constellations = false;
        app.config.display.deep_sky = false;
        app.config.display.labels = false;
        app.config.display.planets = false;

        app.config.display.landscape = LandscapeMode::Off;
        let off = lines_text(&sky_lines(
            &app,
            &[],
            &app.config.location,
            40,
            12,
            palette(Theme::Midnight),
        ));
        assert!(!off.contains('_'));
        assert!(!off.contains('|'));

        app.config.display.landscape = LandscapeMode::Horizon;
        let horizon_lines = sky_lines(
            &app,
            &[],
            &app.config.location,
            40,
            12,
            palette(Theme::Midnight),
        );
        let horizon = lines_text(&horizon_lines);
        assert!(!horizon.contains('_'));
        assert!(!horizon.contains('|'));
        assert!(
            horizon_lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .all(|span| span.style.bg != Some(palette(Theme::Midnight).panel)),
            "ground-ring landscape should not paint an opaque bottom band"
        );
        assert!(
            horizon_lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .any(|span| {
                    span.content.as_ref() == " "
                        && span.style.bg != Some(palette(Theme::Midnight).bg)
                }),
            "ground-ring landscape should paint textured cells outside the sky dome"
        );

        app.config.display.landscape = LandscapeMode::Bearings;
        let bearings = lines_text(&sky_lines(
            &app,
            &[],
            &app.config.location,
            40,
            12,
            palette(Theme::Midnight),
        ));
        assert!(!bearings.contains('_'));
        assert!(bearings.contains('|'));
    }

    #[test]
    fn horizon_transition_draws_ground_ring_with_sky_shape() {
        let mut app = app_for_test(false);
        app.config.display.constellations = false;
        app.config.display.deep_sky = false;
        app.config.display.labels = false;
        app.config.display.planets = false;
        app.config.display.landscape = LandscapeMode::Horizon;
        let test_palette = palette(Theme::Midnight);

        let expanded = horizon_sky_grid(&app, 40, 12, test_palette, 0.5, 0.0);
        let shaped = horizon_sky_grid(&app, 40, 12, test_palette, 0.5, 0.85);

        let textured_cells = |grid: &[Vec<SkyCell>]| {
            grid.iter()
                .flat_map(|row| row.iter())
                .filter(|cell| cell.ch == ' ' && cell.style.bg != Some(test_palette.bg))
                .count()
        };

        assert_eq!(textured_cells(&expanded), 0);
        assert!(textured_cells(&shaped) > 0);
    }

    #[test]
    fn draw_moon_places_phase_marker_on_sky() {
        let app = app_for_test(false);
        let test_palette = palette(Theme::Midnight);
        let style = Style::default().fg(test_palette.muted).bg(test_palette.bg);
        let empty = SkyCell { ch: ' ', style };
        let mut grid = vec![vec![empty; 20]; 8];

        draw_moon(
            &mut grid,
            &app,
            |_ra, _dec| Some((5, 2)),
            test_palette,
            true,
        );

        let expected = i18n::moon_symbol(astro::moon_phase(app.now()))
            .chars()
            .next()
            .unwrap();
        assert_eq!(grid[2][5].ch, expected);
        assert_eq!(grid[2][5].style.fg, Some(test_palette.moon));
    }

    #[test]
    fn sky_orientation_flips_east_west_labels() {
        let mut app = app_for_test(false);
        app.config.display.constellations = false;
        app.config.display.deep_sky = false;
        app.config.display.labels = false;
        app.config.display.landscape = LandscapeMode::Off;
        app.config.display.planets = false;

        let observer = lines_text(&sky_lines(
            &app,
            &[],
            &app.config.location,
            41,
            13,
            palette(Theme::Midnight),
        ));
        let observer_e = first_label_column(&observer, 'E').unwrap();
        let observer_w = first_label_column(&observer, 'W').unwrap();
        assert!(observer_e < observer_w);

        app.config.display.sky_orientation = SkyOrientation::Map;
        let map = lines_text(&sky_lines(
            &app,
            &[],
            &app.config.location,
            41,
            13,
            palette(Theme::Midnight),
        ));
        let map_e = first_label_column(&map, 'E').unwrap();
        let map_w = first_label_column(&map, 'W').unwrap();
        assert!(map_e > map_w);
    }

    #[test]
    fn transparent_help_text_leaves_blank_cells_untouched() {
        let backend = TestBackend::new(12, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame
                    .buffer_mut()
                    .set_string(0, 0, "************", Style::default());
                draw_transparent_text(frame, 0, 0, "a b", Style::default(), 12);
            })
            .unwrap();
        terminal.backend().assert_buffer_lines(["a*b*********"]);
    }

    #[test]
    fn canvas_text_accounts_for_wide_chinese_cells() {
        let style = Style::default();
        let empty = SkyCell { ch: ' ', style };
        let mut grid = vec![vec![empty; 8]];
        draw_text_overlay(&mut grid, 0, 0, "北京A", style, true);
        let lines = grid_to_lines(grid);
        let row = lines[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(row.starts_with("北京A"));
        assert_eq!(UnicodeWidthStr::width(row.as_str()), 8);
        assert_eq!(canvas_label_width("北京A", true), 5);
    }

    #[test]
    fn help_backdrop_dims_existing_symbols_without_clearing_them() {
        let backend = TestBackend::new(6, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let test_palette = palette(Theme::Midnight);
        terminal
            .draw(|frame| {
                frame.buffer_mut().set_string(
                    0,
                    0,
                    "* .  +",
                    Style::default().fg(test_palette.warm),
                );
                dim_help_background(frame, Rect::new(0, 0, 6, 1), test_palette);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let symbols = (0..6).map(|x| buffer[(x, 0)].symbol()).collect::<String>();
        assert_eq!(symbols, "* .  +");
        assert_eq!(buffer[(0, 0)].fg, test_palette.veil);
        assert_eq!(buffer[(2, 0)].fg, test_palette.veil);
        assert_eq!(buffer[(5, 0)].fg, test_palette.veil);
        assert_eq!(buffer[(1, 0)].fg, test_palette.warm);
    }

    #[test]
    fn setup_presets_are_available() {
        assert!(PRESETS.len() >= 10);
        assert!(PRESETS.iter().any(|preset| preset.custom));
    }

    #[test]
    fn line_chars_are_single_width_ascii() {
        assert_eq!(star_symbol(-1.0, false, 0), '*');
        assert_eq!(star_symbol(2.0, false, 0), '.');
        assert_eq!(star_symbol(4.0, false, 0), '.');
        for ch in [
            star_symbol(-1.0, false, 0),
            star_symbol(1.0, false, 0),
            star_symbol(2.0, false, 0),
            line_char(0, 1, false),
            line_char(1, 0, false),
            line_char(1, 1, false),
            line_char(1, -1, false),
        ] {
            assert!(ch.is_ascii());
        }
    }

    #[test]
    fn zh_target_lines_show_chinese_and_english_star_name() {
        let mut app = app_for_test(false);
        app.config.language = Language::Zh;
        app.selected_target = Some(Target::Star(8102));
        let text = lines_text(&target_lines(&app, palette(Theme::Midnight)));
        assert!(text.contains("天仓五 / Tau Ceti"));
    }

    #[test]
    fn target_lines_show_selected_moon() {
        let mut app = app_for_test(false);
        app.selected_target = Some(Target::Moon);
        let text = lines_text(&target_lines(&app, palette(Theme::Midnight)));

        assert!(text.contains("Moon"));
        assert!(text.contains("Illumination"));
    }

    #[test]
    fn constellation_target_lines_hint_zoom_key() {
        let mut app = app_for_test(false);
        app.config.language = Language::Zh;
        let code = app.visible_constellation_codes().first().cloned().unwrap();
        app.selected_target = Some(Target::Constellation(code));
        let text = lines_text(&target_lines(&app, palette(Theme::Midnight)));
        assert!(text.contains("z 放大"));
        app.constellation_zoom = true;
        let text = lines_text(&target_lines(&app, palette(Theme::Midnight)));
        assert!(text.contains("z/Esc 退出放大"));
    }

    #[test]
    fn constellation_target_lines_explain_hidden_target() {
        let mut app = app_for_test(false);
        app.config.language = Language::Zh;
        let hidden = constellations::CONSTELLATION_META
            .iter()
            .find(|meta| !app.constellation_visible(meta.code))
            .unwrap();
        app.selected_target = Some(Target::Constellation(hidden.code.to_string()));
        let text = lines_text(&target_lines(&app, palette(Theme::Midnight)));
        assert!(text.contains("当前城市此刻不可见"));
        assert!(text.contains("已拉回全局天空"));
        assert!(text.contains("Tab 可切换到可见星座"));
    }

    #[test]
    fn moon_art_is_wide_enough_for_terminal_cells() {
        let art = moon_art(0.66, false);
        assert_eq!(art.len(), 9);
        assert!(
            art.iter()
                .any(|row| row.chars().filter(|ch| *ch != ' ').count() >= 12)
        );
        assert!(art.iter().all(|row| row.is_ascii()));
    }

    #[test]
    fn earth_texture_has_expected_shape_and_variation() {
        assert_eq!(
            EARTH_TEXTURE.len(),
            EARTH_TEXTURE_WIDTH * EARTH_TEXTURE_HEIGHT * 3
        );
        let sahara = earth_texture_color(10.0, 24.0);
        let pacific = earth_texture_color(-140.0, 0.0);
        assert!(
            (sahara.r - pacific.r).abs()
                + (sahara.g - pacific.g).abs()
                + (sahara.b - pacific.b).abs()
                > 40.0
        );
    }

    #[test]
    fn renders_themes_and_charsets() {
        for theme in Theme::ALL {
            for charset in [Charset::Auto, Charset::Ascii, Charset::Unicode] {
                let backend = TestBackend::new(100, 30);
                let mut terminal = Terminal::new(backend).unwrap();
                let mut app = app_for_test(false);
                app.config.display.theme = theme;
                app.config.display.charset = charset;
                terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            }
        }
    }

    fn lines_text(lines: &[Line<'static>]) -> String {
        let mut text = String::new();
        for line in lines {
            for span in &line.spans {
                text.push_str(span.content.as_ref());
            }
            text.push('\n');
        }
        text
    }

    fn first_label_column(text: &str, label: char) -> Option<usize> {
        text.lines().find_map(|line| {
            line.char_indices()
                .find(|(_, ch)| *ch == label)
                .map(|(x, _)| x)
        })
    }

    fn buffer_text(terminal: &Terminal<TestBackend>, width: u16, height: u16) -> String {
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..height {
            for x in 0..width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }
}
