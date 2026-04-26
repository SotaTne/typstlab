#[macro_export]
macro_rules! impl_model {
    ($($t:ty),+) => {
        $(impl $crate::Model for $t {})*
    };
}

#[macro_export]
macro_rules! impl_entity {
    ($t:ty { $($item:tt)* }) => {
        impl $crate::Model for $t {}
        impl $crate::Entity for $t {
            $($item)*
        }
    };
}

#[macro_export]
macro_rules! impl_locatable {
    ($t:ty, $r:ty { $($item:tt)* }) => {
        impl $crate::Model for $t {}
        impl $crate::Locatable<$r> for $t {
            $($item)*
        }
    };
}
