use http::Method;

use crate::{action::RawResponse, type_cache::TypeCacheKey, PathIter, RequestState};

/// `Resolve` is a trait used to construct values needed to call a given `System`. All parameters
/// of a `System` must implement `Resolve` to be valid.
pub trait Resolve<'a>: Sized {
    type Output: 'a;

    fn resolve(ctx: &'a RequestState, path_iter: &mut PathIter) -> ResolveGuard<Self::Output>;
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

/// Get request guard. A system with this as a parameter requires that the method be GET in order
/// to run.
pub struct Get;

impl<'a> Resolve<'a> for Get {
    type Output = Self;

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self> {
        if ctx.request.method() == Method::GET {
            ResolveGuard::Value(Get)
        } else {
            ResolveGuard::None
        }
    }
}

/// Get request guard. A system with this as a parameter requires that the method be POST in order
/// to run.
pub struct Post;

impl<'a> Resolve<'a> for Post {
    type Output = Self;

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self> {
        if ctx.request.method() == Method::POST {
            ResolveGuard::Value(Post)
        } else {
            ResolveGuard::None
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

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self> {
        ctx.global_cache
            .read()
            .unwrap()
            .get::<K>()
            .map(|v| Query(v.clone()))
            .into()
    }
}

/// A function with `Endpoint` as a parameter requires that the internal `path_iter` of the
/// `RequestState` must be empty. This will only run if there are no trailing path parts of the
/// uri.
pub struct Endpoint;

impl<'a> Resolve<'a> for Endpoint {
    type Output = Self;

    fn resolve(_ctx: &RequestState, path_iter: &mut PathIter) -> ResolveGuard<Self> {
        match path_iter.peek() {
            Some(v) if !v.is_empty() => ResolveGuard::None,
            _ => ResolveGuard::Value(Endpoint),
        }
    }
}

/// Consumes the next part of the url `path_iter`. Note that this will happen on call to its
/// `resolve` method so ordering of parameters matter. Place any necessary guards before this
/// method.
pub struct UrlPart(pub String);

impl<'a> Resolve<'a> for UrlPart {
    type Output = Self;

    fn resolve(_ctx: &'a RequestState, path_iter: &mut PathIter) -> ResolveGuard<Self> {
        path_iter.next().map(|i| UrlPart(i.to_string())).into()
    }
}

/// Collect the entire remaining url into a `Vec` Note that this will happen on call to its
/// `resolve` method so ordering of parameters matter. Place any necessary guards before this
/// method.
pub struct UrlCollect(pub Vec<String>);

impl<'a> Resolve<'a> for UrlCollect {
    type Output = Self;

    fn resolve(_ctx: &'a RequestState, path_iter: &mut PathIter) -> ResolveGuard<Self> {
        let mut collect = Vec::new();

        for part in path_iter.by_ref().map(|i| i.to_string()) {
            collect.push(part.to_string())
        }

        ResolveGuard::Value(UrlCollect(collect))
    }
}

impl<'a, 'b> Resolve<'a> for &'b Vec<u8> {
    type Output = &'a Vec<u8>;

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        ResolveGuard::Value(ctx.request.body().get())
    }
}

impl<'a, 'b> Resolve<'a> for &'b str {
    type Output = &'a str;

    fn resolve(ctx: &'a RequestState, _path_iter: &mut PathIter) -> ResolveGuard<Self::Output> {
        std::str::from_utf8(ctx.request.body().get()).ok().into()
    }
}
