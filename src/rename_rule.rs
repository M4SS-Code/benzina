use std::str::FromStr;

use heck::{
    ToKebabCase as _, ToLowerCamelCase as _, ToPascalCase as _, ToShoutyKebabCase,
    ToShoutySnakeCase as _, ToSnakeCase as _,
};

#[derive(Copy, Clone)]
pub(crate) enum RenameRule {
    None,
    LowerCase,
    SnakeCase,
    UpperCase,
    PascalCase,
    CamelCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameRule {
    pub(crate) fn format(self, val: &str) -> String {
        match self {
            Self::None => val.to_owned(),
            Self::LowerCase => val.to_lowercase(),
            Self::UpperCase => val.to_uppercase(),
            Self::PascalCase => val.to_pascal_case(),
            Self::CamelCase => val.to_lower_camel_case(),
            Self::SnakeCase => val.to_snake_case(),
            Self::ScreamingSnakeCase => val.to_shouty_snake_case(),
            Self::KebabCase => val.to_kebab_case(),
            Self::ScreamingKebabCase => val.to_shouty_kebab_case(),
        }
    }
}

impl FromStr for RenameRule {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebabCase),
            _ => Err("invalid rename rule".to_owned()),
        }
    }
}
