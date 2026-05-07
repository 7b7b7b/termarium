#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeepSkyObject {
    pub name: &'static str,
    pub kind: &'static str,
    pub ra_hours: f64,
    pub dec_degrees: f64,
    pub magnitude: Option<f64>,
    pub constellation: &'static str,
    pub common: &'static str,
}

const DEEP_SKY_CATALOG: &str = include_str!("../data/deep_sky.csv");

pub fn load() -> Vec<DeepSkyObject> {
    DEEP_SKY_CATALOG
        .lines()
        .skip(1)
        .filter_map(parse_object)
        .collect()
}

fn parse_object(line: &'static str) -> Option<DeepSkyObject> {
    let mut fields = line.split(',');
    let name = fields.next()?.trim();
    let kind = fields.next()?.trim();
    let ra_hours = fields.next()?.parse().ok()?;
    let dec_degrees = fields.next()?.parse().ok()?;
    let magnitude = fields.next().and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.parse().ok()).flatten()
    });
    let constellation = fields.next()?.trim();
    let common = fields.next().unwrap_or("").trim();

    Some(DeepSkyObject {
        name,
        kind,
        ra_hours,
        dec_degrees,
        magnitude,
        constellation,
        common,
    })
}

pub fn matches_query(object: DeepSkyObject, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    object.name.to_lowercase().contains(&query)
        || object.common.to_lowercase().contains(&query)
        || object.kind.to_lowercase().contains(&query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_messier_catalog() {
        let objects = load();
        assert_eq!(objects.len(), 110);
        assert!(objects.iter().any(|object| object.name == "M31"));
        assert!(objects.iter().any(|object| object.common == "Pleiades"));
    }
}
