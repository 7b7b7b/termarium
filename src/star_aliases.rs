use crate::{catalog::Star, config::Language};

#[derive(Debug, Clone, Copy)]
pub struct StarAlias {
    pub hip: u32,
    pub display: &'static str,
    pub aliases: &'static [&'static str],
}

pub const STAR_ALIASES: &[StarAlias] = &[
    StarAlias {
        hip: 8102,
        display: "Tau Ceti",
        aliases: &[
            "Tau Ceti",
            "τ Ceti",
            "52 Ceti",
            "HD 10700",
            "HR 509",
            "Gliese 71",
            "天仓五",
        ],
    },
    StarAlias {
        hip: 71683,
        display: "Alpha Centauri",
        aliases: &[
            "Alpha Centauri",
            "α Centauri",
            "Alpha Cen",
            "α Cen",
            "Rigil Kentaurus",
            "南门二",
        ],
    },
    StarAlias {
        hip: 32349,
        display: "Sirius",
        aliases: &[
            "Sirius",
            "Alpha Canis Majoris",
            "α Canis Majoris",
            "Alpha CMa",
            "α CMa",
            "天狼星",
        ],
    },
    StarAlias {
        hip: 91262,
        display: "Vega",
        aliases: &[
            "Vega",
            "Alpha Lyrae",
            "α Lyrae",
            "Alpha Lyr",
            "α Lyr",
            "织女一",
        ],
    },
    StarAlias {
        hip: 97649,
        display: "Altair",
        aliases: &[
            "Altair",
            "Alpha Aquilae",
            "α Aquilae",
            "Alpha Aql",
            "α Aql",
            "河鼓二",
        ],
    },
    StarAlias {
        hip: 102098,
        display: "Deneb",
        aliases: &[
            "Deneb",
            "Alpha Cygni",
            "α Cygni",
            "Alpha Cyg",
            "α Cyg",
            "天津四",
        ],
    },
    StarAlias {
        hip: 69673,
        display: "Arcturus",
        aliases: &[
            "Arcturus",
            "Alpha Bootis",
            "α Bootis",
            "Alpha Boo",
            "α Boo",
            "大角星",
        ],
    },
    StarAlias {
        hip: 11767,
        display: "Polaris",
        aliases: &[
            "Polaris",
            "Alpha Ursae Minoris",
            "α Ursae Minoris",
            "Alpha UMi",
            "α UMi",
            "北极星",
            "勾陈一",
        ],
    },
    StarAlias {
        hip: 21421,
        display: "Aldebaran",
        aliases: &[
            "Aldebaran",
            "Alpha Tauri",
            "α Tauri",
            "Alpha Tau",
            "α Tau",
            "毕宿五",
        ],
    },
    StarAlias {
        hip: 27989,
        display: "Betelgeuse",
        aliases: &[
            "Betelgeuse",
            "Alpha Orionis",
            "α Orionis",
            "Alpha Ori",
            "α Ori",
            "参宿四",
        ],
    },
    StarAlias {
        hip: 24436,
        display: "Rigel",
        aliases: &[
            "Rigel",
            "Beta Orionis",
            "β Orionis",
            "Beta Ori",
            "β Ori",
            "参宿七",
        ],
    },
    StarAlias {
        hip: 37279,
        display: "Procyon",
        aliases: &[
            "Procyon",
            "Alpha Canis Minoris",
            "α Canis Minoris",
            "Alpha CMi",
            "α CMi",
            "南河三",
        ],
    },
    StarAlias {
        hip: 24608,
        display: "Capella",
        aliases: &[
            "Capella",
            "Alpha Aurigae",
            "α Aurigae",
            "Alpha Aur",
            "α Aur",
            "五车二",
        ],
    },
    StarAlias {
        hip: 65474,
        display: "Spica",
        aliases: &[
            "Spica",
            "Alpha Virginis",
            "α Virginis",
            "Alpha Vir",
            "α Vir",
            "角宿一",
        ],
    },
    StarAlias {
        hip: 80763,
        display: "Antares",
        aliases: &[
            "Antares",
            "Alpha Scorpii",
            "α Scorpii",
            "Alpha Sco",
            "α Sco",
            "心宿二",
        ],
    },
    StarAlias {
        hip: 113368,
        display: "Fomalhaut",
        aliases: &[
            "Fomalhaut",
            "Alpha Piscis Austrini",
            "α Piscis Austrini",
            "Alpha PsA",
            "α PsA",
            "北落师门",
        ],
    },
    StarAlias {
        hip: 30438,
        display: "Canopus",
        aliases: &[
            "Canopus",
            "Alpha Carinae",
            "α Carinae",
            "Alpha Car",
            "α Car",
            "老人星",
        ],
    },
    StarAlias {
        hip: 7588,
        display: "Achernar",
        aliases: &[
            "Achernar",
            "Alpha Eridani",
            "α Eridani",
            "Alpha Eri",
            "α Eri",
            "水委一",
        ],
    },
    StarAlias {
        hip: 85927,
        display: "Shaula",
        aliases: &[
            "Shaula",
            "Lambda Scorpii",
            "λ Scorpii",
            "Lambda Sco",
            "λ Sco",
            "尾宿八",
        ],
    },
    StarAlias {
        hip: 26451,
        display: "Tianguan",
        aliases: &[
            "Tianguan",
            "Zeta Tauri",
            "ζ Tauri",
            "Zeta Tau",
            "ζ Tau",
            "天关",
        ],
    },
    StarAlias {
        hip: 17702,
        display: "Alcyone",
        aliases: &[
            "Alcyone",
            "Eta Tauri",
            "η Tauri",
            "Eta Tau",
            "η Tau",
            "昴宿六",
        ],
    },
    StarAlias {
        hip: 14135,
        display: "Menkar",
        aliases: &[
            "Menkar",
            "Alpha Ceti",
            "α Ceti",
            "Alpha Cet",
            "α Cet",
            "天囷一",
        ],
    },
    StarAlias {
        hip: 3419,
        display: "Diphda",
        aliases: &[
            "Diphda",
            "Beta Ceti",
            "β Ceti",
            "Beta Cet",
            "β Cet",
            "土司空",
        ],
    },
];

pub fn for_hip(hip: u32) -> Option<StarAlias> {
    STAR_ALIASES.iter().copied().find(|entry| entry.hip == hip)
}

pub fn display_name(star: Star) -> String {
    if let Some(entry) = for_hip(star.hip) {
        entry.display.to_string()
    } else if !star.proper.is_empty() {
        star.proper.to_string()
    } else {
        format!("HIP {}", star.hip)
    }
}

pub fn display_name_for(star: Star, language: Language) -> String {
    let english = display_name(star);
    if language == Language::Zh {
        if let Some(chinese) = chinese_name(star) {
            if chinese != english {
                return format!("{chinese} / {english}");
            }
        }
    }
    english
}

pub fn summary_aliases_for(star: Star, language: Language) -> Option<String> {
    let entry = for_hip(star.hip)?;
    let chinese = chinese_name(star);
    let aliases = entry
        .aliases
        .iter()
        .copied()
        .filter(|alias| !alias.eq_ignore_ascii_case(entry.display))
        .filter(|alias| chinese != Some(*alias))
        .filter(|alias| language == Language::Zh || !contains_cjk(alias))
        .take(4)
        .collect::<Vec<_>>();
    (!aliases.is_empty()).then(|| aliases.join(", "))
}

pub fn chinese_name(star: Star) -> Option<&'static str> {
    for_hip(star.hip)?
        .aliases
        .iter()
        .copied()
        .find(|alias| contains_cjk(alias))
}

pub fn matches_query(star: Star, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return false;
    }

    if hip_query(query) == Some(star.hip) {
        return true;
    }

    let normalized_query = normalize(query);
    let compact_query = compact(query);
    if !star.proper.is_empty()
        && (normalize(star.proper).contains(&normalized_query)
            || compact(star.proper).contains(&compact_query))
    {
        return true;
    }

    for identifier in [format!("HIP {}", star.hip), format!("HIP{}", star.hip)] {
        if normalize(&identifier).contains(&normalized_query)
            || compact(&identifier).contains(&compact_query)
        {
            return true;
        }
    }

    for_hip(star.hip).is_some_and(|entry| {
        entry.aliases.iter().any(|alias| {
            normalize(alias).contains(&normalized_query) || compact(alias).contains(&compact_query)
        })
    })
}

fn hip_query(query: &str) -> Option<u32> {
    let lowered = query.trim().to_lowercase();
    let stripped = lowered
        .strip_prefix("hip")
        .unwrap_or(lowered.as_str())
        .trim();
    stripped.parse().ok()
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn compact(value: &str) -> String {
    normalize(value)
        .chars()
        .filter(|character| {
            !character.is_whitespace()
                && !matches!(character, '-' | '_' | '\'' | '"' | '.' | ',' | '/')
        })
        .collect()
}

fn contains_cjk(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '\u{3400}'..='\u{4DBF}'
                | '\u{4E00}'..='\u{9FFF}'
                | '\u{F900}'..='\u{FAFF}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn finds_tau_ceti_aliases() {
        let catalog = Catalog::load();
        let tau = catalog
            .stars
            .iter()
            .copied()
            .find(|star| star.hip == 8102)
            .unwrap();
        for query in ["Tau Ceti", "τ Ceti", "天仓五", "HD10700", "HIP 8102"] {
            assert!(matches_query(tau, query), "{query} should match Tau Ceti");
        }
        assert_eq!(display_name(tau), "Tau Ceti");
        assert_eq!(display_name_for(tau, Language::Zh), "天仓五 / Tau Ceti");
        assert_eq!(display_name_for(tau, Language::En), "Tau Ceti");
        assert!(
            !summary_aliases_for(tau, Language::En)
                .unwrap()
                .contains("天仓五")
        );
    }

    #[test]
    fn curated_aliases_point_to_existing_stars() {
        let catalog = Catalog::load();
        for entry in STAR_ALIASES {
            assert!(
                catalog.stars.iter().any(|star| star.hip == entry.hip),
                "{} points to missing HIP {}",
                entry.display,
                entry.hip
            );
        }
    }
}
