use super::value::{Params, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceValue {
    Found(Value),
    Missing,
    Unknown,
}

pub trait ParamSource {
    fn resolve_param(&self, key: &str) -> SourceValue;
}

impl ParamSource for Params {
    fn resolve_param(&self, key: &str) -> SourceValue {
        self.get(key)
            .cloned()
            .map(SourceValue::Found)
            .unwrap_or(SourceValue::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_resolve_existing_value() {
        let mut params = Params::new();
        params.insert("author".to_string(), Value::String("SomeOne".to_string()));

        assert_eq!(
            params.resolve_param("author"),
            SourceValue::Found(Value::String("SomeOne".to_string()))
        );
    }

    #[test]
    fn params_resolve_unknown_key() {
        let params = Params::new();

        assert_eq!(params.resolve_param("author"), SourceValue::Unknown);
    }
}
