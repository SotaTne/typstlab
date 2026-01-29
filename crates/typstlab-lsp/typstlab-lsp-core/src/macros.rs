//! Helper macros for version-gating and feature management.
//!
//! These macros provide a DSL-like syntax for defining Typst versions,
//! support ranges, and feature-specific rules.

/// Helper to handle version components.
///
/// Supports `min` (0), `max` (u16::MAX), or literal numbers.
#[macro_export]
macro_rules! ver_part {
    (max) => {
        u16::MAX
    };
    (min) => {
        0
    };
    ($n:literal) => {
        $n
    };
}

/// Creates a [`Version`] instance.
///
/// # Examples
///
/// ```
/// # use typstlab_lsp_core::version;
/// let v = version!(0, 12, 0);
/// assert_eq!(v.major, 0);
/// assert_eq!(v.patch, 0);
///
/// // Special markers
/// let v_max = version!(0, 0, max);
/// assert_eq!(v_max.patch, u16::MAX);
/// ```
#[macro_export]
macro_rules! version {
    ($maj:tt, $min:tt, $patch:tt) => {
        $crate::Version::new(
            $crate::ver_part!($maj),
            $crate::ver_part!($min),
            $crate::ver_part!($patch),
        )
    };
}

/// Creates a [`SupportRange`].
///
/// # Syntax
///
/// - `support!(maj, min, pat)`: Supported since this version (inclusive, no upper bound).
/// - `support!(to_incl maj, min, pat)`: Supported until this version (inclusive, starts from 0.0.0).
/// - `support!(maj1, min1, pat1 to_incl maj2, min2, pat2)`: A closed range.
///
/// # Examples
///
/// ```
/// # use typstlab_lsp_core::{support, version};
/// let range = support!(0, 12, 0);
/// assert!(range.supports(version!(0, 12, 0)));
///
/// let closed = support!(0, 12, 0 to_incl 0, 13, 0);
/// assert!(!closed.supports(version!(0, 14, 0)));
/// ```
#[macro_export]
macro_rules! support {
    ($maj:tt, $min:tt, $patch:tt) => {
        $crate::SupportRange::new($crate::version!($maj, $min, $patch), None)
    };
    ($maj1:tt, $min1:tt, $pat1:tt to_incl $maj2:tt, $min2:tt, $pat2:tt) => {
        $crate::SupportRange::new(
            $crate::version!($maj1, $min1, $pat1),
            Some($crate::version!($maj2, $min2, $pat2)),
        )
    };
    (to_incl $maj:tt, $min:tt, $pat:tt) => {
        $crate::SupportRange::new(
            $crate::version!(0, 0, 0),
            Some($crate::version!($maj, $min, $pat)),
        )
    };
}

/// Creates a [`Rules`] container from multiple [`FeatureRule`]s.
///
/// This macro ensures the rules set is not empty at runtime.
#[macro_export]
macro_rules! rules {
    ( $( $rule:expr ),+ $(,)? ) => {
        $crate::Rules::new(&[$( $rule ),+])
    };
}

/// Creates an `Unsupported` [`FeatureRule`].
///
/// This indicates that the feature is NOT supported within the specified version range.
#[macro_export]
macro_rules! unsupported {
    ($a:tt, $b:tt, $c:tt) => {
        $crate::FeatureRule { range: $crate::support!($a,$b,$c), kind: $crate::RuleKind::Unsupported }
    };
    (to_incl $a:tt, $b:tt, $c:tt) => {
        $crate::FeatureRule { range: $crate::support!(to_incl $a,$b,$c), kind: $crate::RuleKind::Unsupported }
    };
    ($a:tt, $b:tt, $c:tt to_incl $d:tt, $e:tt, $f:tt) => {
        $crate::FeatureRule { range: $crate::support!($a,$b,$c to_incl $d,$e,$f), kind: $crate::RuleKind::Unsupported }
    };
}

/// Creates a `Deprecated` [`FeatureRule`].
///
/// This indicates that the feature IS supported but deprecated within the specified version range.
#[macro_export]
macro_rules! deprecated {
    ($a:tt, $b:tt, $c:tt) => {
        $crate::FeatureRule { range: $crate::support!($a,$b,$c), kind: $crate::RuleKind::Deprecated }
    };
    (to_incl $a:tt, $b:tt, $c:tt) => {
        $crate::FeatureRule { range: $crate::support!(to_incl $a,$b,$c), kind: $crate::RuleKind::Deprecated }
    };
    ($a:tt, $b:tt, $c:tt to_incl $d:tt, $e:tt, $f:tt) => {
        $crate::FeatureRule { range: $crate::support!($a,$b,$c to_incl $d,$e,$f), kind: $crate::RuleKind::Deprecated }
    };
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_version_parts() {
        assert_eq!(version!(1, 2, 3), Version::new(1, 2, 3));
        assert_eq!(version!(0, 0, max), Version::new(0, 0, u16::MAX));
        assert_eq!(version!(min, min, min), Version::new(0, 0, 0));
    }

    #[test]
    fn test_support_ranges() {
        let r1 = support!(1, 15, 0);
        assert_eq!(r1.since, version!(1, 15, 0));
        assert!(r1.until.is_none());

        let r2 = support!(to_incl 0, 15, 0);
        assert_eq!(r2.since, version!(0, 0, 0));
        assert_eq!(r2.until, Some(version!(0, 15, 0)));

        let r3 = support!(0, 14, 0 to_incl 0, 14, max);
        assert_eq!(r3.since, version!(0, 14, 0));
        assert_eq!(r3.until, Some(version!(0, 14, max)));
    }

    #[test]
    fn test_rules_construction() {
        const RS: Rules = rules!(
            unsupported!(to_incl 0, 11, max),
            deprecated!(0, 13, 0),
            unsupported!(1, 0, 0 to_incl 1, 1, 0),
        );

        assert_eq!(RS.0.len(), 3, "Should contain 3 rules");
        assert_eq!(RS.0[0].kind, RuleKind::Unsupported);
        assert_eq!(RS.0[1].kind, RuleKind::Deprecated);
        assert_eq!(RS.0[2].kind, RuleKind::Unsupported);

        assert_eq!(RS.0[0].range.until, Some(version!(0, 11, max)));
        assert_eq!(RS.0[1].range.since, version!(0, 13, 0));
        assert_eq!(RS.0[2].range.since, version!(1, 0, 0));
    }
}
