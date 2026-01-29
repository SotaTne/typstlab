#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    VersionUnsupported,
    // ParseError,
    // LintFoo,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::VersionUnsupported => "E_VERSION_UNSUPPORTED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(
            ErrorCode::VersionUnsupported.as_str(),
            "E_VERSION_UNSUPPORTED"
        );
    }
}
