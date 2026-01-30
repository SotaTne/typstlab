use crate::FeatureId;
use crate::Version;

pub trait Feature {
    const IGNORE: bool;
    const FEATURES: &'static [FeatureId];

    #[inline]
    fn ignore() -> bool {
        Self::IGNORE
    }

    #[inline]
    fn features() -> &'static [FeatureId] {
        if Self::IGNORE { &[] } else { Self::FEATURES }
    }

    #[inline]
    fn collect_unsupported(v: Version) -> Vec<FeatureId> {
        Self::features()
            .iter()
            .copied()
            .filter(|f| !f.supports(v))
            .collect()
    }

    #[inline]
    fn collect_deprecated(v: Version) -> Vec<FeatureId> {
        Self::features()
            .iter()
            .copied()
            .filter(|f| f.deprecated(v))
            .collect()
    }

    #[inline]
    fn has_unsupported(v: Version) -> bool {
        Self::features().iter().any(|f| !f.supports(v))
    }

    #[inline]
    fn has_deprecated(v: Version) -> bool {
        Self::features().iter().any(|f| f.deprecated(v))
    }
}
