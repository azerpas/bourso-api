use std::io::{stdout, Write};

pub struct TextProgressBar {
    width: usize,
}

impl TextProgressBar {
    pub fn new(width: usize) -> Self {
        Self { width }
    }

    pub fn render(&self, step: usize, total: usize, description: &str) {
        let (percentage, filled) = if total > 0 {
            let percentage = (step as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
            let filled = ((self.width as f32) * (step as f32 / total as f32)) as usize;
            (percentage, filled)
        } else {
            (0.0, 0usize)
        };

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(self.width - filled));

        print!(
            "\x1B[2K\r[{}] {:3.0}% - {}/{} - {}",
            bar, percentage, step, total, description
        );
        let _ = stdout().flush();
    }

    pub fn finish(&self) {
        println!();
    }
}
