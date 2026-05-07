use std::{
    collections::BTreeSet,
    io,
    path::PathBuf,
    time::{Duration as StdDuration, Instant},
};

use chrono::{DateTime, Duration, Utc};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{
    astro,
    catalog::{Catalog, Star},
    config::{self, Config, Language, Location},
    constellations::{self, ConstellationLine},
    deep_sky::{self, DeepSkyObject},
    i18n, planets, star_aliases,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Sky,
    Setup,
    Search,
    Settings,
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
    pub field: usize,
    pub name: String,
    pub latitude: String,
    pub longitude: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub selected: usize,
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
    DeepSky(&'static str),
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub label: String,
    pub detail: String,
    pub target: Target,
}

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub selected: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PointerState {
    pub active: bool,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub hovered: Option<u32>,
    pub hits: Vec<u32>,
}

pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    pub catalog: Catalog,
    pub constellation_lines: Vec<ConstellationLine>,
    pub deep_sky: Vec<DeepSkyObject>,
    pub screen: Screen,
    pub setup: SetupState,
    pub settings: SettingsState,
    pub search: SearchState,
    pub pointer: PointerState,
    pub selected_target: Option<Target>,
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
        let setup = SetupState::from_location(&config.location);
        let time_base = time_override.unwrap_or_else(Utc::now);
        Self {
            config,
            config_path,
            catalog,
            constellation_lines: constellations::load(),
            deep_sky: deep_sky::load(),
            screen: if start_setup {
                Screen::Setup
            } else {
                Screen::Sky
            },
            setup,
            settings: SettingsState::default(),
            search: SearchState::default(),
            pointer: PointerState::default(),
            selected_target: None,
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
        }
    }

    pub fn now(&self) -> DateTime<Utc> {
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
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> io::Result<()> {
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
        if self.pointer.active {
            match key.code {
                KeyCode::Esc => {
                    self.pointer.active = false;
                    return Ok(());
                }
                KeyCode::Left => {
                    self.move_pointer(-1, 0);
                    return Ok(());
                }
                KeyCode::Right => {
                    self.move_pointer(1, 0);
                    return Ok(());
                }
                KeyCode::Up => {
                    self.move_pointer(0, -1);
                    return Ok(());
                }
                KeyCode::Down => {
                    self.move_pointer(0, 1);
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
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.selected_target.is_some() {
                    self.selected_target = None;
                    self.message = i18n::tr(self.config.language, "selection_cleared").to_string();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('s') => {
                self.setup = SetupState::from_location(&self.config.location);
                self.screen = Screen::Setup;
                self.message.clear();
            }
            KeyCode::Char('/') => {
                self.search = SearchState::default();
                self.screen = Screen::Search;
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
            KeyCode::Char('[') => self.shift_time(Duration::hours(-1)),
            KeyCode::Char(']') => self.shift_time(Duration::hours(1)),
            KeyCode::Char('{') => self.shift_time(Duration::days(-1)),
            KeyCode::Char('}') => self.shift_time(Duration::days(1)),
            KeyCode::Char('r') => self.reset_time(),
            KeyCode::Char('t') => {
                self.config.language = self.config.language.toggle();
                self.save()?;
            }
            KeyCode::Char('T') => {
                self.config.display.theme = self.config.display.theme.next();
                self.save()?;
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
            KeyCode::Char('x') => self.toggle_pointer(),
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

    fn handle_settings_key(&mut self, key: KeyEvent) -> io::Result<()> {
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

    fn adjust_setting(&mut self, forward: bool) -> io::Result<()> {
        match self.settings.selected {
            0 => self.config.language = self.config.language.toggle(),
            1 => self.config.display.theme = self.config.display.theme.next(),
            2 => self.config.display.charset = self.config.display.charset.next(),
            3 => self.config.display.animations = !self.config.display.animations,
            4 => self.config.display.planets = !self.config.display.planets,
            5 => self.config.display.deep_sky = !self.config.display.deep_sky,
            6 => self.config.display.moon_panel = !self.config.display.moon_panel,
            7 => self.config.display.labels = !self.config.display.labels,
            8 => self.config.display.constellations = !self.config.display.constellations,
            9 => self.config.display.side_panel = !self.config.display.side_panel,
            10 => {
                let delta = if forward { 0.1 } else { -0.1 };
                self.config.display.limiting_magnitude =
                    (self.config.display.limiting_magnitude + delta).clamp(-1.5, 6.0);
            }
            _ => {}
        }
        self.save()
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Sky,
            KeyCode::Enter => {
                if let Some(result) = self.search.results.get(self.search.selected).cloned() {
                    self.selected_target = Some(result.target);
                    self.message = result.label;
                }
                self.screen = Screen::Sky;
            }
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

    fn handle_setup_key(&mut self, key: KeyEvent) -> io::Result<()> {
        match key.code {
            KeyCode::Esc => {
                if self.can_cancel_setup {
                    self.screen = Screen::Sky;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('q') if self.can_cancel_setup => self.screen = Screen::Sky,
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab | KeyCode::Down => self.setup.field = (self.setup.field + 1).min(5),
            KeyCode::BackTab | KeyCode::Up => {
                self.setup.field = self.setup.field.saturating_sub(1);
            }
            KeyCode::Left => {
                if self.setup.field == 0 {
                    self.cycle_preset(false);
                }
            }
            KeyCode::Right => {
                if self.setup.field == 0 {
                    self.cycle_preset(true);
                }
            }
            KeyCode::Enter => {
                if self.setup.field == 5 {
                    self.commit_setup()?;
                } else {
                    self.setup.field = (self.setup.field + 1).min(5);
                }
            }
            KeyCode::Backspace if matches!(self.setup.field, 1..=4) => {
                self.active_setup_input().pop();
            }
            KeyCode::Char(c) => {
                if matches!(self.setup.field, 1..=4) {
                    self.active_setup_input().push(c);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn active_setup_input(&mut self) -> &mut String {
        match self.setup.field {
            2 => &mut self.setup.latitude,
            3 => &mut self.setup.longitude,
            4 => &mut self.setup.timezone,
            _ => &mut self.setup.name,
        }
    }

    fn cycle_preset(&mut self, forward: bool) {
        let len = PRESETS.len();
        self.setup.preset_index = if forward {
            (self.setup.preset_index + 1) % len
        } else if self.setup.preset_index == 0 {
            len - 1
        } else {
            self.setup.preset_index - 1
        };

        let preset = PRESETS[self.setup.preset_index];
        if !preset.custom {
            self.setup.name = preset.en.to_string();
            self.setup.latitude = format!("{:.4}", preset.latitude);
            self.setup.longitude = format!("{:.4}", preset.longitude);
            self.setup.timezone = preset.timezone.to_string();
        }
    }

    fn commit_setup(&mut self) -> io::Result<()> {
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
        self.config.location = Location {
            name,
            latitude,
            longitude,
            timezone,
        };

        if let Err(err) = self.config.validate() {
            self.message = err;
            return Ok(());
        }

        self.save()?;
        self.can_cancel_setup = true;
        self.screen = Screen::Sky;
        self.message = i18n::tr(self.config.language, "saved").to_string();
        if self.config.display.animations {
            self.opening_ticks = 14;
        }
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
        self.config.location = Location {
            name: preset.en.to_string(),
            latitude: preset.latitude,
            longitude: preset.longitude,
            timezone: preset.timezone.to_string(),
        };
        self.setup = SetupState::from_location(&self.config.location);
        self.message = format!(
            "{}: {}",
            i18n::tr(self.config.language, "observer"),
            self.config.location.name
        );
        if persist { self.save() } else { Ok(()) }
    }

    fn toggle_pause(&mut self) {
        if self.paused {
            self.reset_time();
        } else {
            self.time_base = self.now();
            self.clock_base = Instant::now();
            self.paused = true;
            self.message = i18n::tr(self.config.language, "paused").to_string();
        }
    }

    fn shift_time(&mut self, delta: Duration) {
        self.time_base = self.now() + delta;
        self.clock_base = Instant::now();
        self.message = i18n::tr(self.config.language, "time_shifted").to_string();
    }

    fn reset_time(&mut self) {
        self.time_base = Utc::now();
        self.clock_base = Instant::now();
        self.paused = false;
        self.message = i18n::tr(self.config.language, "live").to_string();
    }

    fn toggle_tour(&mut self) {
        if let Some(origin) = self.tour_origin.take() {
            self.config.location = origin;
            self.setup = SetupState::from_location(&self.config.location);
            self.message = i18n::tr(self.config.language, "tour_off").to_string();
        } else {
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
        self.search.results = self.search_results(&self.search.query);
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

        for meta in constellations::CONSTELLATION_META {
            if constellations::matches_query(meta.code, query) {
                results.push(SearchResult {
                    label: format!("{} / {}", meta.code, meta.en),
                    detail: format!("constellation · {}", meta.zh),
                    target: Target::Constellation(meta.code.to_string()),
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
        let current = self.selected_constellation_code().and_then(|code| {
            codes
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(code))
        });
        let next = next_position(current, codes.len(), forward);
        self.selected_target = Some(Target::Constellation(codes[next].clone()));
    }

    pub fn selected_constellation_code(&self) -> Option<&str> {
        match &self.selected_target {
            Some(Target::Constellation(code)) => Some(code.as_str()),
            _ => None,
        }
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

    pub fn visible_constellation_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        let now = self.now();
        for line in &self.constellation_lines {
            if line.hips.windows(2).any(|pair| {
                self.line_endpoint_visible(pair[0], now) && self.line_endpoint_visible(pair[1], now)
            }) {
                codes.insert(line.code.to_string());
            }
        }
        codes.into_iter().collect()
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

    fn update_pointer_hover(&mut self) {
        let width = self.pointer.width;
        let height = self.pointer.height;
        if width == 0 || height == 0 {
            self.pointer.hovered = None;
            self.pointer.hits.clear();
            return;
        }

        let visible = astro::visible_stars(
            &self.catalog.stars,
            &self.config.location,
            self.now(),
            self.config.display.limiting_magnitude,
            width,
            height,
        );
        let hits = visible
            .into_iter()
            .filter(|visible| visible.x == self.pointer.x && visible.y == self.pointer.y)
            .collect::<Vec<_>>();
        let mut sorted_hits = hits
            .iter()
            .map(|visible| (visible.star.hip, visible.star.magnitude))
            .collect::<Vec<_>>();
        sorted_hits.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        self.pointer.hits = sorted_hits.into_iter().map(|(hip, _)| hip).collect();
        let nearest = hits.into_iter().min_by(|star_a, star_b| {
            star_a
                .star
                .magnitude
                .partial_cmp(&star_b.star.magnitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(visible) = nearest {
            self.pointer.hovered = Some(visible.star.hip);
            self.selected_target = Some(Target::Star(visible.star.hip));
            self.message = star_aliases::display_name_for(visible.star, self.config.language);
        } else {
            self.pointer.hovered = None;
            self.selected_target = None;
            self.message = i18n::tr(self.config.language, "pointer_no_star").to_string();
        }
    }

    pub fn stars_at_pointer(&self) -> Vec<Star> {
        self.pointer
            .hits
            .iter()
            .filter_map(|hip| self.star_by_hip(*hip))
            .collect()
    }

    fn line_endpoint_visible(&self, hip: u32, time: DateTime<Utc>) -> bool {
        let Some(star) = self.star_by_hip(hip) else {
            return false;
        };
        if star.magnitude > self.config.display.limiting_magnitude {
            return false;
        }
        astro::horizontal_position(star.ra_hours, star.dec_degrees, &self.config.location, time)
            .altitude
            > 0.0
    }

    fn adjust_magnitude(&mut self, delta: f64) -> io::Result<()> {
        self.config.display.limiting_magnitude =
            (self.config.display.limiting_magnitude + delta).clamp(-1.5, 6.0);
        self.save()
    }

    fn save(&self) -> io::Result<()> {
        config::save_config(&self.config_path, &self.config)
    }
}

fn city_indices() -> Vec<usize> {
    PRESETS
        .iter()
        .enumerate()
        .filter_map(|(index, preset)| (!preset.custom).then_some(index))
        .collect()
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
    11
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DisplayConfig;

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
                },
            },
            PathBuf::from("/tmp/termarium-test-city-config.json"),
            Catalog { stars: Vec::new() },
            false,
            true,
            Some(Utc::now()),
        )
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
    fn time_shift_and_pause_are_stable() {
        let base = Utc::now();
        let mut app = test_app(Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        });
        app.time_base = base;
        app.paused = true;
        app.shift_time(Duration::hours(1));
        assert_eq!(app.now(), base + Duration::hours(1));
        app.toggle_pause();
        assert!(!app.paused);
        assert!((Utc::now() - app.now()).num_seconds().abs() < 2);
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
        assert_eq!(app.pointer.x, start_x + 1);

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
        app.pointer.x = target.x;
        app.pointer.y = target.y;
        app.update_pointer_hover();
        assert_eq!(app.selected_target, Some(Target::Star(target.star.hip)));
        assert_eq!(app.pointer.hovered, Some(target.star.hip));
        assert_eq!(app.pointer.hits, vec![target.star.hip]);
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
        assert_eq!(app.selected_target, app.pointer.hovered.map(Target::Star));
        let stars = app.stars_at_pointer();
        assert_eq!(stars.len(), app.pointer.hits.len());
        assert!(
            stars
                .windows(2)
                .all(|pair| pair[0].magnitude <= pair[1].magnitude)
        );
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
