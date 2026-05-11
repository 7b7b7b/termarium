use chrono::{DateTime, Utc};

use crate::astro;

pub const ISS_ALTITUDE_KM: f64 = 420.0;

const ISS_INCLINATION_DEGREES: f64 = 51.64;
const ISS_ORBIT_PERIOD_MINUTES: f64 = 92.68;
const ISS_RAAN_DEGREES: f64 = 214.0;
const ISS_PHASE_OFFSET_DEGREES: f64 = 34.0;

#[derive(Debug, Clone, Copy)]
pub struct IssPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_km: f64,
}

pub fn position(time: DateTime<Utc>) -> IssPosition {
    let elapsed_minutes = (time.timestamp() as f64
        + f64::from(time.timestamp_subsec_nanos()) / 1_000_000_000.0)
        / 60.0;
    let mean_motion = std::f64::consts::TAU * elapsed_minutes / ISS_ORBIT_PERIOD_MINUTES
        + ISS_PHASE_OFFSET_DEGREES.to_radians();
    let inclination = ISS_INCLINATION_DEGREES.to_radians();
    let raan = ISS_RAAN_DEGREES.to_radians();
    let (sin_u, cos_u) = mean_motion.sin_cos();
    let (sin_i, cos_i) = inclination.sin_cos();
    let (sin_raan, cos_raan) = raan.sin_cos();

    let x = cos_raan * cos_u - sin_raan * sin_u * cos_i;
    let y = sin_raan * cos_u + cos_raan * sin_u * cos_i;
    let z = sin_u * sin_i;
    let inertial_longitude = y.atan2(x).to_degrees();
    let gmst_degrees = astro::local_sidereal_time_hours(time, 0.0) * 15.0;

    IssPosition {
        latitude: z.clamp(-1.0, 1.0).asin().to_degrees(),
        longitude: normalize_degrees(inertial_longitude - gmst_degrees),
        altitude_km: ISS_ALTITUDE_KM,
    }
}

fn normalize_degrees(degrees: f64) -> f64 {
    ((degrees + 180.0).rem_euclid(360.0)) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn iss_position_stays_in_orbital_bounds() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let position = position(time);

        assert!((-ISS_INCLINATION_DEGREES..=ISS_INCLINATION_DEGREES).contains(&position.latitude));
        assert!((-180.0..=180.0).contains(&position.longitude));
        assert_eq!(position.altitude_km, ISS_ALTITUDE_KM);
    }

    #[test]
    fn iss_position_moves_over_time() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let start = position(time);
        let later = position(time + chrono::Duration::minutes(10));

        assert!((start.latitude - later.latitude).abs() > 1.0);
        assert!((start.longitude - later.longitude).abs() > 1.0);
    }
}
