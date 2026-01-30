//! Position / range utilities used across typstlab-lsp.
//!
//! Design goals:
//! - Fast: absolute byte offsets + O(log n) line lookup (binary search).
//! - Small: ranges are `[start, end)` in absolute byte offsets.
//! - Neutral: does not depend on LSP or tree-sitter types directly.
//!
//! Notes:
//! - Offsets/cols are **byte-based** by default.
//!   (Typst/tree-sitter operate on bytes; LSP uses UTF-16 columns, so you
//!    will convert at the boundary if needed.)

use core::fmt;
use core::ops::{Range, RangeFrom, RangeInclusive, RangeTo, RangeToInclusive};

/// Absolute byte offset in the file (0..=len).
pub type AbsOffset = u32;

/// A half-open absolute byte range: `[start, end)`
///
/// - `start` is inclusive
/// - `end` is exclusive
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AbsTextRange {
    pub start: AbsOffset,
    pub end: AbsOffset,
}

impl AbsTextRange {
    #[inline]
    pub const fn new(start: AbsOffset, end: AbsOffset) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    #[inline]
    pub const fn len(self) -> AbsOffset {
        self.end - self.start
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    #[inline]
    pub const fn contains(self, abs: AbsOffset) -> bool {
        self.start <= abs && abs < self.end
    }

    #[inline]
    pub const fn contains_inclusive_end(self, abs: AbsOffset) -> bool {
        self.start <= abs && abs <= self.end
    }

    #[inline]
    pub const fn intersects(self, other: AbsTextRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    #[inline]
    pub const fn union(self, other: AbsTextRange) -> AbsTextRange {
        let start = if self.start < other.start {
            self.start
        } else {
            other.start
        };
        let end = if self.end > other.end {
            self.end
        } else {
            other.end
        };
        AbsTextRange { start, end }
    }

    #[inline]
    pub const fn clamp(self, max_end: AbsOffset) -> AbsTextRange {
        let end = if self.end < max_end {
            self.end
        } else {
            max_end
        };
        let start = if self.start < end { self.start } else { end };
        AbsTextRange { start, end }
    }

    /// Convert to `Range<usize>` (useful for slicing).
    #[inline]
    pub fn to_usize_range(self) -> Range<usize> {
        (self.start as usize)..(self.end as usize)
    }

    /// Convert to `Range<u32>`.
    #[inline]
    pub const fn as_range_u32(self) -> Range<u32> {
        self.start..self.end
    }

    /// Shift the range by a delta.
    ///
    /// Uses saturating addition to prevent overflow/underflow, and ensures
    /// that start <= end.
    #[inline]
    pub fn shift(self, delta: i32) -> Self {
        let shift_u32 = |val: u32| {
            if delta >= 0 {
                val.saturating_add(delta as u32)
            } else {
                val.saturating_sub(delta.unsigned_abs())
            }
        };
        let s = shift_u32(self.start);
        let e = shift_u32(self.end);
        let (start, end) = if s <= e { (s, e) } else { (e, s) };
        Self::new(start, end)
    }

    /// Create from `start..` with known text_len.
    #[inline]
    pub fn from_range_from(r: RangeFrom<u32>, text_len: u32) -> Self {
        AbsTextRange::new(r.start, text_len)
    }

    /// Create from `..end` (exclusive).
    #[inline]
    pub fn from_range_to(r: RangeTo<u32>) -> Self {
        AbsTextRange::new(0, r.end)
    }

    /// Create from `..=end` (inclusive) -> `[0, end+1)`.
    #[inline]
    pub fn from_range_to_inclusive(r: RangeToInclusive<u32>) -> Self {
        let end = r
            .end
            .checked_add(1)
            .expect("AbsTextRange: inclusive end == u32::MAX is not representable");
        AbsTextRange::new(0, end)
    }
}

impl From<Range<usize>> for AbsTextRange {
    #[inline]
    fn from(r: Range<usize>) -> Self {
        // strict: panic if overflow
        let start: u32 = r
            .start
            .try_into()
            .expect("AbsTextRange: start overflows u32");
        let end: u32 = r.end.try_into().expect("AbsTextRange: end overflows u32");
        AbsTextRange::new(start, end)
    }
}

impl From<Range<u32>> for AbsTextRange {
    #[inline]
    fn from(r: Range<u32>) -> Self {
        AbsTextRange::new(r.start, r.end)
    }
}

impl From<RangeTo<u32>> for AbsTextRange {
    #[inline]
    fn from(r: RangeTo<u32>) -> Self {
        Self::from_range_to(r)
    }
}

impl From<RangeToInclusive<u32>> for AbsTextRange {
    #[inline]
    fn from(r: RangeToInclusive<u32>) -> Self {
        Self::from_range_to_inclusive(r)
    }
}

/// `a..=b` (inclusive) -> `[a, b+1)`
///
/// Panics if `b == u32::MAX` because it's not representable.
impl From<RangeInclusive<u32>> for AbsTextRange {
    #[inline]
    fn from(r: RangeInclusive<u32>) -> Self {
        let start = *r.start();
        let end_incl = *r.end();
        let end = end_incl
            .checked_add(1)
            .expect("AbsTextRange: inclusive end == u32::MAX is not representable");
        AbsTextRange::new(start, end)
    }
}

impl fmt::Display for AbsTextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{})", self.start, self.end)
    }
}

/// A line/column coordinate (byte-based column by default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineCol {
    #[inline]
    pub const fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 0-based display (matches internal representation)
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A range expressed in lines (inclusive start line, inclusive end line).
///
/// This is useful for "which lines are touched" computations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineRange {
    pub start_line: u32,
    pub end_line: u32,
}

impl LineRange {
    #[inline]
    pub const fn new(start_line: u32, end_line: u32) -> Self {
        assert!(start_line <= end_line);
        Self {
            start_line,
            end_line,
        }
    }
}

/// Minimal trait to expose an absolute range.
pub trait AbsRange {
    fn abs_range(&self) -> AbsTextRange;
}

impl AbsRange for AbsTextRange {
    #[inline]
    fn abs_range(&self) -> AbsTextRange {
        *self
    }
}

/// Line indexer interface.
///
/// The implementor maps `AbsOffset` <-> line/col using precomputed line starts.
pub trait LineIndex {
    /// Returns the 0-based line index that contains `abs`.
    ///
    /// Precondition: `abs <= text_len` (EOF is allowed).
    fn line_of(&self, abs: AbsOffset) -> u32;

    /// Returns the absolute offset of the first byte of `line`.
    fn line_start(&self, line: u32) -> AbsOffset;

    /// Returns (line, col) for `abs` (byte-based col).
    #[inline]
    fn line_col(&self, abs: AbsOffset) -> LineCol {
        let line = self.line_of(abs);
        let start = self.line_start(line);
        LineCol::new(line, abs - start)
    }

    /// Returns absolute offset for a given (line, col).
    ///
    /// This is a *best effort* mapping:
    /// - It does not clamp to the actual line length unless the implementation chooses to.
    fn abs_of(&self, lc: LineCol) -> AbsOffset {
        self.line_start(lc.line) + lc.col
    }
}

/// Stores absolute byte offsets of each line start.
///
/// Invariant: `starts[0] == 0`, strictly increasing, and within text length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineStarts {
    starts: Vec<AbsOffset>,
    text_len: AbsOffset,
}

impl LineStarts {
    /// Build line starts from bytes by scanning `\n`.
    ///
    /// - Always includes `0` as the first line start.
    /// - After every `\n` at index `i`, adds `i+1` as the start of the next line.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let text_len = bytes.len() as AbsOffset;
        // Estimate capacity based on average line length of 32 bytes.
        let capacity = (text_len / 32).max(128) as usize;
        let mut starts = Vec::with_capacity(capacity);
        starts.push(0);
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                let next = (i + 1) as AbsOffset;
                starts.push(next);
            }
        }
        Self { starts, text_len }
    }

    /// Access raw starts.
    #[inline]
    pub fn starts(&self) -> &[AbsOffset] {
        &self.starts
    }

    /// Access text length.
    #[inline]
    pub const fn text_len(&self) -> AbsOffset {
        self.text_len
    }

    /// Number of lines (at least 1).
    #[inline]
    pub fn line_count(&self) -> u32 {
        self.starts.len() as u32
    }

    /// Returns start offset of `line`, panics if out of bounds.
    #[inline]
    pub fn start_of(&self, line: u32) -> AbsOffset {
        self.starts[line as usize]
    }

    /// Returns the start offset of the next line, or `text_len` if `line` is the last line.
    ///
    /// This is the "end (exclusive) including newline" boundary for that line.
    #[inline]
    pub fn end_exclusive_including_newline(&self, line: u32) -> AbsOffset {
        let i = line as usize;
        if i + 1 < self.starts.len() {
            self.starts[i + 1]
        } else {
            self.text_len
        }
    }

    /// Returns the line length in bytes *including* newline if it exists.
    #[inline]
    pub fn line_len_including_newline(&self, line: u32) -> AbsOffset {
        self.end_exclusive_including_newline(line) - self.start_of(line)
    }

    /// Convert an absolute range into a line range (inclusive).
    ///
    /// - `range.start == range.end` still maps to a single line (the line containing `start`).
    /// - Precondition: `range.end <= text_len` (or clamp it first).
    #[inline]
    pub fn line_range_of(&self, range: AbsTextRange) -> LineRange {
        let start_line = self.line_of(range.start);
        // `end` is exclusive; for line coverage we treat `end-1` as last covered byte if non-empty.
        let end_line = if range.is_empty() {
            start_line
        } else {
            // If end==0 can't happen because start<=end and non-empty implies end>start>=0
            self.line_of(range.end - 1)
        };
        LineRange::new(start_line, end_line)
    }

    /// Clamp a (line, col) to the existing line bounds.
    ///
    /// This ensures `abs_of(clamped)` is within `0..=text_len` and within the line.
    pub fn clamp_line_col(&self, lc: LineCol) -> LineCol {
        let last = self.line_count().saturating_sub(1);
        let line = if lc.line < last { lc.line } else { last };

        let line_start = self.start_of(line);
        let line_end = self.end_exclusive_including_newline(line);
        let max_col = line_end.saturating_sub(line_start);
        let col = if lc.col < max_col { lc.col } else { max_col };
        LineCol::new(line, col)
    }

    /// A safer abs_of that clamps to the line end (including newline boundary).
    pub fn abs_of_clamped(&self, lc: LineCol) -> AbsOffset {
        let lc = self.clamp_line_col(lc);
        self.start_of(lc.line) + lc.col
    }
}

impl LineIndex for LineStarts {
    #[inline]
    fn line_of(&self, abs: AbsOffset) -> u32 {
        debug_assert!(
            abs <= self.text_len,
            "abs offset {} out of bounds (len {})",
            abs,
            self.text_len
        );
        // Find the greatest i where starts[i] <= abs.
        // `binary_search` returns:
        // - Ok(i): exact match (abs is a line start)
        // - Err(i): insertion point, so the previous index is the containing line.
        let i = match self.starts.binary_search(&abs) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        i as u32
    }

    #[inline]
    fn line_start(&self, line: u32) -> AbsOffset {
        self.start_of(line)
    }

    #[inline]
    fn abs_of(&self, lc: LineCol) -> AbsOffset {
        self.start_of(lc.line) + lc.col
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts_basic() {
        let text = b"aaa\nbbb\nccc";
        let ls = LineStarts::from_bytes(text);
        assert_eq!(ls.starts(), &[0, 4, 8]);
        assert_eq!(ls.line_count(), 3);

        assert_eq!(ls.line_of(0), 0);
        assert_eq!(ls.line_of(3), 0);
        assert_eq!(ls.line_of(4), 1); // 'b' start
        assert_eq!(ls.line_of(7), 1);
        assert_eq!(ls.line_of(8), 2);
        assert_eq!(ls.line_of(text.len() as u32), 2); // EOF belongs to last line by convention

        assert_eq!(ls.line_col(0), LineCol::new(0, 0));
        assert_eq!(ls.line_col(2), LineCol::new(0, 2));
        assert_eq!(ls.line_col(4), LineCol::new(1, 0));
        assert_eq!(ls.line_col(9), LineCol::new(2, 1));
    }

    #[test]
    fn abs_text_range_helpers() {
        let r = AbsTextRange::new(10, 20);
        assert_eq!(r.len(), 10);
        assert!(r.contains(10));
        assert!(r.contains(19));
        assert!(!r.contains(20));
        assert!(r.intersects(AbsTextRange::new(19, 21)));
        assert!(!r.intersects(AbsTextRange::new(20, 30)));
    }

    #[test]
    fn line_range_of_range() {
        let text = b"aaa\nbbb\nccc";
        let len = text.len() as u32;
        let ls = LineStarts::from_bytes(text);

        // "bbb\n" is [4..8)
        let r = AbsTextRange::new(4, 8);
        let lr = ls.line_range_of(r);
        assert_eq!(lr, LineRange::new(1, 1));

        // spans end of line1 into line2
        let r2 = AbsTextRange::new(6, 9);
        let lr2 = ls.line_range_of(r2);
        assert_eq!(lr2, LineRange::new(1, 2));

        // empty range maps to single line
        let r3 = AbsTextRange::new(len, len);
        let lr3 = ls.line_range_of(r3);
        assert_eq!(lr3, LineRange::new(2, 2));
    }

    #[test]
    fn abs_of_clamped() {
        let text = b"aaa\nbbb\n";
        let ls = LineStarts::from_bytes(text);

        // line 0 ends at 4 (including newline)
        let abs = ls.abs_of_clamped(LineCol::new(0, 999));
        assert_eq!(abs, 4);

        // last line (line 1) ends at len
        let abs2 = ls.abs_of_clamped(LineCol::new(999, 999));
        assert_eq!(abs2, ls.text_len());
    }

    #[test]
    fn range_conversions() {
        let r_usize = 10usize..20usize;
        let tr_usize: AbsTextRange = r_usize.into();
        assert_eq!(tr_usize, AbsTextRange::new(10, 20));

        let r_u32 = 10u32..20u32;
        let tr_u32: AbsTextRange = r_u32.into();
        assert_eq!(tr_u32, AbsTextRange::new(10, 20));

        let r_incl = 10u32..=19u32;
        let tr_incl: AbsTextRange = r_incl.into();
        assert_eq!(tr_incl, AbsTextRange::new(10, 20));

        let tr_from = AbsTextRange::from_range_from(10u32.., 100);
        assert_eq!(tr_from, AbsTextRange::new(10, 100));

        let tr_to: AbsTextRange = (..20u32).into();
        assert_eq!(tr_to, AbsTextRange::new(0, 20));

        let tr_to_incl: AbsTextRange = (..=19u32).into();
        assert_eq!(tr_to_incl, AbsTextRange::new(0, 20));

        assert_eq!(tr_usize.to_usize_range(), 10..20);
        assert_eq!(tr_usize.as_range_u32(), 10..20);
    }

    #[test]
    fn abs_text_range_utils() {
        let r = AbsTextRange::new(10, 20);

        // shift positive
        assert_eq!(r.shift(5), AbsTextRange::new(15, 25));

        // shift negative
        assert_eq!(r.shift(-5), AbsTextRange::new(5, 15));

        // saturating at 0
        assert_eq!(r.shift(-100), AbsTextRange::new(0, 0));

        // shift 0
        assert_eq!(r.shift(0), r);
    }

    #[test]
    fn line_starts_initialization() {
        let text = b"aaa\nbbb\n";
        let ls = LineStarts::from_bytes(text);
        assert_eq!(ls.starts().len(), 3); // 0, 4, 8
        assert_eq!(ls.text_len, 8);
    }
}
