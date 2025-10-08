//! Formats registry, capabilities and pretty table rendering.

use comfy_table::{presets::UTF8_FULL, Attribute, Cell, CellAlignment, Color, ContentArrangement, Row, Table};
use strip_ansi_escapes::strip as strip_ansi;
use terminal_size::{terminal_size, Width};
use unicode_width::UnicodeWidthStr;

use crate::utils::colors::{BLUE, RESET, YELLOW};
/// Accepted formats for input and output.
///
/// Notes:
/// - "Archive" formats can hold multiple files (tar/zip/7z/rar).
/// - "Compressor" formats compress a single stream; combine with `tar` for folders.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CompressionFormat {
    // Archive formats
    Tar,
    Zip,
    #[cfg(feature = "unrar")]
    Rar,
    SevenZip,

    // Compressors
    Gzip,
    Bzip,
    Bzip3,
    Xz,
    Lzma,
    Lzip,
    Lz4,
    Snappy,
    Zstd,
    Brotli,
}

impl CompressionFormat {
    /// Returns whether this format is an archive (can hold multiple files).
    #[inline]
    pub fn is_archive(&self) -> bool {
        match self {
            Self::Tar | Self::Zip | Self::SevenZip => true,
            #[cfg(feature = "unrar")]
            Self::Rar => true,
            _ => false,
        }
    }

    /// Returns a human-friendly long name for the format (role-free).
    #[inline]
    pub fn long_name(&self) -> &'static str {
        match self {
            Self::Tar => "Tar",
            Self::Zip => "ZIP",
            Self::SevenZip => "7-Zip",
            #[cfg(feature = "unrar")]
            Self::Rar => "RAR",

            Self::Gzip => "Gzip",
            Self::Bzip => "Bzip2",
            Self::Bzip3 => "Bzip3",
            Self::Xz => "XZ (LZMA2)",
            Self::Lzma => "LZMA (v1)",
            Self::Lzip => "Lzip",
            Self::Lz4 => "LZ4",
            Self::Snappy => "Snappy (sz)",
            Self::Zstd => "Zstandard",
            Self::Brotli => "Brotli",
        }
    }

    /// Returns (can_compress, can_decompress) as supported by this tool.
    #[inline]
    pub fn capabilities(&self) -> (bool, bool) {
        match self {
            // Archives
            Self::Tar => (true, true),
            Self::Zip => (true, true),
            Self::SevenZip => (true, true),
            #[cfg(feature = "unrar")]
            Self::Rar => (false, true),

            // Pure compressors
            Self::Gzip => (true, true),
            Self::Bzip => (true, true),
            Self::Bzip3 => (true, true),
            Self::Xz => (true, true),
            Self::Lzma => (false, true), // LZMA1 compression not supported (use .xz)
            Self::Lzip => (false, true), // Lzip compression not supported
            Self::Lz4 => (true, true),
            Self::Snappy => (true, true),
            Self::Zstd => (true, true),
            Self::Brotli => (true, true),
        }
    }

    /// Optional notes to show in the table (no capability duplication here).
    #[inline]
    pub fn notes(&self) -> Option<&'static str> {
        match self {
            Self::Zip | Self::SevenZip => Some("cannot be streamed when chained"),
            _ => None,
        }
    }

    /// Canonical extension to display (single token, no shorthands).
    #[inline]
    pub fn canonical_ext(&self) -> &'static str {
        match self {
            Self::Tar => "tar",
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            #[cfg(feature = "unrar")]
            Self::Rar => "rar",

            Self::Gzip => "gz",
            Self::Bzip => "bz2",
            Self::Bzip3 => "bz3",
            Self::Xz => "xz",
            Self::Lzma => "lzma",
            Self::Lzip => "lz",
            Self::Lz4 => "lz4",
            Self::Snappy => "sz",
            Self::Zstd => "zst",
            Self::Brotli => "br",
        }
    }

    /// Real aliases (not chain shorthands like `tgz`).
    #[inline]
    pub fn real_aliases(&self) -> &'static [&'static str] {
        match self {
            Self::Bzip => &["bz"],   // bz -> bz2
            Self::Lzip => &["lzip"], // lzip -> lz
            _ => &[],
        }
    }
}

/// Registry macro: single source of truth for extensions/shorthands.
///
/// Generates:
/// - `KNOWN_SINGLE_EXTS: &[&str]`
/// - `KNOWN_SHORTHANDS: &[&str]`
/// - `ext_to_formats(ext: &str) -> Option<&'static [CompressionFormat]>`
macro_rules! define_ext_registry {
    (
        singles { $(
            $( #[$smeta:meta] )*
            $s:literal => $sfmt:path
        ),* $(,)? }
        shorthands { $(
            $( #[$hmeta:meta] )*
            $h:literal => [$($hfmt:path),+]
        ),* $(,)? }
    ) => {
        pub const KNOWN_SINGLE_EXTS: &[&str] = &[
            $(
                $( #[$smeta] )*
                $s
            ),*
        ];

        pub const KNOWN_SHORTHANDS: &[&str] = &[
            $(
                $( #[$hmeta] )*
                $h
            ),*
        ];

        #[inline]
        pub fn ext_to_formats(ext: &str) -> Option<&'static [CompressionFormat]> {
            match ext {
                $(
                    $( #[$smeta] )*
                    $s => Some(&[$sfmt]),
                )*
                $(
                    $( #[$hmeta] )*
                    $h => Some(&[$($hfmt),+]),
                )*
                _ => None,
            }
        }
    }
}

define_ext_registry! {
    singles {
        "tar"  => CompressionFormat::Tar,
        "zip"  => CompressionFormat::Zip,
        #[cfg(feature = "unrar")]
        "rar"  => CompressionFormat::Rar,
        "7z"   => CompressionFormat::SevenZip,

        "gz"   => CompressionFormat::Gzip,
        "bz"   => CompressionFormat::Bzip,
        "bz2"  => CompressionFormat::Bzip,
        "bz3"  => CompressionFormat::Bzip3,
        "xz"   => CompressionFormat::Xz,
        "lzma" => CompressionFormat::Lzma,
        "lz"   => CompressionFormat::Lzip,
        "lz4"  => CompressionFormat::Lz4,
        "sz"   => CompressionFormat::Snappy,
        "zst"  => CompressionFormat::Zstd,
        "br"   => CompressionFormat::Brotli,
    }
    shorthands {
        "tgz"   => [CompressionFormat::Tar, CompressionFormat::Gzip],
        "tbz"   => [CompressionFormat::Tar, CompressionFormat::Bzip],
        "tbz2"  => [CompressionFormat::Tar, CompressionFormat::Bzip],
        "tbz3"  => [CompressionFormat::Tar, CompressionFormat::Bzip3],
        "tlz4"  => [CompressionFormat::Tar, CompressionFormat::Lz4],
        "txz"   => [CompressionFormat::Tar, CompressionFormat::Xz],
        "tlzma" => [CompressionFormat::Tar, CompressionFormat::Lzma],
        "tlz"   => [CompressionFormat::Tar, CompressionFormat::Lzip],
        "tsz"   => [CompressionFormat::Tar, CompressionFormat::Snappy],
        "tzst"  => [CompressionFormat::Tar, CompressionFormat::Zstd],
    }
}

/// Which subset of formats to show in the table.
#[derive(Copy, Clone)]
pub enum SupportedOp {
    Any,
    Compress,
    Decompress,
}

/// Small explanation printed before the capability table.
pub fn intro() -> String {
    // Colorize ROLE keywords exactly like in the table (archive = blue, compressor = yellow).
    format!(
        "\
ROLE column:
  • {b}archive{r} — holds multiple files in one container (e.g., tar, zip, 7z)
  • {y}compressor{r} — compresses a single stream; use with 'tar' for directories

COMPRESS / DECOMPRESS columns show what this tool supports for each format.",
        b = *BLUE,
        y = *YELLOW,
        r = *RESET,
    )
}

/// Formats in display order: archives first, then compressors.
fn formats_in_display_order() -> Vec<CompressionFormat> {
    vec![
        CompressionFormat::Tar,
        CompressionFormat::Zip,
        #[cfg(feature = "unrar")]
        CompressionFormat::Rar,
        CompressionFormat::SevenZip,
        CompressionFormat::Gzip,
        CompressionFormat::Bzip,
        CompressionFormat::Bzip3,
        CompressionFormat::Xz,
        CompressionFormat::Lzma,
        CompressionFormat::Lzip,
        CompressionFormat::Lz4,
        CompressionFormat::Snappy,
        CompressionFormat::Zstd,
        CompressionFormat::Brotli,
    ]
}

/// Pretty capability table. `yes` -> green, `no` -> red, ROLE is colored.
pub fn table(op: SupportedOp) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("FORMAT").add_attribute(Attribute::Bold),
        Cell::new("LONG NAME").add_attribute(Attribute::Bold),
        Cell::new("ROLE").add_attribute(Attribute::Bold),
        Cell::new("COMPRESS").add_attribute(Attribute::Bold),
        Cell::new("DECOMPRESS").add_attribute(Attribute::Bold),
        Cell::new("NOTES").add_attribute(Attribute::Bold),
    ]);

    // Spacing & alignment
    if let Some(col) = table.column_mut(2) {
        col.set_padding((1, 2)); // ROLE
    }
    if let Some(col) = table.column_mut(3) {
        col.set_cell_alignment(CellAlignment::Center);
        col.set_padding((1, 1)); // COMPRESS
    }
    if let Some(col) = table.column_mut(4) {
        col.set_cell_alignment(CellAlignment::Center);
        col.set_padding((1, 1)); // DECOMPRESS
    }

    let yn = |b: bool| {
        if b {
            Cell::new("yes").fg(Color::Green)
        } else {
            Cell::new("no").fg(Color::Red)
        }
    };

    for f in formats_in_display_order() {
        let (c_ok, d_ok) = f.capabilities();

        // Filter by op
        if matches!(op, SupportedOp::Compress) && !c_ok {
            continue;
        }
        if matches!(op, SupportedOp::Decompress) && !d_ok {
            continue;
        }

        let role_cell = if f.is_archive() {
            Cell::new("archive").fg(Color::Blue)
        } else {
            Cell::new("compressor").fg(Color::Yellow)
        };

        // FORMAT cell text: include alias only for bzip as "bz2/bz"
        let format_text = match f {
            CompressionFormat::Bzip => "bz2/bz".to_string(),
            _ => f.canonical_ext().to_string(),
        };

        let notes = f.notes().unwrap_or("");

        table.add_row(Row::from(vec![
            Cell::new(format_text),
            Cell::new(f.long_name()),
            role_cell,
            yn(c_ok),
            yn(d_ok),
            Cell::new(notes),
        ]));
    }

    table.to_string()
}

/// Intro shown before shorthand table.
pub fn shorthands_intro() -> &'static str {
    "Shorthand chains (left → expands to right).\n\
These are filename shortcuts only.\n\
Example: 'tgz' expands to 'tar.gz'."
}

/// Pretty table for shorthands like `tgz -> tar.gz`.
pub fn shorthand_table() -> String {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_content_arrangement(ContentArrangement::Dynamic);

    t.set_header(vec![
        Cell::new("SHORTHAND").add_attribute(Attribute::Bold),
        Cell::new("EXPANDS TO").add_attribute(Attribute::Bold),
    ]);

    for &sh in KNOWN_SHORTHANDS {
        if let Some(chain) = ext_to_formats(sh) {
            // Render as "tar.gz", "tar.xz", ...
            let expands = chain.iter().map(|f| f.canonical_ext()).collect::<Vec<_>>().join(".");

            t.add_row(Row::from(vec![Cell::new(sh), Cell::new(expands)]));
        }
    }

    t.to_string()
}

fn env_terminal_width() -> Option<usize> {
    if let Some((Width(w), _)) = terminal_size() {
        Some(w as usize)
    } else {
        std::env::var("COLUMNS").ok().and_then(|s| s.parse::<usize>().ok())
    }
}

fn visible_width(s: &str) -> usize {
    // Strip ANSI, then compute display width (Unicode-aware).
    let bytes = strip_ansi(s.as_bytes()); // returns Vec<u8> in 0.2.x
    let clean = String::from_utf8_lossy(&bytes);
    UnicodeWidthStr::width(clean.as_ref())
}

/// Pads the top of the shorter block so both have the same initial line count.
/// Used only for side-by-side layout so headings align vertically.
fn pad_top_to_equal_lines(left: &str, right: &str) -> (String, String) {
    let l = left.lines().count();
    let r = right.lines().count();
    if l == r {
        return (left.to_string(), right.to_string());
    }
    if l < r {
        let pad = "\n".repeat(r - l);
        (format!("{pad}{left}"), right.to_string())
    } else {
        let pad = "\n".repeat(l - r);
        (left.to_string(), format!("{pad}{right}"))
    }
}

fn side_by_side(left: &str, right: &str, min_gap: usize) -> Option<String> {
    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();

    // Compute visible widths, ignoring ANSI sequences.
    let left_w = left_lines.iter().map(|l| visible_width(l)).max().unwrap_or(0);
    let right_w = right_lines.iter().map(|l| visible_width(l)).max().unwrap_or(0);

    // Need a bit of breathing room.
    let total_needed = left_w + min_gap + right_w;
    let term_w = env_terminal_width().unwrap_or(0);

    if term_w > 0 && total_needed + 2 <= term_w {
        let mut out = String::new();
        let rows = left_lines.len().max(right_lines.len());
        for i in 0..rows {
            let l = if i < left_lines.len() { left_lines[i] } else { "" };
            let r = if i < right_lines.len() { right_lines[i] } else { "" };

            // Pad using *visible* width so ANSI colors don’t break alignment.
            let l_pad = left_w.saturating_sub(visible_width(l));
            out.push_str(l);
            for _ in 0..l_pad {
                out.push(' ');
            }
            for _ in 0..min_gap {
                out.push(' ');
            }
            out.push_str(r);
            out.push('\n');
        }
        Some(out)
    } else {
        None
    }
}

/// Renders capability intro+table and shorthands intro+table.
/// If there is enough terminal width (via $COLUMNS), prints them side by side
/// and top-aligns headings with conditional padding. Otherwise, prints them
/// stacked with a blank line in between (no extra padding in this path).
pub fn render_capabilities_and_shorthands(op: SupportedOp) -> String {
    // Unpadded blocks for the vertical (stacked) fallback.
    let left_intro_plain = intro();
    let right_intro_plain = shorthands_intro();
    let left_plain = format!("{left_intro_plain}\n\n{}", table(op));
    let right_plain = format!("{right_intro_plain}\n\n{}", shorthand_table());

    // Side-by-side attempt with top padding to align headings.
    let (left_intro_pad, right_intro_pad) = pad_top_to_equal_lines(left_intro_plain.as_str(), right_intro_plain);
    let left_sbs = format!("{left_intro_pad}\n\n{}", table(op));
    let right_sbs = format!("{right_intro_pad}\n\n{}", shorthand_table());

    if let Some(sbs) = side_by_side(left_sbs.as_str(), right_sbs.as_str(), 4) {
        sbs
    } else {
        format!("{left_plain}\n\n{right_plain}")
    }
}

/// Returns a comma-separated list of canonical single extensions supported for the given op.
/// Shorthands are NOT included here.
pub fn compact_extensions(op: SupportedOp) -> String {
    let mut exts = Vec::new();
    for f in formats_in_display_order() {
        let (can_c, can_d) = f.capabilities();
        if matches!(op, SupportedOp::Compress) && !can_c {
            continue;
        }
        if matches!(op, SupportedOp::Decompress) && !can_d {
            continue;
        }
        // Canonical single token (e.g., "bz2", "zst", "tar", "zip"...)
        let ext = f.canonical_ext();
        if !exts.contains(&ext) {
            exts.push(ext);
        }
    }
    exts.join(", ")
}

/// Returns a comma-separated list of all supported shorthands (e.g., "tgz, tbz, ...").
pub fn compact_shorthands() -> String {
    KNOWN_SHORTHANDS.join(", ")
}
