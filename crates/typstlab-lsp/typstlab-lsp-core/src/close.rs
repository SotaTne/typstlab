//! Explicit close hooks for LSP/CLI/GUI.
//!
//! - `Close::close()` is a *logical cleanup* hook (drop caches, clear containers, release resources).
//! - `Close::close_and_shrink()` is an optional *memory release* hook (best-effort).
//!
//! Notes:
//! - `Drop` is implicit and timing is not controllable.
//! - `Close` is explicit and should be called on `didClose` (and also usable by CLI/GUI).
//! - For async locks (`tokio::sync::*`), this sync trait intentionally does NOT provide impls.
//!   If you need async close, define a separate `AsyncClose` trait.
use core::mem;

pub trait Close {
    /// Logical cleanup hook (fast path).
    fn close(&mut self);

    /// Optional memory release hook (best-effort).
    /// Default: just `close()`.
    #[inline]
    fn close_and_shrink(&mut self) {
        self.close();
    }
}

// --------------------
// Base no-op
// --------------------

impl Close for () {
    #[inline]
    fn close(&mut self) {}
}

// Primitive-ish “safe defaults”

macro_rules! impl_primitive_close {
    ($($T:ty),+) => {
        $(
            impl Close for $T {
                #[inline]
                fn close(&mut self) {}
            }
        )+
    };
}

impl_primitive_close!(
    bool, u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, char
);

// --------------------
// Pointer-like containers
// --------------------

impl<T: Close + ?Sized> Close for Box<T> {
    #[inline]
    fn close(&mut self) {
        (**self).close();
    }
    #[inline]
    fn close_and_shrink(&mut self) {
        (**self).close_and_shrink();
    }
}

impl<T: Close> Close for Option<T> {
    #[inline]
    fn close(&mut self) {
        if let Some(v) = self.as_mut() {
            v.close();
        }
        *self = None;
    }
    #[inline]
    fn close_and_shrink(&mut self) {
        if let Some(v) = self.as_mut() {
            v.close_and_shrink();
        }
        *self = None;
    }
}

impl<T: Close, E: Close> Close for Result<T, E> {
    #[inline]
    fn close(&mut self) {
        match self {
            Ok(v) => v.close(),
            Err(e) => e.close(),
        }
    }
    #[inline]
    fn close_and_shrink(&mut self) {
        match self {
            Ok(v) => v.close_and_shrink(),
            Err(e) => e.close_and_shrink(),
        }
    }
}

impl<T: ?Sized> Close for &T {
    #[inline]
    fn close(&mut self) {}
}

impl<T: ?Sized> Close for &mut T {
    #[inline]
    fn close(&mut self) {}
}

// --------------------
// Common std types
// --------------------

impl Close for String {
    #[inline]
    fn close(&mut self) {
        self.clear();
    }
    #[inline]
    fn close_and_shrink(&mut self) {
        self.clear();
        self.shrink_to_fit();
    }
}

// --------------------
// Collections (std)
// --------------------

impl<T: Close> Close for Vec<T> {
    fn close(&mut self) {
        for v in self.iter_mut() {
            v.close();
        }
        self.clear();
    }
    fn close_and_shrink(&mut self) {
        for v in self.iter_mut() {
            v.close_and_shrink();
        }
        self.clear();
        self.shrink_to_fit();
    }
}

impl<T: Close, const N: usize> Close for [T; N] {
    fn close(&mut self) {
        for v in self.iter_mut() {
            v.close();
        }
    }
    fn close_and_shrink(&mut self) {
        for v in self.iter_mut() {
            v.close_and_shrink();
        }
    }
}

impl<T: Close> Close for std::collections::VecDeque<T> {
    fn close(&mut self) {
        for v in self.iter_mut() {
            v.close();
        }
        self.clear();
    }
    fn close_and_shrink(&mut self) {
        for v in self.iter_mut() {
            v.close_and_shrink();
        }
        self.clear();
        self.shrink_to_fit();
    }
}

impl<K, V, S> Close for std::collections::HashMap<K, V, S>
where
    K: core::hash::Hash + Eq,
    V: Close,
    S: std::hash::BuildHasher,
{
    fn close(&mut self) {
        // drain() で所有して close できる
        for (_k, mut v) in self.drain() {
            v.close();
        }
    }
    fn close_and_shrink(&mut self) {
        for (_k, mut v) in self.drain() {
            v.close_and_shrink();
        }
        self.shrink_to_fit();
    }
}

impl<T, S> Close for std::collections::HashSet<T, S>
where
    T: core::hash::Hash + Eq + Close,
    S: std::hash::BuildHasher,
{
    fn close(&mut self) {
        // HashSet は要素の &mut を取れないので drain() で所有して close するのが正攻法
        for mut v in self.drain() {
            v.close();
        }
    }
    fn close_and_shrink(&mut self) {
        for mut v in self.drain() {
            v.close_and_shrink();
        }
        self.shrink_to_fit();
    }
}

impl<K, V> Close for std::collections::BTreeMap<K, V>
where
    V: Close,
{
    fn close(&mut self) {
        // 安定: take して所有し close
        let old = mem::take(self);
        for (_k, mut v) in old.into_iter() {
            v.close();
        }
    }
    fn close_and_shrink(&mut self) {
        let old = mem::take(self);
        for (_k, mut v) in old.into_iter() {
            v.close_and_shrink();
        }
    }
}

impl<T> Close for std::collections::BTreeSet<T>
where
    T: Close,
{
    fn close(&mut self) {
        let old = mem::take(self);
        for mut v in old.into_iter() {
            v.close();
        }
    }
    fn close_and_shrink(&mut self) {
        let old = mem::take(self);
        for mut v in old.into_iter() {
            v.close_and_shrink();
        }
    }
}

// --------------------
// std::sync locks (同期で完結するもののみ)
// --------------------

impl<T: Close> Close for std::sync::Mutex<T> {
    fn close(&mut self) {
        let mut guard = match self.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.close();
    }
    fn close_and_shrink(&mut self) {
        let mut guard = match self.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.close_and_shrink();
    }
}

impl<T: Close> Close for std::sync::RwLock<T> {
    fn close(&mut self) {
        let mut guard = match self.write() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.close();
    }
    fn close_and_shrink(&mut self) {
        let mut guard = match self.write() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.close_and_shrink();
    }
}

impl<'a, T> Close for std::borrow::Cow<'a, T>
where
    T: ToOwned + ?Sized,
{
    #[inline]
    fn close(&mut self) {
        // 参照か所有かを問わず「閉じる」必要はないので no-op でOK
    }
}

// --------------------
// Tuples
// --------------------

macro_rules! impl_close_tuple {
    ($($idx:tt : $T:ident),+ $(,)?) => {
        impl<$($T: Close),+> Close for ($($T,)+) {
            fn close(&mut self) {
                $( self.$idx.close(); )+
            }
            fn close_and_shrink(&mut self) {
                $( self.$idx.close_and_shrink(); )+
            }
        }
    };
}

impl_close_tuple!(0:A);
impl_close_tuple!(0:A, 1:B);
impl_close_tuple!(0:A, 1:B, 2:C);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G, 7:H);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G, 7:H, 8:I);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G, 7:H, 8:I, 9:J);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G, 7:H, 8:I, 9:J, 10:K);
impl_close_tuple!(0:A, 1:B, 2:C, 3:D, 4:E, 5:F, 6:G, 7:H, 8:I, 9:J, 10:K, 11:L);

// --------------------
// Helpers
// --------------------

/// A marker newtype for values that must not be closed by derive macros.
/// (If you don't want `#[close(skip)]`, wrapping with `NoClose<T>` is an escape hatch.)
pub struct NoClose<T>(pub T);

impl<T> Close for NoClose<T> {
    #[inline]
    fn close(&mut self) {}
    #[inline]
    fn close_and_shrink(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::sync::{Mutex, RwLock};

    macro_rules! assert_closed {
        ($val:expr) => {{
            let mut v = $val;
            v.close();
            assert!(is_empty_ish(&v), "Expected empty after close()");
        }};
    }

    macro_rules! assert_shrunk {
        ($val:expr) => {{
            let mut v = $val;
            v.close_and_shrink();
            assert!(is_empty_ish(&v), "Expected empty after close_and_shrink()");
            // Capacity check for supported types
            check_shrunk(&v);
        }};
    }

    fn is_empty_ish<T: Close + 'static>(v: &T) -> bool {
        let any = v as &dyn std::any::Any;
        if let Some(s) = any.downcast_ref::<String>() {
            return s.is_empty();
        }
        if let Some(v) = any.downcast_ref::<Vec<i32>>() {
            return v.is_empty();
        }
        if let Some(o) = any.downcast_ref::<Option<i32>>() {
            return o.is_none();
        }
        true
    }

    fn check_shrunk<T: Close + 'static>(v: &T) {
        let any = v as &dyn std::any::Any;
        if let Some(s) = any.downcast_ref::<String>() {
            assert_eq!(s.capacity(), 0);
        }
        if let Some(v) = any.downcast_ref::<Vec<i32>>() {
            assert_eq!(v.capacity(), 0);
        }
    }

    #[test]
    fn test_containers() {
        assert_closed!(vec![1, 2, 3]);
        assert_shrunk!(vec![1, 2, 3]);

        let mut dq = std::collections::VecDeque::new();
        dq.push_back(1);
        assert_closed!(dq);

        assert_closed!(String::from("hello"));
        assert_shrunk!(String::from("hello"));
    }

    #[test]
    fn test_options_results() {
        assert_closed!(Some(123i32));

        let mut res: Result<Vec<i32>, String> = Ok(vec![1]);
        res.close();
        assert!(res.unwrap().is_empty());

        let mut res: Result<Vec<i32>, String> = Err(String::from("err"));
        res.close();
        assert!(res.unwrap_err().is_empty());
    }

    #[test]
    fn test_maps_sets() {
        let mut m = HashMap::new();
        m.insert(1, vec![1]);
        assert_closed!(m);

        let mut s = HashSet::new();
        s.insert(vec![1]);
        assert_closed!(s);

        let mut bm = BTreeMap::new();
        bm.insert(1, vec![1]);
        assert_closed!(bm);

        let mut bs = BTreeSet::new();
        bs.insert(vec![1]);
        assert_closed!(bs);
    }

    #[test]
    fn test_locks() {
        assert_closed!(Mutex::new(vec![1]));
        assert_closed!(RwLock::new(vec![1]));
    }

    #[test]
    fn test_recursive_complex() {
        let mut t = (vec![1], (Some(vec![2]), vec![3]));
        t.close();
        assert!(t.0.is_empty());
        assert!((t.1).0.is_none());
        assert!((t.1).1.is_empty());
    }

    #[test]
    fn test_wrappers() {
        assert_closed!(Box::new(vec![1]));

        let mut nc = NoClose(vec![1, 2, 3]);
        nc.close();
        assert_eq!(nc.0.len(), 3);
    }

    #[test]
    fn test_primitives() {
        let mut b = true;
        b.close();
        assert!(b);
        let mut n = 100u64;
        n.close();
        assert_eq!(n, 100);
    }
}
