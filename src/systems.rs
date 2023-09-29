use http::{Method, Response, Version};

use crate::{tasks::RequestState, http_utils::IntoRawBytes};

pub type RawResponse = Response<Vec<u8>>;

/// `Resolve` is a trait 
pub trait Resolve: Sized {
    type Output;

    fn resolve(ctx: &mut RequestState) -> Self::Output;
}

pub trait IntoResponse {
    fn response(self) -> RawResponse;
}

pub trait MaybeIntoResponse {
    fn maybe_response(self) -> Option<RawResponse>;
}

impl<T> MaybeIntoResponse for T where T: IntoResponse {
    fn maybe_response(self) -> Option<RawResponse> {
        Some(self.response())
    }
}

impl MaybeIntoResponse for () {
    fn maybe_response(self) -> Option<RawResponse> {
        None
    }
}

impl<T> MaybeIntoResponse for Option<T> where T: MaybeIntoResponse {
    fn maybe_response(self) -> Option<RawResponse> {
        self.and_then(MaybeIntoResponse::maybe_response)
    }
}

impl<T, E> MaybeIntoResponse for Result<T, E> where T: MaybeIntoResponse, E: MaybeIntoResponse {
    fn maybe_response(self) -> Option<RawResponse> {
        match self {
            Ok(v) => v.maybe_response(),
            Err(e) => e.maybe_response(),
        }
    }
}

impl MaybeIntoResponse for u16 {
    fn maybe_response(self) -> Option<RawResponse> {
        Some(
            Response::builder()
                .version(Version::HTTP_10)
                .status(self)
                .header("Content-Type", "text/plain; charset=UTF-8")
                .header("Content-Length", "0")
                .body(Vec::new())
                .expect("Failed to build request"),
        )
    }
}

impl<T> MaybeIntoResponse for Response<T> where T: IntoRawBytes {
    fn maybe_response(self) -> Option<RawResponse> {
        Some(self.map(IntoRawBytes::into_raw_bytes))
    }
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
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut RequestState) -> Self::Output {
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
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut RequestState) -> Self::Output {
        if ctx.request.method() == Method::POST {
            ResolveGuard::Value(Post)            
        } else {
            ResolveGuard::None
        }
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
        impl<R, $($x,)* T> System<(R, $($x,)*)> for T
        where
            T: Fn($($x,)*) -> R,
            $($x: Resolve<Output = ResolveGuard<$x>>,)*
            R: MaybeIntoResponse,
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

all! { A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q }
