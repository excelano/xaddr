// Addressing errors, carrying where in the input they happened.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

use core::fmt;
use core::ops::Range;

/// Which stage rejected the address.
///
/// The split matters to callers with two error paths: a `Syntax` error is wrong no matter what
/// table it is aimed at, while a `Lookup` error is about *this* table and would succeed against
/// another. A GUI can mark syntax red as the user types and defer lookup until resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The text is not a well-formed address.
    Syntax,
    /// Well-formed, but it does not name anything in this table.
    Lookup,
}

/// A rejected address, with the byte range of the input responsible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: Kind,
    pub message: String,
    /// Byte range into the spec passed to [`crate::parse`]. Always a valid slice range of that
    /// input, so `&spec[err.span.clone()]` is safe and `span.start` can place a caret.
    /// Resolution errors, which have no narrower home, span the whole item.
    pub span: Range<usize>,
}

impl Error {
    pub(crate) fn syntax(message: impl Into<String>, span: Range<usize>) -> Self {
        Error {
            kind: Kind::Syntax,
            message: message.into(),
            span,
        }
    }

    pub(crate) fn lookup(message: impl Into<String>, span: Range<usize>) -> Self {
        Error {
            kind: Kind::Lookup,
            message: message.into(),
            span,
        }
    }

    /// The offending text, given the spec the error came from.
    ///
    /// Returns `""` if handed a different string that the span does not fit, rather than
    /// panicking — an error is a poor place to acquire a second failure mode.
    pub fn snippet<'a>(&self, spec: &'a str) -> &'a str {
        spec.get(self.span.clone()).unwrap_or("")
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = core::result::Result<T, Error>;
