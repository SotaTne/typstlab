use chrono::NaiveDate;

use super::source::{ParamSource, SourceValue};
use super::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinContext {
    pub today: NaiveDate,
}

impl ParamSource for BuiltinContext {
    fn resolve_param(&self, key: &str) -> SourceValue {
        match key {
            "today" => SourceValue::Found(Value::Date(self.today)),
            _ => SourceValue::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn resolves_today_builtin() {
        let context = BuiltinContext {
            today: date(2026, 5, 22),
        };

        assert_eq!(
            context.resolve_param("today"),
            SourceValue::Found(Value::Date(date(2026, 5, 22)))
        );
    }

    #[test]
    fn resolves_unknown_builtin_key() {
        let context = BuiltinContext {
            today: date(2026, 5, 22),
        };

        assert_eq!(context.resolve_param("unknown"), SourceValue::Unknown);
    }
}
