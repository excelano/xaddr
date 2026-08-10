// xaddr — spreadsheet-style addressing for tabular data.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

//! Spreadsheet-style addressing for tabular data: the grammar behind `A:C`, `[first]:[last]`,
//! `C5`, `3:$`, and `D,B,B`.
//!
//! Anything that shows a user a table eventually has to let them say *which part*. The
//! vocabulary for that is already settled — every spreadsheet since VisiCalc uses column
//! letters and `A1:C5` — so this crate implements that vocabulary once instead of each tool
//! growing its own dialect that agrees at the centre and drifts at the edges.
//!
//! # The grammar
//!
//! An address is a comma-separated list. Order is preserved and duplicates are kept, because
//! whether repetition is meaningful depends on the verb using it.
//!
//! | Form                | Means                                                    |
//! |---------------------|----------------------------------------------------------|
//! | `C`, `AF`           | a whole column, by letter (case-insensitive)             |
//! | `[first name]`      | a whole column, by header name (exact; `]]` is a literal `]`) |
//! | `5`                 | a whole row, numbered from 1 as written                  |
//! | `$`                 | the last row                                             |
//! | `C5`, `[price]12`   | one cell                                                 |
//! | `A:C`, `[a]:[b]`    | an inclusive range, either direction                     |
//! | `A:`, `:C`          | open-ended — runs to the table's edge                    |
//! | `B,D:F,B`           | a list, in the order written                             |
//!
//! Predicates are deliberately absent. Regular-expression selection, comparisons, and set
//! algebra over cells need an evaluator and a pattern engine; they belong to the tool that has
//! those, operating on the cells this crate hands back.
//!
//! # Parsing and resolving are separate steps
//!
//! [`parse`] needs no table. It reports a [`Kind::Syntax`] error with a byte [`Error::span`],
//! so an editor can underline the offending character while the user is still typing.
//! Resolution comes later, against anything implementing [`Grid`], and reports [`Kind::Lookup`]
//! for an address that is well-formed but names nothing here.
//!
//! ```
//! use xaddr::{parse, Bounds, Grid};
//!
//! struct Sheet { header: Vec<String>, nrows: usize }
//! impl Grid for Sheet {
//!     fn nrows(&self) -> usize { self.nrows }
//!     fn ncols(&self) -> usize { self.header.len() }
//!     fn header(&self) -> Option<&[String]> { Some(&self.header) }
//! }
//!
//! let sheet = Sheet {
//!     header: ["id", "first", "last", "fy2024", "fy2025"].map(String::from).to_vec(),
//!     nrows: 40,
//! };
//!
//! let spec = parse("[first]:[last],A").unwrap();
//! assert_eq!(spec.columns(&sheet, Bounds::Strict).unwrap(), vec![1, 2, 0]);
//!
//! // An open end runs to the edge of this table, whatever its width.
//! assert_eq!(parse("D:").unwrap().columns(&sheet, Bounds::Strict).unwrap(), vec![3, 4]);
//!
//! // A selection, for a grid that wants a rectangle rather than a cell set.
//! let rect = parse("B2:C5").unwrap().rect(&sheet, Bounds::Strict).unwrap().unwrap();
//! assert_eq!((rect.top, rect.left, rect.bottom, rect.right), (1, 1, 4, 2));
//! ```
//!
//! # Running off the edge
//!
//! Addressing past the end of a table has two defensible answers and this crate holds neither
//! as a house style — [`Bounds`] makes the caller say. [`Bounds::Clamp`] stops at the edge, the
//! way `sed` reads `2,$`. [`Bounds::Strict`] refuses, which is what you want when the address
//! drives something destructive.

mod error;
mod letters;
mod parse;
mod resolve;

pub use error::{Error, Kind, Result};
pub use letters::{cell_ref, col_to_letter, letter_to_col, quote_name, rect_ref, MAX_COL};
pub use parse::{parse, parse_prefix, ColRef, Item, Pos, Spec};
pub use resolve::{Bounds, Grid, Rect};
