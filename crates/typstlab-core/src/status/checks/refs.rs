//! References check - validates bibliography setup

use crate::status::engine::{CheckContext, CheckResult, StatusCheck};

pub struct RefsCheck;

impl StatusCheck for RefsCheck {
    fn name(&self) -> &str {
        "refs"
    }

    fn run(&self, _context: &CheckContext) -> CheckResult {
        // TODO: Implement refs checks in Phase 5
        CheckResult::pass()
    }
}
