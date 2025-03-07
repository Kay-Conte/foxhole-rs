use std::collections::{HashMap, VecDeque};

use crate::{
    action::RawResponse,
    error::{Error, IntoResponseError},
    routing::Captures,
    type_cache::TypeCacheKey,
    FoxholeResult, RequestState,
};

/// `Resolve` is a trait used to construct values needed to call a given `System`. All parameters
/// of a `System` must implement `Resolve` to be valid.
pub trait Resolve: Sized {
    type Output<'a>;

    fn resolve<'a>(
        ctx: &'a RequestState,
        captures: &mut Captures,
    ) -> FoxholeResult<Self::Output<'a>>;
}

/// `ResolveGuard` is the expected return type of top level `Resolve`able objects. Only types that
/// return `ResolveGuard` can be used as function parameters
pub enum ResolveGuard<T> {
    /// Succesful value, run the system
    Value(T),
    /// Don't run this system or any others, respond early with this response
    Respond(RawResponse),
    /// Don't run this system, but continue routing to other systems
    Err(Box<dyn IntoResponseError>),
}

impl<T> ResolveGuard<T> {
    pub fn map<N>(self, f: fn(T) -> N) -> ResolveGuard<N> {
        match self {
            ResolveGuard::Value(v) => ResolveGuard::Value(f(v)),
            ResolveGuard::Respond(v) => ResolveGuard::Respond(v),
            ResolveGuard::Err(e) => ResolveGuard::Err(e),
        }
    }

    pub fn err<E>(err: E) -> ResolveGuard<T>
    where
        E: IntoResponseError,
    {
        ResolveGuard::Err(Box::new(err))
    }
}

/// "Query" a value from the global_cache of the `RequestState`
pub struct Query<'a, K>(pub &'a K::Value)
where
    K: TypeCacheKey;

impl<K> Resolve for Query<'_, K>
where
    K: TypeCacheKey,
{
    type Output<'a> = Query<'a, K>;

    fn resolve<'b>(
        ctx: &'b RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'b>, Box<dyn IntoResponseError>> {
        match ctx.global_cache.get::<K>().map(|v| Query(v)) {
            Some(v) => Ok(v),
            None => Err(Box::new(Error::QueryNotInCache)),
        }
    }
}

/// Returns a reference to the path of the request.
pub struct Url<'a>(pub &'a str);

impl Resolve for Url<'_> {
    type Output<'a> = Url<'a>;

    fn resolve<'b>(
        ctx: &'b RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'b>, Box<dyn IntoResponseError>> {
        Ok(Url(ctx.request.uri().path()))
    }
}

/// Consumes the next part of the capture group. The capture group will be set in the same order it
/// is defined in the route url.
pub struct UrlPart(pub String);

impl Resolve for UrlPart {
    type Output<'a> = Self;

    fn resolve(
        _ctx: &RequestState,
        captures: &mut Captures,
    ) -> Result<Self, Box<dyn IntoResponseError>> {
        let Some(part) = captures.pop_front() else {
            return Err(Box::new(Error::MissingUrlPart));
        };

        Ok(UrlPart(part))
    }
}

/// Consumes all parts of the capture group. The capture group will be set in the same order it is
/// defined in the route url.
pub struct UrlCollect(pub Vec<String>);

impl Resolve for UrlCollect {
    type Output<'a> = Self;

    fn resolve(
        _ctx: &RequestState,
        captures: &mut Captures,
    ) -> Result<Self, Box<dyn IntoResponseError>> {
        let mut new = VecDeque::new();

        std::mem::swap(&mut new, captures);

        Ok(UrlCollect(Vec::from(new)))
    }
}

/// A case insensitive `HashMap` of headers
pub struct HeaderMap<'a>(pub &'a http::HeaderMap);

impl Resolve for HeaderMap<'_> {
    type Output<'b> = HeaderMap<'b>;

    fn resolve<'c>(
        ctx: &'c RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'c>, Box<dyn IntoResponseError>> {
        Ok(HeaderMap(ctx.request.headers()))
    }
}

/// A map of all url query parameters. Ex: "?foo=bar"
pub struct ArgMap<'a>(pub &'a HashMap<String, String>);

impl Resolve for ArgMap<'_> {
    type Output<'b> = ArgMap<'b>;

    fn resolve<'c>(
        ctx: &'c RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'c>, Box<dyn IntoResponseError>> {
        Ok(ArgMap(&ctx.query))
    }
}

impl<T> Resolve for Option<T>
where
    T: Resolve,
{
    type Output<'b> = Option<T::Output<'b>>;

    fn resolve<'c>(
        ctx: &'c RequestState,
        captures: &mut Captures,
    ) -> Result<Self::Output<'c>, Box<dyn IntoResponseError>> {
        match T::resolve(ctx, captures) {
            Ok(v) => Ok(Some(v)),
            _ => Ok(None),
        }
    }
}

impl Resolve for &[u8] {
    type Output<'b> = &'b [u8];

    fn resolve<'c>(
        ctx: &'c RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'c>, Box<dyn IntoResponseError>> {
        Ok(ctx.request.body().get_as_slice())
    }
}

impl Resolve for &str {
    type Output<'b> = &'b str;

    fn resolve<'c>(
        ctx: &'c RequestState,
        _captures: &mut Captures,
    ) -> Result<Self::Output<'c>, Box<dyn IntoResponseError>> {
        match std::str::from_utf8(ctx.request.body().get_as_slice()) {
            Ok(v) => Ok(v),
            Err(_) => Err(Box::new(Error::MalformedRequest)),
        }
    }
}
