use http::{Method, Response, Version};

use crate::{http_utils::IntoRawBytes, tasks::RequestState, type_cache::TypeCacheKey};

pub type RawResponse = Response<Vec<u8>>;

pub trait IntoResponse {
    fn response(self) -> RawResponse;
}

pub trait MaybeIntoResponse {
    fn maybe_response(self) -> Option<RawResponse>;
}

impl<T> MaybeIntoResponse for T
where
    T: IntoResponse,
{
    fn maybe_response(self) -> Option<RawResponse> {
        Some(self.response())
    }
}

impl<T> MaybeIntoResponse for Response<T>
where
    T: IntoRawBytes,
{
    fn maybe_response(self) -> Option<RawResponse> {
        Some(self.map(IntoRawBytes::into_raw_bytes))
    }
}

impl MaybeIntoResponse for () {
    fn maybe_response(self) -> Option<RawResponse> {
        None
    }
}

impl<T> MaybeIntoResponse for Option<T>
where
    T: MaybeIntoResponse,
{
    fn maybe_response(self) -> Option<RawResponse> {
        self.and_then(MaybeIntoResponse::maybe_response)
    }
}

impl<T, E> MaybeIntoResponse for Result<T, E>
where
    T: MaybeIntoResponse,
    E: MaybeIntoResponse,
{
    fn maybe_response(self) -> Option<RawResponse> {
        match self {
            Ok(v) => v.maybe_response(),
            Err(e) => e.maybe_response(),
        }
    }
}

impl IntoResponse for u16 {
    fn response(self) -> RawResponse {
        Response::builder()
            .version(Version::HTTP_11)
            .status(self)
            .header("Content-Type", "text/plain; charset=UTF-8")
            .header("Content-Length", "0")
            .body(Vec::new())
            .expect("Failed to build request")
    }
}

pub struct Html(pub String);

impl IntoResponse for Html {
    fn response(self) -> Response<Vec<u8>> {
        let bytes = self.0.into_bytes();

        Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", format!("{}", bytes.len()))
            .body(bytes)
            .unwrap()
    }
}

/// `Resolve` is a trait
pub trait Resolve: Sized {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self>;
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

impl Resolve for Get {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
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

impl Resolve for Post {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
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

impl<K> Resolve for Query<K>
where
    K: TypeCacheKey,
    K::Value: Clone,
{
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
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

impl Resolve for Endpoint {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
        match ctx.path_iter.peek() {
            Some(v) if !v.is_empty() => ResolveGuard::None,
            _ => ResolveGuard::Value(Endpoint),
        }
    }
}

/// Consumes the next part of the url `path_iter`. Note that this will happen on call to its
/// `resolve` method so ordering of parameters matter. Place any necessary guards before this
/// method.
pub struct UrlPart(pub String);

impl Resolve for UrlPart {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
        ctx.path_iter.next().map(|i| UrlPart(i.to_string())).into()
    }
}

pub struct UrlCollect(pub Vec<String>);

impl Resolve for UrlCollect {
    fn resolve(ctx: &mut RequestState) -> ResolveGuard<Self> {
        let mut collect = Vec::new();

        for part in ctx.path_iter.by_ref().map(|i| i.to_string()) {
            collect.push(part.to_string())
        }

        ResolveGuard::Value(UrlCollect(collect))
    }
}

pub trait System<T> {
    fn run(self, ctx: &mut RequestState) -> Option<RawResponse>;
}

pub struct DynSystem {
    inner: Box<dyn Fn(&mut RequestState) -> Option<RawResponse> + 'static + Send + Sync>,
}

impl DynSystem {
    pub fn new<T, A>(system: T) -> Self
    where
        T: System<A> + 'static + Copy + Send + Sync,
    {
        DynSystem {
            inner: Box::new(move |ctx| system.run(ctx)),
        }
    }

    pub fn call(&self, ctx: &mut RequestState) -> Option<RawResponse> {
        (self.inner)(ctx)
    }
}

macro_rules! system {
    ($($x:ident),* $(,)?) => {
        impl<RESPONSE, $($x,)* BASE> System<(RESPONSE, $($x,)*)> for BASE
        where
            BASE: Fn($($x,)*) -> RESPONSE,
            $($x: Resolve,)*
            RESPONSE: MaybeIntoResponse,
        {
            #[allow(unused)]
            fn run(self, ctx: &mut RequestState) -> Option<RawResponse> {


                $(
                #[allow(non_snake_case)]
                let $x = match $x::resolve(ctx) {
                    ResolveGuard::Value(v) => v,
                    ResolveGuard::None => return None,
                    ResolveGuard::Respond(r) => return Some(r), };)*

                let r = self($($x,)*);

                r.maybe_response()
            }
        }
    }
}

macro_rules! all {
    () => {
        system! { }
    };

    ($first:ident, $($x:ident),*$(,)?)  => {
        system! { $first, $($x,)* }

        all! { $($x,)*}
    }
}

all! { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z }
