#[derive(Debug, Clone, Copy)]
pub struct Star {
    pub hip: u32,
    pub proper: &'static str,
    pub ra_hours: f64,
    pub dec_degrees: f64,
    pub magnitude: f64,
    pub color_index: Option<f64>,
    pub constellation: &'static str,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub stars: Vec<Star>,
}

const HYG_BRIGHT_STARS: &str = include_str!("../data/hyg_v42_bright.csv");
const DEFAULT_LIMITING_MAGNITUDE_CEILING: f64 = 6.0;

impl Catalog {
    pub fn load() -> Self {
        let stars = HYG_BRIGHT_STARS
            .lines()
            .skip(1)
            .filter_map(parse_star)
            .collect::<Vec<_>>();
        Self { stars }
    }

    pub fn faintest_magnitude(&self) -> f64 {
        self.stars
            .iter()
            .map(|star| star.magnitude)
            .filter(|magnitude| magnitude.is_finite())
            .fold(DEFAULT_LIMITING_MAGNITUDE_CEILING, f64::max)
    }
}

fn parse_star(line: &'static str) -> Option<Star> {
    let mut fields = line.split(',');
    let hip = fields.next()?.parse().ok()?;
    let proper = fields.next().unwrap_or("").trim_matches('"');
    let ra_hours = fields.next()?.parse().ok()?;
    let dec_degrees = fields.next()?.parse().ok()?;
    let magnitude = fields.next()?.parse().ok()?;
    let color_index = fields.next().and_then(|value| value.parse().ok());
    let constellation = fields.next().unwrap_or("").trim_matches('"');

    Some(Star {
        hip,
        proper,
        ra_hours,
        dec_degrees,
        magnitude,
        color_index,
        constellation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_catalog() {
        let catalog = Catalog::load();
        assert!(catalog.stars.len() > 4_000);
        assert!(catalog.stars.len() > 5_500);
        assert!(catalog.stars.iter().any(|star| star.magnitude > 6.0));
        assert!(catalog.faintest_magnitude() > 6.0);
    }

    #[test]
    fn includes_polaris() {
        let catalog = Catalog::load();
        assert!(catalog.stars.iter().any(|star| star.proper == "Polaris"));
    }
}
