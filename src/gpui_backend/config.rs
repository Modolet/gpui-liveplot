/// Configuration for the GPUI plot view.
#[derive(Debug, Clone)]
pub struct PlotViewConfig {
    /// Pixel threshold for starting a drag.
    pub drag_threshold_px: f32,
    /// Pixel threshold for pin hit testing.
    pub pin_threshold_px: f32,
    /// Pixel threshold for unpin hit testing.
    pub unpin_threshold_px: f32,
    /// Padding fraction applied when auto-fitting data.
    pub padding_frac: f64,
    /// Minimum padding applied when auto-fitting data.
    pub min_padding: f64,
    /// Show legend overlay.
    pub show_legend: bool,
    /// Show hover coordinate readout.
    pub show_hover: bool,
}

impl Default for PlotViewConfig {
    fn default() -> Self {
        Self {
            drag_threshold_px: 4.0,
            pin_threshold_px: 12.0,
            unpin_threshold_px: 18.0,
            padding_frac: 0.05,
            min_padding: 1e-6,
            show_legend: true,
            show_hover: true,
        }
    }
}
