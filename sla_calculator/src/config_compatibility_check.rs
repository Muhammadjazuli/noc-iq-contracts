// Config version negotiation query for client SDKs. Complements
// version_negotiation.rs.
use soroban_sdk::{contracttype, Env, Symbol, Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum CompatibilityStatus {
    Compatible,
    UpgradeRecommended,
    Incompatible,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum CompatibilityCategory {
    Compatible,
    OptionalAddition,
    BreakingAddition,
    BreakingRemoval,
    IncompatibleType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ConfigFieldSchema {
    pub path: Symbol,
    pub field_type: Symbol,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ConfigSchema {
    pub fields: Vec<ConfigFieldSchema>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ConfigCompatibilityResult {
    pub compatible: bool,
    pub category: CompatibilityCategory,
    pub field_path: Option<Symbol>,
}

pub fn check_config_compatibility(
    active_version: u32,
    client_version: u32,
) -> CompatibilityStatus {
    if client_version == active_version {
        CompatibilityStatus::Compatible
    } else if client_version + 1 == active_version {
        CompatibilityStatus::UpgradeRecommended
    } else {
        CompatibilityStatus::Incompatible
    }
}

pub fn check_schema_compatibility(
    baseline: &ConfigSchema,
    candidate: &ConfigSchema,
) -> ConfigCompatibilityResult {
    for i in 0..baseline.fields.len() {
        let old = baseline.fields.get(i).unwrap();
        match find_field(&candidate.fields, &old.path) {
            None if old.required => {
                return result(false, CompatibilityCategory::BreakingRemoval, Some(old.path));
            }
            None => {}
            Some(new) if new.field_type != old.field_type => {
                return result(
                    false,
                    CompatibilityCategory::IncompatibleType,
                    Some(old.path),
                );
            }
            Some(_) => {}
        }
    }

    for i in 0..candidate.fields.len() {
        let new = candidate.fields.get(i).unwrap();
        if find_field(&baseline.fields, &new.path).is_none() {
            let (compatible, category) = if new.required {
                (false, CompatibilityCategory::BreakingAddition)
            } else {
                (true, CompatibilityCategory::OptionalAddition)
            };
            return result(compatible, category, Some(new.path));
        }
    }

    result(true, CompatibilityCategory::Compatible, None)
}

fn find_field(fields: &Vec<ConfigFieldSchema>, path: &Symbol) -> Option<ConfigFieldSchema> {
    for i in 0..fields.len() {
        let field = fields.get(i).unwrap();
        if field.path == *path {
            return Some(field);
        }
    }
    None
}

fn result(
    compatible: bool,
    category: CompatibilityCategory,
    field_path: Option<Symbol>,
) -> ConfigCompatibilityResult {
    ConfigCompatibilityResult {
        compatible,
        category,
        field_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(env: &Env, path: &str, field_type: &str, required: bool) -> ConfigFieldSchema {
        ConfigFieldSchema {
            path: Symbol::new(env, path),
            field_type: Symbol::new(env, field_type),
            required,
        }
    }

    fn schema(env: &Env, fields: &[ConfigFieldSchema]) -> ConfigSchema {
        let mut stored = Vec::new(env);
        for field in fields {
            stored.push_back(field.clone());
        }
        ConfigSchema { fields: stored }
    }

    #[test]
    fn version_gap_still_reports_upgrade_recommended() {
        assert_eq!(
            check_config_compatibility(3, 2),
            CompatibilityStatus::UpgradeRecommended
        );
    }

    #[test]
    fn adding_an_optional_field_is_classified_as_optional_addition() {
        let env = Env::default();
        let baseline = schema(&env, &[field(&env, "threshold", "u32", true)]);
        let candidate = schema(
            &env,
            &[
                field(&env, "threshold", "u32", true),
                field(&env, "note", "string", false),
            ],
        );

        let result = check_schema_compatibility(&baseline, &candidate);

        assert!(result.compatible);
        assert_eq!(result.category, CompatibilityCategory::OptionalAddition);
        assert_eq!(result.field_path, Some(Symbol::new(&env, "note")));
    }

    #[test]
    fn removing_a_required_field_is_classified_as_breaking_removal() {
        let env = Env::default();
        let baseline = schema(
            &env,
            &[
                field(&env, "threshold", "u32", true),
                field(&env, "penalty", "i128", true),
            ],
        );
        let candidate = schema(&env, &[field(&env, "threshold", "u32", true)]);

        let result = check_schema_compatibility(&baseline, &candidate);

        assert!(!result.compatible);
        assert_eq!(result.category, CompatibilityCategory::BreakingRemoval);
        assert_eq!(result.field_path, Some(Symbol::new(&env, "penalty")));
    }

    #[test]
    fn changing_a_field_type_is_classified_as_incompatible_type() {
        let env = Env::default();
        let baseline = schema(&env, &[field(&env, "threshold", "u32", true)]);
        let candidate = schema(&env, &[field(&env, "threshold", "i128", true)]);

        let result = check_schema_compatibility(&baseline, &candidate);

        assert!(!result.compatible);
        assert_eq!(result.category, CompatibilityCategory::IncompatibleType);
        assert_eq!(result.field_path, Some(Symbol::new(&env, "threshold")));
    }

    #[test]
    fn identical_schemas_are_compatible_without_a_field_path() {
        let env = Env::default();
        let baseline = schema(&env, &[field(&env, "threshold", "u32", true)]);

        let result = check_schema_compatibility(&baseline, &baseline);

        assert!(result.compatible);
        assert_eq!(result.category, CompatibilityCategory::Compatible);
        assert_eq!(result.field_path, None);
    }
}
