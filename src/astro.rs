use chrono::{DateTime, Utc};

use crate::{
    catalog::Star,
    config::{Location, SkyOrientation},
};

const SYNODIC_MONTH_DAYS: f64 = 29.530_588_853;
const KNOWN_NEW_MOON_JD: f64 = 2_451_550.259_72;

#[derive(Debug, Clone, Copy)]
pub struct HorizontalPosition {
    pub altitude: f64,
    pub azimuth: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct VisibleStar {
    pub star: Star,
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct MoonPhase {
    pub age_days: f64,
    pub illumination: f64,
    pub phase_fraction: f64,
}

pub fn visible_stars(
    stars: &[Star],
    location: &Location,
    time: DateTime<Utc>,
    limiting_magnitude: f64,
    width: usize,
    height: usize,
) -> Vec<VisibleStar> {
    visible_stars_for_orientation(
        stars,
        location,
        time,
        limiting_magnitude,
        width,
        height,
        SkyOrientation::Observer,
    )
}

pub fn visible_stars_for_orientation(
    stars: &[Star],
    location: &Location,
    time: DateTime<Utc>,
    limiting_magnitude: f64,
    width: usize,
    height: usize,
    orientation: SkyOrientation,
) -> Vec<VisibleStar> {
    let mut visible = stars
        .iter()
        .copied()
        .filter(|star| star.magnitude <= limiting_magnitude)
        .filter_map(|star| {
            let horizontal = horizontal_position(star.ra_hours, star.dec_degrees, location, time);
            let (x, y) = project_dome_for_orientation(
                horizontal.altitude,
                horizontal.azimuth,
                width,
                height,
                orientation,
            )?;
            Some(VisibleStar { star, x, y })
        })
        .collect::<Vec<_>>();
    visible.sort_by(|a, b| {
        a.star
            .magnitude
            .partial_cmp(&b.star.magnitude)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    visible
}

pub fn horizontal_position(
    ra_hours: f64,
    dec_degrees: f64,
    location: &Location,
    time: DateTime<Utc>,
) -> HorizontalPosition {
    let lst_hours = local_sidereal_time_hours(time, location.longitude);
    let mut hour_angle = normalize_degrees((lst_hours - ra_hours) * 15.0);
    if hour_angle > 180.0 {
        hour_angle -= 360.0;
    }

    let ha = hour_angle.to_radians();
    let dec = dec_degrees.to_radians();
    let lat = location.latitude.to_radians();

    let sin_alt = dec.sin() * lat.sin() + dec.cos() * lat.cos() * ha.cos();
    let altitude = sin_alt.clamp(-1.0, 1.0).asin();
    let azimuth = (-ha.sin()).atan2(dec.tan() * lat.cos() - lat.sin() * ha.cos());

    HorizontalPosition {
        altitude: altitude.to_degrees(),
        azimuth: normalize_degrees(azimuth.to_degrees()),
    }
}

pub fn project_dome(
    altitude: f64,
    azimuth: f64,
    width: usize,
    height: usize,
) -> Option<(usize, usize)> {
    project_dome_for_orientation(altitude, azimuth, width, height, SkyOrientation::Observer)
}

pub fn project_dome_for_orientation(
    altitude: f64,
    azimuth: f64,
    width: usize,
    height: usize,
    orientation: SkyOrientation,
) -> Option<(usize, usize)> {
    if altitude < 0.0 || width == 0 || height == 0 {
        return None;
    }

    let radius = ((90.0 - altitude) / 90.0).clamp(0.0, 1.0);
    let center_x = (width.saturating_sub(1)) as f64 / 2.0;
    let center_y = (height.saturating_sub(1)) as f64 / 2.0;
    let x_radius = center_x.max(1.0);
    let y_radius = center_y.max(1.0);
    let az = azimuth.to_radians();
    let east_sign = match orientation {
        SkyOrientation::Observer => -1.0,
        SkyOrientation::Map => 1.0,
    };
    let x = center_x + east_sign * az.sin() * radius * x_radius;
    let y = center_y - az.cos() * radius * y_radius;

    if !x.is_finite() || !y.is_finite() {
        return None;
    }

    let x = x.round().clamp(0.0, width.saturating_sub(1) as f64) as usize;
    let y = y.round().clamp(0.0, height.saturating_sub(1) as f64) as usize;
    Some((x, y))
}

pub fn julian_day(time: DateTime<Utc>) -> f64 {
    let seconds = time.timestamp() as f64;
    let nanos = time.timestamp_subsec_nanos() as f64 / 1_000_000_000.0;
    2_440_587.5 + (seconds + nanos) / 86_400.0
}

pub fn moon_phase(time: DateTime<Utc>) -> MoonPhase {
    let days_since_new = julian_day(time) - KNOWN_NEW_MOON_JD;
    let age_days = positive_mod(days_since_new, SYNODIC_MONTH_DAYS);
    let phase_fraction = age_days / SYNODIC_MONTH_DAYS;
    let illumination = (1.0 - (std::f64::consts::TAU * phase_fraction).cos()) / 2.0;
    MoonPhase {
        age_days,
        illumination,
        phase_fraction,
    }
}

pub fn local_sidereal_time_hours(time: DateTime<Utc>, longitude: f64) -> f64 {
    let jd = julian_day(time);
    let days_since_j2000 = jd - 2_451_545.0;
    let gmst = 18.697_374_558 + 24.065_709_824_419_08 * days_since_j2000;
    normalize_hours(gmst + longitude / 15.0)
}

fn normalize_hours(hours: f64) -> f64 {
    positive_mod(hours, 24.0)
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
    fn polaris_altitude_is_near_observer_latitude() {
        let location = Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        };
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let polaris = horizontal_position(2.530_301, 89.264_109, &location, time);
        assert!((polaris.altitude - location.latitude).abs() < 1.0);
    }

    #[test]
    fn projection_stays_inside_bounds() {
        for altitude in [0.0, 20.0, 45.0, 89.0] {
            for azimuth in [0.0, 90.0, 180.0, 270.0] {
                let (x, y) = project_dome(altitude, azimuth, 80, 24).unwrap();
                assert!(x < 80);
                assert!(y < 24);
            }
        }
    }

    #[test]
    fn projection_uses_look_up_handedness() {
        let width = 81;
        let height = 25;
        let center_x = width / 2;
        let (east_x, _) = project_dome(0.0, 90.0, width, height).unwrap();
        let (west_x, _) = project_dome(0.0, 270.0, width, height).unwrap();
        assert!(east_x < center_x, "east should appear left when looking up");
        assert!(
            west_x > center_x,
            "west should appear right when looking up"
        );
    }

    #[test]
    fn map_projection_places_east_on_right() {
        let width = 81;
        let height = 25;
        let center_x = width / 2;
        let (east_x, _) =
            project_dome_for_orientation(0.0, 90.0, width, height, SkyOrientation::Map).unwrap();
        let (west_x, _) =
            project_dome_for_orientation(0.0, 270.0, width, height, SkyOrientation::Map).unwrap();

        assert!(east_x > center_x);
        assert!(west_x < center_x);
    }

    #[test]
    fn moon_phase_values_are_normalized() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let phase = moon_phase(time);
        assert!((0.0..SYNODIC_MONTH_DAYS).contains(&phase.age_days));
        assert!((0.0..=1.0).contains(&phase.illumination));
        assert!((0.0..=1.0).contains(&phase.phase_fraction));
    }
}
