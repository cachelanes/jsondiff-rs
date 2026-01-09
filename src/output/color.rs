use colored::Color;

pub struct ColorScheme {
    pub added: Color,
    pub removed: Color,
    pub modified_old: Color,
    pub modified_new: Color,
    pub path: Color,
    pub separator: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            added: Color::Green,
            removed: Color::Red,
            modified_old: Color::Red,
            modified_new: Color::Green,
            path: Color::Cyan,
            separator: Color::BrightBlack,
        }
    }
}
