// Spreadsheet column letters, both directions.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

/// The largest column index [`col_to_letter`] and [`letter_to_col`] round-trip.
///
/// Bijective base 26 grows fast, so the ceiling is nowhere near a real table: it exists to
/// give absurd input (`letter_to_col("AAAAAAAAAAAA")`) a defined answer instead of an
/// arithmetic overflow. Spreadsheets stop at `XFD` (16383); this is far past that.
pub const MAX_COL: usize = 0x00FF_FFFF;

/// Column letters to a 0-based index: `A` → 0, `Z` → 25, `AA` → 26. Letters are uppercased
/// first, so `af` and `AF` are the same column.
///
/// `None` when `s` is empty, holds anything but ASCII letters, or names a column past
/// [`MAX_COL`]. It is total by design — a GUI parses whatever the user has typed so far,
/// and a half-typed address must not panic.
pub fn letter_to_col(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let mut n: usize = 0;
    for ch in s.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        // Bijective base 26: no zero digit, so `A` is 1 here and the -1 comes at the end.
        n = n
            .checked_mul(26)?
            .checked_add(ch.to_ascii_uppercase() as usize - 'A' as usize + 1)?;
        if n > MAX_COL + 1 {
            return None;
        }
    }
    Some(n - 1)
}

/// A 0-based index to column letters. Inverse of [`letter_to_col`].
///
/// It is base 26 without a zero, which is why the carry subtracts one.
pub fn col_to_letter(mut c: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (c % 26) as u8);
        if c < 26 {
            break;
        }
        c = c / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).expect("every byte pushed is an ASCII letter")
}

/// A cell as a user would write it: `(0, 0)` → `A1`. Rows are 0-based here and 1-based on
/// the page, matching every spreadsheet.
pub fn cell_ref(row: usize, col: usize) -> String {
    format!("{}{}", col_to_letter(col), row + 1)
}

/// A rectangle as a user would write it: `A1:C5`. A single cell renders without the colon.
pub fn rect_ref(r1: usize, c1: usize, r2: usize, c2: usize) -> String {
    let (r1, r2) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
    let (c1, c2) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    if r1 == r2 && c1 == c2 {
        cell_ref(r1, c1)
    } else {
        format!("{}:{}", cell_ref(r1, c1), cell_ref(r2, c2))
    }
}

/// A column name, bracketed and escaped so [`fn@crate::parse`] reads back the name given.
///
/// A literal `]` inside a name doubles, the same escape the parser accepts.
pub fn quote_name(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_twenty_six_columns_are_single_letters() {
        assert_eq!(col_to_letter(0), "A");
        assert_eq!(col_to_letter(25), "Z");
        assert_eq!(col_to_letter(26), "AA");
    }

    #[test]
    fn letters_round_trip() {
        for c in [0usize, 1, 25, 26, 27, 51, 52, 701, 702, 16383, MAX_COL] {
            assert_eq!(letter_to_col(&col_to_letter(c)), Some(c), "at {c}");
        }
    }

    #[test]
    fn letters_are_case_insensitive() {
        assert_eq!(letter_to_col("af"), letter_to_col("AF"));
    }

    #[test]
    fn junk_letters_are_none_not_a_panic() {
        assert_eq!(letter_to_col(""), None);
        assert_eq!(letter_to_col("A1"), None);
        assert_eq!(letter_to_col("-"), None);
        assert_eq!(letter_to_col("AAAAAAAAAAAAAAAAAAAA"), None);
    }

    #[test]
    fn cells_and_rects_render_one_based_rows() {
        assert_eq!(cell_ref(0, 0), "A1");
        assert_eq!(cell_ref(4, 2), "C5");
        assert_eq!(rect_ref(0, 0, 4, 2), "A1:C5");
        assert_eq!(rect_ref(4, 2, 0, 0), "A1:C5");
        assert_eq!(rect_ref(1, 1, 1, 1), "B2");
    }

    #[test]
    fn quoted_names_escape_brackets() {
        assert_eq!(quote_name("first name"), "[first name]");
        assert_eq!(quote_name("weird]name"), "[weird]]name]");
    }
}
