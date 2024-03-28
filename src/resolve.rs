use crate::{action::RawResponse, routing::Captures, type_cache::TypeCacheKey, RequestState};

/// `Resolve` is a trait used to construct values needed to call a given `System`. All parameters
/// of a `System` must implement `Resolve` to be valid.
pub trait Resolve<'a>: Sized {
    type Output: 'a;

    fn resolve(
        ctx: &'a RequestState,
        captures: &mut Captures,
    ) -> ResolveGuard<Self::Output>;
}

/// `ResolveGuard` is the expected return type of top level `Resolve`able objects. Only types that
/// return `ResolveGuard` can be used as function parameters
pub enum ResolveGuard<T> {
    /// Succesful value, run the system
    Value(T),
    /// Don't run this system or any others, respond early with this response
    Respond(RawResponse),
    /// Don't run this system, but continue routing to other systems
    None,
}

impl<T> From<Option<T>> for ResolveGuard<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => ResolveGuard::Value(v),
            None => ResolveGuard::None,
        }
    }
}

impl<T> ResolveGuard<T> {
    pub fn map<N>(self, f: fn(T) -> N) -> ResolveGuard<N> {
        match self {
            ResolveGuard::Value(v) => ResolveGuard::Value(f(v)),
            ResolveGuard::Respond(v) => ResolveGuard::Respond(v),
            ResolveGuard::None => ResolveGuard::None,
        }
    }
}

/// "Query" a value from the global_cache of the `RequestState` and clone it.
pub struct Query<K>(pub K::Value)
where
    K: TypeCacheKey;

impl<'a, K> Resolve<'a> for Query<K>
where
    K: TypeCacheKey,
    K::Value: Clone,
{
    type Output = Self;

    fn resolve(ctx: &'a RequestState, _captures: &mut Captures) -> ResolveGuard<Self> {
        ctx.global_cache.get::<K>().map(|v| Query(v.clone())).into()
    }
}

/// Consumes the next part of the url `path_iter`. Note that this will happen on call to its
/// `resolve` method so ordering of parameters matter. Place any necessary guards before this
/// method.
pub struct UrlPart(pub String);

impl<'a> Resolve<'a> for UrlPart {
    type Output = Self;

    fn resolve(_ctx: &'a RequestState, captures: &mut Captures) -> ResolveGuard<Self> {
        let Some(part) = captures.pop_front() else {
            return ResolveGuard::None;
        };

        ResolveGuard::Value(UrlPart(part))
    }
}

/// Collect the entire remaining url into a `Vec` Note that this will happen on call to its
/// `resolve` method so ordering of parameters matter. Place any necessary guards before this
/// method.
pub struct UrlCollect(pub Vec<String>);

impl<'a> Resolve<'a> for UrlCollect {
    type Output = Self;

    fn resolve(_ctx: &'a RequestState, captures: &mut Captures) -> ResolveGuard<Self> {
        let mut new = Vec::new();

        while let Some(part) = captures.pop_front() {
            new.push(part)
        }

        ResolveGuard::Value(UrlCollect(new))
    }
}

impl<'a, 'b> Resolve<'a> for &'b [u8] {
    type Output = &'a [u8];

    fn resolve(
        ctx: &'a RequestState,
        _captures: &mut Captures,
    ) -> ResolveGuard<Self::Output> {
        ResolveGuard::Value(ctx.request.body().get_as_slice())
    }
}

impl<'a, 'b> Resolve<'a> for &'b str {
    type Output = &'a str;

    fn resolve(
        ctx: &'a RequestState,
        _captures: &mut Captures,
    ) -> ResolveGuard<Self::Output> {
        std::str::from_utf8(ctx.request.body().get_as_slice())
            .ok()
            .into()
    }
}
