use chrono::Local;

/// Events that can be logged in the devtools timeline.
#[derive(Clone, Debug)]
pub enum DevtoolsEvent {
    /// A navigation was initiated.
    Navigate { from: String, to: String },
    /// A navigation completed and the route was matched.
    NavigateComplete { path: String, route_id: String },
    /// A loader started fetching data.
    LoaderStart { route_id: String },
    /// A loader completed successfully.
    LoaderComplete { route_id: String, duration_ms: u64 },
    /// A loader failed with an error.
    LoaderError { route_id: String, error: String },
    /// A path was matched to a route.
    Match { path: String, route_id: String },
    /// A path did not match any route.
    NoMatch { path: String },
    /// A route was preloaded.
    Preload { route_id: String },
}

impl DevtoolsEvent {
    /// Short label for the event type badge.
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Navigate { .. } => "NAV",
            Self::NavigateComplete { .. } => "DONE",
            Self::LoaderStart { .. } => "LOAD",
            Self::LoaderComplete { .. } => "OK",
            Self::LoaderError { .. } => "ERR",
            Self::Match { .. } => "MATCH",
            Self::NoMatch { .. } => "MISS",
            Self::Preload { .. } => "PRE",
        }
    }

    /// RGB color for the badge.
    pub fn badge_color(&self) -> u32 {
        match self {
            Self::Navigate { .. } => 0x569cd6,
            Self::NavigateComplete { .. } => 0x4ec9b0,
            Self::LoaderStart { .. } => 0xdcdcaa,
            Self::LoaderComplete { .. } => 0x6a9955,
            Self::LoaderError { .. } => 0xf44747,
            Self::Match { .. } => 0x4ec9b0,
            Self::NoMatch { .. } => 0xf44747,
            Self::Preload { .. } => 0xc586c0,
        }
    }

    /// Human-readable description of the event.
    pub fn description(&self) -> String {
        match self {
            Self::Navigate { from, to } => format!("{} → {}", from, to),
            Self::NavigateComplete { path, route_id } => {
                format!("{} → [{}]", path, route_id)
            }
            Self::LoaderStart { route_id } => format!("Loading [{}]", route_id),
            Self::LoaderComplete {
                route_id,
                duration_ms,
            } => format!("[{}] loaded in {}ms", route_id, duration_ms),
            Self::LoaderError { route_id, error } => {
                format!("[{}] error: {}", route_id, error)
            }
            Self::Match { path, route_id } => format!("{} matched [{}]", path, route_id),
            Self::NoMatch { path } => format!("No match for {}", path),
            Self::Preload { route_id } => format!("Preloading [{}]", route_id),
        }
    }
}

/// A logged event with a timestamp.
#[derive(Clone, Debug)]
pub struct LoggedEvent {
    /// Unix timestamp in milliseconds.
    pub timestamp_millis: i64,
    /// The event payload.
    pub event: DevtoolsEvent,
}

impl LoggedEvent {
    pub fn new(event: DevtoolsEvent) -> Self {
        Self {
            timestamp_millis: Local::now().timestamp_millis(),
            event,
        }
    }

    /// Format the timestamp as `HH:MM:SS.mmm`.
    pub fn formatted_time(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.timestamp_millis)
            .map(|dt| dt.format("%H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| "—".to_string())
    }
}
