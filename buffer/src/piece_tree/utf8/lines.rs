use super::chars::Chars;

// LF: Line Feed, U+000A (UTF-8 in hex: 0A)
// VT: Vertical Tab, U+000B (UTF-8 in hex: 0B)
// FF: Form Feed, U+000C (UTF-8 in hex: 0C)
// CR: Carriage Return, U+000D (UTF-8 in hex: 0D)
// CR+LF: CR (U+000D) followed by LF (U+000A) (UTF-8 in hex: 0D 0A)
// NEL: Next Line, U+0085 (UTF-8 in hex: C2 85)
// LS: Line Separator, U+2028 (UTF-8 in hex: E2 80 A8)
// PS: Paragraph Separator, U+2029 (UTF-8 in hex: E2 80 A9)

#[derive(Debug, Clone)]
pub struct Lines<'a> {
    chars: Chars<'a>,
}
