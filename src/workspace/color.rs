//! Workspace accent colors.
//!
//! A workspace color is an sRGB triple stored and sent as lowercase `#rrggbb`.
//! Users pick one of the fixed light swatches or give any hex value.

use ratatui::style::Color;

use crate::app::state::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorkspaceColor {
    r: u8,
    g: u8,
    b: u8,
}

/// Light pastel swatches (OKLCH L=0.80, C=0.11) with evenly spread hues, in menu order.
pub const SWATCHES: [(&str, WorkspaceColor); 8] = [
    ("rose", swatch(0xfca0a7)),
    ("orange", swatch(0xf2ad73)),
    ("yellow", swatch(0xcec068)),
    ("green", swatch(0x90d192)),
    ("teal", swatch(0x59d4cd)),
    ("sky", swatch(0x71c9fa)),
    ("violet", swatch(0xbcb2ff)),
    ("pink", swatch(0xeea2d4)),
];

const fn swatch(hex: u32) -> WorkspaceColor {
    WorkspaceColor {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
    }
}

impl WorkspaceColor {
    /// Parses `#rrggbb`, `rrggbb`, `#rgb`, or a swatch name, ignoring case and
    /// surrounding whitespace. Does not allocate.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if let Some(&(_, color)) = SWATCHES
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(value))
        {
            return Some(color);
        }
        let (digits, short_form) = match value.strip_prefix('#') {
            Some(digits) => (digits, true),
            None => (value, false),
        };
        match *digits.as_bytes() {
            [r1, r2, g1, g2, b1, b2] => Some(Self {
                r: hex_byte(r1, r2)?,
                g: hex_byte(g1, g2)?,
                b: hex_byte(b1, b2)?,
            }),
            [r, g, b] if short_form => Some(Self {
                r: hex_byte(r, r)?,
                g: hex_byte(g, g)?,
                b: hex_byte(b, b)?,
            }),
            _ => None,
        }
    }

    /// Lowercase `#rrggbb`, the stored and wire form.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn rgb(self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    pub fn to_ratatui(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    /// Black or white, whichever reads better as text drawn on this color.
    pub fn contrast_fg(self) -> Color {
        crate::ui::contrast_fg_for_rgb(self.rgb())
    }

    /// This color drawn as text or a glyph on `palette`'s background,
    /// darkened on light palettes where the light swatches wash out.
    pub fn foreground(self, palette: &Palette) -> Color {
        crate::ui::foreground_on_palette(self.rgb(), palette)
    }
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    let high = char::from(high).to_digit(16)?;
    let low = char::from(low).to_digit(16)?;
    u8::try_from(high * 16 + low).ok()
}

/// Picks the swatch that `others` use least, never `exclude`. Ties go to the
/// first such swatch in palette order, or to a `seed`-chosen one.
pub fn least_used(
    others: impl IntoIterator<Item = WorkspaceColor>,
    exclude: Option<WorkspaceColor>,
    seed: Option<u64>,
) -> WorkspaceColor {
    let mut uses = [0usize; SWATCHES.len()];
    for color in others {
        if let Some(index) = SWATCHES.iter().position(|&(_, swatch)| swatch == color) {
            uses[index] += 1;
        }
    }
    let candidates = || {
        SWATCHES
            .iter()
            .zip(uses)
            .filter(|&(&(_, swatch), _)| Some(swatch) != exclude)
    };
    let fewest = candidates().map(|(_, count)| count).min().unwrap_or(0);
    let tied = || {
        candidates()
            .filter(move |&(_, count)| count == fewest)
            .map(|(&(_, swatch), _)| swatch)
    };
    let pick = seed.map_or(0, |seed| seed % tied().count().max(1) as u64);
    tied().nth(pick as usize).unwrap_or(SWATCHES[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> WorkspaceColor {
        SWATCHES
            .iter()
            .find(|(swatch_name, _)| *swatch_name == name)
            .map(|&(_, color)| color)
            .unwrap()
    }

    #[test]
    fn parse_accepts_hex_forms_and_swatch_names() {
        let teal = Some(named("teal"));
        assert_eq!(WorkspaceColor::parse("#59d4cd"), teal);
        assert_eq!(WorkspaceColor::parse("59D4CD"), teal);
        assert_eq!(WorkspaceColor::parse("  #59D4cd\n"), teal);
        assert_eq!(WorkspaceColor::parse("teal"), teal);
        assert_eq!(WorkspaceColor::parse(" TeAl "), teal);
        assert_eq!(
            WorkspaceColor::parse("#F0a").map(WorkspaceColor::rgb),
            Some((0xff, 0x00, 0xaa))
        );
    }

    #[test]
    fn parse_rejects_invalid_input() {
        for invalid in [
            "",
            " ",
            "#",
            "f0a",
            "#f0",
            "#f0a0",
            "#f0a0a0a",
            "#gggggg",
            "#+f+f+f",
            "##f0a0a0",
            "#ééé",
            "red",
            "cyan",
            "random",
            "none",
            "rgb(1,2,3)",
        ] {
            assert_eq!(WorkspaceColor::parse(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn hex_round_trips_in_lowercase() {
        let color = WorkspaceColor::parse("#ABCDEF").unwrap();
        assert_eq!(color.to_hex(), "#abcdef");
        assert_eq!(WorkspaceColor::parse(&color.to_hex()), Some(color));
        assert_eq!(color.to_ratatui(), Color::Rgb(0xab, 0xcd, 0xef));
        for (_, swatch) in SWATCHES {
            assert_eq!(WorkspaceColor::parse(&swatch.to_hex()), Some(swatch));
        }
    }

    #[test]
    fn swatches_have_the_documented_names_and_values() {
        let swatches: Vec<_> = SWATCHES
            .iter()
            .map(|&(name, color)| (name, color.to_hex()))
            .collect();
        assert_eq!(
            swatches,
            [
                ("rose", "#fca0a7"),
                ("orange", "#f2ad73"),
                ("yellow", "#cec068"),
                ("green", "#90d192"),
                ("teal", "#59d4cd"),
                ("sky", "#71c9fa"),
                ("violet", "#bcb2ff"),
                ("pink", "#eea2d4"),
            ]
            .map(|(name, hex)| (name, hex.to_string()))
        );
    }

    #[test]
    fn contrast_fg_picks_black_on_light_swatches_and_white_on_dark_colors() {
        for (_, swatch) in SWATCHES {
            assert_eq!(swatch.contrast_fg(), Color::Rgb(0, 0, 0));
        }
        let navy = WorkspaceColor::parse("#101040").unwrap();
        assert_eq!(navy.contrast_fg(), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn least_used_prefers_unused_swatches_in_palette_order() {
        assert_eq!(least_used([], None, None), named("rose"));
        assert_eq!(
            least_used([named("rose"), named("orange")], None, None),
            named("yellow")
        );
        // Custom colors do not count against any swatch.
        let custom = WorkspaceColor::parse("#123456").unwrap();
        assert_eq!(least_used([custom], None, None), named("rose"));
    }

    #[test]
    fn least_used_never_returns_the_excluded_color() {
        assert_eq!(least_used([], Some(named("rose")), None), named("orange"));
        // The current color is skipped even when it is the only unused swatch.
        let others: Vec<_> = SWATCHES[1..].iter().map(|&(_, color)| color).collect();
        assert_eq!(
            least_used(others.clone(), Some(named("rose")), None),
            named("orange")
        );
        for seed in 0..32 {
            assert_ne!(
                least_used(others.clone(), Some(named("rose")), Some(seed)),
                named("rose")
            );
        }
    }

    #[test]
    fn least_used_wraps_around_once_every_swatch_is_used() {
        let mut others: Vec<_> = SWATCHES.iter().map(|&(_, color)| color).collect();
        assert_eq!(least_used(others.clone(), None, None), named("rose"));
        others.push(named("rose"));
        others.push(named("orange"));
        assert_eq!(least_used(others, None, None), named("yellow"));
    }

    #[test]
    fn least_used_seed_breaks_ties_only_among_the_least_used() {
        let others = [named("rose"), named("orange"), named("yellow")];
        let tied = [
            named("green"),
            named("teal"),
            named("sky"),
            named("violet"),
            named("pink"),
        ];
        for seed in 0..10u64 {
            let picked = least_used(others, None, Some(seed));
            assert_eq!(picked, tied[(seed % 5) as usize]);
        }
    }
}
