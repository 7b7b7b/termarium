use std::{
    collections::BTreeSet,
    io,
    path::PathBuf,
    time::{Duration as StdDuration, Instant},
};

use chrono::{DateTime, Duration, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::{
    astro,
    catalog::{Catalog, Star},
    config::{self, Config, Language, Location, SkyCulture, SkyOrientation, Theme},
    constellations::{self, ConstellationLine},
    deep_sky::{self, DeepSkyObject},
    i18n, planets, star_aliases,
};

const POINTER_FAST_STEP: isize = 2;
const POINTER_HOLD_WINDOW: StdDuration = StdDuration::from_millis(160);
const SKY_TRANSITION_DURATION: StdDuration = StdDuration::from_millis(720);
const TIME_SHIFT_HOLD_WINDOW: StdDuration = StdDuration::from_millis(180);
const TIME_REPEAT_TRANSITION_DURATION: StdDuration = StdDuration::from_millis(160);
const SETUP_PRESET_FIELD: usize = 0;
const SETUP_SAVE_FIELD: usize = 1;
const HORIZON_TRANSITION_DURATION: StdDuration = StdDuration::from_secs(3);
const GROUND_ROTATION_DURATION: StdDuration = StdDuration::from_millis(420);
const GROUND_LAT_STEP: f64 = 2.5;
const GROUND_LON_STEP: f64 = 2.5;
const GROUND_HOLD_WINDOW: StdDuration = StdDuration::from_millis(160);
const GROUND_FAST_STEP: f64 = 2.0;
const SETTINGS_THEME_FIELD: usize = 1;
const APP_MIN_LIMITING_MAGNITUDE: f64 = -1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Sky,
    Setup,
    Search,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Sky,
    Ground,
}

#[derive(Debug, Clone)]
pub struct GroundState {
    pub cursor_lat: f64,
    pub cursor_lon: f64,
    pub preview_location: Location,
}

impl GroundState {
    fn from_location(location: &Location) -> Self {
        let preview_location = Location {
            name: location.name.clone(),
            latitude: location.latitude,
            longitude: normalize_longitude(location.longitude),
            timezone: location.timezone.clone(),
        };
        Self {
            cursor_lat: preview_location.latitude,
            cursor_lon: preview_location.longitude,
            preview_location,
        }
    }

    fn from_cursor(latitude: f64, longitude: f64) -> Self {
        let latitude = latitude.clamp(-89.5, 89.5);
        let longitude = normalize_longitude(longitude);
        let preview_location = map_preview_location(latitude, longitude);
        Self {
            cursor_lat: latitude,
            cursor_lon: longitude,
            preview_location,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub en: &'static str,
    pub zh: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: &'static str,
    pub custom: bool,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        en: "Shanghai",
        zh: "上海",
        latitude: 31.2304,
        longitude: 121.4737,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Beijing",
        zh: "北京",
        latitude: 39.9042,
        longitude: 116.4074,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Guangzhou",
        zh: "广州",
        latitude: 23.1291,
        longitude: 113.2644,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Shenzhen",
        zh: "深圳",
        latitude: 22.5431,
        longitude: 114.0579,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Hangzhou",
        zh: "杭州",
        latitude: 30.2741,
        longitude: 120.1551,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Nanjing",
        zh: "南京",
        latitude: 32.0603,
        longitude: 118.7969,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Chengdu",
        zh: "成都",
        latitude: 30.5728,
        longitude: 104.0668,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Chongqing",
        zh: "重庆",
        latitude: 29.5630,
        longitude: 106.5516,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Wuhan",
        zh: "武汉",
        latitude: 30.5928,
        longitude: 114.3055,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Xi'an",
        zh: "西安",
        latitude: 34.3416,
        longitude: 108.9398,
        timezone: "Asia/Shanghai",
        custom: false,
    },
    Preset {
        en: "Hong Kong",
        zh: "香港",
        latitude: 22.3193,
        longitude: 114.1694,
        timezone: "Asia/Hong_Kong",
        custom: false,
    },
    Preset {
        en: "Taipei",
        zh: "台北",
        latitude: 25.0330,
        longitude: 121.5654,
        timezone: "Asia/Taipei",
        custom: false,
    },
    Preset {
        en: "Urumqi",
        zh: "乌鲁木齐",
        latitude: 43.8256,
        longitude: 87.6168,
        timezone: "Asia/Urumqi",
        custom: false,
    },
    Preset {
        en: "Tokyo",
        zh: "东京",
        latitude: 35.6762,
        longitude: 139.6503,
        timezone: "Asia/Tokyo",
        custom: false,
    },
    Preset {
        en: "Osaka",
        zh: "大阪",
        latitude: 34.6937,
        longitude: 135.5023,
        timezone: "Asia/Tokyo",
        custom: false,
    },
    Preset {
        en: "Seoul",
        zh: "首尔",
        latitude: 37.5665,
        longitude: 126.9780,
        timezone: "Asia/Seoul",
        custom: false,
    },
    Preset {
        en: "Singapore",
        zh: "新加坡",
        latitude: 1.3521,
        longitude: 103.8198,
        timezone: "Asia/Singapore",
        custom: false,
    },
    Preset {
        en: "Bangkok",
        zh: "曼谷",
        latitude: 13.7563,
        longitude: 100.5018,
        timezone: "Asia/Bangkok",
        custom: false,
    },
    Preset {
        en: "Kuala Lumpur",
        zh: "吉隆坡",
        latitude: 3.1390,
        longitude: 101.6869,
        timezone: "Asia/Kuala_Lumpur",
        custom: false,
    },
    Preset {
        en: "Jakarta",
        zh: "雅加达",
        latitude: -6.2088,
        longitude: 106.8456,
        timezone: "Asia/Jakarta",
        custom: false,
    },
    Preset {
        en: "Manila",
        zh: "马尼拉",
        latitude: 14.5995,
        longitude: 120.9842,
        timezone: "Asia/Manila",
        custom: false,
    },
    Preset {
        en: "Delhi",
        zh: "德里",
        latitude: 28.6139,
        longitude: 77.2090,
        timezone: "Asia/Kolkata",
        custom: false,
    },
    Preset {
        en: "Mumbai",
        zh: "孟买",
        latitude: 19.0760,
        longitude: 72.8777,
        timezone: "Asia/Kolkata",
        custom: false,
    },
    Preset {
        en: "Dubai",
        zh: "迪拜",
        latitude: 25.2048,
        longitude: 55.2708,
        timezone: "Asia/Dubai",
        custom: false,
    },
    Preset {
        en: "Istanbul",
        zh: "伊斯坦布尔",
        latitude: 41.0082,
        longitude: 28.9784,
        timezone: "Europe/Istanbul",
        custom: false,
    },
    Preset {
        en: "London",
        zh: "伦敦",
        latitude: 51.5072,
        longitude: -0.1276,
        timezone: "Europe/London",
        custom: false,
    },
    Preset {
        en: "Paris",
        zh: "巴黎",
        latitude: 48.8566,
        longitude: 2.3522,
        timezone: "Europe/Paris",
        custom: false,
    },
    Preset {
        en: "Berlin",
        zh: "柏林",
        latitude: 52.5200,
        longitude: 13.4050,
        timezone: "Europe/Berlin",
        custom: false,
    },
    Preset {
        en: "Rome",
        zh: "罗马",
        latitude: 41.9028,
        longitude: 12.4964,
        timezone: "Europe/Rome",
        custom: false,
    },
    Preset {
        en: "Madrid",
        zh: "马德里",
        latitude: 40.4168,
        longitude: -3.7038,
        timezone: "Europe/Madrid",
        custom: false,
    },
    Preset {
        en: "Amsterdam",
        zh: "阿姆斯特丹",
        latitude: 52.3676,
        longitude: 4.9041,
        timezone: "Europe/Amsterdam",
        custom: false,
    },
    Preset {
        en: "Stockholm",
        zh: "斯德哥尔摩",
        latitude: 59.3293,
        longitude: 18.0686,
        timezone: "Europe/Stockholm",
        custom: false,
    },
    Preset {
        en: "Moscow",
        zh: "莫斯科",
        latitude: 55.7558,
        longitude: 37.6173,
        timezone: "Europe/Moscow",
        custom: false,
    },
    Preset {
        en: "New York",
        zh: "纽约",
        latitude: 40.7128,
        longitude: -74.0060,
        timezone: "America/New_York",
        custom: false,
    },
    Preset {
        en: "Boston",
        zh: "波士顿",
        latitude: 42.3601,
        longitude: -71.0589,
        timezone: "America/New_York",
        custom: false,
    },
    Preset {
        en: "Chicago",
        zh: "芝加哥",
        latitude: 41.8781,
        longitude: -87.6298,
        timezone: "America/Chicago",
        custom: false,
    },
    Preset {
        en: "Seattle",
        zh: "西雅图",
        latitude: 47.6062,
        longitude: -122.3321,
        timezone: "America/Los_Angeles",
        custom: false,
    },
    Preset {
        en: "Los Angeles",
        zh: "洛杉矶",
        latitude: 34.0522,
        longitude: -118.2437,
        timezone: "America/Los_Angeles",
        custom: false,
    },
    Preset {
        en: "San Francisco",
        zh: "旧金山",
        latitude: 37.7749,
        longitude: -122.4194,
        timezone: "America/Los_Angeles",
        custom: false,
    },
    Preset {
        en: "Toronto",
        zh: "多伦多",
        latitude: 43.6532,
        longitude: -79.3832,
        timezone: "America/Toronto",
        custom: false,
    },
    Preset {
        en: "Mexico City",
        zh: "墨西哥城",
        latitude: 19.4326,
        longitude: -99.1332,
        timezone: "America/Mexico_City",
        custom: false,
    },
    Preset {
        en: "Sydney",
        zh: "悉尼",
        latitude: -33.8688,
        longitude: 151.2093,
        timezone: "Australia/Sydney",
        custom: false,
    },
    Preset {
        en: "Melbourne",
        zh: "墨尔本",
        latitude: -37.8136,
        longitude: 144.9631,
        timezone: "Australia/Melbourne",
        custom: false,
    },
    Preset {
        en: "Auckland",
        zh: "奥克兰",
        latitude: -36.8485,
        longitude: 174.7633,
        timezone: "Pacific/Auckland",
        custom: false,
    },
    Preset {
        en: "Cape Town",
        zh: "开普敦",
        latitude: -33.9249,
        longitude: 18.4241,
        timezone: "Africa/Johannesburg",
        custom: false,
    },
    Preset {
        en: "Cairo",
        zh: "开罗",
        latitude: 30.0444,
        longitude: 31.2357,
        timezone: "Africa/Cairo",
        custom: false,
    },
    Preset {
        en: "Santiago",
        zh: "圣地亚哥",
        latitude: -33.4489,
        longitude: -70.6693,
        timezone: "America/Santiago",
        custom: false,
    },
    Preset {
        en: "Buenos Aires",
        zh: "布宜诺斯艾利斯",
        latitude: -34.6037,
        longitude: -58.3816,
        timezone: "America/Argentina/Buenos_Aires",
        custom: false,
    },
    Preset {
        en: "Sao Paulo",
        zh: "圣保罗",
        latitude: -23.5505,
        longitude: -46.6333,
        timezone: "America/Sao_Paulo",
        custom: false,
    },
    Preset {
        en: "Custom",
        zh: "自定义",
        latitude: 0.0,
        longitude: 0.0,
        timezone: "UTC",
        custom: true,
    },
];

#[derive(Debug, Clone)]
pub struct SetupState {
    pub preset_index: usize,
    pub preset_query: String,
    pub field: usize,
    pub name: String,
    pub latitude: String,
    pub longitude: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub selected: usize,
    pub theme_selected: usize,
    pub theme_picker: bool,
}

impl SetupState {
    pub fn from_location(location: &Location) -> Self {
        let preset_index = PRESETS
            .iter()
            .position(|preset| {
                !preset.custom
                    && (preset.latitude - location.latitude).abs() < 0.0001
                    && (preset.longitude - location.longitude).abs() < 0.0001
            })
            .unwrap_or(PRESETS.len() - 1);
        Self {
            preset_index,
            preset_query: String::new(),
            field: 0,
            name: location.name.clone(),
            latitude: format!("{:.4}", location.latitude),
            longitude: format!("{:.4}", location.longitude),
            timezone: location.timezone.clone(),
        }
    }

    pub fn preset_label(&self, language: Language) -> &'static str {
        let preset = PRESETS
            .get(self.preset_index)
            .unwrap_or(&PRESETS[PRESETS.len() - 1]);
        match language {
            Language::En => preset.en,
            Language::Zh => preset.zh,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Star(u32),
    Constellation(String),
    Planet(&'static str),
    Moon,
    DeepSky(&'static str),
    City(usize),
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub label: String,
    pub detail: String,
    pub target: Target,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Sky,
    City,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub mode: SearchMode,
}

impl SearchState {
    fn new(mode: SearchMode, language: Language) -> Self {
        let results = match mode {
            SearchMode::Sky => Vec::new(),
            SearchMode::City => city_search_results("", language),
        };
        Self {
            query: String::new(),
            results,
            selected: 0,
            mode,
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new(SearchMode::Sky, Language::En)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PointerState {
    pub active: bool,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub hovered: Option<Target>,
    pub hits: Vec<Target>,
}

#[derive(Debug, Clone)]
struct ProjectedTarget {
    target: Target,
    x: usize,
    y: usize,
    score: f64,
    sort_key: String,
}

#[derive(Debug, Clone, Copy)]
struct ZoomBounds {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

#[derive(Debug, Clone, Copy)]
struct ZoomViewport {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

#[derive(Debug, Clone)]
struct SkyTransition {
    kind: SkyTransitionKind,
    started: Instant,
    duration: StdDuration,
}

#[derive(Debug, Clone)]
struct HorizonTransition {
    from: ViewMode,
    to: ViewMode,
    started: Instant,
    duration: StdDuration,
}

#[derive(Debug, Clone)]
struct GroundRotationTransition {
    from_lat: f64,
    from_lon: f64,
    to_lat: f64,
    to_lon: f64,
    started: Instant,
    duration: StdDuration,
}

#[derive(Debug, Clone)]
enum SkyTransitionKind {
    City {
        from: Location,
        to: Location,
    },
    Zoom {
        from_code: String,
        to_code: String,
    },
    ZoomIn {
        code: String,
    },
    ZoomOut {
        code: String,
    },
    CityZoomOut {
        from: Location,
        to: Location,
        code: String,
    },
    Time {
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SkyViewMapper {
    viewport: ZoomViewport,
    source_width: usize,
    source_height: usize,
    width: usize,
    height: usize,
    orientation: SkyOrientation,
}

#[derive(Debug, Clone)]
pub(crate) struct ZoomRenderView {
    pub(crate) stars: Vec<astro::VisibleStar>,
    mapper: SkyViewMapper,
}

impl ZoomRenderView {
    pub(crate) fn project_horizontal(&self, altitude: f64, azimuth: f64) -> Option<(usize, usize)> {
        self.mapper.project_horizontal(altitude, azimuth)
    }
}

pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub catalog: Catalog,
    pub constellation_lines: Vec<ConstellationLine>,
    pub chinese_constellation_lines: Vec<ConstellationLine>,
    pub deep_sky: Vec<DeepSkyObject>,
    pub screen: Screen,
    pub view_mode: ViewMode,
    pub setup: SetupState,
    pub settings: SettingsState,
    pub search: SearchState,
    pub pointer: PointerState,
    pub ground: GroundState,
    pub session_location: Option<Location>,
    pub selected_target: Option<Target>,
    pub constellation_zoom: bool,
    pub constellation_zoom_code: Option<String>,
    pub message: String,
    pub can_cancel_setup: bool,
    pub help: bool,
    pub should_quit: bool,
    pub time_base: DateTime<Utc>,
    pub clock_base: Instant,
    pub paused: bool,
    pub show_recommendations: bool,
    pub tour_origin: Option<Location>,
    pub last_tour_switch: Instant,
    pub animation_tick: u64,
    pub opening_ticks: u8,
    last_pointer_move: Option<(KeyCode, Instant)>,
    last_time_shift: Option<(KeyCode, Instant)>,
    last_ground_move: Option<(KeyCode, Instant)>,
    sky_transition: Option<SkyTransition>,
    horizon_transition: Option<HorizonTransition>,
    ground_rotation_transition: Option<GroundRotationTransition>,
    pending_ground_transition: bool,
}

impl App {
    pub fn new(
        config: Config,
        config_path: PathBuf,
        catalog: Catalog,
        start_setup: bool,
        can_cancel_setup: bool,
        time_override: Option<DateTime<Utc>>,
    ) -> Self {
        let mut config = config;
        let magnitude_ceiling = catalog.faintest_magnitude();
        config.display.limiting_magnitude = config
            .display
            .limiting_magnitude
            .clamp(APP_MIN_LIMITING_MAGNITUDE, magnitude_ceiling);
        let setup = SetupState::from_location(&config.location);
        let ground = GroundState::from_location(&config.location);
        let time_base = time_override.unwrap_or_else(Utc::now);
        let language = config.language;
        let screen = if start_setup {
            Screen::Search
        } else {
            Screen::Sky
        };
        let view_mode = if start_setup {
            ViewMode::Ground
        } else {
            ViewMode::Sky
        };
        let search = if start_setup {
            SearchState::new(SearchMode::City, language)
        } else {
            SearchState::default()
        };
        Self {
            config,
            config_path,
            catalog,
            constellation_lines: constellations::load(),
            chinese_constellation_lines: constellations::load_chinese(),
            deep_sky: deep_sky::load(),
            screen,
            view_mode,
            setup,
            settings: SettingsState::default(),
            search,
            pointer: PointerState::default(),
            ground,
            session_location: None,
            selected_target: None,
            constellation_zoom: false,
            constellation_zoom_code: None,
            message: String::new(),
            can_cancel_setup,
            help: false,
            should_quit: false,
            time_base,
            clock_base: Instant::now(),
            paused: time_override.is_some(),
            show_recommendations: false,
            tour_origin: None,
            last_tour_switch: Instant::now(),
            animation_tick: 0,
            opening_ticks: 0,
            last_pointer_move: None,
            last_time_shift: None,
            last_ground_move: None,
            sky_transition: None,
            horizon_transition: None,
            ground_rotation_transition: None,
            pending_ground_transition: false,
        }
    }

    pub fn now(&self) -> DateTime<Utc> {
        if let Some(SkyTransition {
            kind: SkyTransitionKind::Time { from, to },
            ..
        }) = &self.sky_transition
        {
            return lerp_time(*from, *to, self.transition_progress().unwrap_or(1.0));
        }
        if self.paused {
            self.time_base
        } else {
            self.time_base
                + Duration::from_std(self.clock_base.elapsed()).unwrap_or_else(|_| Duration::zero())
        }
    }

    pub fn tick(&mut self) {
        if self.config.display.animations {
            self.animation_tick = self.animation_tick.wrapping_add(1);
            self.opening_ticks = self.opening_ticks.saturating_sub(1);
        }
        if self.tour_origin.is_some()
            && self.last_tour_switch.elapsed() >= StdDuration::from_secs(4)
        {
            self.advance_tour_city();
        }
        if self
            .sky_transition
            .as_ref()
            .is_some_and(|transition| transition.is_finished())
        {
            self.sky_transition = None;
            if self.pending_ground_transition {
                self.pending_ground_transition = false;
                self.enter_ground_view(ViewMode::Sky);
            } else if self.pointer.active {
                self.update_pointer_hover();
            }
        }
        if self
            .horizon_transition
            .as_ref()
            .is_some_and(|transition| transition.is_finished())
        {
            self.horizon_transition = None;
        }
        if self
            .ground_rotation_transition
            .as_ref()
            .is_some_and(GroundRotationTransition::is_finished)
        {
            self.ground_rotation_transition = None;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> io::Result<()> {
        if matches!(key.kind, KeyEventKind::Release) {
            return Ok(());
        }

        if self.help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => self.help = false,
                _ => {
                    self.help = false;
                    return self.handle_key(key);
                }
            }
            return Ok(());
        }

        if matches!(key.code, KeyCode::Char('?')) {
            self.help = true;
            return Ok(());
        }

        match self.screen {
            Screen::Sky => self.handle_sky_key(key),
            Screen::Setup => self.handle_setup_key(key),
            Screen::Search => self.handle_search_key(key),
            Screen::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_sky_key(&mut self, key: KeyEvent) -> io::Result<()> {
        if self.view_mode == ViewMode::Ground {
            return self.handle_ground_key(key);
        }

        if self.pointer.active {
            match key.code {
                KeyCode::Esc => {
                    self.pointer.active = false;
                    self.last_pointer_move = None;
                    return Ok(());
                }
                KeyCode::Left => {
                    let step = self.pointer_move_step(key);
                    self.move_pointer(-step, 0);
                    return Ok(());
                }
                KeyCode::Right => {
                    let step = self.pointer_move_step(key);
                    self.move_pointer(step, 0);
                    return Ok(());
                }
                KeyCode::Up => {
                    let step = self.pointer_move_step(key);
                    self.move_pointer(0, -step);
                    return Ok(());
                }
                KeyCode::Down => {
                    let step = self.pointer_move_step(key);
                    self.move_pointer(0, step);
                    return Ok(());
                }
                KeyCode::Char('x') => {
                    self.toggle_pointer();
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.constellation_zoom {
                    self.toggle_constellation_zoom();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Esc => {
                if self.constellation_zoom {
                    self.toggle_constellation_zoom();
                } else if self.selected_target.is_some() {
                    self.selected_target = None;
                    self.constellation_zoom = false;
                    self.constellation_zoom_code = None;
                    self.sky_transition = None;
                    self.pending_ground_transition = false;
                    self.message = i18n::tr(self.config.language, "selection_cleared").to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('s') => {
                self.open_setup_from(self.active_location().clone());
            }
            KeyCode::Char('/') => {
                self.open_sky_search();
            }
            KeyCode::Char('o') => {
                self.screen = Screen::Settings;
                self.message.clear();
            }
            KeyCode::Tab => self.cycle_visible_constellation(true),
            KeyCode::BackTab => self.cycle_visible_constellation(false),
            KeyCode::Left => self.cycle_city(false)?,
            KeyCode::Right => self.cycle_city(true)?,
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('[') => self.shift_hour_time(key, Duration::hours(-1)),
            KeyCode::Char(']') => self.shift_hour_time(key, Duration::hours(1)),
            KeyCode::Char('{') => self.shift_day_time(key, Duration::days(-1)),
            KeyCode::Char('}') => self.shift_day_time(key, Duration::days(1)),
            KeyCode::Char('r') => self.reset_time(),
            KeyCode::Char('t') => {
                self.config.language = self.config.language.toggle();
                self.save()?;
            }
            KeyCode::Char('T') => {
                self.open_theme_picker();
            }
            KeyCode::Char('m') => {
                self.config.display.moon_panel = !self.config.display.moon_panel;
                self.save()?;
            }
            KeyCode::Char('l') => {
                self.config.display.labels = !self.config.display.labels;
                self.save()?;
            }
            KeyCode::Char('c') => {
                self.config.display.constellations = !self.config.display.constellations;
                self.save()?;
            }
            KeyCode::Char('h') => {
                self.show_recommendations = !self.show_recommendations;
                self.config.display.side_panel = true;
            }
            KeyCode::Char('v') => self.toggle_tour(),
            KeyCode::Char('a') => {
                self.config.display.animations = !self.config.display.animations;
                self.save()?;
            }
            KeyCode::Char('p') => {
                self.config.display.planets = !self.config.display.planets;
                self.save()?;
            }
            KeyCode::Char('d') => {
                self.config.display.deep_sky = !self.config.display.deep_sky;
                self.save()?;
            }
            KeyCode::Char('u') => {
                self.config.display.charset = self.config.display.charset.next();
                self.save()?;
            }
            KeyCode::Char('y') => self.toggle_sky_culture()?,
            KeyCode::Char('x') => self.toggle_pointer(),
            KeyCode::Char('z') => self.toggle_constellation_zoom(),
            KeyCode::Char('g') => self.toggle_ground_sky(),
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_magnitude(0.1)?;
            }
            KeyCode::Char('-') => {
                self.adjust_magnitude(-0.1)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_ground_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('g') => self.toggle_ground_sky(),
            KeyCode::Char('s') => self.open_setup_from(self.ground.preview_location.clone()),
            KeyCode::Char('/') => self.open_city_search(),
            KeyCode::Left => {
                let step = self.ground_move_step(key);
                self.move_ground_cursor(-GROUND_LON_STEP * step, 0.0);
            }
            KeyCode::Right => {
                let step = self.ground_move_step(key);
                self.move_ground_cursor(GROUND_LON_STEP * step, 0.0);
            }
            KeyCode::Up => {
                let step = self.ground_move_step(key);
                self.move_ground_cursor(0.0, GROUND_LAT_STEP * step);
            }
            KeyCode::Down => {
                let step = self.ground_move_step(key);
                self.move_ground_cursor(0.0, -GROUND_LAT_STEP * step);
            }
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('[') => self.shift_hour_time(key, Duration::hours(-1)),
            KeyCode::Char(']') => self.shift_hour_time(key, Duration::hours(1)),
            KeyCode::Char('{') => self.shift_day_time(key, Duration::days(-1)),
            KeyCode::Char('}') => self.shift_day_time(key, Duration::days(1)),
            KeyCode::Char('r') => self.reset_time(),
            KeyCode::Char('o') => {
                self.screen = Screen::Settings;
                self.message.clear();
            }
            KeyCode::Char('t') => {
                self.config.language = self.config.language.toggle();
                self.save()?;
            }
            KeyCode::Char('T') => {
                self.open_theme_picker();
            }
            KeyCode::Char('a') => {
                self.config.display.animations = !self.config.display.animations;
                self.save()?;
            }
            KeyCode::Char('u') => {
                self.config.display.charset = self.config.display.charset.next();
                self.save()?;
            }
            KeyCode::Char('y') => self.toggle_sky_culture()?,
            _ => {}
        }
        Ok(())
    }

    fn handle_settings_key(&mut self, key: KeyEvent) -> io::Result<()> {
        if self.settings.theme_picker {
            return self.handle_theme_picker_key(key);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('o') => self.screen = Screen::Sky,
            KeyCode::Up | KeyCode::BackTab => {
                self.settings.selected = self.settings.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Tab => {
                self.settings.selected = (self.settings.selected + 1).min(settings_count() - 1);
            }
            KeyCode::Left => self.adjust_setting(false)?,
            KeyCode::Right | KeyCode::Enter => self.adjust_setting(true)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_theme_picker_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.settings.theme_picker = false,
            KeyCode::Up | KeyCode::BackTab => {
                self.select_theme(self.settings.theme_selected.saturating_sub(1));
            }
            KeyCode::Down | KeyCode::Tab => {
                self.select_theme(self.settings.theme_selected.saturating_add(1));
            }
            KeyCode::Enter | KeyCode::Right => self.apply_selected_theme()?,
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn select_setting(&mut self, index: usize) {
        self.settings.selected = index.min(settings_count() - 1);
    }

    pub(crate) fn open_theme_picker(&mut self) {
        self.screen = Screen::Settings;
        self.settings.selected = SETTINGS_THEME_FIELD;
        self.settings.theme_selected = self.config.display.theme.index();
        self.settings.theme_picker = true;
        self.message.clear();
    }

    pub(crate) fn select_theme(&mut self, index: usize) {
        self.settings.theme_selected = index.min(Theme::ALL.len() - 1);
    }

    pub(crate) fn apply_selected_theme(&mut self) -> io::Result<()> {
        self.config.display.theme = Theme::from_index(self.settings.theme_selected);
        self.settings.theme_picker = false;
        self.save()
    }

    pub(crate) fn adjust_setting(&mut self, forward: bool) -> io::Result<()> {
        match self.settings.selected {
            0 => self.config.language = self.config.language.toggle(),
            SETTINGS_THEME_FIELD => {
                self.open_theme_picker();
                return Ok(());
            }
            2 => self.config.display.charset = self.config.display.charset.next(),
            3 => self.config.display.animations = !self.config.display.animations,
            4 => self.config.display.planets = !self.config.display.planets,
            5 => self.config.display.deep_sky = !self.config.display.deep_sky,
            6 => self.config.display.moon_panel = !self.config.display.moon_panel,
            7 => self.config.display.labels = !self.config.display.labels,
            8 => self.config.display.constellations = !self.config.display.constellations,
            9 => self.cycle_sky_culture(),
            10 => self.config.display.side_panel = !self.config.display.side_panel,
            11 => self.config.display.landscape = self.config.display.landscape.next(),
            12 => self.config.display.sky_orientation = self.config.display.sky_orientation.next(),
            13 => {
                let delta = if forward { 0.1 } else { -0.1 };
                self.set_limiting_magnitude(self.config.display.limiting_magnitude + delta);
            }
            _ => {}
        }
        self.save()
    }

    fn toggle_sky_culture(&mut self) -> io::Result<()> {
        self.cycle_sky_culture();
        self.save()
    }

    fn cycle_sky_culture(&mut self) {
        self.config.display.sky_culture = self.config.display.sky_culture.next();
        if matches!(self.selected_target, Some(Target::Constellation(_))) {
            self.selected_target = None;
            self.constellation_zoom = false;
            self.constellation_zoom_code = None;
            self.sky_transition = None;
        }
        let culture_key = match self.config.display.sky_culture {
            SkyCulture::Western => "western",
            SkyCulture::Chinese => "chinese",
        };
        self.message = format!(
            "{}: {}",
            i18n::tr(self.config.language, "sky_culture"),
            i18n::tr(self.config.language, culture_key)
        );
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Esc => {
                if self.search.mode == SearchMode::City && !self.can_cancel_setup {
                    self.should_quit = true;
                } else {
                    self.screen = Screen::Sky;
                }
            }
            KeyCode::Enter => self.activate_search_result(self.search.selected)?,
            KeyCode::Up => {
                self.search.selected = self.search.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if !self.search.results.is_empty() {
                    self.search.selected =
                        (self.search.selected + 1).min(self.search.results.len() - 1);
                }
            }
            KeyCode::Backspace => {
                self.search.query.pop();
                self.refresh_search();
            }
            KeyCode::Char(c) => {
                self.search.query.push(c);
                self.refresh_search();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn select_search_result(&mut self, index: usize) {
        if !self.search.results.is_empty() {
            self.search.selected = index.min(self.search.results.len() - 1);
        }
    }

    pub(crate) fn activate_search_result(&mut self, index: usize) -> io::Result<()> {
        self.select_search_result(index);
        if let Some(result) = self.search.results.get(self.search.selected).cloned() {
            if let Target::City(index) = result.target {
                self.apply_ground_city(index)?;
            } else {
                if !matches!(result.target, Target::Constellation(_)) {
                    self.constellation_zoom = false;
                    self.constellation_zoom_code = None;
                    self.sky_transition = None;
                } else if self.constellation_zoom
                    && let Target::Constellation(code) = &result.target
                {
                    if let Some(from_code) =
                        self.selected_constellation_code().map(ToString::to_string)
                    {
                        self.start_zoom_transition(from_code, code.clone());
                    }
                    self.constellation_zoom_code = Some(code.clone());
                }
                self.selected_target = Some(result.target);
                self.message = result.label;
            }
        }
        self.screen = Screen::Sky;
        Ok(())
    }

    fn handle_setup_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Esc => {
                if self.can_cancel_setup {
                    self.screen = Screen::Sky;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Tab | KeyCode::Down => self.move_setup_field(SETUP_SAVE_FIELD),
            KeyCode::BackTab | KeyCode::Up => {
                self.move_setup_field(SETUP_PRESET_FIELD);
            }
            KeyCode::Left => {
                if self.setup.field == SETUP_PRESET_FIELD {
                    self.cycle_preset(false);
                }
            }
            KeyCode::Right => {
                if self.setup.field == SETUP_PRESET_FIELD {
                    self.cycle_preset(true);
                }
            }
            KeyCode::Enter => {
                if self.setup.field == SETUP_SAVE_FIELD {
                    self.commit_setup()?;
                } else {
                    self.move_setup_field(SETUP_SAVE_FIELD);
                }
            }
            KeyCode::Backspace if self.setup.field == SETUP_PRESET_FIELD => {
                if self.setup.preset_query.pop().is_some() && !self.setup.preset_query.is_empty() {
                    self.refresh_setup_preset(false);
                }
            }
            KeyCode::Char(c) if self.setup.field == SETUP_PRESET_FIELD => {
                self.setup.preset_query.push(c);
                self.refresh_setup_preset(true);
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn move_setup_field(&mut self, field: usize) {
        let field = field.min(SETUP_SAVE_FIELD);
        if self.setup.field == SETUP_PRESET_FIELD && field != SETUP_PRESET_FIELD {
            self.confirm_setup_preset_search();
        }
        self.setup.field = field;
    }

    pub(crate) fn open_setup_from(&mut self, location: Location) {
        self.setup = SetupState::from_location(&location);
        self.screen = Screen::Setup;
        self.message.clear();
    }

    pub(crate) fn open_sky_search(&mut self) {
        self.search = SearchState::new(SearchMode::Sky, self.config.language);
        self.screen = Screen::Search;
    }

    pub(crate) fn open_city_search(&mut self) {
        self.search = SearchState::new(SearchMode::City, self.config.language);
        self.screen = Screen::Search;
    }

    pub(crate) fn cycle_preset(&mut self, forward: bool) {
        let matches = self.setup_preset_matches();
        if matches.is_empty() {
            return;
        }
        let current = matches
            .iter()
            .position(|index| *index == self.setup.preset_index);
        self.setup.preset_index = matches[next_position(current, matches.len(), forward)];
        self.apply_setup_preset();
    }

    fn refresh_setup_preset(&mut self, prefer_first: bool) {
        let matches = self.setup_preset_matches();
        if matches.is_empty() {
            return;
        }
        let current_in_matches = matches
            .iter()
            .any(|index| *index == self.setup.preset_index);
        if prefer_first || !current_in_matches {
            self.setup.preset_index = matches[0];
        }
        self.apply_setup_preset();
    }

    fn confirm_setup_preset_search(&mut self) {
        if self.setup.preset_query.is_empty() {
            return;
        }
        self.refresh_setup_preset(false);
        self.setup.preset_query.clear();
    }

    fn setup_preset_matches(&self) -> Vec<usize> {
        let query = self.setup.preset_query.trim();
        if query.is_empty() {
            return (0..PRESETS.len()).collect();
        }
        PRESETS
            .iter()
            .enumerate()
            .filter_map(|(index, preset)| {
                (!preset.custom && city_matches_query(preset, query)).then_some(index)
            })
            .collect()
    }

    fn apply_setup_preset(&mut self) {
        let preset = PRESETS[self.setup.preset_index];
        if !preset.custom {
            self.setup.name = preset.en.to_string();
            self.setup.latitude = format!("{:.4}", preset.latitude);
            self.setup.longitude = format!("{:.4}", preset.longitude);
            self.setup.timezone = preset.timezone.to_string();
        }
    }

    pub(crate) fn commit_setup(&mut self) -> io::Result<()> {
        self.confirm_setup_preset_search();
        let from_location = self.config.location.clone();
        let latitude = match self.setup.latitude.trim().parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                self.message = i18n::tr(self.config.language, "invalid_coord").to_string();
                return Ok(());
            }
        };
        let longitude = match self.setup.longitude.trim().parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                self.message = i18n::tr(self.config.language, "invalid_coord").to_string();
                return Ok(());
            }
        };
        let timezone = self.setup.timezone.trim().to_string();
        let name = if self.setup.name.trim().is_empty() {
            "Custom".to_string()
        } else {
            self.setup.name.trim().to_string()
        };
        let to_location = Location {
            name,
            latitude,
            longitude,
            timezone,
        };
        self.config.location = to_location.clone();

        if let Err(err) = self.config.validate() {
            self.config.location = from_location;
            self.message = err;
            return Ok(());
        }

        self.save()?;
        self.start_city_transition(from_location, to_location);
        self.session_location = None;
        self.ground = GroundState::from_location(&self.config.location);
        self.ground_rotation_transition = None;
        self.view_mode = ViewMode::Sky;
        self.horizon_transition = None;
        self.can_cancel_setup = true;
        self.screen = Screen::Sky;
        self.message = i18n::tr(self.config.language, "saved").to_string();
        Ok(())
    }

    fn cycle_city(&mut self, forward: bool) -> io::Result<()> {
        self.stop_tour_without_restore();
        let city_indices = city_indices();
        if city_indices.is_empty() {
            return Ok(());
        }
        let current_position = current_preset_index(&self.config.location).and_then(|index| {
            city_indices
                .iter()
                .position(|candidate| *candidate == index)
        });
        let next_position = next_position(current_position, city_indices.len(), forward);
        self.apply_preset(city_indices[next_position], true)
    }

    fn apply_preset(&mut self, index: usize, persist: bool) -> io::Result<()> {
        let preset = PRESETS[index];
        let from_location = self.config.location.clone();
        let to_location = location_for_preset(preset);
        let hidden_zoom_code = self
            .constellation_zoom
            .then(|| self.selected_constellation_code().map(ToString::to_string))
            .flatten()
            .filter(|code| !self.constellation_visible_at(code, &to_location));
        if let Some(code) = hidden_zoom_code {
            self.constellation_zoom = false;
            self.constellation_zoom_code = None;
            self.selected_target = Some(Target::Constellation(code.clone()));
            self.config.display.side_panel = true;
            self.start_city_zoom_out_transition(from_location, to_location.clone(), code.clone());
            self.message = self.constellation_not_visible_message(&code, &to_location);
        } else {
            self.start_city_transition(from_location, to_location.clone());
            self.message = format!(
                "{}: {}",
                i18n::tr(self.config.language, "observer"),
                to_location.name
            );
        }
        self.config.location = to_location;
        self.session_location = None;
        self.ground = GroundState::from_location(&self.config.location);
        self.ground_rotation_transition = None;
        self.setup = SetupState::from_location(&self.config.location);
        if persist { self.save() } else { Ok(()) }
    }

    pub(crate) fn apply_ground_city(&mut self, index: usize) -> io::Result<()> {
        let Some(preset) = PRESETS.get(index).copied().filter(|preset| !preset.custom) else {
            return Ok(());
        };
        let location = location_for_preset(preset);
        let first_location_pick = !self.can_cancel_setup;
        let animate_from = (self.config.display.animations
            && self.view_mode == ViewMode::Ground
            && self.horizon_transition.is_none())
        .then(|| self.render_ground_center());
        self.stop_tour_without_restore();
        self.pointer.active = false;
        self.last_pointer_move = None;
        self.last_ground_move = None;
        self.view_mode = ViewMode::Ground;
        self.ground = GroundState::from_location(&location);
        if let Some((from_lat, from_lon)) = animate_from {
            self.ground_rotation_transition = Some(GroundRotationTransition {
                from_lat,
                from_lon,
                to_lat: self.ground.cursor_lat,
                to_lon: self.ground.cursor_lon,
                started: Instant::now(),
                duration: GROUND_ROTATION_DURATION,
            });
        } else {
            self.ground_rotation_transition = None;
        }
        self.setup = SetupState::from_location(&location);
        self.horizon_transition = None;
        self.sky_transition = None;
        self.pending_ground_transition = false;
        self.selected_target = None;

        if first_location_pick {
            let previous = self.config.location.clone();
            self.config.location = location.clone();
            if let Err(err) = self.config.validate() {
                self.config.location = previous;
                self.message = err;
                return Ok(());
            }
            self.session_location = None;
            self.can_cancel_setup = true;
            self.save()?;
            self.message = format!(
                "{}: {}",
                i18n::tr(self.config.language, "saved"),
                preset_display_name(preset, self.config.language)
            );
        } else {
            self.session_location = Some(location.clone());
            self.message = format!(
                "{}: {}",
                i18n::tr(self.config.language, "ground_cursor"),
                preset_display_name(preset, self.config.language)
            );
        }
        Ok(())
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            self.reset_time();
        } else {
            self.time_base = self.now();
            self.clock_base = Instant::now();
            self.paused = true;
            self.last_time_shift = None;
            self.clear_time_transition();
            self.message = i18n::tr(self.config.language, "paused").to_string();
        }
    }

    fn shift_time_immediate(&mut self, delta: Duration) {
        let to_time = self.committed_time() + delta;
        self.time_base = to_time;
        self.clock_base = Instant::now();
        self.clear_time_transition();
        self.message = i18n::tr(self.config.language, "time_shifted").to_string();
    }

    fn shift_hour_time(&mut self, key: KeyEvent, delta: Duration) {
        let is_repeat = self.is_repeated_time_shift(key);
        if is_repeat || !self.config.display.animations {
            self.shift_time_immediate(delta);
        } else {
            self.shift_time_animated(delta, SKY_TRANSITION_DURATION);
        }
    }

    fn shift_day_time(&mut self, key: KeyEvent, delta: Duration) {
        let is_repeat = self.is_repeated_time_shift(key);
        if !self.config.display.animations {
            self.shift_time_immediate(delta);
            return;
        }
        let duration = if is_repeat {
            TIME_REPEAT_TRANSITION_DURATION
        } else {
            SKY_TRANSITION_DURATION
        };
        self.shift_time_animated(delta, duration);
    }

    fn shift_time_animated(&mut self, delta: Duration, duration: StdDuration) {
        let from_time = self.now();
        let to_time = self.committed_time() + delta;
        self.time_base = to_time;
        self.clock_base = Instant::now();
        self.start_time_transition(from_time, to_time, duration);
        self.message = i18n::tr(self.config.language, "time_shifted").to_string();
    }

    fn reset_time(&mut self) {
        self.time_base = Utc::now();
        self.clock_base = Instant::now();
        self.paused = false;
        self.last_time_shift = None;
        self.clear_time_transition();
        self.message = i18n::tr(self.config.language, "live").to_string();
    }

    fn toggle_tour(&mut self) {
        if let Some(origin) = self.tour_origin.take() {
            self.start_city_transition(self.config.location.clone(), origin.clone());
            self.config.location = origin;
            self.session_location = None;
            self.ground = GroundState::from_location(&self.config.location);
            self.ground_rotation_transition = None;
            self.setup = SetupState::from_location(&self.config.location);
            self.message = i18n::tr(self.config.language, "tour_off").to_string();
        } else {
            self.session_location = None;
            self.tour_origin = Some(self.config.location.clone());
            self.last_tour_switch = Instant::now() - StdDuration::from_secs(4);
            self.advance_tour_city();
            self.message = i18n::tr(self.config.language, "tour_on").to_string();
        }
    }

    fn stop_tour_without_restore(&mut self) {
        self.tour_origin = None;
    }

    fn advance_tour_city(&mut self) {
        let city_indices = city_indices();
        if city_indices.is_empty() {
            return;
        }
        let current_position = current_preset_index(&self.config.location).and_then(|index| {
            city_indices
                .iter()
                .position(|candidate| *candidate == index)
        });
        let next = next_position(current_position, city_indices.len(), true);
        let _ = self.apply_preset(city_indices[next], false);
        self.last_tour_switch = Instant::now();
    }

    fn refresh_search(&mut self) {
        self.search.results = match self.search.mode {
            SearchMode::Sky => self.search_results(&self.search.query),
            SearchMode::City => city_search_results(&self.search.query, self.config.language),
        };
        self.search.selected = self
            .search
            .selected
            .min(self.search.results.len().saturating_sub(1));
    }

    fn search_results(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let needle = query.to_lowercase();
        let mut results = Vec::new();

        for meta in constellations::metadata_for_culture(self.config.display.sky_culture) {
            if constellations::matches_query(self.config.display.sky_culture, meta.code, query) {
                results.push(SearchResult {
                    label: match self.config.display.sky_culture {
                        SkyCulture::Western => format!("{} / {}", meta.code, meta.en),
                        SkyCulture::Chinese => format!("{} / {}", meta.zh, meta.en),
                    },
                    detail: format!(
                        "{} · {}",
                        i18n::tr(self.config.language, self.figure_kind_key()),
                        meta.zh
                    ),
                    target: Target::Constellation(meta.code.to_string()),
                });
            }
        }

        for star in &self.catalog.stars {
            if star_aliases::matches_query(*star, query) {
                let label = star_aliases::display_name_for(*star, self.config.language);
                let canonical = star_aliases::display_name(*star);
                let detail_name = if canonical == format!("HIP {}", star.hip) {
                    "star".to_string()
                } else {
                    format!("star · {label}")
                };
                results.push(SearchResult {
                    label,
                    detail: format!(
                        "{detail_name} · HIP {} · mag {:.1}",
                        star.hip, star.magnitude
                    ),
                    target: Target::Star(star.hip),
                });
            }
        }

        for planet in planets::visible_planets(self.now()) {
            if planets::matches_query(planet, &needle) {
                results.push(SearchResult {
                    label: planet.name.to_string(),
                    detail: "planet".to_string(),
                    target: Target::Planet(planet.name),
                });
            }
        }

        for object in &self.deep_sky {
            if deep_sky::matches_query(*object, &needle) {
                let common = if object.common.is_empty() {
                    object.kind.to_string()
                } else {
                    format!("{} · {}", object.kind, object.common)
                };
                results.push(SearchResult {
                    label: object.name.to_string(),
                    detail: common,
                    target: Target::DeepSky(object.name),
                });
            }
        }

        results.truncate(14);
        results
    }

    fn cycle_visible_constellation(&mut self, forward: bool) {
        let codes = self.visible_constellation_codes();
        if codes.is_empty() {
            self.message = i18n::tr(self.config.language, "no_visible_constellations").to_string();
            return;
        }
        let previous = self.selected_constellation_code().map(ToString::to_string);
        let current = self.selected_constellation_code().and_then(|code| {
            codes
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(code))
        });
        let next = next_position(current, codes.len(), forward);
        self.selected_target = Some(Target::Constellation(codes[next].clone()));
        if self.constellation_zoom {
            if let Some(from_code) = previous {
                self.start_zoom_transition(from_code, codes[next].clone());
            }
            self.constellation_zoom_code = Some(codes[next].clone());
            if self.pointer.active {
                self.update_pointer_hover();
            }
        }
    }

    pub(crate) fn select_constellation_code(&mut self, code: String) {
        let previous = self.selected_constellation_code().map(ToString::to_string);
        self.pointer.active = false;
        self.pointer.hovered = None;
        self.pointer.hits.clear();
        self.last_pointer_move = None;
        self.selected_target = Some(Target::Constellation(code.clone()));
        self.config.display.side_panel = true;

        if self.constellation_zoom {
            if previous.as_deref() != Some(code.as_str()) {
                if let Some(from_code) = previous {
                    self.start_zoom_transition(from_code, code.clone());
                }
            }
            self.constellation_zoom_code = Some(code.clone());
        }

        let meta = self.figure_meta(&code);
        self.message = match self.config.display.sky_culture {
            SkyCulture::Western => format!("{} · {}", meta.code, meta.en),
            SkyCulture::Chinese => format!("{} · {}", meta.zh, meta.en),
        };
    }

    pub fn selected_constellation_code(&self) -> Option<&str> {
        if self.constellation_zoom {
            if let Some(code) = self.constellation_zoom_code.as_deref() {
                return Some(code);
            }
        }
        match &self.selected_target {
            Some(Target::Constellation(code)) => Some(code.as_str()),
            _ => None,
        }
    }

    pub fn active_location(&self) -> &Location {
        if self.view_mode == ViewMode::Ground {
            return &self.ground.preview_location;
        }
        self.session_location
            .as_ref()
            .unwrap_or(&self.config.location)
    }

    pub fn render_location(&self) -> Location {
        if let Some(transition) = &self.sky_transition {
            match &transition.kind {
                SkyTransitionKind::City { from, to }
                | SkyTransitionKind::CityZoomOut { from, to, .. } => {
                    return lerp_location(from, to, transition.eased_progress());
                }
                _ => {}
            }
        }
        self.active_location().clone()
    }

    pub(crate) fn render_ground_center(&self) -> (f64, f64) {
        self.ground_rotation_transition
            .as_ref()
            .map(GroundRotationTransition::center)
            .unwrap_or((self.ground.cursor_lat, self.ground.cursor_lon))
    }

    pub fn zoom_transition(&self) -> Option<(&str, &str, f64)> {
        let Some(SkyTransition {
            kind: SkyTransitionKind::Zoom { from_code, to_code },
            ..
        }) = &self.sky_transition
        else {
            return None;
        };
        Some((
            from_code.as_str(),
            to_code.as_str(),
            self.transition_progress()?,
        ))
    }

    pub fn zoom_render_code(&self) -> Option<&str> {
        if let Some(SkyTransition {
            kind:
                SkyTransitionKind::ZoomIn { code }
                | SkyTransitionKind::ZoomOut { code }
                | SkyTransitionKind::CityZoomOut { code, .. },
            ..
        }) = &self.sky_transition
        {
            return Some(code.as_str());
        }
        if let Some((from_code, to_code, progress)) = self.zoom_transition() {
            return Some(if progress < 0.5 { from_code } else { to_code });
        }
        self.selected_constellation_code()
    }

    pub fn should_render_zoomed_sky(&self) -> bool {
        self.constellation_zoom
            || matches!(
                self.sky_transition,
                Some(SkyTransition {
                    kind: SkyTransitionKind::ZoomOut { .. } | SkyTransitionKind::CityZoomOut { .. },
                    ..
                })
            )
    }

    pub fn horizon_transition(&self) -> Option<(ViewMode, ViewMode, f64)> {
        self.horizon_transition
            .as_ref()
            .map(|transition| (transition.from, transition.to, transition.eased_progress()))
    }

    pub fn star_by_hip(&self, hip: u32) -> Option<Star> {
        self.catalog
            .stars
            .iter()
            .copied()
            .find(|star| star.hip == hip)
    }

    pub fn deep_sky_by_name(&self, name: &str) -> Option<DeepSkyObject> {
        self.deep_sky
            .iter()
            .copied()
            .find(|object| object.name.eq_ignore_ascii_case(name))
    }

    pub fn planet_by_name(&self, name: &str) -> Option<planets::Planet> {
        planets::visible_planets(self.now())
            .into_iter()
            .find(|planet| planet.name.eq_ignore_ascii_case(name))
    }

    pub fn setup_preset_match_count(&self) -> usize {
        self.setup_preset_matches().len()
    }

    pub fn active_constellation_lines(&self) -> &[ConstellationLine] {
        match self.config.display.sky_culture {
            SkyCulture::Western => &self.constellation_lines,
            SkyCulture::Chinese => &self.chinese_constellation_lines,
        }
    }

    pub fn figure_meta(&self, code: &str) -> constellations::ConstellationMeta {
        constellations::meta_for_culture(self.config.display.sky_culture, code)
    }

    pub fn figure_label(&self, code: &str) -> String {
        constellations::figure_label(self.config.display.sky_culture, code, self.config.language)
    }

    pub fn figure_kind_key(&self) -> &'static str {
        match self.config.display.sky_culture {
            SkyCulture::Western => "constellation",
            SkyCulture::Chinese => "asterism",
        }
    }

    pub fn figure_node_legend_key(&self) -> &'static str {
        match self.config.display.sky_culture {
            SkyCulture::Western => "legend_constellation",
            SkyCulture::Chinese => "legend_asterism",
        }
    }

    pub fn figure_contains_hip(&self, code: &str, hip: u32) -> bool {
        self.active_constellation_lines()
            .iter()
            .filter(|line| line.code.eq_ignore_ascii_case(code))
            .any(|line| line.hips.contains(&hip))
    }

    pub fn visible_constellation_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        let now = self.now();
        for line in self.active_constellation_lines() {
            if line.hips.windows(2).any(|pair| {
                self.line_endpoint_visible(pair[0], now) && self.line_endpoint_visible(pair[1], now)
            }) {
                codes.insert(line.code.to_string());
            }
        }
        codes.into_iter().collect()
    }

    pub fn constellation_visible(&self, code: &str) -> bool {
        self.constellation_visible_at(code, self.active_location())
    }

    fn constellation_visible_at(&self, code: &str, location: &Location) -> bool {
        let now = self.now();
        let line_visible = self
            .active_constellation_lines()
            .iter()
            .filter(|line| line.code.eq_ignore_ascii_case(code))
            .any(|line| {
                line.hips.windows(2).any(|pair| {
                    self.line_endpoint_visible_at(pair[0], location, now)
                        && self.line_endpoint_visible_at(pair[1], location, now)
                })
            });
        line_visible
            || (self.config.display.sky_culture == SkyCulture::Western
                && self.catalog.stars.iter().any(|star| {
                    star.constellation.eq_ignore_ascii_case(code)
                        && star.magnitude <= self.config.display.limiting_magnitude
                        && astro::horizontal_position(
                            star.ra_hours,
                            star.dec_degrees,
                            location,
                            now,
                        )
                        .altitude
                            > 0.0
                }))
    }

    pub fn set_pointer_canvas(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            self.pointer.width = 0;
            self.pointer.height = 0;
            self.pointer.hovered = None;
            self.pointer.hits.clear();
            return;
        }

        if self.pointer.width == 0 || self.pointer.height == 0 {
            self.pointer.x = width / 2;
            self.pointer.y = height / 2;
        } else if self.pointer.width != width || self.pointer.height != height {
            self.pointer.x =
                (self.pointer.x.saturating_mul(width) / self.pointer.width).min(width - 1);
            self.pointer.y =
                (self.pointer.y.saturating_mul(height) / self.pointer.height).min(height - 1);
        } else {
            self.pointer.x = self.pointer.x.min(width - 1);
            self.pointer.y = self.pointer.y.min(height - 1);
        }

        self.pointer.width = width;
        self.pointer.height = height;
        if self.pointer.active {
            self.update_pointer_hover();
        }
    }

    fn toggle_pointer(&mut self) {
        self.pointer.active = !self.pointer.active;
        self.last_pointer_move = None;
        if self.pointer.active {
            if self.pointer.width > 0 && self.pointer.height > 0 {
                self.pointer.x = self.pointer.x.min(self.pointer.width - 1);
                self.pointer.y = self.pointer.y.min(self.pointer.height - 1);
            }
            self.config.display.side_panel = true;
            self.update_pointer_hover();
            if self.pointer.width == 0 || self.pointer.height == 0 {
                self.message = i18n::tr(self.config.language, "pointer_on").to_string();
            }
        } else {
            self.pointer.hovered = None;
            self.pointer.hits.clear();
            self.message = i18n::tr(self.config.language, "pointer_off").to_string();
        }
    }

    pub(crate) fn toggle_ground_sky(&mut self) {
        let from = self.view_mode;
        match self.view_mode {
            ViewMode::Sky => {
                if self.config.display.animations && self.constellation_zoom {
                    let code = self.selected_constellation_code().map(ToString::to_string);
                    self.pointer.active = false;
                    self.last_pointer_move = None;
                    self.constellation_zoom = false;
                    self.constellation_zoom_code = None;
                    self.pending_ground_transition = code.is_some();
                    if let Some(code) = code {
                        self.start_zoom_out_transition(code);
                    } else {
                        self.enter_ground_view(from);
                    }
                    self.message = i18n::tr(self.config.language, "ground_mode").to_string();
                    return;
                }
                self.enter_ground_view(from);
            }
            ViewMode::Ground => {
                self.session_location = Some(self.ground.preview_location.clone());
                self.constellation_zoom = false;
                self.constellation_zoom_code = None;
                self.sky_transition = None;
                self.pending_ground_transition = false;
                self.view_mode = ViewMode::Sky;
                self.message = i18n::tr(self.config.language, "sky_mode").to_string();
                self.start_horizon_transition(from, self.view_mode);
            }
        }
    }

    fn enter_ground_view(&mut self, from: ViewMode) {
        self.pointer.active = false;
        self.last_pointer_move = None;
        self.constellation_zoom = false;
        self.constellation_zoom_code = None;
        self.ground = GroundState::from_location(self.active_location());
        self.ground_rotation_transition = None;
        self.view_mode = ViewMode::Ground;
        self.message = i18n::tr(self.config.language, "ground_mode").to_string();
        self.start_horizon_transition(from, self.view_mode);
    }

    fn start_horizon_transition(&mut self, from: ViewMode, to: ViewMode) {
        if self.config.display.animations {
            self.horizon_transition = Some(HorizonTransition {
                from,
                to,
                started: Instant::now(),
                duration: HORIZON_TRANSITION_DURATION,
            });
        } else {
            self.horizon_transition = None;
        }
    }

    fn move_ground_cursor(&mut self, delta_lon: f64, delta_lat: f64) {
        let next_lat = (self.ground.cursor_lat + delta_lat).clamp(-89.5, 89.5);
        let next_lon = normalize_longitude(self.ground.cursor_lon + delta_lon);
        self.preview_ground_location(next_lat, next_lon);
    }

    pub(crate) fn preview_ground_location(&mut self, latitude: f64, longitude: f64) {
        let (from_lat, from_lon) = self.render_ground_center();
        let to_lat = latitude.clamp(-89.5, 89.5);
        let to_lon = normalize_longitude(longitude);
        self.ground = GroundState::from_cursor(to_lat, to_lon);
        if self.config.display.animations
            && self.view_mode == ViewMode::Ground
            && self.horizon_transition.is_none()
        {
            self.ground_rotation_transition = Some(GroundRotationTransition {
                from_lat,
                from_lon,
                to_lat,
                to_lon,
                started: Instant::now(),
                duration: GROUND_ROTATION_DURATION,
            });
        } else {
            self.ground_rotation_transition = None;
        }
        self.message = format!(
            "{} {:+.1} {:+.1}",
            i18n::tr(self.config.language, "ground_cursor"),
            self.ground.cursor_lat,
            self.ground.cursor_lon
        );
    }

    fn ground_move_step(&mut self, key: KeyEvent) -> f64 {
        let now = Instant::now();
        let is_fast_repeat = matches!(key.kind, KeyEventKind::Repeat)
            || self.last_ground_move.is_some_and(|(code, at)| {
                code == key.code && now.duration_since(at) <= GROUND_HOLD_WINDOW
            });
        self.last_ground_move = Some((key.code, now));
        if is_fast_repeat {
            GROUND_FAST_STEP
        } else {
            1.0
        }
    }

    fn pointer_move_step(&mut self, key: KeyEvent) -> isize {
        let now = Instant::now();
        let is_fast_repeat = matches!(key.kind, KeyEventKind::Repeat)
            || self.last_pointer_move.is_some_and(|(code, at)| {
                code == key.code && now.duration_since(at) <= POINTER_HOLD_WINDOW
            });
        self.last_pointer_move = Some((key.code, now));
        if is_fast_repeat { POINTER_FAST_STEP } else { 1 }
    }

    fn is_repeated_time_shift(&mut self, key: KeyEvent) -> bool {
        let now = Instant::now();
        let is_repeat = matches!(key.kind, KeyEventKind::Repeat)
            || self.last_time_shift.is_some_and(|(code, at)| {
                code == key.code && now.duration_since(at) <= TIME_SHIFT_HOLD_WINDOW
            });
        self.last_time_shift = Some((key.code, now));
        is_repeat
    }

    fn committed_time(&self) -> DateTime<Utc> {
        if let Some(SkyTransition {
            kind: SkyTransitionKind::Time { to, .. },
            ..
        }) = &self.sky_transition
        {
            *to
        } else {
            self.now()
        }
    }

    fn start_city_transition(&mut self, from: Location, to: Location) {
        if !self.config.display.animations || same_location(&from, &to) {
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::City { from, to },
            started: Instant::now(),
            duration: SKY_TRANSITION_DURATION,
        });
    }

    fn start_zoom_transition(&mut self, from_code: String, to_code: String) {
        if !self.config.display.animations || from_code.eq_ignore_ascii_case(&to_code) {
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::Zoom { from_code, to_code },
            started: Instant::now(),
            duration: SKY_TRANSITION_DURATION,
        });
    }

    fn start_zoom_in_transition(&mut self, code: String) {
        if !self.config.display.animations {
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::ZoomIn { code },
            started: Instant::now(),
            duration: SKY_TRANSITION_DURATION,
        });
    }

    fn start_zoom_out_transition(&mut self, code: String) {
        if !self.config.display.animations {
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::ZoomOut { code },
            started: Instant::now(),
            duration: SKY_TRANSITION_DURATION,
        });
    }

    fn start_city_zoom_out_transition(&mut self, from: Location, to: Location, code: String) {
        if !self.config.display.animations {
            self.sky_transition = None;
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::CityZoomOut { from, to, code },
            started: Instant::now(),
            duration: SKY_TRANSITION_DURATION,
        });
    }

    fn start_time_transition(
        &mut self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        duration: StdDuration,
    ) {
        if !self.config.display.animations || from == to {
            return;
        }
        self.sky_transition = Some(SkyTransition {
            kind: SkyTransitionKind::Time { from, to },
            started: Instant::now(),
            duration,
        });
    }

    fn clear_time_transition(&mut self) {
        if matches!(
            self.sky_transition,
            Some(SkyTransition {
                kind: SkyTransitionKind::Time { .. },
                ..
            })
        ) {
            self.sky_transition = None;
        }
    }

    fn transition_progress(&self) -> Option<f64> {
        self.sky_transition
            .as_ref()
            .map(SkyTransition::eased_progress)
    }

    fn toggle_constellation_zoom(&mut self) {
        if self.constellation_zoom {
            let code = self.selected_constellation_code().map(ToString::to_string);
            self.constellation_zoom = false;
            self.constellation_zoom_code = None;
            if let Some(code) = code {
                self.start_zoom_out_transition(code);
            } else {
                self.sky_transition = None;
            }
            if self.pointer.active {
                self.update_pointer_hover();
            }
            self.message = i18n::tr(self.config.language, "constellation_zoom_off").to_string();
            return;
        }

        if self.selected_constellation_code().is_none() {
            self.cycle_visible_constellation(true);
        }

        if let Some(code) = self.selected_constellation_code().map(ToString::to_string) {
            if !self.constellation_visible_at(&code, self.active_location()) {
                self.constellation_zoom = false;
                self.constellation_zoom_code = None;
                self.config.display.side_panel = true;
                self.message =
                    self.constellation_not_visible_message(&code, self.active_location());
                return;
            }
            let meta = self.figure_meta(&code);
            self.constellation_zoom = true;
            self.constellation_zoom_code = Some(code.clone());
            self.start_zoom_in_transition(code);
            self.config.display.side_panel = true;
            if self.pointer.active {
                self.update_pointer_hover();
            }
            self.message = format!(
                "{} {}",
                constellations::display_name_for_culture(
                    self.config.display.sky_culture,
                    &meta.code
                ),
                i18n::tr(self.config.language, "constellation_zoom_on")
            );
        }
    }

    fn move_pointer(&mut self, dx: isize, dy: isize) {
        if self.pointer.width == 0 || self.pointer.height == 0 {
            return;
        }
        self.pointer.x = (self.pointer.x as isize + dx)
            .clamp(0, self.pointer.width.saturating_sub(1) as isize)
            as usize;
        self.pointer.y = (self.pointer.y as isize + dy)
            .clamp(0, self.pointer.height.saturating_sub(1) as isize)
            as usize;
        self.update_pointer_hover();
    }

    pub(crate) fn activate_pointer_at(&mut self, x: usize, y: usize, pick_radius: usize) {
        let width = self.pointer.width;
        let height = self.pointer.height;
        if width == 0 || height == 0 {
            return;
        }
        self.pointer.active = true;
        self.config.display.side_panel = true;
        self.last_pointer_move = None;
        self.pointer.x = x.min(width - 1);
        self.pointer.y = y.min(height - 1);

        let targets = self.projected_pointer_targets(width, height);
        let exact_hits = targets
            .iter()
            .filter(|target| target.x == self.pointer.x && target.y == self.pointer.y)
            .cloned()
            .collect::<Vec<_>>();
        if !exact_hits.is_empty() {
            self.apply_pointer_hits(exact_hits);
            return;
        }

        let mut nearby = targets
            .iter()
            .filter(|target| {
                target.x.abs_diff(self.pointer.x) <= pick_radius
                    && target.y.abs_diff(self.pointer.y) <= pick_radius
            })
            .collect::<Vec<_>>();
        nearby.sort_by(|a, b| {
            let a_distance =
                a.x.abs_diff(self.pointer.x).pow(2) + a.y.abs_diff(self.pointer.y).pow(2);
            let b_distance =
                b.x.abs_diff(self.pointer.x).pow(2) + b.y.abs_diff(self.pointer.y).pow(2);
            a_distance.cmp(&b_distance).then_with(|| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.sort_key.cmp(&b.sort_key))
            })
        });

        if let Some((nearest_x, nearest_y)) = nearby.first().map(|target| (target.x, target.y)) {
            self.pointer.x = nearest_x;
            self.pointer.y = nearest_y;
            let hits = targets
                .into_iter()
                .filter(|target| target.x == nearest_x && target.y == nearest_y)
                .collect::<Vec<_>>();
            self.apply_pointer_hits(hits);
        } else {
            self.apply_pointer_hits(Vec::new());
        }
    }

    fn update_pointer_hover(&mut self) {
        let width = self.pointer.width;
        let height = self.pointer.height;
        if width == 0 || height == 0 {
            self.pointer.hovered = None;
            self.pointer.hits.clear();
            return;
        }

        let hits = self
            .projected_pointer_targets(width, height)
            .into_iter()
            .filter(|target| target.x == self.pointer.x && target.y == self.pointer.y)
            .collect::<Vec<_>>();
        self.apply_pointer_hits(hits);
    }

    fn projected_pointer_targets(&self, width: usize, height: usize) -> Vec<ProjectedTarget> {
        let location = self.render_location();
        let time = self.now();
        let zoom_view = self
            .should_render_zoomed_sky()
            .then(|| self.zoom_render_view(width, height))
            .flatten();
        let visible = if let Some(view) = zoom_view.as_ref() {
            view.stars.clone()
        } else {
            astro::visible_stars_for_orientation(
                &self.catalog.stars,
                &location,
                time,
                self.config.display.limiting_magnitude,
                width,
                height,
                self.config.display.sky_orientation,
            )
        };

        let mut targets = Vec::new();
        for visible in visible {
            targets.push(ProjectedTarget {
                target: Target::Star(visible.star.hip),
                x: visible.x,
                y: visible.y,
                score: visible.star.magnitude,
                sort_key: format!("star:{:010}", visible.star.hip),
            });
        }

        if self.config.display.planets {
            for planet in planets::visible_planets(time) {
                let horizontal = astro::horizontal_position(
                    planet.ra_hours,
                    planet.dec_degrees,
                    &location,
                    time,
                );
                let projected = if let Some(view) = zoom_view.as_ref() {
                    view.project_horizontal(horizontal.altitude, horizontal.azimuth)
                } else {
                    astro::project_dome_for_orientation(
                        horizontal.altitude,
                        horizontal.azimuth,
                        width,
                        height,
                        self.config.display.sky_orientation,
                    )
                };
                if let Some((x, y)) = projected {
                    targets.push(ProjectedTarget {
                        target: Target::Planet(planet.name),
                        x,
                        y,
                        score: planet.magnitude - 0.25,
                        sort_key: format!("planet:{}", planet.name),
                    });
                }
            }
        }

        let moon = astro::moon_position(time);
        let moon_horizontal =
            astro::horizontal_position(moon.ra_hours, moon.dec_degrees, &location, time);
        let moon_projected = if let Some(view) = zoom_view.as_ref() {
            view.project_horizontal(moon_horizontal.altitude, moon_horizontal.azimuth)
        } else {
            astro::project_dome_for_orientation(
                moon_horizontal.altitude,
                moon_horizontal.azimuth,
                width,
                height,
                self.config.display.sky_orientation,
            )
        };
        if let Some((x, y)) = moon_projected {
            targets.push(ProjectedTarget {
                target: Target::Moon,
                x,
                y,
                score: -12.0,
                sort_key: "moon".to_string(),
            });
        }

        if self.config.display.deep_sky {
            for object in &self.deep_sky {
                if object.magnitude.unwrap_or(99.0) > 9.5 {
                    continue;
                }
                let horizontal = astro::horizontal_position(
                    object.ra_hours,
                    object.dec_degrees,
                    &location,
                    time,
                );
                let projected = if let Some(view) = zoom_view.as_ref() {
                    view.project_horizontal(horizontal.altitude, horizontal.azimuth)
                } else {
                    astro::project_dome_for_orientation(
                        horizontal.altitude,
                        horizontal.azimuth,
                        width,
                        height,
                        self.config.display.sky_orientation,
                    )
                };
                if let Some((x, y)) = projected {
                    targets.push(ProjectedTarget {
                        target: Target::DeepSky(object.name),
                        x,
                        y,
                        score: object.magnitude.unwrap_or(99.0) + 10.0,
                        sort_key: format!("deep:{}", object.name),
                    });
                }
            }
        }

        targets
    }

    fn apply_pointer_hits(&mut self, mut hits: Vec<ProjectedTarget>) {
        hits.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.sort_key.cmp(&b.sort_key))
        });
        hits.dedup_by(|a, b| a.target == b.target);
        self.pointer.hits = hits.into_iter().map(|target| target.target).collect();

        if let Some(target) = self.pointer.hits.first().cloned() {
            self.pointer.hovered = Some(target.clone());
            self.selected_target = Some(target.clone());
            self.message = self.pointer_target_label(&target);
        } else {
            self.pointer.hovered = None;
            self.selected_target = None;
            self.message = i18n::tr(self.config.language, "pointer_no_star").to_string();
        }
    }

    #[cfg(test)]
    pub fn stars_at_pointer(&self) -> Vec<Star> {
        self.pointer
            .hits
            .iter()
            .filter_map(|target| match target {
                Target::Star(hip) => self.star_by_hip(*hip),
                _ => None,
            })
            .collect()
    }

    pub fn targets_at_pointer(&self) -> Vec<Target> {
        self.pointer.hits.clone()
    }

    fn pointer_target_label(&self, target: &Target) -> String {
        match target {
            Target::Star(hip) => self
                .star_by_hip(*hip)
                .map(|star| star_aliases::display_name_for(star, self.config.language))
                .unwrap_or_else(|| format!("HIP {hip}")),
            Target::Planet(name) => (*name).to_string(),
            Target::Moon => i18n::tr(self.config.language, "moon").to_string(),
            Target::DeepSky(name) => self
                .deep_sky_by_name(name)
                .map(|object| {
                    if object.common.is_empty() {
                        object.name.to_string()
                    } else {
                        format!("{} · {}", object.name, object.common)
                    }
                })
                .unwrap_or_else(|| (*name).to_string()),
            Target::Constellation(code) => {
                constellations::display_name_for_culture(self.config.display.sky_culture, code)
            }
            Target::City(index) => PRESETS
                .get(*index)
                .map(|preset| preset.en.to_string())
                .unwrap_or_else(|| "city".to_string()),
        }
    }

    #[cfg(test)]
    pub fn zoomed_visible_stars_for_current_view(
        &self,
        width: usize,
        height: usize,
    ) -> Option<Vec<astro::VisibleStar>> {
        self.zoom_render_view(width, height).map(|view| view.stars)
    }

    pub(crate) fn zoom_render_view(&self, width: usize, height: usize) -> Option<ZoomRenderView> {
        if width == 0 || height == 0 {
            return None;
        }

        let source_width = width.saturating_mul(4).max(width).max(1);
        let source_height = height.saturating_mul(4).max(height).max(1);
        let location = self.render_location();
        let orientation = self.config.display.sky_orientation;
        let source_visible = astro::visible_stars_for_orientation(
            &self.catalog.stars,
            &location,
            self.now(),
            self.catalog.faintest_magnitude(),
            source_width,
            source_height,
            orientation,
        );
        let viewport = self.current_zoom_viewport(&source_visible, source_width, source_height)?;
        let mapper = SkyViewMapper::new(
            viewport,
            source_width,
            source_height,
            width,
            height,
            orientation,
        );
        Some(ZoomRenderView {
            stars: map_zoomed_visible(source_visible, mapper),
            mapper,
        })
    }

    fn current_zoom_viewport(
        &self,
        source_visible: &[astro::VisibleStar],
        source_width: usize,
        source_height: usize,
    ) -> Option<ZoomViewport> {
        match &self.sky_transition {
            Some(SkyTransition {
                kind: SkyTransitionKind::Zoom { from_code, to_code },
                ..
            }) => {
                let from_bounds = self.constellation_zoom_bounds(
                    source_visible,
                    from_code,
                    source_width,
                    source_height,
                )?;
                let to_bounds = self.constellation_zoom_bounds(
                    source_visible,
                    to_code,
                    source_width,
                    source_height,
                )?;
                Some(from_bounds.interpolate(to_bounds, self.transition_progress()?))
            }
            Some(SkyTransition {
                kind: SkyTransitionKind::ZoomIn { code },
                ..
            }) => {
                let full_bounds = ZoomBounds::full(source_width, source_height);
                let target_bounds = self.constellation_zoom_bounds(
                    source_visible,
                    code,
                    source_width,
                    source_height,
                )?;
                Some(full_bounds.interpolate(target_bounds, self.transition_progress()?))
            }
            Some(SkyTransition {
                kind: SkyTransitionKind::ZoomOut { code },
                ..
            }) => {
                let full_bounds = ZoomBounds::full(source_width, source_height);
                let target_bounds = self.constellation_zoom_bounds(
                    source_visible,
                    code,
                    source_width,
                    source_height,
                )?;
                Some(target_bounds.interpolate(full_bounds, self.transition_progress()?))
            }
            Some(SkyTransition {
                kind: SkyTransitionKind::CityZoomOut { from, code, .. },
                ..
            }) => {
                let full_bounds = ZoomBounds::full(source_width, source_height);
                let from_visible = astro::visible_stars_for_orientation(
                    &self.catalog.stars,
                    from,
                    self.now(),
                    self.catalog.faintest_magnitude(),
                    source_width,
                    source_height,
                    self.config.display.sky_orientation,
                );
                let target_bounds = self
                    .constellation_zoom_bounds(&from_visible, code, source_width, source_height)
                    .unwrap_or(full_bounds);
                Some(target_bounds.interpolate(full_bounds, self.transition_progress()?))
            }
            _ => {
                let code = self.selected_constellation_code()?;
                let target_bounds = self.constellation_zoom_bounds(
                    source_visible,
                    code,
                    source_width,
                    source_height,
                )?;
                Some(target_bounds.interpolate(target_bounds, 1.0))
            }
        }
    }

    fn constellation_zoom_bounds(
        &self,
        visible: &[astro::VisibleStar],
        code: &str,
        width: usize,
        height: usize,
    ) -> Option<ZoomBounds> {
        let points = visible
            .iter()
            .map(|star| (star.star.hip, (star.x, star.y)))
            .collect::<std::collections::HashMap<_, _>>();
        let mut coords = self
            .active_constellation_lines()
            .iter()
            .filter(|line| line.code.eq_ignore_ascii_case(code))
            .flat_map(|line| line.hips.iter())
            .filter_map(|hip| points.get(hip).copied())
            .collect::<Vec<_>>();

        if coords.len() < 2 && self.config.display.sky_culture == SkyCulture::Western {
            coords.extend(
                visible
                    .iter()
                    .filter(|star| star.star.constellation.eq_ignore_ascii_case(code))
                    .map(|star| (star.x, star.y)),
            );
        }
        if coords.is_empty() {
            return None;
        }

        let min_x = coords.iter().map(|(x, _)| *x).min()?;
        let max_x = coords.iter().map(|(x, _)| *x).max()?;
        let min_y = coords.iter().map(|(_, y)| *y).min()?;
        let max_y = coords.iter().map(|(_, y)| *y).max()?;
        Some(ZoomBounds::expanded(
            min_x, max_x, min_y, max_y, width, height,
        ))
    }

    fn line_endpoint_visible(&self, hip: u32, time: DateTime<Utc>) -> bool {
        self.line_endpoint_visible_at(hip, self.active_location(), time)
    }

    fn line_endpoint_visible_at(&self, hip: u32, location: &Location, time: DateTime<Utc>) -> bool {
        let Some(star) = self.star_by_hip(hip) else {
            return false;
        };
        if star.magnitude > self.config.display.limiting_magnitude {
            return false;
        }
        astro::horizontal_position(star.ra_hours, star.dec_degrees, location, time).altitude > 0.0
    }

    fn constellation_not_visible_message(&self, code: &str, location: &Location) -> String {
        let meta = self.figure_meta(code);
        format!(
            "{}: {} ({}) · {}",
            i18n::tr(self.config.language, "constellation_not_visible_here"),
            constellations::display_name_for_culture(self.config.display.sky_culture, meta.code),
            meta.en,
            location.name
        )
    }

    fn adjust_magnitude(&mut self, delta: f64) -> io::Result<()> {
        self.set_limiting_magnitude(self.config.display.limiting_magnitude + delta);
        self.save()
    }

    fn set_limiting_magnitude(&mut self, value: f64) {
        self.config.display.limiting_magnitude = value.clamp(
            APP_MIN_LIMITING_MAGNITUDE,
            self.catalog.faintest_magnitude(),
        );
    }

    fn save(&self) -> io::Result<()> {
        config::save_config(&self.config_path, &self.config)
    }
}

impl ZoomBounds {
    fn full(width: usize, height: usize) -> Self {
        Self {
            min_x: 0,
            max_x: width.saturating_sub(1),
            min_y: 0,
            max_y: height.saturating_sub(1),
        }
    }

    fn expanded(
        min_x: usize,
        max_x: usize,
        min_y: usize,
        max_y: usize,
        width: usize,
        height: usize,
    ) -> Self {
        let span_x = max_x.saturating_sub(min_x).max(1);
        let span_y = max_y.saturating_sub(min_y).max(1);
        let pad_x = (span_x / 3).max(8);
        let pad_y = (span_y / 3).max(4);
        Self {
            min_x: min_x.saturating_sub(pad_x),
            max_x: max_x.saturating_add(pad_x).min(width.saturating_sub(1)),
            min_y: min_y.saturating_sub(pad_y),
            max_y: max_y.saturating_add(pad_y).min(height.saturating_sub(1)),
        }
    }

    fn interpolate(self, other: Self, progress: f64) -> ZoomViewport {
        let t = progress.clamp(0.0, 1.0);
        ZoomViewport {
            min_x: lerp(self.min_x as f64, other.min_x as f64, t),
            max_x: lerp(self.max_x as f64, other.max_x as f64, t),
            min_y: lerp(self.min_y as f64, other.min_y as f64, t),
            max_y: lerp(self.max_y as f64, other.max_y as f64, t),
        }
    }
}

impl ZoomViewport {
    fn contains(self, x: usize, y: usize) -> bool {
        let x = x as f64;
        let y = y as f64;
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    fn map(self, x: usize, y: usize, width: usize, height: usize) -> (usize, usize) {
        let span_x = (self.max_x - self.min_x).max(1.0);
        let span_y = (self.max_y - self.min_y).max(1.0);
        let x =
            (((x as f64 - self.min_x) / span_x) * width.saturating_sub(1) as f64).round() as usize;
        let y =
            (((y as f64 - self.min_y) / span_y) * height.saturating_sub(1) as f64).round() as usize;
        (
            x.min(width.saturating_sub(1)),
            y.min(height.saturating_sub(1)),
        )
    }
}

impl SkyViewMapper {
    fn new(
        viewport: ZoomViewport,
        source_width: usize,
        source_height: usize,
        width: usize,
        height: usize,
        orientation: SkyOrientation,
    ) -> Self {
        Self {
            viewport,
            source_width,
            source_height,
            width,
            height,
            orientation,
        }
    }

    fn map_source(self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.viewport
            .contains(x, y)
            .then(|| self.viewport.map(x, y, self.width, self.height))
    }

    fn project_horizontal(self, altitude: f64, azimuth: f64) -> Option<(usize, usize)> {
        let (x, y) = astro::project_dome_for_orientation(
            altitude,
            azimuth,
            self.source_width,
            self.source_height,
            self.orientation,
        )?;
        self.map_source(x, y)
    }
}

impl SkyTransition {
    fn progress(&self) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.started.elapsed().as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    fn eased_progress(&self) -> f64 {
        smoothstep(self.progress())
    }

    fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }
}

impl HorizonTransition {
    fn progress(&self) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.started.elapsed().as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    fn eased_progress(&self) -> f64 {
        smoothstep(self.progress())
    }

    fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }
}

impl GroundRotationTransition {
    fn progress(&self) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.started.elapsed().as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    fn eased_progress(&self) -> f64 {
        smoothstep(self.progress())
    }

    fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }

    fn center(&self) -> (f64, f64) {
        let progress = self.eased_progress();
        let lat = lerp(self.from_lat, self.to_lat, progress);
        let lon = lerp_longitude(self.from_lon, self.to_lon, progress);
        (lat, lon)
    }
}

fn map_zoomed_visible(
    visible: Vec<astro::VisibleStar>,
    mapper: SkyViewMapper,
) -> Vec<astro::VisibleStar> {
    visible
        .into_iter()
        .filter_map(|visible| {
            let (x, y) = mapper.map_source(visible.x, visible.y)?;
            Some(astro::VisibleStar {
                star: visible.star,
                x,
                y,
            })
        })
        .collect()
}

fn lerp_location(from: &Location, to: &Location, progress: f64) -> Location {
    let t = progress.clamp(0.0, 1.0);
    Location {
        name: format!("{} -> {}", from.name, to.name),
        latitude: lerp(from.latitude, to.latitude, t),
        longitude: lerp_longitude(from.longitude, to.longitude, t),
        timezone: to.timezone.clone(),
    }
}

fn same_location(left: &Location, right: &Location) -> bool {
    (left.latitude - right.latitude).abs() < 0.0001
        && (left.longitude - right.longitude).abs() < 0.0001
        && left.timezone == right.timezone
}

fn lerp(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}

fn lerp_longitude(from: f64, to: f64, progress: f64) -> f64 {
    let delta = ((to - from + 540.0) % 360.0) - 180.0;
    normalize_longitude(from + delta * progress)
}

fn lerp_time(from: DateTime<Utc>, to: DateTime<Utc>, progress: f64) -> DateTime<Utc> {
    let t = progress.clamp(0.0, 1.0);
    let from_millis = from.timestamp_millis() as f64;
    let to_millis = to.timestamp_millis() as f64;
    let millis = lerp(from_millis, to_millis, t).round() as i64;
    DateTime::from_timestamp_millis(millis).unwrap_or(to)
}

fn map_preview_location(latitude: f64, longitude: f64) -> Location {
    Location {
        name: format!("Globe {latitude:+.1} {longitude:+.1}"),
        latitude,
        longitude,
        timezone: approximate_timezone(longitude),
    }
}

fn location_for_preset(preset: Preset) -> Location {
    Location {
        name: preset.en.to_string(),
        latitude: preset.latitude,
        longitude: preset.longitude,
        timezone: preset.timezone.to_string(),
    }
}

fn approximate_timezone(longitude: f64) -> String {
    let offset = (longitude / 15.0).round().clamp(-12.0, 12.0) as i32;
    match offset.cmp(&0) {
        std::cmp::Ordering::Equal => "UTC".to_string(),
        std::cmp::Ordering::Greater => format!("Etc/GMT-{offset}"),
        std::cmp::Ordering::Less => format!("Etc/GMT+{}", offset.abs()),
    }
}

fn normalize_longitude(value: f64) -> f64 {
    ((value + 180.0).rem_euclid(360.0)) - 180.0
}

fn smoothstep(progress: f64) -> f64 {
    let t = progress.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn city_indices() -> Vec<usize> {
    PRESETS
        .iter()
        .enumerate()
        .filter_map(|(index, preset)| (!preset.custom).then_some(index))
        .collect()
}

fn city_matches_query(preset: &Preset, query: &str) -> bool {
    let needle = query.to_lowercase();
    let compact_needle = needle.split_whitespace().collect::<String>();
    [preset.en, preset.zh, preset.timezone].iter().any(|value| {
        let haystack = value.to_lowercase();
        haystack.contains(&needle)
            || (!compact_needle.is_empty()
                && haystack
                    .split_whitespace()
                    .collect::<String>()
                    .contains(&compact_needle))
    })
}

fn city_search_results(query: &str, language: Language) -> Vec<SearchResult> {
    let query = query.trim();
    PRESETS
        .iter()
        .enumerate()
        .filter(|(_, preset)| !preset.custom)
        .filter(|(_, preset)| query.is_empty() || city_matches_query(preset, query))
        .map(|(index, preset)| {
            let label = match language {
                Language::En => preset.en.to_string(),
                Language::Zh => preset.zh.to_string(),
            };
            SearchResult {
                label,
                detail: format!(
                    "{} · {:+.4} {:+.4} · {}",
                    preset.en, preset.latitude, preset.longitude, preset.timezone
                ),
                target: Target::City(index),
            }
        })
        .collect()
}

fn preset_display_name(preset: Preset, language: Language) -> &'static str {
    match language {
        Language::En => preset.en,
        Language::Zh => preset.zh,
    }
}

fn current_preset_index(location: &Location) -> Option<usize> {
    PRESETS.iter().position(|preset| {
        !preset.custom
            && (preset.latitude - location.latitude).abs() < 0.0001
            && (preset.longitude - location.longitude).abs() < 0.0001
    })
}

fn next_position(current: Option<usize>, len: usize, forward: bool) -> usize {
    match (current, forward) {
        (Some(position), true) => (position + 1) % len,
        (Some(0), false) => len - 1,
        (Some(position), false) => position - 1,
        (None, true) => 0,
        (None, false) => len - 1,
    }
}

pub fn settings_count() -> usize {
    14
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DisplayConfig, LandscapeMode, SkyCulture, SkyOrientation, Theme};
    use chrono::TimeZone;

    fn test_app(location: Location) -> App {
        App::new(
            Config {
                version: config::CONFIG_VERSION,
                language: Language::En,
                location,
                display: DisplayConfig {
                    limiting_magnitude: 5.8,
                    labels: true,
                    moon_panel: true,
                    constellations: true,
                    theme: Default::default(),
                    charset: Default::default(),
                    animations: true,
                    planets: true,
                    deep_sky: true,
                    side_panel: true,
                    landscape: LandscapeMode::Horizon,
                    sky_orientation: SkyOrientation::Observer,
                    sky_culture: SkyCulture::Western,
                },
            },
            PathBuf::from("/tmp/termarium-test-city-config.json"),
            Catalog { stars: Vec::new() },
            false,
            true,
            Some(Utc::now()),
        )
    }

    fn preset_location(preset: Preset) -> Location {
        Location {
            name: preset.en.to_string(),
            latitude: preset.latitude,
            longitude: preset.longitude,
            timezone: preset.timezone.to_string(),
        }
    }

    #[test]
    fn sky_arrows_cycle_city_presets() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.cycle_city(true).unwrap();
        assert_eq!(app.config.location.name, "Beijing");
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::City { .. },
                ..
            })
        ));
        assert!(app.render_location().name.contains("Shanghai -> Beijing"));
        app.cycle_city(false).unwrap();
        assert_eq!(app.config.location.name, "Shanghai");
    }

    #[test]
    fn city_presets_have_valid_timezones() {
        assert!(PRESETS.iter().filter(|preset| !preset.custom).count() >= 40);
        for preset in PRESETS {
            assert!(
                preset.timezone.parse::<chrono_tz::Tz>().is_ok(),
                "{} has invalid timezone {}",
                preset.en,
                preset.timezone
            );
        }
    }

    #[test]
    fn sky_arrows_leave_custom_location_for_cycle_edge() {
        let mut app = test_app(Location {
            name: "Custom".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            timezone: "UTC".to_string(),
        });
        app.cycle_city(true).unwrap();
        assert_eq!(app.config.location.name, "Shanghai");
        app.config.location = Location {
            name: "Custom".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            timezone: "UTC".to_string(),
        };
        app.cycle_city(false).unwrap();
        let last_city = PRESETS.iter().rev().find(|preset| !preset.custom).unwrap();
        assert_eq!(app.config.location.name, last_city.en);
    }

    #[test]
    fn starts_in_sky_view() {
        let app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        assert_eq!(app.view_mode, ViewMode::Sky);
        assert!(app.horizon_transition().is_none());
    }

    #[test]
    fn first_start_opens_ground_city_search() {
        let app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-first-ground-search-config.json"),
            Catalog { stars: Vec::new() },
            true,
            false,
            Some(Utc::now()),
        );

        assert_eq!(app.view_mode, ViewMode::Ground);
        assert_eq!(app.screen, Screen::Search);
        assert_eq!(app.search.mode, SearchMode::City);
        assert!(!app.search.results.is_empty());
        assert!(!app.can_cancel_setup);
    }

    #[test]
    fn ground_toggle_starts_and_finishes_horizon_transition() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::Ground);
        assert!(matches!(
            app.horizon_transition(),
            Some((ViewMode::Sky, ViewMode::Ground, _))
        ));
        if let Some(transition) = app.horizon_transition.as_mut() {
            transition.started = Instant::now() - HORIZON_TRANSITION_DURATION;
        }
        app.tick();
        assert!(app.horizon_transition().is_none());

        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        assert_eq!(app.view_mode, ViewMode::Sky);
        assert!(matches!(
            app.horizon_transition(),
            Some((ViewMode::Ground, ViewMode::Sky, _))
        ));
    }

    #[test]
    fn ground_toggle_from_zoomed_sky_zooms_out_before_horizon_transition() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-zoom-to-ground-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );

        app.handle_key(KeyEvent::from(KeyCode::Char('z'))).unwrap();
        let code = app.selected_constellation_code().unwrap().to_string();
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();

        assert_eq!(app.view_mode, ViewMode::Sky);
        assert!(!app.constellation_zoom);
        assert!(app.pending_ground_transition);
        assert!(app.horizon_transition().is_none());
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::ZoomOut {
                    code: transition_code,
                },
                ..
            }) if transition_code == &code
        ));
        assert!(app.should_render_zoomed_sky());

        if let Some(transition) = app.sky_transition.as_mut() {
            transition.started = Instant::now() - SKY_TRANSITION_DURATION;
        }
        app.tick();

        assert_eq!(app.view_mode, ViewMode::Ground);
        assert!(!app.pending_ground_transition);
        assert!(app.sky_transition.is_none());
        assert!(matches!(
            app.horizon_transition(),
            Some((ViewMode::Sky, ViewMode::Ground, _))
        ));
    }

    #[test]
    fn ground_arrows_move_preview_without_saving_config_location() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Right)).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Up)).unwrap();
        assert_eq!(app.config.location.name, "Shanghai");
        assert_eq!(app.config.location.timezone, "Asia/Shanghai");
        assert_ne!(
            app.ground.preview_location.longitude,
            app.config.location.longitude
        );
        assert_ne!(
            app.ground.preview_location.latitude,
            app.config.location.latitude
        );
    }

    #[test]
    fn ground_city_search_jumps_preview_without_saving_existing_config() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });

        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('/'))).unwrap();
        for ch in "lon".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch))).unwrap();
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();

        assert_eq!(app.view_mode, ViewMode::Ground);
        assert_eq!(app.screen, Screen::Sky);
        assert_eq!(app.ground.preview_location.name, "London");
        assert_eq!(app.ground.preview_location.timezone, "Europe/London");
        assert_eq!(app.config.location.name, "Shanghai");
        assert_eq!(
            app.session_location
                .as_ref()
                .map(|location| location.name.as_str()),
            Some("London")
        );
    }

    #[test]
    fn first_start_city_search_saves_selected_city() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-first-city-save-config.json"),
            Catalog { stars: Vec::new() },
            true,
            false,
            Some(Utc::now()),
        );

        for ch in "lon".chars() {
            app.handle_key(KeyEvent::from(KeyCode::Char(ch))).unwrap();
        }
        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();

        assert_eq!(app.view_mode, ViewMode::Ground);
        assert_eq!(app.screen, Screen::Sky);
        assert_eq!(app.config.location.name, "London");
        assert!(app.session_location.is_none());
        assert!(app.can_cancel_setup);
    }

    #[test]
    fn map_preview_timezones_parse() {
        for longitude in [-179.0, -75.0, 0.0, 121.0, 179.0] {
            let timezone = approximate_timezone(longitude);
            assert!(
                timezone.parse::<chrono_tz::Tz>().is_ok(),
                "{timezone} should parse"
            );
        }
    }

    #[test]
    fn setup_preset_field_searches_and_saves_city() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });

        app.handle_key(KeyEvent::from(KeyCode::Char('s'))).unwrap();
        assert_eq!(app.screen, Screen::Setup);
        app.handle_key(KeyEvent::from(KeyCode::Down)).unwrap();
        assert_eq!(app.setup.field, 1);
        app.handle_key(KeyEvent::from(KeyCode::Up)).unwrap();
        assert_eq!(app.setup.field, 0);
        app.handle_key(KeyEvent::from(KeyCode::Char('l'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('o'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('n'))).unwrap();
        assert_eq!(app.setup.preset_label(Language::En), "London");
        assert_eq!(app.setup.name, "London");
        assert_eq!(app.setup.timezone, "Europe/London");
        assert_eq!(app.setup.preset_query, "lon");
        app.handle_key(KeyEvent::from(KeyCode::Backspace)).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Backspace)).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Backspace)).unwrap();
        assert!(app.setup.preset_query.is_empty());
        assert_eq!(app.setup.preset_label(Language::En), "London");
        app.handle_key(KeyEvent::from(KeyCode::Char('l'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('o'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('n'))).unwrap();

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.setup.field, 1);
        assert!(app.setup.preset_query.is_empty());

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.screen, Screen::Sky);
        assert_eq!(app.config.location.name, "London");
        assert_eq!(app.config.location.timezone, "Europe/London");
        assert_eq!(app.opening_ticks, 0);
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::City { .. },
                ..
            })
        ));
    }

    #[test]
    fn setup_preset_search_matches_chinese_city_names() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });

        app.screen = Screen::Setup;
        app.handle_key(KeyEvent::from(KeyCode::Char('东'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('京'))).unwrap();
        assert_eq!(app.setup.preset_label(Language::En), "Tokyo");
        assert_eq!(app.setup.name, "Tokyo");
    }

    #[test]
    fn city_switch_pulls_back_when_zoom_target_is_hidden() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-hidden-zoom-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap()),
        );

        let mut scenario = None;
        'outer: for (from_index, from_preset) in PRESETS.iter().copied().enumerate() {
            if from_preset.custom {
                continue;
            }
            app.config.location = preset_location(from_preset);
            for code in app.visible_constellation_codes() {
                for (to_index, to_preset) in PRESETS.iter().copied().enumerate() {
                    if to_preset.custom || from_index == to_index {
                        continue;
                    }
                    let to_location = preset_location(to_preset);
                    if !app.constellation_visible_at(&code, &to_location) {
                        scenario = Some((from_index, to_index, code));
                        break 'outer;
                    }
                }
            }
        }
        let (from_index, to_index, code) = scenario.expect("need a constellation hidden elsewhere");
        app.config.location = preset_location(PRESETS[from_index]);
        app.selected_target = Some(Target::Constellation(code.clone()));
        app.constellation_zoom = true;
        app.constellation_zoom_code = Some(code.clone());

        app.apply_preset(to_index, false).unwrap();

        assert!(!app.constellation_zoom);
        assert!(app.constellation_zoom_code.is_none());
        assert_eq!(
            app.selected_target,
            Some(Target::Constellation(code.clone()))
        );
        assert!(app.message.contains(i18n::tr(
            app.config.language,
            "constellation_not_visible_here"
        )));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::CityZoomOut {
                    code: transition_code,
                    ..
                },
                ..
            }) if transition_code == &code
        ));
        assert!(app.should_render_zoomed_sky());
    }

    #[test]
    fn time_shift_and_pause_are_stable() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;

        app.handle_key(KeyEvent::from(KeyCode::Char(']'))).unwrap();

        assert_eq!(app.time_base, base + Duration::hours(1));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::Time { from, to },
                duration,
                ..
            }) if *from == base && *to == base + Duration::hours(1) && *duration == SKY_TRANSITION_DURATION
        ));
        app.sky_transition = None;
        assert_eq!(app.now(), base + Duration::hours(1));
        app.toggle_pause();
        assert!(!app.paused);
        assert!((Utc::now() - app.now()).num_seconds().abs() < 2);
    }

    #[test]
    fn repeated_hour_shift_is_immediate_and_uses_committed_target() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;

        app.handle_key(KeyEvent::from(KeyCode::Char(']'))).unwrap();
        app.handle_key(KeyEvent::new_with_kind(
            KeyCode::Char(']'),
            crossterm::event::KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ))
        .unwrap();

        assert_eq!(app.time_base, base + Duration::hours(2));
        assert!(app.sky_transition.is_none());
        assert_eq!(app.now(), base + Duration::hours(2));
    }

    #[test]
    fn day_shift_uses_full_animation_for_single_press() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;

        app.handle_key(KeyEvent::from(KeyCode::Char('}'))).unwrap();

        assert_eq!(app.time_base, base + Duration::days(1));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::Time { from, to },
                duration,
                ..
            }) if *from == base && *to == base + Duration::days(1) && *duration == SKY_TRANSITION_DURATION
        ));
    }

    #[test]
    fn repeated_day_shift_uses_fast_animation_and_committed_target() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;

        app.handle_key(KeyEvent::from(KeyCode::Char('}'))).unwrap();
        app.handle_key(KeyEvent::new_with_kind(
            KeyCode::Char('}'),
            crossterm::event::KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ))
        .unwrap();

        assert_eq!(app.time_base, base + Duration::days(2));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::Time { to, .. },
                duration,
                ..
            }) if *to == base + Duration::days(2) && *duration == TIME_REPEAT_TRANSITION_DURATION
        ));
    }

    #[test]
    fn close_day_shift_press_is_treated_as_fast_repeat() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;

        app.handle_key(KeyEvent::from(KeyCode::Char('{'))).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Char('{'))).unwrap();

        assert_eq!(app.time_base, base - Duration::days(2));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::Time { to, .. },
                duration,
                ..
            }) if *to == base - Duration::days(2) && *duration == TIME_REPEAT_TRANSITION_DURATION
        ));
    }

    #[test]
    fn time_shift_does_not_animate_when_animations_are_off() {
        let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;
        app.config.display.animations = false;

        app.handle_key(KeyEvent::from(KeyCode::Char(']'))).unwrap();
        assert_eq!(app.time_base, base + Duration::hours(1));
        assert!(app.sky_transition.is_none());

        app.handle_key(KeyEvent::from(KeyCode::Char('}'))).unwrap();
        assert_eq!(app.time_base, base + Duration::hours(1) + Duration::days(1));
        assert!(app.sky_transition.is_none());
    }

    #[test]
    fn esc_clears_selected_target_before_quitting() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.selected_target = Some(Target::Constellation("Ori".to_string()));
        app.handle_key(KeyEvent::from(KeyCode::Esc)).unwrap();
        assert!(app.selected_target.is_none());
        assert!(!app.should_quit);
        app.handle_key(KeyEvent::from(KeyCode::Esc)).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn help_closes_or_passes_shortcuts_through() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.help = true;
        app.handle_key(KeyEvent::from(KeyCode::Char('q'))).unwrap();
        assert!(!app.help);
        assert!(!app.should_quit);

        app.help = true;
        assert!(app.config.display.constellations);
        app.handle_key(KeyEvent::from(KeyCode::Char('c'))).unwrap();
        assert!(!app.help);
        assert!(!app.config.display.constellations);
    }

    #[test]
    fn landscape_setting_cycles_modes() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.screen = Screen::Settings;
        app.settings.selected = 11;

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.landscape, LandscapeMode::Bearings);

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.landscape, LandscapeMode::Off);
    }

    #[test]
    fn sky_culture_setting_cycles_modes() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.screen = Screen::Settings;
        app.settings.selected = 9;

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.sky_culture, SkyCulture::Chinese);

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.sky_culture, SkyCulture::Western);
    }

    #[test]
    fn magnitude_adjustment_uses_catalog_faintest_star_as_ceiling() {
        let catalog = Catalog {
            stars: vec![
                Star {
                    hip: 1,
                    proper: "",
                    ra_hours: 0.0,
                    dec_degrees: 0.0,
                    magnitude: 4.0,
                    color_index: None,
                    constellation: "",
                },
                Star {
                    hip: 2,
                    proper: "",
                    ra_hours: 1.0,
                    dec_degrees: 1.0,
                    magnitude: 8.7,
                    color_index: None,
                    constellation: "",
                },
            ],
        };
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-magnitude-ceiling-config.json"),
            catalog,
            false,
            true,
            Some(Utc::now()),
        );

        app.adjust_magnitude(20.0).unwrap();

        assert_eq!(app.config.display.limiting_magnitude, 8.7);
    }

    #[test]
    fn sky_culture_shortcut_cycles_modes_without_footer_setting() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.selected_target = Some(Target::Constellation("Ori".to_string()));
        app.constellation_zoom = true;
        app.constellation_zoom_code = Some("Ori".to_string());

        app.handle_key(KeyEvent::from(KeyCode::Char('y'))).unwrap();
        assert_eq!(app.config.display.sky_culture, SkyCulture::Chinese);
        assert_eq!(app.selected_target, None);
        assert!(!app.constellation_zoom);
        assert_eq!(app.constellation_zoom_code, None);

        app.view_mode = ViewMode::Ground;
        app.handle_key(KeyEvent::from(KeyCode::Char('y'))).unwrap();
        assert_eq!(app.config.display.sky_culture, SkyCulture::Western);
    }

    #[test]
    fn chinese_sky_tab_and_zoom_use_asterisms() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-chinese-tab-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.config.display.sky_culture = SkyCulture::Chinese;

        app.handle_key(KeyEvent::from(KeyCode::Tab)).unwrap();
        let selected = app.selected_constellation_code().unwrap().to_string();
        assert!(selected.starts_with("CN"));

        app.handle_key(KeyEvent::from(KeyCode::Char('z'))).unwrap();
        assert!(app.constellation_zoom);
        assert_eq!(
            app.constellation_zoom_code.as_deref(),
            Some(selected.as_str())
        );
    }

    #[test]
    fn sky_orientation_setting_cycles_modes() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.screen = Screen::Settings;
        app.settings.selected = 12;

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.sky_orientation, SkyOrientation::Map);

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();
        assert_eq!(app.config.display.sky_orientation, SkyOrientation::Observer);
    }

    #[test]
    fn theme_shortcut_opens_picker_without_cycling_theme() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        assert_eq!(app.config.display.theme, Theme::Midnight);

        app.handle_key(KeyEvent::from(KeyCode::Char('T'))).unwrap();

        assert_eq!(app.screen, Screen::Settings);
        assert_eq!(app.config.display.theme, Theme::Midnight);
        assert!(app.settings.theme_picker);
        assert_eq!(app.settings.selected, 1);
    }

    #[test]
    fn theme_setting_opens_picker_and_applies_selected_theme() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.screen = Screen::Settings;
        app.settings.selected = 1;

        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();

        assert!(app.settings.theme_picker);
        assert_eq!(app.config.display.theme, Theme::Midnight);

        app.handle_key(KeyEvent::from(KeyCode::Down)).unwrap();
        app.handle_key(KeyEvent::from(KeyCode::Enter)).unwrap();

        assert_eq!(app.config.display.theme, Theme::Aurora);
        assert!(!app.settings.theme_picker);
        assert_eq!(app.screen, Screen::Settings);
    }

    #[test]
    fn constellation_zoom_toggles_and_survives_tab_cycle() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-constellation-zoom-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.handle_key(KeyEvent::from(KeyCode::Char('z'))).unwrap();
        assert!(app.constellation_zoom);
        let first = app.selected_constellation_code().unwrap().to_string();
        assert_eq!(app.constellation_zoom_code.as_deref(), Some(first.as_str()));
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::ZoomIn { code },
                ..
            }) if code == &first
        ));
        assert!(app.should_render_zoomed_sky());
        assert!(
            app.zoomed_visible_stars_for_current_view(100, 28)
                .unwrap()
                .iter()
                .all(|visible| visible.x < 100 && visible.y < 28)
        );

        app.handle_key(KeyEvent::from(KeyCode::Tab)).unwrap();
        assert!(app.constellation_zoom);
        assert_ne!(app.selected_constellation_code(), Some(first.as_str()));
        assert_eq!(
            app.constellation_zoom_code.as_deref(),
            app.selected_constellation_code()
        );
        let (from_code, to_code, progress) = app.zoom_transition().unwrap();
        assert_eq!(from_code, first);
        assert_eq!(Some(to_code), app.constellation_zoom_code.as_deref());
        assert!((0.0..=1.0).contains(&progress));
        assert!(
            app.zoomed_visible_stars_for_current_view(100, 28)
                .unwrap()
                .iter()
                .all(|visible| visible.x < 100 && visible.y < 28)
        );

        app.handle_key(KeyEvent::from(KeyCode::Esc)).unwrap();
        assert!(!app.constellation_zoom);
        assert!(app.constellation_zoom_code.is_none());
        assert!(app.selected_constellation_code().is_some());
        assert!(matches!(
            app.sky_transition.as_ref(),
            Some(SkyTransition {
                kind: SkyTransitionKind::ZoomOut { .. },
                ..
            })
        ));
        assert!(app.should_render_zoomed_sky());
    }

    #[test]
    fn q_exits_constellation_zoom_before_quitting() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-q-zoom-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.handle_key(KeyEvent::from(KeyCode::Char('z'))).unwrap();
        assert!(app.constellation_zoom);

        app.handle_key(KeyEvent::from(KeyCode::Char('q'))).unwrap();
        assert!(!app.constellation_zoom);
        assert!(!app.should_quit);

        app.sky_transition = None;
        app.handle_key(KeyEvent::from(KeyCode::Char('q'))).unwrap();
        assert!(app.should_quit);
    }

    #[test]
    fn zoom_pointer_selects_star_without_leaving_zoom() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-zoom-pointer-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.handle_key(KeyEvent::from(KeyCode::Char('z'))).unwrap();
        let code = app.selected_constellation_code().unwrap().to_string();
        if let Some(transition) = app.sky_transition.as_mut() {
            transition.started = Instant::now() - SKY_TRANSITION_DURATION;
        }
        app.set_pointer_canvas(100, 28);
        let target = app
            .zoomed_visible_stars_for_current_view(app.pointer.width, app.pointer.height)
            .unwrap()
            .into_iter()
            .find(|visible| visible.star.constellation.eq_ignore_ascii_case(&code))
            .unwrap();
        app.pointer.active = true;
        app.pointer.x = target.x;
        app.pointer.y = target.y;
        app.update_pointer_hover();

        assert!(app.constellation_zoom);
        assert_eq!(app.constellation_zoom_code.as_deref(), Some(code.as_str()));
        assert_eq!(app.selected_target, Some(Target::Star(target.star.hip)));
        assert_eq!(app.selected_constellation_code(), Some(code.as_str()));
    }

    #[test]
    fn search_finds_vega_and_orion() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-search-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.search.query = "Vega".to_string();
        app.refresh_search();
        assert!(
            app.search
                .results
                .iter()
                .any(|result| result.label == "Vega")
        );
        app.search.query = "猎户".to_string();
        app.refresh_search();
        assert!(
            app.search
                .results
                .iter()
                .any(|result| matches!(result.target, Target::Constellation(_)))
        );
    }

    #[test]
    fn chinese_sky_search_finds_asterism_names() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-chinese-sky-search-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.config.display.sky_culture = SkyCulture::Chinese;
        for query in ["参宿", "Shen Xiu", "Three Stars"] {
            app.search.query = query.to_string();
            app.refresh_search();
            assert!(
                app.search
                    .results
                    .iter()
                    .any(|result| result.target == Target::Constellation("CN003".to_string())),
                "{query} should find 参宿"
            );
        }
    }

    #[test]
    fn western_sky_search_does_not_return_chinese_asterisms() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-western-search-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.search.query = "参宿".to_string();
        app.refresh_search();
        assert!(
            !app.search
                .results
                .iter()
                .any(|result| result.target == Target::Constellation("CN003".to_string()))
        );
    }

    #[test]
    fn search_finds_tau_ceti_aliases() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-tau-search-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        for query in ["Tau Ceti", "τ Ceti", "天仓五", "HD 10700", "HIP 8102"] {
            app.search.query = query.to_string();
            app.refresh_search();
            assert!(
                app.search
                    .results
                    .iter()
                    .any(|result| result.target == Target::Star(8102)),
                "{query} should find Tau Ceti"
            );
        }

        app.config.language = Language::Zh;
        app.search.query = "天仓五".to_string();
        app.refresh_search();
        assert!(
            app.search
                .results
                .iter()
                .any(|result| result.label == "天仓五 / Tau Ceti"),
            "Chinese UI should show Chinese and English star names"
        );
    }

    #[test]
    fn search_finds_imported_chinese_star_names() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-imported-star-name-search-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        for query in ["王良增九", "Wang Liang Added IX"] {
            app.search.query = query.to_string();
            app.refresh_search();
            assert!(
                app.search
                    .results
                    .iter()
                    .any(|result| result.target == Target::Star(43)),
                "{query} should find HIP 43"
            );
        }
    }

    #[test]
    fn pointer_mode_moves_and_hovers_visible_star() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-pointer-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.set_pointer_canvas(80, 24);
        app.handle_key(KeyEvent::from(KeyCode::Char('x'))).unwrap();
        assert!(app.pointer.active);
        let start_x = app.pointer.x;
        app.handle_key(KeyEvent::from(KeyCode::Right)).unwrap();
        let after_press = start_x + 1;
        assert_eq!(app.pointer.x, after_press);

        app.handle_key(KeyEvent::new_with_kind(
            KeyCode::Right,
            crossterm::event::KeyModifiers::NONE,
            KeyEventKind::Repeat,
        ))
        .unwrap();
        assert_eq!(app.pointer.x, after_press + 2);

        let visible = astro::visible_stars(
            &app.catalog.stars,
            &app.config.location,
            app.now(),
            app.config.display.limiting_magnitude,
            app.pointer.width,
            app.pointer.height,
        );
        let deep_sky_cells = app
            .deep_sky
            .iter()
            .filter(|object| object.magnitude.unwrap_or(99.0) <= 9.5)
            .filter_map(|object| {
                let horizontal = astro::horizontal_position(
                    object.ra_hours,
                    object.dec_degrees,
                    &app.config.location,
                    app.now(),
                );
                astro::project_dome(
                    horizontal.altitude,
                    horizontal.azimuth,
                    app.pointer.width,
                    app.pointer.height,
                )
            })
            .collect::<Vec<_>>();
        let planet_cells = planets::visible_planets(app.now())
            .into_iter()
            .filter_map(|planet| {
                let horizontal = astro::horizontal_position(
                    planet.ra_hours,
                    planet.dec_degrees,
                    &app.config.location,
                    app.now(),
                );
                astro::project_dome(
                    horizontal.altitude,
                    horizontal.azimuth,
                    app.pointer.width,
                    app.pointer.height,
                )
            })
            .collect::<Vec<_>>();
        let target = visible
            .iter()
            .find(|candidate| {
                visible
                    .iter()
                    .filter(|other| other.x == candidate.x && other.y == candidate.y)
                    .count()
                    == 1
                    && !deep_sky_cells
                        .iter()
                        .any(|(x, y)| *x == candidate.x && *y == candidate.y)
                    && !planet_cells
                        .iter()
                        .any(|(x, y)| *x == candidate.x && *y == candidate.y)
            })
            .unwrap();
        app.pointer.x = target.x;
        app.pointer.y = target.y;
        app.update_pointer_hover();
        assert_eq!(app.selected_target, Some(Target::Star(target.star.hip)));
        assert_eq!(app.pointer.hovered, Some(Target::Star(target.star.hip)));
        assert!(app.pointer.hits.contains(&Target::Star(target.star.hip)));
    }

    #[test]
    fn pointer_mouse_activation_selects_visible_star() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-pointer-mouse-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.set_pointer_canvas(80, 24);
        let visible = astro::visible_stars(
            &app.catalog.stars,
            &app.config.location,
            app.now(),
            app.config.display.limiting_magnitude,
            app.pointer.width,
            app.pointer.height,
        );
        let target = visible
            .iter()
            .find(|candidate| {
                visible
                    .iter()
                    .filter(|other| other.x == candidate.x && other.y == candidate.y)
                    .count()
                    == 1
            })
            .unwrap();

        app.activate_pointer_at(target.x, target.y, 5);

        assert!(app.pointer.active);
        assert_eq!(app.pointer.hovered, Some(Target::Star(target.star.hip)));
        assert_eq!(app.selected_target, Some(Target::Star(target.star.hip)));
    }

    #[test]
    fn pointer_can_select_visible_moon() {
        let mut app = test_app(Location {
            name: "Equator".to_string(),
            latitude: 0.0,
            longitude: 0.0,
            timezone: "UTC".to_string(),
        });
        app.set_pointer_canvas(100, 28);
        app.pointer.active = true;

        let mut moon_cell = None;
        for hour in 0..24 {
            app.time_base = Utc.with_ymd_and_hms(2026, 5, 9, hour, 0, 0).unwrap();
            app.clock_base = Instant::now();
            app.paused = true;
            moon_cell = app
                .projected_pointer_targets(app.pointer.width, app.pointer.height)
                .into_iter()
                .find(|target| target.target == Target::Moon)
                .map(|target| (target.x, target.y));
            if moon_cell.is_some() {
                break;
            }
        }

        let (x, y) = moon_cell.expect("moon should be visible during at least one test hour");
        app.pointer.x = x;
        app.pointer.y = y;
        app.update_pointer_hover();

        assert_eq!(app.pointer.hovered, Some(Target::Moon));
        assert_eq!(app.selected_target, Some(Target::Moon));
        assert_eq!(app.message, "Moon");
    }

    #[test]
    fn pointer_hover_keeps_all_stars_in_same_cell() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-pointer-list-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc::now()),
        );
        app.set_pointer_canvas(24, 12);
        let visible = astro::visible_stars(
            &app.catalog.stars,
            &app.config.location,
            app.now(),
            app.config.display.limiting_magnitude,
            app.pointer.width,
            app.pointer.height,
        );
        let target = visible
            .iter()
            .find(|candidate| {
                visible
                    .iter()
                    .filter(|other| other.x == candidate.x && other.y == candidate.y)
                    .count()
                    > 1
            })
            .unwrap();
        app.pointer.x = target.x;
        app.pointer.y = target.y;
        app.update_pointer_hover();
        assert!(app.pointer.hits.len() > 1);
        assert_eq!(app.selected_target, app.pointer.hovered.clone());
        let stars = app.stars_at_pointer();
        assert_eq!(
            stars.len(),
            app.pointer
                .hits
                .iter()
                .filter(|target| matches!(target, Target::Star(_)))
                .count()
        );
        assert!(
            stars
                .windows(2)
                .all(|pair| pair[0].magnitude <= pair[1].magnitude)
        );
    }

    #[test]
    fn pointer_hover_can_select_deep_sky_objects() {
        let mut app = App::new(
            Config::default(),
            PathBuf::from("/tmp/termarium-test-pointer-deep-sky-config.json"),
            Catalog::load(),
            false,
            true,
            Some(Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap()),
        );
        app.set_pointer_canvas(120, 36);
        app.pointer.active = true;
        let location = app.render_location();
        let time = app.now();
        let visible_stars = astro::visible_stars(
            &app.catalog.stars,
            &location,
            time,
            app.config.display.limiting_magnitude,
            app.pointer.width,
            app.pointer.height,
        );
        let visible_planets = planets::visible_planets(time)
            .into_iter()
            .filter_map(|planet| {
                let horizontal = astro::horizontal_position(
                    planet.ra_hours,
                    planet.dec_degrees,
                    &location,
                    time,
                );
                astro::project_dome(
                    horizontal.altitude,
                    horizontal.azimuth,
                    app.pointer.width,
                    app.pointer.height,
                )
            })
            .collect::<Vec<_>>();
        let deep_sky_positions = app
            .deep_sky
            .iter()
            .filter(|object| object.magnitude.unwrap_or(99.0) <= 9.5)
            .filter_map(|object| {
                let horizontal = astro::horizontal_position(
                    object.ra_hours,
                    object.dec_degrees,
                    &location,
                    time,
                );
                let (x, y) = astro::project_dome(
                    horizontal.altitude,
                    horizontal.azimuth,
                    app.pointer.width,
                    app.pointer.height,
                )?;
                Some((object.name, x, y))
            })
            .collect::<Vec<_>>();
        let (name, x, y) = deep_sky_positions
            .iter()
            .find(|(_, x, y)| {
                deep_sky_positions
                    .iter()
                    .filter(|(_, other_x, other_y)| other_x == x && other_y == y)
                    .count()
                    == 1
                    && !visible_stars
                        .iter()
                        .any(|star| star.x == *x && star.y == *y)
                    && !visible_planets
                        .iter()
                        .any(|(planet_x, planet_y)| planet_x == x && planet_y == y)
            })
            .copied()
            .expect("a visible deep-sky object should have its own pointer cell");

        app.pointer.x = x;
        app.pointer.y = y;
        app.update_pointer_hover();

        assert!(app.pointer.hits.contains(&Target::DeepSky(name)));
        assert_eq!(app.selected_target, Some(Target::DeepSky(name)));
    }

    #[test]
    fn pointer_hover_empty_sky_clears_target() {
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.set_pointer_canvas(80, 24);
        app.pointer.active = true;
        app.config.display.planets = false;
        app.config.display.deep_sky = false;
        app.selected_target = Some(Target::Star(91262));
        app.update_pointer_hover();
        assert_eq!(app.pointer.hovered, None);
        assert_eq!(app.selected_target, None);
        assert_eq!(
            app.message,
            i18n::tr(app.config.language, "pointer_no_star")
        );
    }
}
