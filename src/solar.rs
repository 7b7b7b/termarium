use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

use crate::{astro, config::Location};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunPosition {
    pub ra_hours: f64,
    pub dec_degrees: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyLight {
    Day,
    CivilTwilight,
    NauticalTwilight,
    AstronomicalTwilight,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarEvents {
    pub sunset: Option<DateTime<Utc>>,
    pub sunrise: Option<DateTime<Utc>>,
}

impl SkyLight {
    pub fn name_en(self) -> &'static str {
        match self {
            Self::Day => "Daylight",
            Self::CivilTwilight => "Civil twilight",
            Self::NauticalTwilight => "Nautical twilight",
            Self::AstronomicalTwilight => "Astronomical twilight",
            Self::Night => "Night",
        }
    }

    pub fn name_zh(self) -> &'static str {
        match self {
            Self::Day => "白昼",
            Self::CivilTwilight => "民用曙暮光",
            Self::NauticalTwilight => "航海曙暮光",
            Self::AstronomicalTwilight => "天文曙暮光",
            Self::Night => "夜晚",
        }
    }
}

pub fn sun_position(time: DateTime<Utc>) -> SunPosition {
    let n = astro::julian_day(time) - 2_451_545.0;
    let mean_longitude = normalize_degrees(280.460 + 0.985_647_4 * n);
    let mean_anomaly = normalize_degrees(357.528 + 0.985_600_3 * n).to_radians();
    let ecliptic_longitude = normalize_degrees(
        mean_longitude + 1.915 * mean_anomaly.sin() + 0.020 * (2.0 * mean_anomaly).sin(),
    )
    .to_radians();
    let obliquity = (23.439 - 0.000_000_4 * n).to_radians();

    let ra = (obliquity.cos() * ecliptic_longitude.sin())
        .atan2(ecliptic_longitude.cos())
        .to_degrees()
        / 15.0;
    let dec = (obliquity.sin() * ecliptic_longitude.sin())
        .asin()
        .to_degrees();

    SunPosition {
        ra_hours: positive_mod(ra, 24.0),
        dec_degrees: dec,
    }
}

pub fn skylight(location: &Location, time: DateTime<Utc>) -> SkyLight {
    let sun = sun_position(time);
    let altitude =
        astro::horizontal_position(sun.ra_hours, sun.dec_degrees, location, time).altitude;
    if altitude >= 0.0 {
        SkyLight::Day
    } else if altitude >= -6.0 {
        SkyLight::CivilTwilight
    } else if altitude >= -12.0 {
        SkyLight::NauticalTwilight
    } else if altitude >= -18.0 {
        SkyLight::AstronomicalTwilight
    } else {
        SkyLight::Night
    }
}

pub fn tonight_events(location: &Location, time: DateTime<Utc>) -> SolarEvents {
    let timezone = location.timezone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
    let local = time.with_timezone(&timezone);
    let date = local.date_naive();
    let sunset = solar_event_utc(date, location, false, 90.833);
    let sunrise = solar_event_utc(date + Duration::days(1), location, true, 90.833);
    SolarEvents { sunset, sunrise }
}

fn solar_event_utc(
    date: NaiveDate,
    location: &Location,
    sunrise: bool,
    zenith_degrees: f64,
) -> Option<DateTime<Utc>> {
    let n = date.ordinal() as f64;
    let longitude_hour = location.longitude / 15.0;
    let approximate_time = if sunrise {
        n + (6.0 - longitude_hour) / 24.0
    } else {
        n + (18.0 - longitude_hour) / 24.0
    };
    let mean_anomaly = 0.9856 * approximate_time - 3.289;
    let true_longitude = normalize_degrees(
        mean_anomaly
            + 1.916 * mean_anomaly.to_radians().sin()
            + 0.020 * (2.0 * mean_anomaly).to_radians().sin()
            + 282.634,
    );
    let mut right_ascension = (0.91764 * true_longitude.to_radians().tan())
        .atan()
        .to_degrees();
    right_ascension = normalize_degrees(right_ascension);
    right_ascension +=
        (true_longitude / 90.0).floor() * 90.0 - (right_ascension / 90.0).floor() * 90.0;
    right_ascension /= 15.0;

    let sin_dec = 0.39782 * true_longitude.to_radians().sin();
    let cos_dec = sin_dec.asin().cos();
    let lat = location.latitude.to_radians();
    let cos_hour_angle =
        (zenith_degrees.to_radians().cos() - sin_dec * lat.sin()) / (cos_dec * lat.cos());
    if !(-1.0..=1.0).contains(&cos_hour_angle) {
        return None;
    }

    let mut hour_angle = cos_hour_angle.acos().to_degrees();
    if sunrise {
        hour_angle = 360.0 - hour_angle;
    }
    hour_angle /= 15.0;
    let local_mean_time = hour_angle + right_ascension - 0.06571 * approximate_time - 6.622;
    let utc_hours = positive_mod(local_mean_time - longitude_hour, 24.0);
    let seconds = (utc_hours * 3600.0).round() as i64;
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .map(|midnight| midnight + Duration::seconds(seconds))
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
    fn sun_position_is_finite() {
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let sun = sun_position(time);
        assert!((0.0..24.0).contains(&sun.ra_hours));
        assert!((-90.0..=90.0).contains(&sun.dec_degrees));
    }

    #[test]
    fn shanghai_has_tonight_events() {
        let location = Location {
            name: "Shanghai".to_string(),
            latitude: 31.2304,
            longitude: 121.4737,
            timezone: "Asia/Shanghai".to_string(),
        };
        let time = Utc.with_ymd_and_hms(2026, 5, 7, 14, 0, 0).unwrap();
        let events = tonight_events(&location, time);
        assert!(events.sunset.is_some());
        assert!(events.sunrise.is_some());
    }
}
