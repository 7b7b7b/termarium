use std::{collections::HashMap, sync::OnceLock};

use crate::config::{Language, SkyCulture};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstellationLine {
    pub code: &'static str,
    pub hips: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstellationMeta {
    pub code: &'static str,
    pub en: &'static str,
    pub zh: &'static str,
}

impl ConstellationLine {
    pub fn segment_count(&self) -> usize {
        self.hips.len().saturating_sub(1)
    }
}

const CONSTELLATION_LINES: &str = include_str!("../data/constellation_lines_hip.csv");
const CHINESE_SKY_LINES: &str = include_str!("../data/chinese_sky_lines.csv");
const CHINESE_SKY_FIGURES: &str = include_str!("../data/chinese_sky_figures.csv");
const CHINESE_STAR_NAMES: &str = include_str!("../data/chinese_star_names.csv");
static CHINESE_STAR_NAME_INDEX: OnceLock<HashMap<u32, Vec<ChineseStarName>>> = OnceLock::new();

pub const CONSTELLATION_META: &[ConstellationMeta] = &[
    ConstellationMeta {
        code: "And",
        en: "Andromeda",
        zh: "仙女座",
    },
    ConstellationMeta {
        code: "Ant",
        en: "Antlia",
        zh: "唧筒座",
    },
    ConstellationMeta {
        code: "Aps",
        en: "Apus",
        zh: "天燕座",
    },
    ConstellationMeta {
        code: "Aql",
        en: "Aquila",
        zh: "天鹰座",
    },
    ConstellationMeta {
        code: "Aqr",
        en: "Aquarius",
        zh: "宝瓶座",
    },
    ConstellationMeta {
        code: "Ara",
        en: "Ara",
        zh: "天坛座",
    },
    ConstellationMeta {
        code: "Ari",
        en: "Aries",
        zh: "白羊座",
    },
    ConstellationMeta {
        code: "Aur",
        en: "Auriga",
        zh: "御夫座",
    },
    ConstellationMeta {
        code: "Boo",
        en: "Bootes",
        zh: "牧夫座",
    },
    ConstellationMeta {
        code: "CMa",
        en: "Canis Major",
        zh: "大犬座",
    },
    ConstellationMeta {
        code: "CMi",
        en: "Canis Minor",
        zh: "小犬座",
    },
    ConstellationMeta {
        code: "CVn",
        en: "Canes Venatici",
        zh: "猎犬座",
    },
    ConstellationMeta {
        code: "Cae",
        en: "Caelum",
        zh: "雕具座",
    },
    ConstellationMeta {
        code: "Cam",
        en: "Camelopardalis",
        zh: "鹿豹座",
    },
    ConstellationMeta {
        code: "Cap",
        en: "Capricornus",
        zh: "摩羯座",
    },
    ConstellationMeta {
        code: "Car",
        en: "Carina",
        zh: "船底座",
    },
    ConstellationMeta {
        code: "Cas",
        en: "Cassiopeia",
        zh: "仙后座",
    },
    ConstellationMeta {
        code: "Cen",
        en: "Centaurus",
        zh: "半人马座",
    },
    ConstellationMeta {
        code: "Cep",
        en: "Cepheus",
        zh: "仙王座",
    },
    ConstellationMeta {
        code: "Cet",
        en: "Cetus",
        zh: "鲸鱼座",
    },
    ConstellationMeta {
        code: "Cha",
        en: "Chamaeleon",
        zh: "蝘蜓座",
    },
    ConstellationMeta {
        code: "Cir",
        en: "Circinus",
        zh: "圆规座",
    },
    ConstellationMeta {
        code: "Cnc",
        en: "Cancer",
        zh: "巨蟹座",
    },
    ConstellationMeta {
        code: "Col",
        en: "Columba",
        zh: "天鸽座",
    },
    ConstellationMeta {
        code: "Com",
        en: "Coma Berenices",
        zh: "后发座",
    },
    ConstellationMeta {
        code: "CrA",
        en: "Corona Australis",
        zh: "南冕座",
    },
    ConstellationMeta {
        code: "CrB",
        en: "Corona Borealis",
        zh: "北冕座",
    },
    ConstellationMeta {
        code: "Crt",
        en: "Crater",
        zh: "巨爵座",
    },
    ConstellationMeta {
        code: "Cru",
        en: "Crux",
        zh: "南十字座",
    },
    ConstellationMeta {
        code: "Crv",
        en: "Corvus",
        zh: "乌鸦座",
    },
    ConstellationMeta {
        code: "Cyg",
        en: "Cygnus",
        zh: "天鹅座",
    },
    ConstellationMeta {
        code: "Del",
        en: "Delphinus",
        zh: "海豚座",
    },
    ConstellationMeta {
        code: "Dor",
        en: "Dorado",
        zh: "剑鱼座",
    },
    ConstellationMeta {
        code: "Dra",
        en: "Draco",
        zh: "天龙座",
    },
    ConstellationMeta {
        code: "Equ",
        en: "Equuleus",
        zh: "小马座",
    },
    ConstellationMeta {
        code: "Eri",
        en: "Eridanus",
        zh: "波江座",
    },
    ConstellationMeta {
        code: "For",
        en: "Fornax",
        zh: "天炉座",
    },
    ConstellationMeta {
        code: "Gem",
        en: "Gemini",
        zh: "双子座",
    },
    ConstellationMeta {
        code: "Gru",
        en: "Grus",
        zh: "天鹤座",
    },
    ConstellationMeta {
        code: "Her",
        en: "Hercules",
        zh: "武仙座",
    },
    ConstellationMeta {
        code: "Hor",
        en: "Horologium",
        zh: "时钟座",
    },
    ConstellationMeta {
        code: "Hya",
        en: "Hydra",
        zh: "长蛇座",
    },
    ConstellationMeta {
        code: "Hyi",
        en: "Hydrus",
        zh: "水蛇座",
    },
    ConstellationMeta {
        code: "Ind",
        en: "Indus",
        zh: "印第安座",
    },
    ConstellationMeta {
        code: "LMi",
        en: "Leo Minor",
        zh: "小狮座",
    },
    ConstellationMeta {
        code: "Lac",
        en: "Lacerta",
        zh: "蝎虎座",
    },
    ConstellationMeta {
        code: "Leo",
        en: "Leo",
        zh: "狮子座",
    },
    ConstellationMeta {
        code: "Lep",
        en: "Lepus",
        zh: "天兔座",
    },
    ConstellationMeta {
        code: "Lib",
        en: "Libra",
        zh: "天秤座",
    },
    ConstellationMeta {
        code: "Lup",
        en: "Lupus",
        zh: "豺狼座",
    },
    ConstellationMeta {
        code: "Lyn",
        en: "Lynx",
        zh: "天猫座",
    },
    ConstellationMeta {
        code: "Lyr",
        en: "Lyra",
        zh: "天琴座",
    },
    ConstellationMeta {
        code: "Men",
        en: "Mensa",
        zh: "山案座",
    },
    ConstellationMeta {
        code: "Mic",
        en: "Microscopium",
        zh: "显微镜座",
    },
    ConstellationMeta {
        code: "Mon",
        en: "Monoceros",
        zh: "麒麟座",
    },
    ConstellationMeta {
        code: "Mus",
        en: "Musca",
        zh: "苍蝇座",
    },
    ConstellationMeta {
        code: "Nor",
        en: "Norma",
        zh: "矩尺座",
    },
    ConstellationMeta {
        code: "Oct",
        en: "Octans",
        zh: "南极座",
    },
    ConstellationMeta {
        code: "Oph",
        en: "Ophiuchus",
        zh: "蛇夫座",
    },
    ConstellationMeta {
        code: "Ori",
        en: "Orion",
        zh: "猎户座",
    },
    ConstellationMeta {
        code: "Pav",
        en: "Pavo",
        zh: "孔雀座",
    },
    ConstellationMeta {
        code: "Peg",
        en: "Pegasus",
        zh: "飞马座",
    },
    ConstellationMeta {
        code: "Per",
        en: "Perseus",
        zh: "英仙座",
    },
    ConstellationMeta {
        code: "Phe",
        en: "Phoenix",
        zh: "凤凰座",
    },
    ConstellationMeta {
        code: "Pic",
        en: "Pictor",
        zh: "绘架座",
    },
    ConstellationMeta {
        code: "PsA",
        en: "Piscis Austrinus",
        zh: "南鱼座",
    },
    ConstellationMeta {
        code: "Psc",
        en: "Pisces",
        zh: "双鱼座",
    },
    ConstellationMeta {
        code: "Pup",
        en: "Puppis",
        zh: "船尾座",
    },
    ConstellationMeta {
        code: "Pyx",
        en: "Pyxis",
        zh: "罗盘座",
    },
    ConstellationMeta {
        code: "Ret",
        en: "Reticulum",
        zh: "网罟座",
    },
    ConstellationMeta {
        code: "Scl",
        en: "Sculptor",
        zh: "玉夫座",
    },
    ConstellationMeta {
        code: "Sco",
        en: "Scorpius",
        zh: "天蝎座",
    },
    ConstellationMeta {
        code: "Sct",
        en: "Scutum",
        zh: "盾牌座",
    },
    ConstellationMeta {
        code: "Ser",
        en: "Serpens",
        zh: "巨蛇座",
    },
    ConstellationMeta {
        code: "Sex",
        en: "Sextans",
        zh: "六分仪座",
    },
    ConstellationMeta {
        code: "Sge",
        en: "Sagitta",
        zh: "天箭座",
    },
    ConstellationMeta {
        code: "Sgr",
        en: "Sagittarius",
        zh: "人马座",
    },
    ConstellationMeta {
        code: "Tau",
        en: "Taurus",
        zh: "金牛座",
    },
    ConstellationMeta {
        code: "Tel",
        en: "Telescopium",
        zh: "望远镜座",
    },
    ConstellationMeta {
        code: "TrA",
        en: "Triangulum Australe",
        zh: "南三角座",
    },
    ConstellationMeta {
        code: "Tri",
        en: "Triangulum",
        zh: "三角座",
    },
    ConstellationMeta {
        code: "Tuc",
        en: "Tucana",
        zh: "杜鹃座",
    },
    ConstellationMeta {
        code: "UMa",
        en: "Ursa Major",
        zh: "大熊座",
    },
    ConstellationMeta {
        code: "UMi",
        en: "Ursa Minor",
        zh: "小熊座",
    },
    ConstellationMeta {
        code: "Vel",
        en: "Vela",
        zh: "船帆座",
    },
    ConstellationMeta {
        code: "Vir",
        en: "Virgo",
        zh: "室女座",
    },
    ConstellationMeta {
        code: "Vol",
        en: "Volans",
        zh: "飞鱼座",
    },
    ConstellationMeta {
        code: "Vul",
        en: "Vulpecula",
        zh: "狐狸座",
    },
];

pub fn load() -> Vec<ConstellationLine> {
    CONSTELLATION_LINES
        .lines()
        .skip(1)
        .filter_map(parse_line)
        .collect()
}

pub fn load_chinese() -> Vec<ConstellationLine> {
    CHINESE_SKY_LINES
        .lines()
        .skip(1)
        .filter_map(parse_line)
        .collect()
}

pub fn meta_for(code: &str) -> ConstellationMeta {
    CONSTELLATION_META
        .iter()
        .copied()
        .find(|meta| meta.code.eq_ignore_ascii_case(code))
        .unwrap_or(ConstellationMeta {
            code: "",
            en: "Unknown",
            zh: "未知",
        })
}

pub fn chinese_meta() -> Vec<ConstellationMeta> {
    CHINESE_SKY_FIGURES
        .lines()
        .skip(1)
        .filter_map(parse_chinese_meta)
        .collect()
}

pub fn meta_for_culture(culture: SkyCulture, code: &str) -> ConstellationMeta {
    match culture {
        SkyCulture::Western => meta_for(code),
        SkyCulture::Chinese => chinese_meta()
            .into_iter()
            .find(|meta| meta.code.eq_ignore_ascii_case(code))
            .unwrap_or(ConstellationMeta {
                code: "",
                en: "Unknown",
                zh: "未知",
            }),
    }
}

pub fn metadata_for_culture(culture: SkyCulture) -> Vec<ConstellationMeta> {
    match culture {
        SkyCulture::Western => CONSTELLATION_META.to_vec(),
        SkyCulture::Chinese => chinese_meta(),
    }
}

pub fn figure_label(culture: SkyCulture, code: &str, _language: Language) -> String {
    let meta = meta_for_culture(culture, code);
    match culture {
        SkyCulture::Western => meta.code.to_string(),
        SkyCulture::Chinese => meta.zh.to_string(),
    }
}

pub fn display_name_for_culture(culture: SkyCulture, code: &str) -> String {
    let meta = meta_for_culture(culture, code);
    match culture {
        SkyCulture::Western => meta.en.to_string(),
        SkyCulture::Chinese => meta.zh.to_string(),
    }
}

pub fn pinyin_for(code: &str) -> Option<&'static str> {
    CHINESE_SKY_FIGURES.lines().skip(1).find_map(|line| {
        let fields = split_csv_line(line);
        (fields.get(0).copied() == Some(code))
            .then(|| fields.get(2).copied().unwrap_or(""))
            .filter(|value| !value.is_empty())
    })
}

pub fn star_names_for_hip(hip: u32) -> Vec<ChineseStarName> {
    chinese_star_name_index()
        .get(&hip)
        .cloned()
        .unwrap_or_default()
}

pub fn chinese_star_name_for_hip(hip: u32) -> Option<ChineseStarName> {
    chinese_star_name_index()
        .get(&hip)
        .and_then(|names| names.first().copied())
}

pub fn chinese_star_name_matches(hip: u32, query: &str) -> bool {
    let normalized_query = normalize(query);
    let compact_query = compact(query);
    star_names_for_hip(hip).into_iter().any(|name| {
        [name.zh, name.pinyin, name.en, name.desig]
            .iter()
            .any(|value| {
                !value.is_empty()
                    && (normalize(value).contains(&normalized_query)
                        || compact(value).contains(&compact_query))
            })
    })
}

pub fn matches_query(culture: SkyCulture, code: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    let meta = meta_for_culture(culture, code);
    let pinyin = pinyin_for(code).unwrap_or("").to_lowercase();
    code.to_lowercase().contains(&query)
        || meta.code.to_lowercase().contains(&query)
        || meta.en.to_lowercase().contains(&query)
        || meta.zh.contains(query.as_str())
        || pinyin.contains(query.as_str())
}

fn parse_line(line: &'static str) -> Option<ConstellationLine> {
    let (code, hips_raw) = line.split_once(',')?;
    let code = code.trim();
    if code.is_empty() {
        return None;
    }

    let hips = hips_raw
        .split_whitespace()
        .filter_map(|value| value.parse::<u32>().ok())
        .collect::<Vec<_>>();
    (hips.len() >= 2).then_some(ConstellationLine { code, hips })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChineseStarName {
    pub hip: u32,
    pub zh: &'static str,
    pub pinyin: &'static str,
    pub en: &'static str,
    pub desig: &'static str,
}

fn chinese_star_name_index() -> &'static HashMap<u32, Vec<ChineseStarName>> {
    CHINESE_STAR_NAME_INDEX.get_or_init(|| {
        let mut index: HashMap<u32, Vec<ChineseStarName>> = HashMap::new();
        for name in CHINESE_STAR_NAMES
            .lines()
            .skip(1)
            .filter_map(parse_chinese_star_name)
        {
            index.entry(name.hip).or_default().push(name);
        }
        index
    })
}

fn parse_chinese_meta(line: &'static str) -> Option<ConstellationMeta> {
    let fields = split_csv_line(line);
    Some(ConstellationMeta {
        code: fields.first().copied()?.trim(),
        zh: fields.get(1).copied().unwrap_or("").trim(),
        en: fields.get(3).copied().unwrap_or("").trim(),
    })
}

fn parse_chinese_star_name(line: &'static str) -> Option<ChineseStarName> {
    let fields = split_csv_line(line);
    Some(ChineseStarName {
        hip: fields.first()?.parse().ok()?,
        zh: fields.get(1).copied().unwrap_or("").trim(),
        pinyin: fields.get(2).copied().unwrap_or("").trim(),
        en: fields.get(3).copied().unwrap_or("").trim(),
        desig: fields.get(4).copied().unwrap_or("").trim(),
    })
}

fn split_csv_line(line: &'static str) -> Vec<&'static str> {
    line.split(',').collect()
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn parses_bundled_lines() {
        let lines = load();
        let codes = lines.iter().map(|line| line.code).collect::<BTreeSet<_>>();
        let segments = lines
            .iter()
            .map(ConstellationLine::segment_count)
            .sum::<usize>();

        assert_eq!(codes.len(), 88);
        assert_eq!(lines.len(), 100);
        assert_eq!(segments, 725);
    }

    #[test]
    fn all_line_stars_exist_in_catalog() {
        let catalog = Catalog::load();
        let hips = catalog
            .stars
            .iter()
            .map(|star| star.hip)
            .collect::<HashSet<_>>();

        for line in load() {
            for hip in line.hips {
                assert!(hips.contains(&hip), "{} missing HIP {}", line.code, hip);
            }
        }
    }

    #[test]
    fn parses_chinese_sky_figures() {
        let lines = load_chinese();
        let codes = lines.iter().map(|line| line.code).collect::<BTreeSet<_>>();
        let segments = lines
            .iter()
            .map(ConstellationLine::segment_count)
            .sum::<usize>();
        let meta = chinese_meta();

        assert_eq!(meta.len(), 318);
        assert_eq!(codes.len(), 312);
        assert_eq!(lines.len(), 422);
        assert_eq!(segments, 1136);
        assert!(meta.iter().all(|entry| !entry.code.is_empty()));
        assert!(meta.iter().all(|entry| !entry.zh.is_empty()));
        assert!(meta.iter().all(|entry| !entry.en.is_empty()));
    }

    #[test]
    fn all_chinese_line_stars_exist_in_catalog() {
        let catalog = Catalog::load();
        let hips = catalog
            .stars
            .iter()
            .map(|star| star.hip)
            .collect::<HashSet<_>>();

        for line in load_chinese() {
            for hip in line.hips {
                assert!(hips.contains(&hip), "{} missing HIP {}", line.code, hip);
            }
        }
    }

    #[test]
    fn chinese_asterism_search_matches_names() {
        assert!(matches_query(SkyCulture::Chinese, "CN003", "参宿"));
        assert!(matches_query(SkyCulture::Chinese, "CN003", "Shen Xiu"));
        assert!(matches_query(SkyCulture::Chinese, "CN003", "Three Stars"));
    }

    #[test]
    fn chinese_star_names_point_to_existing_stars() {
        let catalog = Catalog::load();
        let hips = catalog
            .stars
            .iter()
            .map(|star| star.hip)
            .collect::<HashSet<_>>();
        let names = CHINESE_STAR_NAMES
            .lines()
            .skip(1)
            .filter_map(parse_chinese_star_name)
            .collect::<Vec<_>>();

        assert_eq!(names.len(), 3056);
        for name in names {
            assert!(
                hips.contains(&name.hip),
                "{} points to missing HIP {}",
                name.zh,
                name.hip
            );
        }
    }
}
