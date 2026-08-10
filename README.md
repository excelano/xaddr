# xaddr — spreadsheet-style addressing for tabular data

xaddr is the grammar behind `A:C`, `[first]:[last]`, `C5`, `3:$`, and `D,B,B` — the vocabulary people already use to say *which part of this table*. It parses that vocabulary and resolves it against any table you can describe, and it does nothing else. No file reading, no reshaping, no evaluation, and no dependencies.

**Project page:** [https://excelano.com/xaddr/](https://excelano.com/xaddr/)

```rust
use xaddr::{parse, Bounds, Grid};

let spec = parse("[fy2024]:[fy2026],A")?;
let columns = spec.columns(&sheet, Bounds::Strict)?;   // [3, 4, 5, 0]

let rect = parse("B2:C5")?.rect(&sheet, Bounds::Clamp)?;
```

## Why

Anything that shows a person a table eventually has to let them say which part they mean, and the vocabulary for that was settled by VisiCalc in 1979. The problem is not designing it — it is that every tool implements it again. Three programs in this family had each grown their own copy, agreeing perfectly on `A:C` and disagreeing at every edge: one accepted `[last]:` and one rejected it with an error about a column named `""`, one clamped `A:Z` to the last column and one refused it. Nothing warned anybody, because there was nothing in common to warn from. A dialect that lives in three places is three dialects wearing one name.

## The grammar

An address is a comma-separated list. Order is preserved and duplicates are kept, because whether repetition means anything depends on the verb consuming it — gathering the same column twice is a mistake, joining it twice is not.

| Form | Means |
|---|---|
| `C`, `AF` | a whole column, by letter, case-insensitive |
| `[first name]` | a whole column, by header name — exact, and `]]` is a literal `]` |
| `5` | a whole row, numbered from 1 as written |
| `$` | the last row |
| `C5`, `[price]12` | one cell |
| `A:C`, `[a]:[b]` | an inclusive range, written in either direction |
| `A:`, `:C` | open-ended — runs to the edge of the table |
| `B,D:F,B` | a list, in the order written |

Predicates are deliberately absent. Regular-expression selection, comparisons, and set algebra over cells all need an evaluator and a pattern engine, so they belong to the tool that has them, operating on the cells xaddr hands back. The line is that xaddr says *where*, and never *which ones match*.

## Two steps, on purpose

Parsing takes no table. `parse` reports a syntax error with a byte span into the input, which is what an editor needs to underline the offending character while someone is still typing rather than waiting for them to finish. Resolution is a second call, against anything implementing `Grid`, and reports a lookup error when an address is well-formed but names nothing in this particular table. A caller with two error paths can route them differently; a caller with one can ignore the distinction.

`Grid` is four methods, one of them defaulted — how many rows, how many columns, the header if there is one, and optionally a different rule for matching names. A CLI buffer, a GTK list model, and a three-line test fixture can all implement it without reshaping themselves around it.

## Running off the edge

Addressing past the end of a table has two defensible answers. Stopping at the edge is how `sed` reads `2,$`, and it is right for reading, filtering, and editing in place, where a table shorter than you assumed is not an error. Refusing is right when the address drives something destructive or structural, where quietly doing less than you were asked is worse than stopping. xaddr holds neither as a house style: `Bounds::Clamp` and `Bounds::Strict` make the caller state which one this call wants, so the answer is a decision at the call site instead of an accident of whichever implementation you happened to link.

## Who uses it

xaddr is the shared floor under the Excelano tabular family — [xled](https://github.com/excelano/xled) edits cell values, [xshape](https://github.com/excelano/xshape) changes a table's geometry, and Comma edits delimited files in a GTK grid. It is a normal crate with no ties to any of them, so anything that shows a table and needs a user to point at part of it can use it the same way.

## License

MIT. Authored by David M. Anderson, with AI assistance.
