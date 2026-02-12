//! Style and theming placeholders.

/// Visual theme for plots.
#[derive(Debug, Clone, Default)]
pub struct Theme {
    _private: (),
}

impl Theme {
    /// Create the default theme.
    pub fn new() -> Self {
        Self::default()
    }
}
