use std::{
    collections::HashMap,
    io,
    time::{Duration as StdDuration, Instant},
};

use chrono_tz::Tz;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event as CEvent},
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

use crate::{
    app::{self, App, Screen, Target},
    astro,
    config::{Language, Location, Theme},
    constellations, i18n, planets, solar, star_aliases,
};

#[cfg(test)]
use crate::app::PRESETS;

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

pub fn run(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), Show, LeaveAlternateScreen)?;
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
            if let CEvent::Key(key) = event::read()? {
                app.handle_key(key)?;
            }
        }
        if last_draw.elapsed() >= tick {
            app.tick();
            last_draw = Instant::now();
        }
    }

    Ok(())
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let palette = palette(app.config.display.theme);
    match app.screen {
        Screen::Sky | Screen::Search | Screen::Settings => draw_sky(frame, app, palette),
        Screen::Setup => draw_setup(frame, app, palette),
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

fn draw_sky(frame: &mut Frame, app: &mut App, palette: Palette) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.bg)),
        area,
    );

    let wants_panel = app.config.display.side_panel
        && (app.config.display.moon_panel
            || app.show_recommendations
            || app.selected_target.is_some()
            || app.pointer.active);

    if wants_panel && area.width >= 94 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(54), Constraint::Length(36)])
            .split(area);
        draw_sky_column(frame, app, chunks[0], palette);
        draw_side_panel(frame, app, chunks[1], palette);
    } else if wants_panel && area.height >= 28 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(16), Constraint::Length(10)])
            .split(area);
        draw_sky_column(frame, app, chunks[0], palette);
        draw_side_panel(frame, app, chunks[1], palette);
    } else {
        draw_sky_column(frame, app, area, palette);
    }
}

fn draw_sky_column(frame: &mut Frame, app: &mut App, area: Rect, palette: Palette) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);
    draw_header(frame, app, rows[0], palette);
    draw_star_canvas(frame, app, rows[1], palette);
    draw_footer(frame, app, rows[2], palette);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, palette: Palette) {
    let language = app.config.language;
    let now = app.now();
    let timezone = app
        .config
        .location
        .timezone
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let local_time = now.with_timezone(&timezone).format("%Y-%m-%d %H:%M:%S %Z");
    let location = &app.config.location;
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
    let visible = astro::visible_stars(
        &app.catalog.stars,
        &sky_location,
        app.now(),
        app.config.display.limiting_magnitude,
        inner.width as usize,
        inner.height as usize,
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

    let lines = if app.should_render_zoomed_sky() && app.zoom_render_code().is_some() {
        zoomed_sky_lines(app, inner.width as usize, inner.height as usize, palette).unwrap_or_else(
            || {
                sky_lines(
                    app,
                    &visible,
                    &sky_location,
                    inner.width as usize,
                    inner.height as usize,
                    palette,
                )
            },
        )
    } else {
        sky_lines(
            app,
            &visible,
            &sky_location,
            inner.width as usize,
            inner.height as usize,
            palette,
        )
    };
    frame.render_widget(Paragraph::new(lines), inner);
}

fn sky_title_line(language: Language, palette: Palette) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {} ", i18n::tr(language, "sky")),
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

fn zoomed_sky_lines(
    app: &App,
    width: usize,
    height: usize,
    palette: Palette,
) -> Option<Vec<Line<'static>>> {
    let code = app.zoom_render_code()?;
    let view = app.zoom_render_view(width, height)?;
    let visible = &view.stars;
    let empty = SkyCell {
        ch: ' ',
        style: Style::default().fg(palette.muted).bg(palette.bg),
    };
    let mut grid = vec![vec![empty; width]; height];
    let unicode = app.config.display.charset.canvas_unicode();
    let location = app.render_location();
    let time = app.now();
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
        let in_constellation = star.star.constellation.eq_ignore_ascii_case(code);
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

    let meta = constellations::meta_for(code);
    draw_text(
        &mut grid,
        1,
        0,
        &format!("{} · {}", meta.code, meta.en),
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
    let horizon_style = Style::default().fg(palette.dim_line).bg(palette.bg);

    for azimuth in (0..360).step_by(2) {
        if let Some((x, y)) = astro::project_dome(0.0, azimuth as f64, width, height) {
            set_cell(&mut grid, x, y, '.', horizon_style);
        }
    }

    if let Some((x, y)) = astro::project_dome(90.0, 0.0, width, height) {
        set_cell(
            &mut grid,
            x,
            y,
            '+',
            Style::default().fg(palette.dim_line).bg(palette.bg),
        );
    }

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

    let time = app.now();
    draw_deep_sky(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome(horizontal.altitude, horizontal.azimuth, width, height)
        },
        palette,
        unicode,
    );
    draw_planets(
        &mut grid,
        app,
        |ra, dec| {
            let horizontal = astro::horizontal_position(ra, dec, location, time);
            astro::project_dome(horizontal.altitude, horizontal.azimuth, width, height)
        },
        palette,
        unicode,
    );

    if app.config.display.constellations {
        draw_constellation_labels(&mut grid, visible, app, width, height, palette, unicode);
    }

    label_cardinal(&mut grid, width, height, 0.0, 'N', palette);
    label_cardinal(&mut grid, width, height, 90.0, 'E', palette);
    label_cardinal(&mut grid, width, height, 180.0, 'S', palette);
    label_cardinal(&mut grid, width, height, 270.0, 'W', palette);

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
            astro::project_dome(horizontal.altitude, horizontal.azimuth, width, height)
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
                astro::project_dome(horizontal.altitude, horizontal.azimuth, width, height)
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
    palette: Palette,
) {
    if let Some((x, y)) = astro::project_dome(0.0, azimuth, width, height) {
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

    for constellation in &app.constellation_lines {
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

    let points = visible
        .iter()
        .map(|star| (star.star.hip, (star.x, star.y)))
        .collect::<HashMap<_, _>>();
    let selected = app.selected_constellation_code();

    for constellation in &app.constellation_lines {
        let highlighted =
            selected.is_some_and(|code| code.eq_ignore_ascii_case(constellation.code));
        let label_style = Style::default()
            .fg(if highlighted {
                palette.selected
            } else {
                palette.line
            })
            .bg(palette.bg)
            .add_modifier(Modifier::BOLD);
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

        endpoints.sort_unstable();
        endpoints.dedup();
        if !endpoints.is_empty() {
            let sum_x = endpoints.iter().map(|(x, _)| *x).sum::<usize>();
            let sum_y = endpoints.iter().map(|(_, y)| *y).sum::<usize>();
            let x = (sum_x / endpoints.len()).min(width.saturating_sub(1));
            let y = (sum_y / endpoints.len()).min(height.saturating_sub(1));
            draw_text(
                grid,
                x.saturating_add(1),
                y,
                constellation.code,
                label_style,
                unicode,
            );
        }
    }
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
        Target::DeepSky(name) => app
            .deep_sky_by_name(name)
            .map(|object| (object.name.to_string(), object.ra_hours, object.dec_degrees)),
        Target::Constellation(_) => None,
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
    for (offset, ch) in text
        .chars()
        .filter_map(|ch| canvas_label_char(ch, unicode))
        .take(width.saturating_sub(x))
        .enumerate()
    {
        set_cell(grid, x + offset, y, ch, style);
    }
}

fn canvas_label_char(ch: char, unicode: bool) -> Option<char> {
    if ch.is_ascii() || (unicode && !ch.is_control()) {
        Some(ch)
    } else {
        None
    }
}

fn set_cell(grid: &mut [Vec<SkyCell>], x: usize, y: usize, ch: char, style: Style) {
    if let Some(row) = grid.get_mut(y) {
        if let Some(cell) = row.get_mut(x) {
            *cell = SkyCell { ch, style };
        }
    }
}

fn grid_to_lines(grid: Vec<Vec<SkyCell>>) -> Vec<Line<'static>> {
    grid.into_iter()
        .map(|row| {
            Line::from(
                row.into_iter()
                    .map(|cell| Span::styled(cell.ch.to_string(), cell.style))
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
    let light = solar::skylight(&app.config.location, app.now());
    let events = solar::tonight_events(&app.config.location, app.now());
    let timezone = app
        .config
        .location
        .timezone
        .parse::<Tz>()
        .unwrap_or(chrono_tz::UTC);
    let light_name = match language {
        Language::En => light.name_en(),
        Language::Zh => light.name_zh(),
    };
    let sun = solar::sun_position(app.now());
    let sun_horizontal = astro::horizontal_position(
        sun.ra_hours,
        sun.dec_degrees,
        &app.config.location,
        app.now(),
    );
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
        &app.config.location,
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
        Target::DeepSky(name) => {
            if let Some(object) = app.deep_sky_by_name(name) {
                push_deep_sky_summary(lines, app, object);
            }
        }
        Target::Constellation(code) => {
            let meta = constellations::meta_for(code);
            lines.push(Line::from(format!("{} · {}", meta.code, meta.en)));
            lines.push(Line::from(meta.zh.to_string()));
        }
    }
}

fn push_planet_summary(lines: &mut Vec<Line<'static>>, app: &App, planet: planets::Planet) {
    push_position(lines, app, planet.name, planet.ra_hours, planet.dec_degrees);
    lines.push(Line::from(format!("planet · mag {:.1}", planet.magnitude)));
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
            let meta = constellations::meta_for(code);
            lines.push(Line::from(format!("{} · {}", meta.code, meta.en)));
            lines.push(Line::from(meta.zh.to_string()));
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
                .constellation_lines
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
        astro::horizontal_position(ra_hours, dec_degrees, &app.config.location, app.now());
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
                &app.config.location,
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
                &app.config.location,
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
        let meta = constellations::meta_for(code);
        lines.push(Line::from(format!(
            "{}: {}",
            i18n::tr(language, "constellation"),
            meta.en
        )));
    }
    lines
}

fn legend_lines(app: &App, palette: Palette) -> Vec<Line<'static>> {
    let language = app.config.language;
    let unicode = app.config.display.charset.canvas_unicode();
    vec![
        Line::from(Span::styled(
            i18n::tr(language, "legend"),
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
            i18n::tr(language, "legend_constellation")
        )),
        Line::from(format!(
            "{} {}",
            if unicode { '◇' } else { 'x' },
            i18n::tr(language, "legend_deep_sky")
        )),
        Line::from(format!("M/S {}", i18n::tr(language, "legend_planets"))),
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
                &app.config.location,
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
    let footer_key = if area.width < 118 {
        "footer_compact"
    } else {
        "footer"
    };
    let text = if app.message.is_empty() {
        i18n::tr(language, footer_key).to_string()
    } else {
        format!("{}   ·   {}", app.message, i18n::tr(language, footer_key))
    };
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette.muted).bg(palette.bg)),
        area,
    );
}

fn draw_setup(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let root = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.bg)),
        root,
    );
    let area = centered_rect(root, 78, 22);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "setup_title")))
        .title_style(
            Style::default()
                .fg(palette.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.dim_line))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = [
        format!(
            "{}: {}",
            i18n::tr(language, "preset"),
            app.setup.preset_label(language)
        ),
        format!("{}: {}", i18n::tr(language, "name"), app.setup.name),
        format!("{}: {}", i18n::tr(language, "latitude"), app.setup.latitude),
        format!(
            "{}: {}",
            i18n::tr(language, "longitude"),
            app.setup.longitude
        ),
        format!("{}: {}", i18n::tr(language, "timezone"), app.setup.timezone),
        i18n::tr(language, "save").to_string(),
    ];

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

    for (index, row) in rows.iter().enumerate() {
        let selected = index == app.setup.field;
        let prefix = if selected { "> " } else { "  " };
        let style = if selected {
            Style::default()
                .fg(palette.warm)
                .bg(Color::Rgb(24, 34, 50))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.silver).bg(palette.panel)
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(palette.cyan).bg(palette.panel)),
            Span::styled(row.clone(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        i18n::tr(language, "setup_hint"),
        Style::default().fg(palette.muted).bg(palette.panel),
    )));
    if !app.message.is_empty() {
        lines.push(Line::from(Span::styled(
            app.message.clone(),
            Style::default().fg(palette.moon).bg(palette.panel),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(palette.silver).bg(palette.panel))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_search(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 58, 18);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(format!(" {} ", i18n::tr(language, "search")))
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

    let mut lines = vec![
        Line::from(vec![
            Span::styled("/", Style::default().fg(palette.cyan)),
            Span::styled(&app.search.query, Style::default().fg(palette.warm)),
        ]),
        Line::from(""),
    ];
    if app.search.results.is_empty() {
        lines.push(Line::from(Span::styled(
            i18n::tr(language, "search_empty"),
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
            .style(Style::default().fg(palette.silver).bg(palette.panel))
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_settings(frame: &mut Frame, app: &App, palette: Palette) {
    let language = app.config.language;
    let area = centered_rect(frame.area(), 60, 19);
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
}

fn settings_rows(app: &App) -> Vec<(String, String)> {
    let language = app.config.language;
    let display = &app.config.display;
    debug_assert_eq!(app::settings_count(), 11);
    vec![
        (
            i18n::tr(language, "language").to_string(),
            app.config.language.to_string(),
        ),
        (
            i18n::tr(language, "theme").to_string(),
            format!("{:?}", display.theme).to_lowercase(),
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
            i18n::tr(language, "side_panel").to_string(),
            on_off(language, display.side_panel).to_string(),
        ),
        (
            i18n::tr(language, "limit").to_string(),
            format!("{:.1}", display.limiting_magnitude),
        ),
    ]
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

    let (left, right) = help_columns(language, palette);
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

fn help_columns(language: Language, palette: Palette) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    match language {
        Language::Zh => (
            vec![
                help_section("导航", palette),
                help_item("q", "退出", palette),
                help_item("Esc", "取消选中 / 退出", palette),
                help_item("←/→", "切换城市", palette),
                help_item("Tab", "下个可见星座", palette),
                help_item("S-Tab", "上个可见星座", palette),
                help_item("z", "放大星座", palette),
                help_item("/", "搜索天体", palette),
                help_item("s", "设置位置", palette),
                help_item("o", "设置面板", palette),
                help_item("x", "指针模式", palette),
                help_item("?", "关闭帮助", palette),
                Line::from(""),
                help_section("时间", palette),
                help_item("space", "暂停 / 回实时", palette),
                help_item("[ / ]", "前后一小时", palette),
                help_item("{ / }", "前后一天", palette),
                help_item("r", "回到实时", palette),
            ],
            vec![
                help_section("面板和模式", palette),
                help_item("h", "今晚推荐", palette),
                help_item("v", "城市巡游", palette),
                help_item("m", "月相面板", palette),
                help_item("l", "星名标签", palette),
                help_item("c", "星座连线", palette),
                help_item("p", "行星", palette),
                help_item("d", "深空天体", palette),
                help_item("a", "动效", palette),
                Line::from(""),
                help_section("外观和设置", palette),
                help_item("t", "中英文", palette),
                help_item("T", "主题", palette),
                help_item("u", "字符集", palette),
                help_item("+ / -", "极限星等", palette),
                help_item("o", "不常用设置", palette),
            ],
        ),
        Language::En => (
            vec![
                help_section("Navigation", palette),
                help_item("q", "quit", palette),
                help_item("Esc", "clear / quit", palette),
                help_item("←/→", "switch city", palette),
                help_item("Tab", "next constellation", palette),
                help_item("S-Tab", "previous constellation", palette),
                help_item("z", "zoom constellation", palette),
                help_item("/", "search object", palette),
                help_item("s", "setup location", palette),
                help_item("o", "settings panel", palette),
                help_item("x", "pointer mode", palette),
                help_item("?", "close help", palette),
                Line::from(""),
                help_section("Time", palette),
                help_item("space", "pause / live", palette),
                help_item("[ / ]", "one hour", palette),
                help_item("{ / }", "one day", palette),
                help_item("r", "return live", palette),
            ],
            vec![
                help_section("Panels and Modes", palette),
                help_item("h", "tonight", palette),
                help_item("v", "city tour", palette),
                help_item("m", "moon panel", palette),
                help_item("l", "star labels", palette),
                help_item("c", "constellation lines", palette),
                help_item("p", "planets", palette),
                help_item("d", "deep sky", palette),
                help_item("a", "animations", palette),
                Line::from(""),
                help_section("Display and Settings", palette),
                help_item("t", "language", palette),
                help_item("T", "theme", palette),
                help_item("u", "charset", palette),
                help_item("+ / -", "limiting mag", palette),
                help_item("o", "less-used settings", palette),
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::TimeZone;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{
        catalog::Catalog,
        config::{Charset, Config},
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
    }

    #[test]
    fn footer_mentions_magnitude_and_constellation_lines() {
        assert!(i18n::tr(Language::Zh, "footer").contains("+/- 星等"));
        assert!(i18n::tr(Language::Zh, "footer").contains("c 连线"));
        assert!(i18n::tr(Language::En, "footer_compact").contains("+/- mag"));
        assert!(i18n::tr(Language::En, "footer_compact").contains("c lines"));
    }

    #[test]
    fn help_columns_keep_common_shortcuts_visible() {
        let (left, right) = help_columns(Language::Zh, palette(Theme::Midnight));
        let help_text = format!("{}{}", lines_text(&left), lines_text(&right));
        let compact_help = help_text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        for expected in ["c星座连线", "+ / -极限星等", "t中英文", "T主题", "u字符集"]
        {
            let compact_expected = expected
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            assert!(
                compact_help.contains(&compact_expected),
                "{expected} missing from help"
            );
        }
        assert!(compact_help.contains("z放大星座"));
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
    fn renders_themes_and_charsets() {
        for theme in [Theme::Midnight, Theme::Aurora, Theme::Amber, Theme::Mono] {
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
}
