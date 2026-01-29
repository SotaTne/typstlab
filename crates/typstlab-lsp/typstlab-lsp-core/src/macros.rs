// typstlab-lsp-core/src/macros.rs
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

#[macro_export]
macro_rules! rules {
    ( $( $rule:expr ),+ $(,)? ) => {
        $crate::Rules::new(&[$( $rule ),+])
    };
}

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
    fn test_version_macro() {
        assert_eq!(version!(1, 2, 3), Version::new(1, 2, 3));
        assert_eq!(version!(0, 0, max), Version::new(0, 0, u16::MAX));
        assert_eq!(version!(min, min, min), Version::new(0, 0, 0));
    }

    #[test]
    fn test_support_macro() {
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
    fn test_rules_unsupported_deprecated_macros() {
        const RS: Rules = rules!(
            unsupported!(to_incl 0, 14, max),
            deprecated!(0, 17, 0),
            unsupported!(1, 0, 0 to_incl 1, 1, 0),
            deprecated!(2, 0, 0 to_incl 2, 5, 0),
        );

        assert_eq!(RS.0.len(), 4);
        assert_eq!(RS.0[0].kind, RuleKind::Unsupported);
        assert_eq!(RS.0[0].range.until, Some(version!(0, 14, max)));

        assert_eq!(RS.0[1].kind, RuleKind::Deprecated);
        assert_eq!(RS.0[1].range.since, version!(0, 17, 0));

        assert_eq!(RS.0[2].kind, RuleKind::Unsupported);
        assert_eq!(RS.0[2].range.since, version!(1, 0, 0));
        assert_eq!(RS.0[2].range.until, Some(version!(1, 1, 0)));

        assert_eq!(RS.0[3].kind, RuleKind::Deprecated);
        assert_eq!(RS.0[3].range.since, version!(2, 0, 0));
        assert_eq!(RS.0[3].range.until, Some(version!(2, 5, 0)));
    }
}
