use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const fn as_tuple(&self) -> (u16, u16, u16) {
        (self.major, self.minor, self.patch)
    }

    /// Packs (major, minor, patch) into u64 for fast ordering.
    /// Assumes each component fits in u16.
    pub const fn to_u64(&self) -> u64 {
        ((self.major as u64) << 32) | ((self.minor as u64) << 16) | (self.patch as u64)
    }

    pub const fn le(self, other: Version) -> bool {
        self.to_u64() <= other.to_u64()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SupportRange {
    pub since: Version,
    pub until: Option<Version>,
}

impl SupportRange {
    pub const fn new(since: Version, until: Option<Version>) -> Self {
        if let Some(u) = until {
            assert!(since.le(u));
        }
        Self { since, until }
    }

    pub const fn supports(&self, v: Version) -> bool {
        let x = v.to_u64();
        let lo = self.since.to_u64();
        if x < lo {
            return false;
        }
        match self.until {
            Some(u) => x <= u.to_u64(),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_to_u64() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(1, 3, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1.to_u64() < v2.to_u64());
        assert!(v2.to_u64() < v3.to_u64());
        assert!(v3.to_u64() < v4.to_u64());
    }

    #[test]
    fn test_support_range_supports() {
        let range = SupportRange::new(Version::new(1, 0, 0), Some(Version::new(2, 0, 0)));

        assert!(!range.supports(Version::new(0, 9, 9)));
        assert!(range.supports(Version::new(1, 0, 0)));
        assert!(range.supports(Version::new(1, 5, 0)));
        assert!(range.supports(Version::new(2, 0, 0)));
        assert!(!range.supports(Version::new(2, 0, 1)));

        let open_range = SupportRange::new(Version::new(1, 0, 0), None);
        assert!(open_range.supports(Version::new(3, 0, 0)));
    }

    #[test]
    #[should_panic]
    fn test_invalid_support_range_panics() {
        // since > until should panic
        SupportRange::new(Version::new(2, 0, 0), Some(Version::new(1, 0, 0)));
    }
}
