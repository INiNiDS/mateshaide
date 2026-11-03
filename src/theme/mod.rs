use crate::Color;
use serde::{Deserialize, Serialize};

pub mod palette;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Appearance {
    pub dark: bool,
    pub halloween: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub canvas: Color,
    pub panel: Color,
    pub surface: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub button_text: Color,
    pub dark: bool,
    pub halloween: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Appearance::default().theme()
    }
}

impl Appearance {
    #[must_use]
    pub fn theme(self) -> Theme {
        let mut theme = if self.dark {
            Theme {
                canvas: Color::hex(0x0b0e14),
                panel: Color::hex(0x111827),
                surface: Color::hex(0x1f2937),
                border: Color::hex(0x4b5563),
                text: Color::hex(0xf3f4f6),
                muted: Color::hex(0xb4bdcb),
                accent: Color::hex(0x6366b9),
                secondary: Color::hex(0x465776),
                success: Color::hex(0x6ee7b7),
                warning: Color::hex(0xfbbf24),
                error: Color::hex(0xfca5a5),
                button_text: Color::WHITE,
                dark: true,
                halloween: self.halloween,
            }
        } else {
            Theme {
                canvas: palette::CANVAS_BG,
                panel: Color::WHITE,
                surface: Color::hex(0xf3f4f6),
                border: Color::hex(0xc5cbd5),
                text: palette::TEXT_PRIMARY,
                muted: Color::hex(0x4b5563),
                accent: Color::hex(0x6250ac),
                secondary: Color::hex(0x465776),
                success: Color::hex(0x166534),
                warning: Color::hex(0x92400e),
                error: Color::hex(0xb91c1c),
                button_text: Color::WHITE,
                dark: false,
                halloween: self.halloween,
            }
        };

        if self.halloween {
            theme.accent = Color::hex(0xb45316);
            theme.secondary = Color::hex(0x68458a);

            if self.dark {
                theme.canvas = Color::hex(0x100e17);
                theme.panel = Color::hex(0x1b1724);
                theme.surface = Color::hex(0x262030);
                theme.border = Color::hex(0x655178);
                theme.text = Color::hex(0xeee8f5);
                theme.muted = Color::hex(0xb5a9c2);
            } else {
                theme.canvas = Color::hex(0xf7f0e5);
                theme.panel = Color::hex(0xfffaf2);
                theme.surface = Color::hex(0xeee4d9);
                theme.border = Color::hex(0x9b829f);
            }
        }

        theme
    }
}

impl Theme {
    #[must_use]
    pub fn display_ink(self, color: Color) -> Color {
        let max = color.r.max(color.g).max(color.b);
        let min = color.r.min(color.g).min(color.b);

        if self.dark && max <= 0.28 && max - min <= 0.12 {
            self.text.with_alpha(color.a)
        } else {
            color
        }
    }
}
