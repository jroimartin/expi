//! Library for working with sets of overlapping ranges.

#![no_std]

use core::cmp::max;
use core::fmt;

/// Represents an error related to a `Range` or a `RangeSet`.
#[derive(Debug)]
pub enum Error {
    /// Invalid range boundaries.
    InvalidBoundaries(u64, u64),

    /// The fixed size array that backs the `RangeSet` is full. It is not
    /// possible to add more ranges.
    FullRangeSet,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidBoundaries(start, end) => {
                write!(f, "invalid range boundaries: [{start}, {end}]")
            }
            Error::FullRangeSet => {
                write!(f, "RangeSet internal buffer is full")
            }
        }
    }
}

/// Represents an inclusive range.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Range {
    /// Range start.
    start: u64,

    /// Range end.
    end: u64,
}

impl Range {
    /// Returns a new `Range`.
    ///
    /// # Errors
    ///
    /// This functions returns `Error::InvalidBoundaries` if the end point is
    /// lower than the start point of the range.
    pub fn new(start: u64, end: u64) -> Result<Self, Error> {
        if start <= end {
            Ok(Range { start, end })
        } else {
            Err(Error::InvalidBoundaries(start, end))
        }
    }

    /// Returns the start point of the range.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Returns the end point of the range.
    pub fn end(&self) -> u64 {
        self.end
    }

    /// Returns `true` if the range contains a given point.
    pub fn contains_point(&self, point: u64) -> bool {
        point >= self.start && point <= self.end
    }

    /// Returns `true` if the range contains a given range.
    pub fn contains_range(&self, range: Range) -> bool {
        self.contains_point(range.start) && self.contains_point(range.end)
    }

    /// Returns `true` if the ranges overlap.
    pub fn overlaps(&self, range: Range) -> bool {
        self.contains_point(range.start)
            || self.contains_point(range.end)
            || range.contains_point(self.start)
            || range.contains_point(self.end)
    }

    /// Returns the size of the range.
    pub fn size(&self) -> u64 {
        self.end - self.start + 1
    }
}

/// Size of the fixed-size array that contains the `RangeSet` entries.
const RANGE_SET_SIZE: usize = 256;

/// Represents a set of ranges.
#[derive(Debug)]
pub struct RangeSet {
    /// Ranges within the `RangeSet`.
    ranges: [Range; RANGE_SET_SIZE],

    /// Number of elements in the fixed size array that are being used.
    in_use: usize,
}

impl Default for RangeSet {
    fn default() -> Self {
        RangeSet::new()
    }
}

impl RangeSet {
    /// Returns an empty `RangeSet`.
    pub fn new() -> Self {
        RangeSet {
            ranges: [Range::default(); RANGE_SET_SIZE],
            in_use: 0,
        }
    }

    /// Returns the ranges in the `RangeSet`.
    pub fn ranges(&self) -> &[Range] {
        &self.ranges[..self.in_use]
    }

    /// Inserts a range into the internal `ranges` array preserving the order
    /// of the array and avoiding duplicated start points.
    fn sort_insert(&mut self, range: Range) -> Result<(), Error> {
        // Find the index of the new range.
        let mut idx = self.in_use;
        for i in 0..self.in_use {
            // If there is a range with the same start point, reuse the same
            // range updating its end point to the greatest value between the
            // new and the old one.
            if range.start == self.ranges[i].start {
                self.ranges[i].end = max(range.end, self.ranges[i].end);
                return Ok(());
            }

            if range.start < self.ranges[i].start {
                idx = i;
                break;
            }
        }

        // There must be space at least for the new range.
        if self.in_use >= self.ranges.len() {
            return Err(Error::FullRangeSet);
        }

        // Create space for the new range, moving the existing ones forward one
        // position.
        self.ranges.copy_within(idx..self.in_use, idx + 1);
        self.ranges[idx] = range;
        self.in_use += 1;

        Ok(())
    }

    /// Merges the overlapping ranges in the internal `ranges` array. It
    /// assumes that the internal `ranges` array is sorted and there are no
    /// duplicated start points. Thus, `RangeSet::sort_insert` must be used
    /// internally to insert new ranges.
    fn merge(&mut self) {
        let mut i = 0;
        while i < self.in_use - 1 {
            // If the ranges are not contiguous or overlapped, continue. We use
            // `saturating_add` because in the following case the ranges must
            // be merged:
            //
            // max] [max => max > max == false => merge
            if self.ranges[i + 1].start > self.ranges[i].end.saturating_add(1) {
                i += 1;
                continue;
            }

            // If the ranges are contiguous or the first end point is contained
            // by the second range, update the first end point with the value
            // of the second one.
            //
            // Note that `end + 1` is used because:
            // 1. Contiguous ranges must be merged.
            // 2. If both ranges share the same end point, there is no need to
            //    udpate it.
            // This avoids checking one extra condition.
            //
            // We use `saturating_add` because in the following case the ranges
            // must be merged:
            //
            // max] [max => right range contains end of left range => merge
            if self.ranges[i + 1]
                .contains_point(self.ranges[i].end.saturating_add(1))
            {
                self.ranges[i].end = self.ranges[i + 1].end;
            }

            // At this point the two ranges have been merged into the first
            // one. Remove the second range from the list and decrement the
            // counter of used array positions.
            self.ranges.copy_within(i + 2..self.in_use, i + 1);
            self.in_use -= 1;
        }
    }

    /// Inserts a `Range` into the `RangeSet`. It takes into account possible
    /// overlappings to create, merge or enlarge existing ranges if necessary.
    pub fn insert(&mut self, range: Range) -> Result<(), Error> {
        self.sort_insert(range)?;
        self.merge();
        Ok(())
    }

    /// Removes a `Range` from the `RangeSet`. It takes into account possible
    /// overlappings to delete, split or shrink existing ranges if necessary.
    pub fn remove(&mut self, range: Range) -> Result<(), Error> {
        let mut i = 0;
        while i < self.in_use {
            // Given that the internal `range` array is sorted, once the start
            // point of a range is above the end point of the range to remove,
            // it is not necessary to continue iterating.
            if self.ranges[i].start > range.end {
                break;
            }

            // If the ranges do not overlap, advance.
            if !self.ranges[i].overlaps(range) {
                i += 1;
                continue;
            }

            if self.ranges[i].contains_range(range) {
                // The range to be removed is contained by the existing range.
                if self.ranges[i] == range {
                    // The range to be removed matches the existing range.
                    // Then, the existing range must be removed.
                    self.ranges.copy_within(i + 1..self.in_use, i);
                    self.in_use -= 1;
                } else if self.ranges[i].start == range.start {
                    // The range to be removed and the existing range share the
                    // same start point. Then, it is enough with updating the
                    // start point of the existing range.
                    self.ranges[i].start = range.end + 1;
                } else if self.ranges[i].end == range.end {
                    // The range to be removed and the existing range share the
                    // same end point. Then, it is enough with updating the end
                    // point of the existing range.
                    self.ranges[i].end = range.start - 1;
                } else {
                    // The range to be removed is in the middle of the existing
                    // range. Then, the existing range must be split and the
                    // start and end points of the new ranges updated
                    // accordingly.
                    if self.in_use >= self.ranges.len() {
                        return Err(Error::FullRangeSet);
                    }

                    let new_range =
                        Range::new(self.ranges[i].start, range.start - 1)?;
                    self.ranges.copy_within(i..self.in_use, i + 1);
                    self.ranges[i] = new_range;
                    self.ranges[i + 1].start = range.end + 1;
                    self.in_use += 1;
                }

                break;
            } else if range.contains_range(self.ranges[i]) {
                // The range to be removed contains the existing range. Then,
                // the existing range must be removed.
                self.ranges.copy_within(i + 1..self.in_use, i);
                self.in_use -= 1;
            } else if self.ranges[i].contains_point(range.start) {
                // The start point of the range to be removed is contained by
                // the existing range. Then, the end point of the existing
                // range must be updated.
                self.ranges[i].end = range.start - 1;
                i += 1;
            } else {
                // The end point of the range to be removed is contained by the
                // existing range. Then, the start point of the existing range
                // must be updated.
                self.ranges[i].start = range.end + 1;
                i += 1;
            }
        }

        Ok(())
    }

    /// Returns the sum of the size of all the ranges in the `RangeSet`.
    pub fn size(&self) -> u64 {
        self.ranges().iter().map(|r| r.size()).sum()
    }

    /// Returns the lowest start point of the [`RangeSet`].
    pub fn start(&self) -> Option<u64> {
        self.ranges().first().map(|r| r.start())
    }

    /// Returns the highest end point of the [`RangeSet`].
    pub fn end(&self) -> Option<u64> {
        self.ranges().last().map(|r| r.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rangeset_insert_not_overlapped() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(15, 15).unwrap()).unwrap();
        let want = [
            Range::new(0, 10).unwrap(),
            Range::new(15, 15).unwrap(),
            Range::new(20, 30).unwrap(),
        ];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_contiguous() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(11, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        let want = [Range::new(0, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_overlapped() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(5, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        let want = [Range::new(0, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_overlapped_start() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        let want = [Range::new(0, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_overlapped_end() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();
        let want = [Range::new(0, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_contained() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 40).unwrap()).unwrap();
        let want = [Range::new(0, 40).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_contained_multiple() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(25, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 40).unwrap()).unwrap();
        let want = [Range::new(0, 40).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert() {
        let mut rangeset = RangeSet::new();

        rangeset.insert(Range::new(61, 70).unwrap()).unwrap();
        rangeset.insert(Range::new(45, 55).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();
        rangeset.insert(Range::new(35, 60).unwrap()).unwrap();

        rangeset.insert(Range::new(0, 5).unwrap()).unwrap();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(5, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 21).unwrap()).unwrap();
        rangeset.insert(Range::new(21, 30).unwrap()).unwrap();

        let want = [Range::new(0, 30).unwrap(), Range::new(35, 70).unwrap()];

        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_full() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE {
            let point = 2 * (i as u64);
            rangeset.insert(Range::new(point, point).unwrap()).unwrap();
        }
    }

    #[test]
    fn test_rangeset_insert_full_middle() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE - 1 {
            let point = 10 * (i as u64);
            rangeset.insert(Range::new(point, point).unwrap()).unwrap();
        }

        rangeset.insert(Range::new(5, 5).unwrap()).unwrap();
    }

    #[test]
    fn test_rangeset_insert_full_plus_one() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE {
            let point = 2 * (i as u64);
            rangeset.insert(Range::new(point, point).unwrap()).unwrap();
        }

        match rangeset.insert(Range::new(1337, 1337).unwrap()) {
            Err(Error::FullRangeSet) => {}
            ret => panic!("unexpected result: {:?}", ret),
        }
    }

    #[test]
    fn test_rangeset_insert_full_reuse() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE {
            let point = 2 * (i as u64);
            rangeset.insert(Range::new(point, point).unwrap()).unwrap();
        }

        rangeset.insert(Range::new(0, 1337).unwrap()).unwrap()
    }

    #[test]
    fn test_rangeset_remove_empty() {
        let mut rangeset = RangeSet::new();

        rangeset.remove(Range::new(20, 30).unwrap()).unwrap();

        assert_eq!(rangeset.ranges(), []);
    }

    #[test]
    fn test_rangeset_remove_unmodified() {
        let mut rangeset = RangeSet::new();

        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(0, 19).unwrap()).unwrap();
        rangeset.remove(Range::new(51, 70).unwrap()).unwrap();

        let want = [Range::new(20, 30).unwrap(), Range::new(40, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_one() {
        // Starting at the start point and finishing at the end point of the
        // removed range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(20, 30).unwrap()).unwrap();

        let want = [Range::new(0, 10).unwrap(), Range::new(40, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Starting before the start point and finishing after the end point of
        // the removed range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(18, 32).unwrap()).unwrap();

        let want = [Range::new(0, 10).unwrap(), Range::new(40, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_split() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(6, 14).unwrap()).unwrap();

        let want = [Range::new(0, 5).unwrap(), Range::new(15, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_split_left() {
        // Starting at the start and finishing at the middle of the modified
        // range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(0, 4).unwrap()).unwrap();

        let want = [Range::new(5, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Starting before the start and finishing at the middle of the
        // modified range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(0, 10).unwrap()).unwrap();

        let want = [Range::new(11, 20).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_split_right() {
        // Starting at the middle and finishing at the end of the modified
        // range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(16, 20).unwrap()).unwrap();

        let want = [Range::new(0, 15).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Starting at the middle and finishing after the end of the modified
        // range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(16, 25).unwrap()).unwrap();

        let want = [Range::new(0, 15).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_overlapped_two() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();

        rangeset.remove(Range::new(6, 24).unwrap()).unwrap();

        let want = [Range::new(0, 5).unwrap(), Range::new(25, 30).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_overlapped_three() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(6, 44).unwrap()).unwrap();

        let want = [Range::new(0, 5).unwrap(), Range::new(45, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_one_plus_overlap() {
        // Starting at the start point of the first range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(20, 44).unwrap()).unwrap();

        let want = [Range::new(45, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Starting before the start point of the first range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(18, 44).unwrap()).unwrap();

        let want = [Range::new(45, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_all() {
        // Starting at the start point of the first range and finishing at the
        // end point of the last range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(20, 50).unwrap()).unwrap();

        assert_eq!(rangeset.ranges(), []);

        // Starting before the start point of the first range and finishing
        // after the end point of the last range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(18, 52).unwrap()).unwrap();

        assert_eq!(rangeset.ranges(), []);
    }

    #[test]
    fn test_rangeset_remove_full_split() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE {
            let point = 10 * (i as u64);
            rangeset
                .insert(Range::new(point, point + 5).unwrap())
                .unwrap();
        }

        match rangeset.remove(Range::new(12, 13).unwrap()) {
            Err(Error::FullRangeSet) => {}
            ret => panic!("unexpected result: {:?}", ret),
        }
    }

    #[test]
    fn test_rangeset_remove_full() {
        let mut rangeset = RangeSet::new();

        for i in 0..RANGE_SET_SIZE - 1 {
            let point = 10 * (i as u64);
            rangeset
                .insert(Range::new(point, point + 5).unwrap())
                .unwrap();
        }

        rangeset.remove(Range::new(12, 13).unwrap()).unwrap();
    }

    #[test]
    fn test_rangeset_remove_edges() {
        // Remove right part of the range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();

        rangeset.remove(Range::new(1, 10).unwrap()).unwrap();

        let want = [Range::new(0, 0).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Remove left part of the range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();

        rangeset.remove(Range::new(0, 9).unwrap()).unwrap();

        let want = [Range::new(10, 10).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Remove central part of the range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();

        rangeset.remove(Range::new(1, 9).unwrap()).unwrap();

        let want = [Range::new(0, 0).unwrap(), Range::new(10, 10).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Remove central part of multiple ranges.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(1, 49).unwrap()).unwrap();

        let want = [Range::new(0, 0).unwrap(), Range::new(50, 50).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_not_overlapped_max() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(15, u64::MAX).unwrap()).unwrap();
        let want = [
            Range::new(0, 10).unwrap(),
            Range::new(15, u64::MAX).unwrap(),
        ];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_contiguous_max() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(11, u64::MAX).unwrap()).unwrap();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        let want = [Range::new(0, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_overlapped_max() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 10).unwrap()).unwrap();
        rangeset.insert(Range::new(5, u64::MAX).unwrap()).unwrap();
        let want = [Range::new(0, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_contained_multiple_max() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, 20).unwrap()).unwrap();
        rangeset.insert(Range::new(25, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(0, u64::MAX).unwrap()).unwrap();
        let want = [Range::new(0, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_insert_max() {
        // Insert two times the range [u64::MAX, u64::MAX].
        let mut rangeset = RangeSet::new();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();
        let want = [Range::new(u64::MAX, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Insert the range [u64::MAX, u64::MAX] overlapping with another
        // range.
        let mut rangeset = RangeSet::new();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();
        rangeset.insert(Range::new(10, u64::MAX).unwrap()).unwrap();
        let want = [Range::new(10, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Insert the range [u64::MAX, u64::MAX] contiguous to another range.
        let mut rangeset = RangeSet::new();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();
        rangeset
            .insert(Range::new(10, u64::MAX - 1).unwrap())
            .unwrap();
        let want = [Range::new(10, u64::MAX).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Insert the range [u64::MAX, u64::MAX] and a range [x, u64::MAX-2]
        // that do not overlap.
        let mut rangeset = RangeSet::new();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();
        rangeset
            .insert(Range::new(10, u64::MAX - 2).unwrap())
            .unwrap();
        let want = [
            Range::new(10, u64::MAX - 2).unwrap(),
            Range::new(u64::MAX, u64::MAX).unwrap(),
        ];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_all_max() {
        let mut rangeset = RangeSet::new();

        rangeset.insert(Range::new(20, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(40, 50).unwrap()).unwrap();

        rangeset.remove(Range::new(20, u64::MAX).unwrap()).unwrap();

        assert_eq!(rangeset.ranges(), []);
    }

    #[test]
    fn test_rangeset_remove_split_max() {
        // Remove right side of range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, 20).unwrap()).unwrap();

        rangeset.remove(Range::new(16, u64::MAX).unwrap()).unwrap();

        let want = [Range::new(0, 15).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Remove right side of range with both ranges ending in `u64::MAX`.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(0, u64::MAX).unwrap()).unwrap();

        rangeset.remove(Range::new(16, u64::MAX).unwrap()).unwrap();

        let want = [Range::new(0, 15).unwrap()];
        assert_eq!(rangeset.ranges(), want);

        // Remove `u64::MAX` from range.
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(10, u64::MAX).unwrap()).unwrap();

        rangeset
            .remove(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();

        let want = [Range::new(10, u64::MAX - 1).unwrap()];
        assert_eq!(rangeset.ranges(), want);
    }

    #[test]
    fn test_rangeset_remove_max() {
        let mut rangeset = RangeSet::new();
        rangeset
            .insert(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();

        rangeset
            .remove(Range::new(u64::MAX, u64::MAX).unwrap())
            .unwrap();

        assert_eq!(rangeset.ranges(), []);
    }

    #[test]
    fn test_rangeset_start() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(25, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(5, 10).unwrap()).unwrap();
        assert_eq!(rangeset.start(), Some(5));
    }

    #[test]
    fn test_rangeset_end() {
        let mut rangeset = RangeSet::new();
        rangeset.insert(Range::new(25, 30).unwrap()).unwrap();
        rangeset.insert(Range::new(5, 10).unwrap()).unwrap();
        assert_eq!(rangeset.end(), Some(30));
    }

    #[test]
    fn test_rangeset_start_end_empty() {
        let rangeset = RangeSet::new();
        assert_eq!(rangeset.start(), None);
        assert_eq!(rangeset.end(), None);
    }
}
