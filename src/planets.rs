use chrono::{DateTime, Utc};

use crate::astro;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planet {
    pub name: &'static str,
    pub symbol: char,
    pub ra_hours: f64,
    pub dec_degrees: f64,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Copy)]
struct Elements {
    name: &'static str,
    symbol: char,
    magnitude: f64,
    a: (f64, f64),
    e: (f64, f64),
    i: (f64, f64),
    l: (f64, f64),
    long_peri: (f64, f64),
    long_node: (f64, f64),
}

const EARTH: Elements = Elements {
    name: "Earth",
    symbol: 'E',
    magnitude: 0.0,
    a: (1.000_002_61, 0.000_005_62),
    e: (0.016_711_23, -0.000_043_92),
    i: (-0.000_015_31, -0.012_946_68),
    l: (100.464_571_66, 35_999.372_449_81),
    long_peri: (102.937_681_93, 0.323_273_64),
    long_node: (0.0, 0.0),
};

const PLANET_ELEMENTS: &[Elements] = &[
    Elements {
        name: "Venus",
        symbol: 'V',
        magnitude: -4.2,
        a: (0.723_335_66, 0.000_003_90),
        e: (0.006_776_72, -0.000_041_07),
        i: (3.394_676_05, -0.000_788_90),
        l: (181.979_099_50, 58_517.815_387_29),
        long_peri: (131.602_467_18, 0.002_683_29),
        long_node: (76.679_842_55, -0.277_694_18),
    },
    Elements {
        name: "Mars",
        symbol: 'M',
        magnitude: -1.0,
        a: (1.523_710_34, 0.000_018_47),
        e: (0.093_394_10, 0.000_078_82),
        i: (1.849_691_42, -0.008_131_31),
        l: (-4.553_432_05, 19_140.302_684_99),
        long_peri: (-23.943_629_59, 0.444_410_88),
        long_node: (49.559_538_91, -0.292_573_43),
    },
    Elements {
        name: "Jupiter",
        symbol: 'J',
        magnitude: -2.2,
        a: (5.202_887_00, -0.000_116_07),
        e: (0.048_386_24, -0.000_132_53),
        i: (1.304_396_95, -0.001_837_14),
        l: (34.396_440_51, 3_034.746_127_75),
        long_peri: (14.728_479_83, 0.212_526_68),
        long_node: (100.473_909_09, 0.204_691_06),
    },
    Elements {
        name: "Saturn",
        symbol: 'S',
        magnitude: 0.7,
        a: (9.536_675_94, -0.001_250_60),
        e: (0.053_861_79, -0.000_509_91),
        i: (2.485_991_87, 0.001_936_09),
        l: (49.954_244_23, 1_222.493_622_01),
        long_peri: (92.598_878_31, -0.418_972_16),
        long_node: (113.662_424_48, -0.288_677_94),
    },
];

pub fn visible_planets(time: DateTime<Utc>) -> Vec<Planet> {
    let earth = heliocentric_xyz(EARTH, time);
    PLANET_ELEMENTS
        .iter()
        .map(|elements| {
            let planet = heliocentric_xyz(*elements, time);
            let x = planet.0 - earth.0;
            let y = planet.1 - earth.1;
            let z = planet.2 - earth.2;
            let (ra_hours, dec_degrees) = ecliptic_to_equatorial(x, y, z, time);
            Planet {
                name: elements.name,
                symbol: elements.symbol,
                ra_hours,
                dec_degrees,
                magnitude: elements.magnitude,
            }
        })
        .collect()
}

pub fn matches_query(planet: Planet, query: &str) -> bool {
    planet
        .name
        .to_lowercase()
        .contains(&query.trim().to_lowercase())
}

fn heliocentric_xyz(elements: Elements, time: DateTime<Utc>) -> (f64, f64, f64) {
    let t = (astro::julian_day(time) - 2_451_545.0) / 36_525.0;
    let a = value_at(elements.a, t);
    let e = value_at(elements.e, t);
    let i = value_at(elements.i, t).to_radians();
    let l = normalize_degrees(value_at(elements.l, t));
    let long_peri = normalize_degrees(value_at(elements.long_peri, t));
    let long_node = normalize_degrees(value_at(elements.long_node, t));
    let arg_peri = (long_peri - long_node).to_radians();
    let mean_anomaly = normalize_degrees(l - long_peri).to_radians();
    let eccentric_anomaly = solve_kepler(mean_anomaly, e);

    let x_prime = a * (eccentric_anomaly.cos() - e);
    let y_prime = a * (1.0 - e * e).sqrt() * eccentric_anomaly.sin();
    let cos_o = long_node.to_radians().cos();
    let sin_o = long_node.to_radians().sin();
    let cos_w = arg_peri.cos();
    let sin_w = arg_peri.sin();
    let cos_i = i.cos();
    let sin_i = i.sin();

    let x = (cos_w * cos_o - sin_w * sin_o * cos_i) * x_prime
        + (-sin_w * cos_o - cos_w * sin_o * cos_i) * y_prime;
    let y = (cos_w * sin_o + sin_w * cos_o * cos_i) * x_prime
        + (-sin_w * sin_o + cos_w * cos_o * cos_i) * y_prime;
    let z = sin_w * sin_i * x_prime + cos_w * sin_i * y_prime;
    (x, y, z)
}

fn ecliptic_to_equatorial(x: f64, y: f64, z: f64, time: DateTime<Utc>) -> (f64, f64) {
    let t = (astro::julian_day(time) - 2_451_545.0) / 36_525.0;
    let obliquity = (23.439_291 - 0.013_004_2 * t).to_radians();
    let x_eq = x;
    let y_eq = y * obliquity.cos() - z * obliquity.sin();
    let z_eq = y * obliquity.sin() + z * obliquity.cos();
    let ra = y_eq.atan2(x_eq).to_degrees() / 15.0;
    let dec = z_eq.atan2((x_eq * x_eq + y_eq * y_eq).sqrt()).to_degrees();
    (positive_mod(ra, 24.0), dec)
}

fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mut eccentric_anomaly = mean_anomaly;
    for _ in 0..10 {
        eccentric_anomaly -=
            (eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - mean_anomaly)
                / (1.0 - eccentricity * eccentric_anomaly.cos());
    }
    eccentric_anomaly
}

fn value_at(pair: (f64, f64), centuries: f64) -> f64 {
    pair.0 + pair.1 * centuries
}

fn normalize_degrees(degrees: f64) -> f64 {
    positive_mod(degrees, 360.0)
}

fn positive_mod(value: f64, modulus: f64) -> f64 {
    ((value % modulus) + modulus) % modulus
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn planet_positions_are_finite() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let planets = visible_planets(time);
        assert_eq!(planets.len(), 4);
        for planet in planets {
            assert!((0.0..24.0).contains(&planet.ra_hours));
            assert!((-90.0..=90.0).contains(&planet.dec_degrees));
        }
    }
}
