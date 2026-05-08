# Termarium 穹顶终端

Termarium is a quiet terminal planetarium. It opens as a full-screen TUI,
lets you choose your observing city from an offline globe the first time, then
renders a real bright-star sky map with constellations, planets, Messier
objects, time travel, a moon phase panel, and a horizon flip between Earth and
sky.

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
- Globe view with offline Blue Marble terrain colors, daylight shading, city labels, rotating preview, and horizon flip animation
- Optional horizon and bearing-oriented ground landscape modes
- Configurable sky orientation: observer view defaults to east on the left and west on the right, with a map-style option
- Offline low-precision Venus, Mars, Jupiter, and Saturn positions
- Sun altitude, daylight/twilight state, sunset, and next sunrise
- First-run city search on the globe, plus setup for custom coordinates and timezone
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
/       search sky targets
s       setup location; type on the preset row to search city presets
g       flip between sky and globe
o       settings panel
?       detailed help
```

In the globe view, arrow keys rotate to a temporary observing point. `/` searches
the built-in city list, and visible city names are decluttered so dense regions
stay readable. The preview is used when you flip back to the sky, but it is not
saved unless you open setup from that preview and confirm it.

Quick toggles still work, but they are also available in the settings panel:

```text
q       quit; exits constellation zoom first
Esc     clear selected target; quit if nothing is selected
←/→     switch city preset; rotate longitude in globe view
↑/↓     adjust latitude in globe view
space   pause / resume
[ / ]   jump one hour
{ / }   jump one day
r       return to live sky
x       pointer mode; arrows move the crosshair, hover selects a star
z       zoom selected constellation
Tab     next visible constellation
S-Tab   previous visible constellation
h       toggle tonight panel
v       city tour
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

Landscape mode and sky orientation are available from the settings panel.

Search accepts proper names, curated aliases, HIP/HD-style identifiers, western
constellation names, Chinese constellation names, planets, and Messier objects.
For example, `Tau Ceti`, `τ Ceti`, `天仓五`, `HD 10700`, and `HIP 8102` all
select the same star.

The first run opens the globe city search directly. The setup preset row also
accepts typing to search preset city names, Chinese names, and timezones. The
matched city fills the location fields, then the usual save row applies it.

## Data And License

The application code is MIT licensed. The bundled star catalog is a filtered
subset derived from the HYG Database v4.2 and remains under CC BY-SA 4.0. The
constellation-line catalog is derived from Marc van der Sluys'
ConstellationLines data and remains under CC BY 4.0. The deep-sky catalog is
derived from OpenNGC v20260501 and remains under CC BY-SA 4.0. See `NOTICE` for
attribution. The bundled world coastline and land data is derived from Natural
Earth 1:110m public domain vector data, and the bundled globe color texture is
downsampled from NASA Blue Marble Next Generation July 2004 topography and
bathymetry imagery. Planet and solar calculations are local approximate
algorithms based on NASA/JPL and NOAA public reference formulas.

## Development

```bash
cargo fmt
cargo test
cargo run
```
