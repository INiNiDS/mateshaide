use crate::core::color::Color;

pub const TRANSPARENT: Color = Color::TRANSPARENT;
pub const BLACK: Color = Color::BLACK;
pub const WHITE: Color = Color::WHITE;

pub const GRAY_50: Color = Color::hex(0xf9fafb);
pub const GRAY_100: Color = Color::hex(0xf3f4f6);
pub const GRAY_200: Color = Color::hex(0xe5e7eb);
pub const GRAY_300: Color = Color::hex(0xd1d5db);
pub const GRAY_400: Color = Color::hex(0x9ca3af);
pub const GRAY_500: Color = Color::hex(0x6b7280);
pub const GRAY_600: Color = Color::hex(0x4b5563);
pub const GRAY_700: Color = Color::hex(0x374151);
pub const GRAY_800: Color = Color::hex(0x1f2937);
pub const GRAY_900: Color = Color::hex(0x111827);

pub const RED_500: Color = Color::hex(0xef4444);
pub const ORANGE_500: Color = Color::hex(0xf97316);
pub const AMBER_500: Color = Color::hex(0xf59e0b);
pub const YELLOW_500: Color = Color::hex(0xeab308);
pub const LIME_500: Color = Color::hex(0x84cc16);
pub const GREEN_500: Color = Color::hex(0x22c55e);
pub const EMERALD_500: Color = Color::hex(0x10b981);
pub const TEAL_500: Color = Color::hex(0x14b8a6);
pub const CYAN_500: Color = Color::hex(0x06b6d4);
pub const SKY_500: Color = Color::hex(0x0ea5e9);
pub const BLUE_500: Color = Color::hex(0x3b82f6);
pub const INDIGO_500: Color = Color::hex(0x6366f1);
pub const VIOLET_500: Color = Color::hex(0x8b5cf6);
pub const PURPLE_500: Color = Color::hex(0xa855f7);
pub const FUCHSIA_500: Color = Color::hex(0xd946ef);
pub const PINK_500: Color = Color::hex(0xec4899);
pub const ROSE_500: Color = Color::hex(0xf43f5e);

pub const SUCCESS: Color = GREEN_500;
pub const WARNING: Color = AMBER_500;
pub const ERROR: Color = RED_500;
pub const INFO: Color = BLUE_500;
pub const ACCENT: Color = INDIGO_500;

pub const CANVAS_BG: Color = Color::hex(0xf0f1f4);
pub const PANEL_BG: Color = GRAY_50;
pub const BORDER: Color = GRAY_300;
pub const TEXT_PRIMARY: Color = GRAY_900;
pub const TEXT_SECONDARY: Color = GRAY_500;
