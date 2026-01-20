use crate::models::DocumentProfile;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct CatalogFile {
    year: i32,
    #[allow(dead_code)]
    source: String,
    categories: Vec<CatalogCategory>,
}

#[derive(Debug, Deserialize)]
struct CatalogCategory {
    category: String,
    disciplines: Vec<String>,
}

#[derive(Debug, Clone)]
struct CatalogIndex {
    year: i32,
    categories: HashSet<String>,
    disciplines: HashMap<String, HashSet<String>>,
    discipline_to_category: HashMap<String, String>,
}

static CATALOG: OnceLock<CatalogIndex> = OnceLock::new();

fn catalog_index() -> &'static CatalogIndex {
    CATALOG.get_or_init(|| {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/data/edu_subject_catalog_2022.json"
        ));
        let parsed: CatalogFile = serde_json::from_str(raw)
            .expect("edu_subject_catalog_2022.json parse failed");

        let mut categories = HashSet::new();
        let mut disciplines = HashMap::new();
        let mut discipline_to_category = HashMap::new();

        for entry in parsed.categories {
            let category = entry.category.trim().to_string();
            if category.is_empty() {
                continue;
            }
            categories.insert(category.clone());
            let mut set = HashSet::new();
            for item in entry.disciplines {
                let trimmed = item.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                set.insert(trimmed.clone());
                discipline_to_category.entry(trimmed).or_insert(category.clone());
            }
            disciplines.insert(category, set);
        }

        CatalogIndex {
            year: parsed.year,
            categories,
            disciplines,
            discipline_to_category,
        }
    })
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProfileValidity {
    Valid,
    Partial,
    Invalid,
}

impl ProfileValidity {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileValidity::Valid => "valid",
            ProfileValidity::Partial => "partial",
            ProfileValidity::Invalid => "invalid",
        }
    }
}

pub fn profile_validity(profile: &DocumentProfile) -> ProfileValidity {
    match profile.validity.as_str() {
        "valid" => ProfileValidity::Valid,
        "partial" => ProfileValidity::Partial,
        "invalid" => ProfileValidity::Invalid,
        _ => ProfileValidity::Partial,
    }
}

pub fn validate_document_profile(profile: &mut DocumentProfile) -> ProfileValidity {
    let catalog = catalog_index();
    let category = profile.category.trim().to_string();
    let mut normalized_category = category.clone();
    if !catalog.categories.contains(&normalized_category) {
        normalized_category = "交叉学科".to_string();
    }
    profile.category = normalized_category.clone();

    let mut discipline_valid = false;
    if let Some(discipline) = profile.discipline.as_ref() {
        let discipline_trim = discipline.trim();
        if let Some(set) = catalog.disciplines.get(&normalized_category) {
            discipline_valid = set.contains(discipline_trim);
        }
        if !discipline_valid {
            if normalized_category == "交叉学科" {
                if let Some(found) = catalog.discipline_to_category.get(discipline_trim) {
                    profile.category = found.to_string();
                    if let Some(set) = catalog.disciplines.get(found) {
                        discipline_valid = set.contains(discipline_trim);
                    }
                }
            }
        }
    }

    let validity = if !catalog.categories.contains(&profile.category) {
        ProfileValidity::Invalid
    } else if discipline_valid {
        ProfileValidity::Valid
    } else {
        ProfileValidity::Partial
    };
    profile.validity = validity.as_str().to_string();
    validity
}

pub fn is_academic_profile(profile: &DocumentProfile) -> bool {
    if profile_validity(profile) == ProfileValidity::Invalid {
        return false;
    }
    let category = profile.category.trim();
    let is_known_domain = catalog_index().categories.contains(category);
    let has_discipline = profile
        .discipline
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
        || profile
            .subfield
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

    let paper_type = profile.paper_type.as_deref().unwrap_or("");
    let paper_lower = paper_type.to_lowercase();
    let is_academic_type = paper_lower.contains("论文")
        || paper_lower.contains("综述")
        || paper_lower.contains("研究")
        || paper_lower.contains("实验")
        || paper_lower.contains("报告")
        || paper_lower.contains("期刊")
        || paper_lower.contains("学位")
        || paper_lower.contains("thesis")
        || paper_lower.contains("paper")
        || paper_lower.contains("research")
        || paper_lower.contains("journal");

    is_academic_type || (is_known_domain && has_discipline)
}

#[allow(dead_code)]
pub fn catalog_year() -> i32 {
    catalog_index().year
}
