use std::{fmt, fs, io, path::PathBuf, str::FromStr};

use clap::ValueEnum;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u8 = 4;
pub const MIN_LIMITING_MAGNITUDE: f64 = -2.0;
pub const MAX_CONFIG_LIMITING_MAGNITUDE: f64 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Zh,
}

impl Language {
    pub fn toggle(self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "en" => Ok(Self::En),
            "zh" => Ok(Self::Zh),
            _ => Err(format!("unsupported language: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    Midnight,
    Aurora,
    Amber,
    Dusk,
    Forest,
    Dracula,
    Nord,
    Gruvbox,
    SolarizedDark,
    TokyoNight,
    Mono,
}

impl Theme {
    pub const ALL: [Self; 11] = [
        Self::Midnight,
        Self::Aurora,
        Self::Amber,
        Self::Dusk,
        Self::Forest,
        Self::Dracula,
        Self::Nord,
        Self::Gruvbox,
        Self::SolarizedDark,
        Self::TokyoNight,
        Self::Mono,
    ];

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|theme| *theme == self)
            .expect("theme list should contain every theme")
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Midnight => "midnight",
            Self::Aurora => "aurora",
            Self::Amber => "amber",
            Self::Dusk => "dusk",
            Self::Forest => "forest",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Gruvbox => "gruvbox",
            Self::SolarizedDark => "solarized-dark",
            Self::TokyoNight => "tokyo-night",
            Self::Mono => "mono",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum Charset {
    #[default]
    Auto,
    Ascii,
    Unicode,
}

impl Charset {
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Ascii,
            Self::Ascii => Self::Unicode,
            Self::Unicode => Self::Auto,
        }
    }

    pub fn canvas_unicode(self) -> bool {
        matches!(self, Self::Unicode)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LandscapeMode {
    Off,
    #[default]
    Horizon,
    Bearings,
}

impl LandscapeMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Horizon,
            Self::Horizon => Self::Bearings,
            Self::Bearings => Self::Off,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Horizon => "horizon",
            Self::Bearings => "bearings",
        }
    }
}

impl fmt::Display for LandscapeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkyOrientation {
    #[default]
    Observer,
    Map,
}

impl SkyOrientation {
    pub fn next(self) -> Self {
        match self {
            Self::Observer => Self::Map,
            Self::Map => Self::Observer,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Observer => "observer",
            Self::Map => "map",
        }
    }
}

impl fmt::Display for SkyOrientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkyCulture {
    #[default]
    Western,
    Chinese,
}

impl SkyCulture {
    pub fn next(self) -> Self {
        match self {
            Self::Western => Self::Chinese,
            Self::Chinese => Self::Western,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Western => "western",
            Self::Chinese => "chinese",
        }
    }
}

impl fmt::Display for SkyCulture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    pub language: Language,
    pub location: Location,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub limiting_magnitude: f64,
    pub labels: bool,
    pub moon_panel: bool,
    #[serde(default = "default_constellations")]
    pub constellations: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub charset: Charset,
    #[serde(default = "default_true")]
    pub animations: bool,
    #[serde(default = "default_true")]
    pub planets: bool,
    #[serde(default = "default_true")]
    pub deep_sky: bool,
    #[serde(default = "default_true")]
    pub side_panel: bool,
    #[serde(default = "default_landscape")]
    pub landscape: LandscapeMode,
    #[serde(default = "default_sky_orientation")]
    pub sky_orientation: SkyOrientation,
    #[serde(default)]
    pub sky_culture: SkyCulture,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: Language::En,
            location: Location {
                name: "Shanghai".to_string(),
                latitude: 31.2304,
                longitude: 121.4737,
                timezone: default_timezone(),
            },
            display: DisplayConfig {
                limiting_magnitude: 5.8,
                labels: true,
                moon_panel: true,
                constellations: true,
                theme: Theme::Midnight,
                charset: Charset::Auto,
                animations: true,
                planets: true,
                deep_sky: true,
                side_panel: true,
                landscape: LandscapeMode::Horizon,
                sky_orientation: SkyOrientation::Observer,
                sky_culture: SkyCulture::Western,
            },
        }
    }
}

fn default_landscape() -> LandscapeMode {
    LandscapeMode::Horizon
}

fn default_sky_orientation() -> SkyOrientation {
    SkyOrientation::Observer
}

fn default_constellations() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_timezone() -> String {
    "Asia/Shanghai".to_string()
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.location.latitude >= -90.0 && self.location.latitude <= 90.0) {
            return Err("latitude must be between -90 and 90".to_string());
        }
        if !(self.location.longitude >= -180.0 && self.location.longitude <= 180.0) {
            return Err("longitude must be between -180 and 180".to_string());
        }
        if self.location.timezone.parse::<chrono_tz::Tz>().is_err() {
            return Err(format!("unsupported timezone: {}", self.location.timezone));
        }
        if !(self.display.limiting_magnitude >= MIN_LIMITING_MAGNITUDE
            && self.display.limiting_magnitude <= MAX_CONFIG_LIMITING_MAGNITUDE)
        {
            return Err(format!(
                "limiting magnitude must be between {MIN_LIMITING_MAGNITUDE:.1} and {MAX_CONFIG_LIMITING_MAGNITUDE:.1}"
            ));
        }
        Ok(())
    }
}

pub fn config_path() -> io::Result<PathBuf> {
    let dirs = ProjectDirs::from("dev", "termarium", "termarium").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine a platform config directory",
        )
    })?;
    Ok(dirs.config_dir().join("config.json"))
}

pub fn load_config(path: &PathBuf) -> io::Result<Option<Config>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let mut config: Config = serde_json::from_str(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if config.version != CONFIG_VERSION {
        config.version = CONFIG_VERSION;
    }
    config
        .validate()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(Some(config))
}

pub fn save_config(path: &PathBuf, config: &Config) -> io::Result<()> {
    config
        .validate()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, format!("{raw}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn config_allows_catalog_backed_faint_magnitude_limits() {
        let mut config = Config::default();
        config.display.limiting_magnitude = 10.3;
        config.validate().unwrap();
    }

    #[test]
    fn default_landscape_is_horizon() {
        assert_eq!(Config::default().display.landscape, LandscapeMode::Horizon);
    }

    #[test]
    fn default_sky_orientation_is_observer() {
        assert_eq!(
            Config::default().display.sky_orientation,
            SkyOrientation::Observer
        );
    }

    #[test]
    fn theme_indices_cover_every_theme() {
        for (index, theme) in Theme::ALL.into_iter().enumerate() {
            assert_eq!(theme.index(), index);
            assert_eq!(Theme::from_index(index), theme);
        }
        assert_eq!(Theme::from_index(usize::MAX), Theme::Mono);
    }

    #[test]
    fn default_sky_culture_is_western() {
        assert_eq!(Config::default().display.sky_culture, SkyCulture::Western);
    }

    #[test]
    fn old_config_without_landscape_orientation_or_culture_uses_new_defaults() {
        let raw = r#"{
            "version": 3,
            "language": "en",
            "location": {
                "name": "Shanghai",
                "latitude": 31.2304,
                "longitude": 121.4737,
                "timezone": "Asia/Shanghai"
            },
            "display": {
                "limiting_magnitude": 5.8,
                "labels": true,
                "moon_panel": true,
                "constellations": true,
                "theme": "midnight",
                "charset": "auto",
                "animations": true,
                "planets": true,
                "deep_sky": true,
                "side_panel": true
            }
        }"#;
        let config: Config = serde_json::from_str(raw).unwrap();
        assert_eq!(config.display.landscape, LandscapeMode::Horizon);
        assert_eq!(config.display.sky_orientation, SkyOrientation::Observer);
        assert_eq!(config.display.sky_culture, SkyCulture::Western);
    }

    #[test]
    fn rejects_invalid_latitude() {
        let mut config = Config::default();
        config.location.latitude = 91.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_invalid_longitude() {
        let mut config = Config::default();
        config.location.longitude = -181.0;
        assert!(config.validate().is_err());
    }
}
