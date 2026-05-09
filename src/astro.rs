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

#[derive(Debug, Clone, Copy)]
pub struct MoonPosition {
    pub ra_hours: f64,
    pub dec_degrees: f64,
    pub distance_earth_radii: f64,
}

#[cfg(test)]
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

#[cfg(test)]
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

pub fn moon_position(time: DateTime<Utc>) -> MoonPosition {
    // Low-precision lunar elements from Paul Schlyter's compact formulae,
    // with the largest longitude/latitude perturbations applied. This is
    // accurate enough for a terminal sky map while staying lightweight.
    let days = julian_day(time) - 2_451_543.5;
    let node = normalize_degrees(125.1228 - 0.052_953_808_3 * days);
    let inclination = 5.1454;
    let argument_perigee = normalize_degrees(318.0634 + 0.164_357_322_3 * days);
    let eccentricity = 0.054_9;
    let mean_anomaly = normalize_degrees(115.3654 + 13.064_992_950_9 * days);

    let eccentric_anomaly = solve_kepler_degrees(mean_anomaly, eccentricity);
    let xv = eccentric_anomaly.cos() - eccentricity;
    let yv = (1.0 - eccentricity * eccentricity).sqrt() * eccentric_anomaly.sin();
    let true_anomaly = normalize_degrees(yv.atan2(xv).to_degrees());
    let mut distance_earth_radii = 60.2666 * (xv * xv + yv * yv).sqrt();

    let mut lon = normalize_degrees(true_anomaly + argument_perigee + node);
    let mut lat = asin_deg(sin_deg(true_anomaly + argument_perigee) * sin_deg(inclination));

    let sun_mean_anomaly = normalize_degrees(356.0470 + 0.985_600_258_5 * days);
    let sun_argument_perihelion = normalize_degrees(282.9404 + 0.000_047_093_5 * days);
    let sun_mean_longitude = normalize_degrees(sun_mean_anomaly + sun_argument_perihelion);
    let moon_mean_longitude = normalize_degrees(node + argument_perigee + mean_anomaly);
    let elongation = normalize_degrees(moon_mean_longitude - sun_mean_longitude);
    let argument_latitude = normalize_degrees(moon_mean_longitude - node);

    lon = normalize_degrees(
        lon - 1.274 * sin_deg(mean_anomaly - 2.0 * elongation) + 0.658 * sin_deg(2.0 * elongation)
            - 0.186 * sin_deg(sun_mean_anomaly)
            - 0.059 * sin_deg(2.0 * mean_anomaly - 2.0 * elongation)
            - 0.057 * sin_deg(mean_anomaly - 2.0 * elongation + sun_mean_anomaly)
            + 0.053 * sin_deg(mean_anomaly + 2.0 * elongation)
            + 0.046 * sin_deg(2.0 * elongation - sun_mean_anomaly)
            + 0.041 * sin_deg(mean_anomaly - sun_mean_anomaly)
            - 0.035 * sin_deg(elongation)
            - 0.031 * sin_deg(mean_anomaly + sun_mean_anomaly)
            - 0.015 * sin_deg(2.0 * argument_latitude - 2.0 * elongation)
            + 0.011 * sin_deg(mean_anomaly - 4.0 * elongation),
    );
    lat = lat
        - 0.173 * sin_deg(argument_latitude - 2.0 * elongation)
        - 0.055 * sin_deg(mean_anomaly - argument_latitude - 2.0 * elongation)
        - 0.046 * sin_deg(mean_anomaly + argument_latitude - 2.0 * elongation)
        + 0.033 * sin_deg(argument_latitude + 2.0 * elongation)
        + 0.017 * sin_deg(2.0 * mean_anomaly + argument_latitude);
    distance_earth_radii +=
        -0.58 * cos_deg(mean_anomaly - 2.0 * elongation) - 0.46 * cos_deg(2.0 * elongation);

    let obliquity = 23.4393 - 0.000_000_356_3 * days;
    let lon_rad = lon.to_radians();
    let lat_rad = lat.to_radians();
    let obliquity_rad = obliquity.to_radians();
    let x = lon_rad.cos() * lat_rad.cos();
    let y = lon_rad.sin() * lat_rad.cos();
    let z = lat_rad.sin();
    let equatorial_y = y * obliquity_rad.cos() - z * obliquity_rad.sin();
    let equatorial_z = y * obliquity_rad.sin() + z * obliquity_rad.cos();
    let ra = normalize_degrees(equatorial_y.atan2(x).to_degrees()) / 15.0;
    let dec = equatorial_z.asin().to_degrees();

    MoonPosition {
        ra_hours: ra,
        dec_degrees: dec,
        distance_earth_radii,
    }
}

pub fn moon_angular_radius_degrees(distance_earth_radii: f64) -> f64 {
    (0.2725 / distance_earth_radii.max(1.0)).asin().to_degrees()
}

pub fn angular_separation_degrees(
    ra_a_hours: f64,
    dec_a_degrees: f64,
    ra_b_hours: f64,
    dec_b_degrees: f64,
) -> f64 {
    let ra_a = (ra_a_hours * 15.0).to_radians();
    let ra_b = (ra_b_hours * 15.0).to_radians();
    let dec_a = dec_a_degrees.to_radians();
    let dec_b = dec_b_degrees.to_radians();
    let cos_sep = dec_a.sin() * dec_b.sin() + dec_a.cos() * dec_b.cos() * (ra_a - ra_b).cos();
    cos_sep.clamp(-1.0, 1.0).acos().to_degrees()
}

pub fn moonlight_limiting_magnitude_loss(
    moon_altitude: f64,
    target_altitude: f64,
    moon_target_separation: f64,
    phase: MoonPhase,
) -> f64 {
    if moon_altitude <= 0.0 || target_altitude <= 0.0 || phase.illumination <= 0.01 {
        return 0.0;
    }

    let phase_angle = (phase.phase_fraction * 360.0 - 180.0).abs();
    let lunar_brightness =
        10f64.powf(-0.4 * (3.84 + 0.026 * phase_angle + 0.000_000_004 * phase_angle.powi(4)));
    let separation = moon_target_separation.clamp(10.0, 180.0);
    let rho = separation.to_radians();
    let scattering =
        10f64.powf(5.36) * (1.06 + rho.cos().powi(2)) + 10f64.powf(6.15 - separation / 40.0);
    let extinction = 0.23;
    let moon_airmass = optical_airmass(moon_altitude);
    let target_airmass = optical_airmass(target_altitude);
    let moonlight_nl = scattering
        * lunar_brightness
        * 10f64.powf(-0.4 * extinction * moon_airmass)
        * (1.0 - 10f64.powf(-0.4 * extinction * target_airmass));
    let dark_sky_nl = 80.0;
    (1.25 * (1.0 + moonlight_nl.max(0.0) / dark_sky_nl).log10()).clamp(0.0, 3.0)
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

fn solve_kepler_degrees(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mean = mean_anomaly.to_radians();
    let mut eccentric = mean + eccentricity * mean.sin() * (1.0 + eccentricity * mean.cos());
    for _ in 0..4 {
        eccentric -= (eccentric - eccentricity * eccentric.sin() - mean)
            / (1.0 - eccentricity * eccentric.cos());
    }
    eccentric
}

fn optical_airmass(altitude: f64) -> f64 {
    let zenith = (90.0 - altitude.clamp(0.0, 90.0)).to_radians();
    (1.0 - 0.96 * zenith.sin().powi(2)).powf(-0.5)
}

fn sin_deg(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

fn cos_deg(degrees: f64) -> f64 {
    degrees.to_radians().cos()
}

fn asin_deg(value: f64) -> f64 {
    value.clamp(-1.0, 1.0).asin().to_degrees()
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

    #[test]
    fn moon_position_values_are_finite() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let moon = moon_position(time);
        assert!((0.0..24.0).contains(&moon.ra_hours));
        assert!((-90.0..=90.0).contains(&moon.dec_degrees));
        assert!((50.0..70.0).contains(&moon.distance_earth_radii));
        assert!((0.2..0.35).contains(&moon_angular_radius_degrees(moon.distance_earth_radii)));
    }

    #[test]
    fn moonlight_model_is_stronger_near_full_moon() {
        let full = MoonPhase {
            age_days: SYNODIC_MONTH_DAYS / 2.0,
            illumination: 1.0,
            phase_fraction: 0.5,
        };
        let new = MoonPhase {
            age_days: 0.0,
            illumination: 0.0,
            phase_fraction: 0.0,
        };
        let near_full = moonlight_limiting_magnitude_loss(60.0, 60.0, 15.0, full);
        let far_full = moonlight_limiting_magnitude_loss(60.0, 60.0, 120.0, full);
        let near_new = moonlight_limiting_magnitude_loss(60.0, 60.0, 15.0, new);

        assert!(near_full > far_full);
        assert!(far_full > near_new);
        assert_eq!(near_new, 0.0);
    }
}
