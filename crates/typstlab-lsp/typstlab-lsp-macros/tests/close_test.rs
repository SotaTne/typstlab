use typstlab_lsp_core::Close;
use typstlab_lsp_macros::Close as DeriveClose;

#[derive(DeriveClose, Default)]
struct Simple {
    a: Vec<i32>,
    #[close(skip)]
    b: Vec<i32>,
}

#[test]
fn test_simple_struct() {
    let mut s = Simple {
        a: vec![1, 2, 3],
        b: vec![4, 5, 6],
    };
    s.close();
    assert!(s.a.is_empty());
    assert_eq!(s.b.len(), 3);
}

#[derive(DeriveClose, Default)]
#[close(shrink)]
struct ShrinkStruct {
    a: String,
    #[close(skip)]
    b: String,
}

#[test]
fn test_shrink_struct() {
    let mut s = ShrinkStruct {
        a: "hello".to_string(),
        b: "world".to_string(),
    };
    s.close();
    assert!(s.a.is_empty());
    assert_eq!(s.a.capacity(), 0);
    assert_eq!(s.b.len(), 5);
}

#[derive(DeriveClose)]
enum TestEnum {
    A(Vec<i32>),
    B {
        #[close(skip)]
        x: Vec<i32>,
        y: String,
    },
    #[warn(unused)]
    C,
}

#[test]
fn test_enum() {
    let mut e = TestEnum::A(vec![1, 2]);
    e.close();
    if let TestEnum::A(v) = e {
        assert!(v.is_empty());
    }

    let mut e = TestEnum::B {
        x: vec![1],
        y: "y".to_string(),
    };
    e.close();
    if let TestEnum::B { x, y } = e {
        assert_eq!(x.len(), 1);
        assert!(y.is_empty());
    }
}

#[derive(DeriveClose, Default)]
struct Nested {
    s: Simple,
    #[close(shrink)]
    opt: Option<String>,
}

#[test]
fn test_nested() {
    let mut n = Nested {
        s: Simple {
            a: vec![1],
            b: vec![2],
        },
        opt: Some("shrunk".to_string()),
    };
    n.close();
    assert!(n.s.a.is_empty());
    assert_eq!(n.s.b.len(), 1);
    // Option::close calls close on inner, which for String means clear.
    // But since opt has #[close(shrink)], it calls close_and_shrink() on the Option.
    // Close for Option calls close() on the value.
    // Actually, implementation of Close for Option<T> calls v.close().
    // If we want it to shrink, the macro calls close_and_shrink() on Option.
    assert!(n.opt.is_none()); // Option::close_and_shrink() makes it None
}

#[derive(DeriveClose, Default)]
struct DeepNested {
    #[close(shrink)]
    outer_shrunk: Nested,
    normal: Nested,
}

#[test]
fn test_deep_nested_shrink() {
    let mut d = DeepNested {
        outer_shrunk: Nested {
            s: Simple {
                a: vec![1, 2, 3],
                b: vec![4],
            },
            opt: Some("shrunk".to_string()),
        },
        normal: Nested {
            s: Simple {
                a: vec![1],
                b: vec![2],
            },
            opt: Some("normal".to_string()),
        },
    };

    // Case 1: close() on DeepNested
    // d.outer_shrunk.close_and_shrink() is called because of #[close(shrink)]
    // d.normal.close() is called
    d.close();

    // d.outer_shrunk was called with close_and_shrink()
    // Nested doesn't have #[close(shrink)] on type, so it calls close() on fields
    // UNLESS the call itself was close_and_shrink().
    // GenMethod::CloseAndShrink propagation means all fields of outer_shrunk
    // are called with close_and_shrink().
    assert_eq!(d.outer_shrunk.s.a.capacity(), 0);
    assert_eq!(d.outer_shrunk.opt.as_ref().map(|s| s.capacity()), None); // Option::close_and_shrink is None

    // d.normal was called with close()
    // Simple.a is close() -> empty but capacity remains
    assert!(d.normal.s.a.is_empty());
    assert!(d.normal.s.a.capacity() >= 1);
}

#[derive(DeriveClose, Default)]
#[close(shrink)]
struct PrecedenceTest {
    #[close(skip)]
    skipped_even_with_shrink_default: Vec<i32>,
    normal_follows_shrink_default: Vec<i32>,
}

#[test]
fn test_attribute_precedence() {
    let mut p = PrecedenceTest {
        skipped_even_with_shrink_default: vec![1, 2, 3],
        normal_follows_shrink_default: vec![4, 5, 6],
    };

    p.close();

    // skip > shrink (default)
    assert_eq!(p.skipped_even_with_shrink_default.len(), 3);

    // shrink (default) > nothing
    assert_eq!(p.normal_follows_shrink_default.capacity(), 0);
}
