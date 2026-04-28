use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEvent<P>
where
    P: Clone + Debug + 'static,
{
    pub scope: EventScope,
    pub level: EventLevel,
    pub presentation: EventPresentation,
    pub audience: EventAudience,
    pub payload: P,
}

impl<P> AppEvent<P>
where
    P: Clone + Debug + 'static,
{
    pub fn line(scope: EventScope, payload: P) -> Self {
        Self {
            scope,
            level: EventLevel::Normal,
            presentation: EventPresentation::Line,
            audience: EventAudience::All,
            payload,
        }
    }

    pub fn verbose(scope: EventScope, payload: P) -> Self {
        Self {
            scope,
            level: EventLevel::Verbose,
            presentation: EventPresentation::Line,
            audience: EventAudience::All,
            payload,
        }
    }

    pub fn cli_progress(scope: EventScope, payload: P) -> Self {
        Self {
            scope,
            level: EventLevel::Normal,
            presentation: EventPresentation::Progress,
            audience: EventAudience::CliOnly,
            payload,
        }
    }

    pub fn verbose_cli_progress(scope: EventScope, payload: P) -> Self {
        Self {
            scope,
            level: EventLevel::Verbose,
            presentation: EventPresentation::Progress,
            audience: EventAudience::CliOnly,
            payload,
        }
    }

    pub fn map_payload<Q, F>(self, map: F) -> AppEvent<Q>
    where
        Q: Clone + Debug + 'static,
        F: FnOnce(P) -> Q,
    {
        AppEvent {
            scope: self.scope,
            level: self.level,
            presentation: self.presentation,
            audience: self.audience,
            payload: map(self.payload),
        }
    }

    pub const fn visible_in_cli(&self, verbose: bool) -> bool {
        !matches!(self.audience, EventAudience::McpOnly)
            && (verbose || matches!(self.level, EventLevel::Normal))
    }

    pub const fn visible_in_mcp(&self, verbose: bool) -> bool {
        !matches!(self.audience, EventAudience::CliOnly)
            && (verbose || matches!(self.level, EventLevel::Normal))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventScope {
    pub action: &'static str,
    pub label: Option<String>,
}

impl EventScope {
    pub const fn new(action: &'static str) -> Self {
        Self {
            action,
            label: None,
        }
    }

    pub fn labeled(action: &'static str, label: impl Into<String>) -> Self {
        Self {
            action,
            label: Some(label.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventLevel {
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPresentation {
    Line,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAudience {
    All,
    CliOnly,
    McpOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestPayload {
        Started,
        Wrapped(&'static str),
    }

    #[test]
    fn test_line_event_defaults_to_normal_all_audience() {
        let event = AppEvent::line(EventScope::new("load"), TestPayload::Started);

        assert_eq!(event.scope, EventScope::new("load"));
        assert_eq!(event.level, EventLevel::Normal);
        assert_eq!(event.presentation, EventPresentation::Line);
        assert_eq!(event.audience, EventAudience::All);
        assert_eq!(event.payload, TestPayload::Started);
    }

    #[test]
    fn test_verbose_cli_progress_is_cli_only_progress() {
        let event = AppEvent::verbose_cli_progress(
            EventScope::labeled("download_docs", "0.14.2"),
            TestPayload::Started,
        );

        assert_eq!(
            event.scope,
            EventScope {
                action: "download_docs",
                label: Some("0.14.2".to_string())
            }
        );
        assert_eq!(event.level, EventLevel::Verbose);
        assert_eq!(event.presentation, EventPresentation::Progress);
        assert_eq!(event.audience, EventAudience::CliOnly);
    }

    #[test]
    fn test_map_payload_preserves_metadata() {
        let event = AppEvent::verbose(
            EventScope::labeled("resolve_docs", "0.14.2"),
            TestPayload::Started,
        );

        let mapped = event.map_payload(|_| TestPayload::Wrapped("docs"));

        assert_eq!(mapped.scope.action, "resolve_docs");
        assert_eq!(mapped.scope.label.as_deref(), Some("0.14.2"));
        assert_eq!(mapped.level, EventLevel::Verbose);
        assert_eq!(mapped.presentation, EventPresentation::Line);
        assert_eq!(mapped.audience, EventAudience::All);
        assert_eq!(mapped.payload, TestPayload::Wrapped("docs"));
    }

    #[test]
    fn test_visible_in_cli_filters_verbose_events_without_verbose_mode() {
        let event = AppEvent::verbose(EventScope::new("load"), TestPayload::Started);

        assert!(!event.visible_in_cli(false));
        assert!(event.visible_in_cli(true));
    }

    #[test]
    fn test_visible_in_cli_filters_mcp_only_events() {
        let event = AppEvent {
            scope: EventScope::new("mcp"),
            level: EventLevel::Normal,
            presentation: EventPresentation::Line,
            audience: EventAudience::McpOnly,
            payload: TestPayload::Started,
        };

        assert!(!event.visible_in_cli(true));
    }

    #[test]
    fn test_visible_in_mcp_filters_cli_only_events() {
        let event = AppEvent::cli_progress(EventScope::new("download"), TestPayload::Started);

        assert!(!event.visible_in_mcp(true));
    }

    #[test]
    fn test_visible_in_mcp_filters_verbose_events_without_verbose_mode() {
        let event = AppEvent::verbose(EventScope::new("resolve"), TestPayload::Started);

        assert!(!event.visible_in_mcp(false));
        assert!(event.visible_in_mcp(true));
    }
}
