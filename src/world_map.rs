#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPoint {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone)]
pub struct MapLine {
    pub points: Vec<MapPoint>,
}

const WORLD_LINES: &str = include_str!("../data/world_110m_lines.csv");
const WORLD_LAND: &str = include_str!("../data/world_110m_land.csv");

pub fn load() -> Vec<MapLine> {
    parse_dataset(WORLD_LINES)
}

pub fn load_land() -> Vec<MapLine> {
    parse_dataset(WORLD_LAND)
}

fn parse_dataset(dataset: &str) -> Vec<MapLine> {
    dataset
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(parse_line)
        .collect()
}

fn parse_line(line: &str) -> Option<MapLine> {
    let points = line.split(';').filter_map(parse_point).collect::<Vec<_>>();
    (points.len() >= 2).then_some(MapLine { points })
}

fn parse_point(raw: &str) -> Option<MapPoint> {
    let (lon, lat) = raw.split_once(',')?;
    let lon = lon.parse::<f64>().ok()?;
    let lat = lat.parse::<f64>().ok()?;
    if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
        return None;
    }
    Some(MapPoint { lon, lat })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_world_lines() {
        let lines = load();
        assert!(lines.len() > 100);
        assert!(lines.iter().map(|line| line.points.len()).sum::<usize>() > 5_000);
    }

    #[test]
    fn parses_bundled_land_rings() {
        let rings = load_land();
        assert!(rings.len() > 100);
        assert!(rings.iter().map(|ring| ring.points.len()).sum::<usize>() > 5_000);
    }

    #[test]
    fn bundled_points_are_valid() {
        for line in load().into_iter().chain(load_land()) {
            for point in line.points {
                assert!((-180.0..=180.0).contains(&point.lon));
                assert!((-90.0..=90.0).contains(&point.lat));
            }
        }
    }

    #[test]
    fn bundled_lines_do_not_wrap_across_dateline() {
        for line in load() {
            for pair in line.points.windows(2) {
                let delta = (pair[1].lon - pair[0].lon).abs();
                assert!(delta <= 180.0, "dateline jump: {delta}");
            }
        }
    }
}
