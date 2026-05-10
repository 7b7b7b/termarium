mod app;
mod astro;
mod catalog;
mod config;
mod constellations;
mod deep_sky;
mod export;
mod i18n;
mod planets;
mod solar;
mod star_aliases;
mod ui;
mod world_map;

use std::{collections::BTreeSet, error::Error};

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};

use crate::{
    app::App,
    catalog::Catalog,
    config::{Charset, Config, Language, Location, SkyCulture, Theme},
};

#[derive(Debug, Parser)]
#[command(
    name = "termarium",
    version,
    about = "A quiet terminal planetarium.",
    long_about = "Termarium opens a full-screen terminal planetarium using a real bright-star catalog."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, help = "Observer latitude in degrees.")]
    lat: Option<f64>,

    #[arg(long, help = "Observer longitude in degrees.")]
    lon: Option<f64>,

    #[arg(long, help = "Observer label shown in the TUI.")]
    name: Option<String>,

    #[arg(long, value_enum, help = "Interface language.")]
    lang: Option<Language>,

    #[arg(
        long,
        help = "Observer timezone as an IANA name, such as Asia/Shanghai."
    )]
    tz: Option<String>,

    #[arg(long, value_enum, help = "Color theme.")]
    theme: Option<Theme>,

    #[arg(long, value_enum, help = "Sky canvas character mode.")]
    charset: Option<Charset>,

    #[arg(long, help = "Freeze the sky at an RFC3339 timestamp.")]
    time: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Open the first-run city picker and location setup flow.")]
    Setup,
    #[command(about = "Print the config file path.")]
    Config,
    #[command(about = "Print bundled star catalog information.")]
    CatalogInfo,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let config_path = config::config_path()?;

    match cli.command {
        Some(Command::Config) => {
            println!("{}", config_path.display());
            return Ok(());
        }
        Some(Command::CatalogInfo) => {
            let catalog = Catalog::load();
            let constellation_lines = constellations::load();
            let chinese_lines = constellations::load_chinese();
            let deep_sky = deep_sky::load();
            let world_lines = world_map::load();
            let world_land = world_map::load_land();
            let western_line_figures = constellation_lines
                .iter()
                .map(|line| line.code)
                .collect::<BTreeSet<_>>()
                .len();
            let chinese_line_figures = chinese_lines
                .iter()
                .map(|line| line.code)
                .collect::<BTreeSet<_>>()
                .len();
            let chinese_figures = constellations::metadata_for_culture(SkyCulture::Chinese).len();
            let named = catalog
                .stars
                .iter()
                .filter(|star| !star.proper.is_empty())
                .count();
            let faintest_magnitude = catalog.faintest_magnitude();
            let hip_min = catalog.stars.iter().map(|star| star.hip).min().unwrap_or(0);
            let hip_max = catalog.stars.iter().map(|star| star.hip).max().unwrap_or(0);
            let constellations = catalog
                .stars
                .iter()
                .map(|star| star.constellation)
                .filter(|code| !code.is_empty())
                .collect::<BTreeSet<_>>()
                .len();
            println!("Termarium star catalog");
            println!("source: HYG Database v4.2");
            println!("star license: CC BY-SA 4.0");
            println!("line source: ConstellationLines");
            println!("line license: CC BY 4.0");
            println!("Chinese sky source: Stellarium Chinese sky culture");
            println!("Chinese sky line license: CC BY-SA");
            println!("Chinese star-name source: Celestial Data");
            println!("Chinese star-name license: BSD-3-Clause");
            println!("deep-sky source: OpenNGC v20260501");
            println!("deep-sky license: CC BY-SA 4.0");
            println!("world map source: Natural Earth 1:110m coastline and land");
            println!("earth texture source: NASA Blue Marble Next Generation July 2004");
            println!("world map license: public domain");
            println!("stars: {}", catalog.stars.len());
            println!("named stars: {named}");
            println!("curated star aliases: {}", star_aliases::STAR_ALIASES.len());
            println!("hip range: {hip_min}..{hip_max}");
            println!("constellations: {constellations}");
            println!("western line figures: {western_line_figures}");
            println!("western line paths: {}", constellation_lines.len());
            println!(
                "western line segments: {}",
                constellation_lines
                    .iter()
                    .map(constellations::ConstellationLine::segment_count)
                    .sum::<usize>()
            );
            println!("Chinese sky figures: {chinese_figures}");
            println!("Chinese line figures: {chinese_line_figures}");
            println!("Chinese line paths: {}", chinese_lines.len());
            println!(
                "Chinese line segments: {}",
                chinese_lines
                    .iter()
                    .map(constellations::ConstellationLine::segment_count)
                    .sum::<usize>()
            );
            println!("deep-sky objects: {}", deep_sky.len());
            println!("world map lines: {}", world_lines.len());
            println!(
                "world map points: {}",
                world_lines
                    .iter()
                    .map(|line| line.points.len())
                    .sum::<usize>()
            );
            println!("world land rings: {}", world_land.len());
            println!(
                "world land points: {}",
                world_land
                    .iter()
                    .map(|line| line.points.len())
                    .sum::<usize>()
            );
            println!("limiting magnitude ceiling: <= {faintest_magnitude:.1}");
            return Ok(());
        }
        _ => {}
    }

    let stored_config = config::load_config(&config_path)?;
    let stored_exists = stored_config.is_some();
    let mut app_config = stored_config.unwrap_or_default();
    let session_location = apply_overrides(&mut app_config, &cli)?;
    let time_override = cli.time.as_deref().map(parse_time).transpose()?;

    let force_setup = matches!(cli.command, Some(Command::Setup));
    let should_setup = force_setup || (!stored_exists && !session_location);
    let catalog = Catalog::load();
    let app = App::new(
        app_config,
        config_path,
        catalog,
        should_setup,
        stored_exists || session_location,
        time_override,
    );

    ui::run(app)?;
    Ok(())
}

fn apply_overrides(config: &mut Config, cli: &Cli) -> Result<bool, Box<dyn Error>> {
    if let Some(language) = cli.lang {
        config.language = language;
    }
    if let Some(theme) = cli.theme {
        config.display.theme = theme;
    }
    if let Some(charset) = cli.charset {
        config.display.charset = charset;
    }

    let has_location_override = cli.lat.is_some() || cli.lon.is_some();
    if has_location_override {
        let lat = cli.lat.ok_or("--lat and --lon must be supplied together")?;
        let lon = cli.lon.ok_or("--lat and --lon must be supplied together")?;
        let name = cli.name.clone().unwrap_or_else(|| "Custom".to_string());
        let timezone = cli
            .tz
            .clone()
            .unwrap_or_else(|| config.location.timezone.clone());
        config.location = Location {
            name,
            latitude: lat,
            longitude: lon,
            timezone,
        };
        config.validate()?;
        return Ok(true);
    }

    if let Some(name) = &cli.name {
        config.location.name = name.clone();
    }
    if let Some(timezone) = &cli.tz {
        config.location.timezone = timezone.clone();
        config.validate()?;
    }

    Ok(false)
}

fn parse_time(raw: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(raw).map(|time| time.with_timezone(&Utc))
}
