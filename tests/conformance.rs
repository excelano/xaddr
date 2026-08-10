// The address grammar, pinned.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)
//
// This file is the contract. xled, xshape, and Comma agree about addressing exactly as far as
// these cases hold — including the six forms that xled and xshape had silently disagreed about
// before they shared an implementation.

use xaddr::{parse, Bounds, Grid, Kind};

struct Sheet {
    header: Option<Vec<String>>,
    nrows: usize,
    ncols: usize,
}

impl Grid for Sheet {
    fn nrows(&self) -> usize {
        self.nrows
    }
    fn ncols(&self) -> usize {
        self.ncols
    }
    fn header(&self) -> Option<&[String]> {
        self.header.as_deref()
    }
}

/// Six columns, matching the fixture the two tools were compared against.
fn sheet() -> Sheet {
    Sheet {
        header: Some(
            ["first", "last", "dept", "fy2024", "fy2025", "fy2026"]
                .map(String::from)
                .to_vec(),
        ),
        nrows: 20,
        ncols: 6,
    }
}

fn headerless() -> Sheet {
    Sheet {
        header: None,
        nrows: 3,
        ncols: 2,
    }
}

fn cols(spec: &str) -> Vec<usize> {
    parse(spec)
        .unwrap()
        .columns(&sheet(), Bounds::Strict)
        .unwrap()
}

// ---------------------------------------------------------------- the agreed core

#[test]
fn letter_forms() {
    assert_eq!(cols("A"), vec![0]);
    assert_eq!(cols("D:F"), vec![3, 4, 5]);
    assert_eq!(cols("A:C"), vec![0, 1, 2]);
}

#[test]
fn name_forms() {
    assert_eq!(cols("[first]"), vec![0]);
    assert_eq!(cols("[fy2024]:[fy2026]"), vec![3, 4, 5]);
    assert_eq!(cols("[first]:[dept]"), vec![0, 1, 2]);
}

#[test]
fn letters_and_names_mix_in_one_range() {
    assert_eq!(cols("[fy2024]:F"), vec![3, 4, 5]);
    assert_eq!(cols("A:[dept]"), vec![0, 1, 2]);
}

#[test]
fn lists_keep_order_and_duplicates() {
    assert_eq!(cols("[first],[last]"), vec![0, 1]);
    assert_eq!(cols("D,B,B"), vec![3, 1, 1]);
    assert_eq!(cols("[dept],D:E"), vec![2, 3, 4]);
}

#[test]
fn a_reversed_range_reads_the_same_as_a_forward_one() {
    assert_eq!(cols("C:A"), cols("A:C"));
}

#[test]
fn letters_are_case_insensitive() {
    assert_eq!(cols("a:c"), vec![0, 1, 2]);
}

// ------------------------------------------------- open ends: the five xshape used to reject

#[test]
fn open_ended_ranges_run_to_the_edge() {
    assert_eq!(cols("B:"), vec![1, 2, 3, 4, 5]);
    assert_eq!(cols(":C"), vec![0, 1, 2]);
    assert_eq!(cols("A:"), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(cols("[last]:"), vec![1, 2, 3, 4, 5]);
    assert_eq!(cols(":[dept]"), vec![0, 1, 2]);
}

#[test]
fn a_range_with_neither_end_is_a_syntax_error() {
    let e = parse(":").unwrap_err();
    assert_eq!(e.kind, Kind::Syntax);
    assert!(e.message.contains("at least one end"), "got: {e}");
}

// ------------------------------------------------------- running off the edge: a stated choice

#[test]
fn clamp_stops_at_the_last_column() {
    let spec = parse("A:Z").unwrap();
    assert_eq!(
        spec.columns(&sheet(), Bounds::Clamp).unwrap(),
        vec![0, 1, 2, 3, 4, 5]
    );
}

#[test]
fn strict_refuses_the_same_address() {
    let spec = parse("A:Z").unwrap();
    let e = spec.columns(&sheet(), Bounds::Strict).unwrap_err();
    assert_eq!(e.kind, Kind::Lookup);
    assert!(
        e.message.contains("beyond the table's 6 columns"),
        "got: {e}"
    );
}

#[test]
fn clamp_selects_nothing_when_the_span_starts_past_the_end() {
    let spec = parse("H:Z").unwrap();
    assert!(spec.columns(&sheet(), Bounds::Clamp).unwrap().is_empty());
}

#[test]
fn a_single_column_past_the_end_is_empty_under_clamp() {
    let spec = parse("Z").unwrap();
    assert!(spec.columns(&sheet(), Bounds::Clamp).unwrap().is_empty());
}

// ------------------------------------------------------------------------- rows and cells

#[test]
fn rows_are_one_based_as_written() {
    let rect = parse("3")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.bottom), (2, 2));
    assert_eq!((rect.left, rect.right), (0, 5));
}

#[test]
fn dollar_is_the_last_row() {
    let rect = parse("$")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.bottom), (19, 19));
}

#[test]
fn a_row_range_can_run_to_the_last_row() {
    let rect = parse("3:$")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.bottom), (2, 19));
}

#[test]
fn a_cell_pins_both_axes() {
    let rect = parse("C5")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.left, rect.bottom, rect.right), (4, 2, 4, 2));
}

#[test]
fn a_cell_range_is_a_rectangle() {
    let rect = parse("B2:C5")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.left, rect.bottom, rect.right), (1, 1, 4, 2));
}

#[test]
fn a_named_column_takes_a_row_number_too() {
    let rect = parse("[dept]4")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!((rect.top, rect.left), (3, 2));
}

#[test]
fn cells_expand_row_major() {
    let cells = parse("A1:B2")
        .unwrap()
        .cells(&sheet(), Bounds::Strict)
        .unwrap();
    let got: Vec<_> = cells.into_iter().collect();
    assert_eq!(got, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
}

#[test]
fn a_row_address_is_rejected_where_only_columns_belong() {
    let e = parse("3")
        .unwrap()
        .columns(&sheet(), Bounds::Strict)
        .unwrap_err();
    assert_eq!(e.kind, Kind::Syntax);
    assert!(e.message.contains("only columns"), "got: {e}");
}

#[test]
fn a_list_is_not_a_rectangle() {
    let e = parse("A,C")
        .unwrap()
        .rect(&sheet(), Bounds::Strict)
        .unwrap_err();
    assert!(e.message.contains("not one rectangle"), "got: {e}");
}

// -------------------------------------------------------------------------- names and errors

#[test]
fn an_escaped_bracket_is_part_of_the_name() {
    let mut s = sheet();
    s.header = Some(vec!["weird]name".into()]);
    s.ncols = 1;
    assert_eq!(
        parse("[weird]]name]")
            .unwrap()
            .columns(&s, Bounds::Strict)
            .unwrap(),
        vec![0]
    );
}

#[test]
fn a_comma_inside_a_name_is_not_a_separator() {
    let mut s = sheet();
    s.header = Some(vec!["last, first".into()]);
    s.ncols = 1;
    assert_eq!(
        parse("[last, first]")
            .unwrap()
            .columns(&s, Bounds::Strict)
            .unwrap(),
        vec![0]
    );
}

#[test]
fn a_colon_inside_a_name_is_not_a_range() {
    let mut s = sheet();
    s.header = Some(vec!["ratio a:b".into()]);
    s.ncols = 1;
    assert_eq!(
        parse("[ratio a:b]")
            .unwrap()
            .columns(&s, Bounds::Strict)
            .unwrap(),
        vec![0]
    );
}

#[test]
fn an_unknown_name_is_a_lookup_error_not_a_syntax_one() {
    let e = parse("[nope]")
        .unwrap()
        .columns(&sheet(), Bounds::Strict)
        .unwrap_err();
    assert_eq!(e.kind, Kind::Lookup);
    assert!(e.message.contains("no column named [nope]"), "got: {e}");
}

#[test]
fn a_name_without_a_header_says_so() {
    let e = parse("[x]")
        .unwrap()
        .columns(&headerless(), Bounds::Strict)
        .unwrap_err();
    assert!(e.message.contains("needs a header row"), "got: {e}");
}

#[test]
fn an_unterminated_name_is_caught_before_any_table() {
    let e = parse("[first").unwrap_err();
    assert_eq!(e.kind, Kind::Syntax);
    assert!(e.message.contains("closing ]"), "got: {e}");
}

#[test]
fn empty_specs_and_empty_items_are_rejected() {
    assert!(parse("").is_err());
    assert!(parse("   ").is_err());
    assert!(parse("A,,B").is_err());
}

#[test]
fn row_zero_is_rejected_because_rows_start_at_one() {
    assert!(parse("A0").is_err());
    assert!(parse("0").is_err());
}

// ---------------------------------------------------------------- spans, for interactive use

#[test]
fn the_error_span_points_at_the_offending_item() {
    let spec = "A,[nope],C";
    let e = parse(spec)
        .unwrap()
        .columns(&sheet(), Bounds::Strict)
        .unwrap_err();
    assert_eq!(e.snippet(spec), "[nope]");
}

#[test]
fn the_error_span_survives_surrounding_whitespace() {
    let spec = "A , [nope] , C";
    let e = parse(spec)
        .unwrap()
        .columns(&sheet(), Bounds::Strict)
        .unwrap_err();
    assert_eq!(e.snippet(spec), "[nope]");
}

#[test]
fn a_syntax_span_points_inside_the_item() {
    let spec = "A,B?,C";
    let e = parse(spec).unwrap_err();
    assert_eq!(e.snippet(spec), "?");
}

// ------------------------------------------------------------------------------- rendering

#[test]
fn rendering_round_trips_through_the_parser() {
    let sheet = sheet();
    let rect = parse("B2:C5")
        .unwrap()
        .rect(&sheet, Bounds::Strict)
        .unwrap()
        .unwrap();
    let text = xaddr::rect_ref(rect.top, rect.left, rect.bottom, rect.right);
    assert_eq!(text, "B2:C5");
    let again = parse(&text)
        .unwrap()
        .rect(&sheet, Bounds::Strict)
        .unwrap()
        .unwrap();
    assert_eq!(again, rect);
}

#[test]
fn a_quoted_name_round_trips_even_with_a_bracket_in_it() {
    let mut s = sheet();
    s.header = Some(vec!["weird]name".into()]);
    s.ncols = 1;
    let text = xaddr::quote_name("weird]name");
    assert_eq!(
        parse(&text).unwrap().columns(&s, Bounds::Strict).unwrap(),
        vec![0]
    );
}

// ------------------------------------------------- embedding an address in a larger language

/// xled's parser reads `A:C s/x/y/` as an address followed by a command. The address has no
/// delimiter, so what ends it is the grammar — which is exactly what it delegates here.
#[test]
fn a_prefix_stops_where_the_address_stops() {
    for (input, used) in [
        ("A:C s/x/y/", 3),
        ("[dept]~/ops/", 6),
        ("$ d", 1),
        ("3:$ d", 3),
        ("A5", 2),
        (":C rest", 2),
        ("[first name] p", 12),
        ("[price]12=0", 9),
    ] {
        let (_, n) = xaddr::parse_prefix(input).unwrap_or_else(|e| panic!("{input:?}: {e}"));
        assert_eq!(n, used, "{input:?}");
    }
}

/// A comma is the host language's to interpret — xled unions with it, and consuming it here
/// would take that decision away.
#[test]
fn a_prefix_leaves_the_comma_alone() {
    let (spec, n) = xaddr::parse_prefix("A,B").unwrap();
    assert_eq!(n, 1);
    assert_eq!(spec.items().len(), 1);
}

#[test]
fn a_comma_inside_a_name_still_belongs_to_the_name() {
    let (_, n) = xaddr::parse_prefix("[last, first]:C x").unwrap();
    assert_eq!(n, 15);
}

#[test]
fn a_prefix_that_starts_with_no_address_says_so() {
    for input in ["/ops/", "~x", "(A:C)", "", "-1"] {
        let e = xaddr::parse_prefix(input).unwrap_err();
        assert_eq!(e.kind, Kind::Syntax, "{input:?}");
        assert!(e.message.contains("expected an address"), "{input:?}: {e}");
    }
}

/// `s` is a perfectly good column address, so `s/x/y/` parses as column S followed by junk the
/// host language owns. Telling a command from an address is the host's job — xled has the
/// context for it (a reserved word in command position) and this crate does not.
#[test]
fn a_bare_letter_is_a_column_even_when_it_looks_like_a_command() {
    let (spec, n) = xaddr::parse_prefix("s/x/y/").unwrap();
    assert_eq!(n, 1);
    assert_eq!(
        spec.columns(&sheet(), Bounds::Clamp).unwrap(),
        Vec::<usize>::new()
    );
}

#[test]
fn a_prefix_resolves_like_any_other_spec() {
    let (spec, _) = xaddr::parse_prefix("[fy2024]:F p").unwrap();
    assert_eq!(
        spec.columns(&sheet(), Bounds::Strict).unwrap(),
        vec![3, 4, 5]
    );
}
