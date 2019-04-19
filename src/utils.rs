/// Combines multiple routes with the same return type together with
/// `or()` and `unify()`
#[macro_export]
macro_rules! or {
    ($filter:expr, $($other_filter:expr),*) => {
        $filter$(.or($other_filter).unify())*
    };
}
