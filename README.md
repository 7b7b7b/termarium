# Termarium 穹顶终端

Termarium is a quiet terminal planetarium. It opens as a full-screen TUI,
asks for your observing location the first time, then renders a real bright-star
sky map with constellations, planets, Messier objects, time travel, and a moon
phase panel.

```bash
cargo install termarium
termarium
```

Or install the prebuilt binary through npm:

```bash
npm install -g termarium
termarium
```

The npm installer currently ships prebuilt binaries for macOS arm64,
Linux x64, and Windows x64.

## Features

- Full-screen terminal UI built with Rust, Ratatui, and Crossterm
- Real HYG v4.2 bright-star subset, bundled for offline use
- 88 western constellation figures from a bundled CC BY 4.0 line catalog
- Offline Messier deep-sky catalog derived from OpenNGC v20260501
- Time travel, city tour, alias-aware target search, constellation highlight, and tonight panel
- Crosshair pointer mode for selecting visible stars directly from the sky map
- Offline low-precision Venus, Mars, Jupiter, and Saturn positions
- Sun altitude, daylight/twilight state, sunset, and next sunrise
- First-run location setup with city presets, custom coordinates, and timezone
- RA/Dec to Alt/Az sky projection for the current observer and time
- Four themes plus ASCII/Unicode character modes
- English by default, with in-app Chinese toggle

## Usage

```bash
termarium
termarium setup
termarium config
termarium catalog-info
termarium --lat 31.2304 --lon 121.4737 --name Shanghai
termarium --lat 31.2304 --lon 121.4737 --name Shanghai --lang zh
termarium --lat 31.2304 --lon 121.4737 --name Shanghai --tz Asia/Shanghai
termarium --theme aurora --charset unicode
termarium --time 2026-05-07T14:00:00Z
```

Inside the TUI:

```text
q       quit
Esc     clear selected target; quit if nothing is selected
←/→     switch city preset
space   pause / resume
[ / ]   jump one hour
{ / }   jump one day
r       return to live sky
/       search targets
x       pointer mode; arrows move the crosshair, hover selects a star
Tab     next visible constellation
S-Tab   previous visible constellation
h       toggle tonight panel
o       settings panel
v       city tour
s       setup location
?       detailed help
```

Quick toggles still work, but they are also available in the settings panel:

```text
a       toggle animations
p       toggle planets
d       toggle deep-sky objects
T       cycle theme
u       cycle charset
t       toggle English / Chinese
m       toggle moon panel
l       toggle labels
c       toggle constellation lines
+ / -   adjust limiting magnitude
```

Search accepts proper names, curated aliases, HIP/HD-style identifiers, western
constellation names, Chinese constellation names, planets, and Messier objects.
For example, `Tau Ceti`, `τ Ceti`, `天仓五`, `HD 10700`, and `HIP 8102` all
select the same star.

## Data And License

The application code is MIT licensed. The bundled star catalog is a filtered
subset derived from the HYG Database v4.2 and remains under CC BY-SA 4.0. The
constellation-line catalog is derived from Marc van der Sluys'
ConstellationLines data and remains under CC BY 4.0. The deep-sky catalog is
derived from OpenNGC v20260501 and remains under CC BY-SA 4.0. See `NOTICE` for
attribution. Planet and solar calculations are local approximate algorithms
based on NASA/JPL and NOAA public reference formulas.

## Development

```bash
cargo fmt
cargo test
cargo run
```
