use gpui::{TextRun, Window, font, px};

use crate::axis::TextMeasurer;

pub(crate) struct GpuiTextMeasurer<'a> {
    window: &'a Window,
}

impl<'a> GpuiTextMeasurer<'a> {
    pub(crate) fn new(window: &'a Window) -> Self {
        Self { window }
    }

    pub(crate) fn measure_multiline(&self, text: &str, size: f32) -> (f32, f32) {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for line in text.lines() {
            let (w, h) = self.measure(line, size);
            width = width.max(w);
            height += h.max(size * 1.2);
        }
        (width + 8.0, height + 8.0)
    }
}

impl TextMeasurer for GpuiTextMeasurer<'_> {
    fn measure(&self, text: &str, size: f32) -> (f32, f32) {
        if text.is_empty() {
            return (0.0, 0.0);
        }
        let run = TextRun {
            len: text.len(),
            font: font(".SystemUIFont"),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped =
            self.window
                .text_system()
                .shape_line(text.to_string().into(), px(size), &[run], None);
        let width = f32::from(shaped.width);
        let height = f32::from(shaped.ascent + shaped.descent);
        (width, height.max(size * 1.2))
    }
}
