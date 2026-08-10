// Resolving a parsed spec against a real table.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

use crate::error::{Error, Result};
use crate::letters::col_to_letter;
use crate::parse::{ColRef, Item, Pos, Spec};
use std::collections::BTreeSet;
use std::ops::Range;

/// What a table has to tell this crate to be addressable: how big it is, and what its columns
/// are called. Deliberately tiny — a CLI buffer, a GTK list model, and a test fixture should
/// all be able to implement it without reshaping themselves around it.
pub trait Grid {
    fn nrows(&self) -> usize;
    fn ncols(&self) -> usize;

    /// The header row, if the table has one. `None` means addresses by name cannot resolve.
    fn header(&self) -> Option<&[String]>;

    /// Column index for a header name. Case-sensitive and exact by default
    /// (`[userId]` is not `[userid]`); override to match differently.
    fn name_to_col(&self, name: &str) -> Option<usize> {
        self.header()?.iter().position(|h| h == name)
    }
}

/// What to do about an address that runs past the edge of the table.
///
/// The two answers are both defensible and the right one depends on what the caller does next,
/// so this crate refuses to have a house style: state it at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounds {
    /// Stop at the edge, sed's reading of `2,$`: addressing past the end selects up to the end
    /// and invents nothing. A span that begins past the end selects nothing at all. Right for
    /// reading, filtering, and editing in place, where a short table is not an error.
    Clamp,
    /// Refuse. Right when the address drives something destructive or structural, where
    /// silently doing less than asked is worse than stopping.
    Strict,
}

/// An inclusive rectangle of cells, 0-based on both axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}

impl Spec {
    /// Resolve to an ordered list of column indices, rejecting anything that names a row.
    ///
    /// Order is preserved and duplicates are kept: `D,B,B` gives `[3, 1, 1]`, because whether
    /// repetition is meaningful is the caller's question, not this crate's.
    pub fn columns<G: Grid>(&self, grid: &G, bounds: Bounds) -> Result<Vec<usize>> {
        let ncols = grid.ncols();
        let mut out = Vec::new();
        for (item, span) in self.items.iter().zip(&self.spans) {
            match item {
                Item::Single(p) => {
                    let c = column_only(p, grid, span)?;
                    match bounds {
                        Bounds::Strict => {
                            check_col(c, ncols, span)?;
                            out.push(c);
                        }
                        // Past the end selects nothing — no phantom columns.
                        Bounds::Clamp if c < ncols => out.push(c),
                        Bounds::Clamp => {}
                    }
                }
                Item::Range { start, end } => {
                    let lo = start
                        .as_ref()
                        .map(|p| column_only(p, grid, span))
                        .transpose()?;
                    let hi = end
                        .as_ref()
                        .map(|p| column_only(p, grid, span))
                        .transpose()?;
                    let (lo, hi) = span_ends(lo, hi, ncols);
                    let (lo, hi) = ordered(lo, hi);
                    match bounds {
                        Bounds::Strict => {
                            check_col(hi, ncols, span)?;
                            out.extend(lo..=hi);
                        }
                        Bounds::Clamp => {
                            if let Some((lo, hi)) = clamp_span(lo, hi, ncols) {
                                out.extend(lo..=hi);
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Resolve to a set of `(row, column)` cells. A `BTreeSet` keeps them in row-major order,
    /// which is the order a table renders in.
    pub fn cells<G: Grid>(&self, grid: &G, bounds: Bounds) -> Result<BTreeSet<(usize, usize)>> {
        let mut set = BTreeSet::new();
        for (item, span) in self.items.iter().zip(&self.spans) {
            let rect = self.item_rect(item, span, grid, bounds)?;
            if let Some(r) = rect {
                for row in r.top..=r.bottom {
                    for col in r.left..=r.right {
                        set.insert((row, col));
                    }
                }
            }
        }
        Ok(set)
    }

    /// The single rectangle this spec names, for callers that want a selection rather than a
    /// cell set — a grid cursor, a status bar, a copy-reference action.
    ///
    /// `Ok(None)` when the spec is empty or resolves to nothing. A spec with more than one
    /// comma-separated item is an error here: a list of disjoint ranges is not a rectangle,
    /// and quietly returning the first one would be a lie.
    pub fn rect<G: Grid>(&self, grid: &G, bounds: Bounds) -> Result<Option<Rect>> {
        match self.items.len() {
            0 => Ok(None),
            1 => self.item_rect(&self.items[0], &self.spans[0], grid, bounds),
            _ => Err(Error::syntax(
                "this address names several separate ranges, which is not one rectangle",
                self.spans[0].start..self.spans[self.spans.len() - 1].end,
            )),
        }
    }

    fn item_rect<G: Grid>(
        &self,
        item: &Item,
        span: &Range<usize>,
        grid: &G,
        bounds: Bounds,
    ) -> Result<Option<Rect>> {
        let nrows = grid.nrows();
        let ncols = grid.ncols();

        let (sr, sc, er, ec) = match item {
            Item::Single(p) => {
                let (r, c) = axes(p, grid, span)?;
                (r, c, r, c)
            }
            Item::Range { start, end } => {
                let (sr, sc) = match start {
                    Some(p) => axes(p, grid, span)?,
                    None => (None, None),
                };
                let (er, ec) = match end {
                    Some(p) => axes(p, grid, span)?,
                    None => (None, None),
                };
                (sr, sc, er, ec)
            }
        };

        let has_row = sr.is_some() || er.is_some();
        let has_col = sc.is_some() || ec.is_some();
        if !has_row && !has_col {
            return Err(Error::syntax(
                "this address pins neither a row nor a column",
                span.clone(),
            ));
        }

        // An axis nobody pinned spans the whole table; a half-pinned one runs to its edge.
        let (r1, r2) = if has_row {
            ordered(
                sr.unwrap_or(0),
                er.unwrap_or_else(|| nrows.saturating_sub(1)),
            )
        } else {
            (0, nrows.saturating_sub(1))
        };
        let (c1, c2) = if has_col {
            ordered(
                sc.unwrap_or(0),
                ec.unwrap_or_else(|| ncols.saturating_sub(1)),
            )
        } else {
            (0, ncols.saturating_sub(1))
        };

        match bounds {
            Bounds::Strict => {
                check_row(r2, nrows, span)?;
                check_col(c2, ncols, span)?;
                Ok(Some(Rect {
                    top: r1,
                    left: c1,
                    bottom: r2,
                    right: c2,
                }))
            }
            Bounds::Clamp => Ok(
                match (clamp_span(r1, r2, nrows), clamp_span(c1, c2, ncols)) {
                    (Some((top, bottom)), Some((left, right))) => Some(Rect {
                        top,
                        left,
                        bottom,
                        right,
                    }),
                    _ => None,
                },
            ),
        }
    }
}

/// The `(row, column)` extents a positional pins, each 0-based; `None` on an axis it leaves free.
fn axes<G: Grid>(p: &Pos, grid: &G, span: &Range<usize>) -> Result<(Option<usize>, Option<usize>)> {
    Ok(match p {
        Pos::Column(c) => (None, Some(col_index(c, grid, span)?)),
        Pos::Row(r) => (Some(*r), None),
        Pos::LastRow => (Some(grid.nrows().saturating_sub(1)), None),
        Pos::Cell { col, row } => (Some(*row), Some(col_index(col, grid, span)?)),
    })
}

/// Resolve a positional that must name a column and only a column.
fn column_only<G: Grid>(p: &Pos, grid: &G, span: &Range<usize>) -> Result<usize> {
    match p {
        Pos::Column(c) => col_index(c, grid, span),
        Pos::Row(_) | Pos::LastRow => Err(Error::syntax(
            "this names a row, but only columns can be addressed here",
            span.clone(),
        )),
        Pos::Cell { .. } => Err(Error::syntax(
            "this names a single cell, but only columns can be addressed here",
            span.clone(),
        )),
    }
}

fn col_index<G: Grid>(c: &ColRef, grid: &G, span: &Range<usize>) -> Result<usize> {
    match c {
        ColRef::Letters(i) => Ok(*i),
        ColRef::Name(name) => grid.name_to_col(name).ok_or_else(|| {
            if grid.header().is_none() {
                Error::lookup(
                    format!(
                        "column name [{name}] needs a header row (this table has none — \
                         address by letter instead)"
                    ),
                    span.clone(),
                )
            } else {
                Error::lookup(format!("no column named [{name}]"), span.clone())
            }
        }),
    }
}

fn span_ends(lo: Option<usize>, hi: Option<usize>, ncols: usize) -> (usize, usize) {
    (
        lo.unwrap_or(0),
        hi.unwrap_or_else(|| ncols.saturating_sub(1)),
    )
}

fn check_col(c: usize, ncols: usize, span: &Range<usize>) -> Result<()> {
    if ncols == 0 {
        return Err(Error::lookup("the table has no columns", span.clone()));
    }
    if c >= ncols {
        return Err(Error::lookup(
            format!(
                "column {} is beyond the table's {ncols} columns",
                col_to_letter(c)
            ),
            span.clone(),
        ));
    }
    Ok(())
}

fn check_row(r: usize, nrows: usize, span: &Range<usize>) -> Result<()> {
    if nrows == 0 {
        return Err(Error::lookup("the table has no rows", span.clone()));
    }
    if r >= nrows {
        return Err(Error::lookup(
            format!("row {} is beyond the table's {nrows} rows", r + 1),
            span.clone(),
        ));
    }
    Ok(())
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Clamp an inclusive `[lo, hi]` span to an axis of length `len`; `None` if it starts past the
/// end, since a span wholly past the end selects nothing rather than the last cell.
fn clamp_span(lo: usize, hi: usize, len: usize) -> Option<(usize, usize)> {
    if len == 0 || lo >= len {
        None
    } else {
        Some((lo, hi.min(len - 1)))
    }
}
