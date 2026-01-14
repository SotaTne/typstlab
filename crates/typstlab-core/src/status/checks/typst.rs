//! Typst installation check

use crate::status::engine::{CheckContext, CheckResult, StatusCheck};

pub struct TypstCheck;

impl StatusCheck for TypstCheck {
    fn name(&self) -> &str {
        "typst"
    }

    fn run(&self, _context: &CheckContext) -> CheckResult {
        // TODO: Implement typst checks in Phase 5
        CheckResult::pass()
    }
}
