use http::{Method, Response, Version};

use crate::thread_pool::Context;

pub type RawResponse = Response<Vec<u8>>;

pub trait Resolve: Sized {
    type Output;

    fn resolve(ctx: &mut Context) -> Self::Output;
}

pub trait MaybeIntoResponse {
    fn response(self) -> Option<RawResponse>;
}

impl MaybeIntoResponse for () {
    fn response(self) -> Option<RawResponse> {
        None
    }
}

impl<T> MaybeIntoResponse for Option<T> where T: MaybeIntoResponse {
    fn response(self) -> Option<RawResponse> {
        self.map(|f| f.response()).flatten()
    }
}

impl<T, E> MaybeIntoResponse for Result<T, E> where T: MaybeIntoResponse, E: MaybeIntoResponse {
    fn response(self) -> Option<RawResponse> {
        match self {
            Ok(v) => v.response(),
            Err(e) => e.response(),
        }
    }
}

impl MaybeIntoResponse for u16 {
    fn response(self) -> Option<RawResponse> {
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

impl MaybeIntoResponse for Response<String> {
    fn response(self) -> Option<RawResponse> {
        Some(self.map(|f| f.into_bytes()))
    }
}

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

pub struct Get;

impl Resolve for Get {
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut Context) -> Self::Output {
        if ctx.request.method() == Method::GET {
            ResolveGuard::Value(Get)
        } else {
            ResolveGuard::None
        }
    }
}

pub struct Post;

impl Resolve for Post {
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut Context) -> Self::Output {
        if ctx.request.method() == Method::POST {
            ResolveGuard::Value(Post)            
        } else {
            ResolveGuard::None
        }
    }
}

pub trait System<T> {
    fn run(self, ctx: &mut Context) -> Option<RawResponse>;
}

pub struct DynSystem {
    inner: Box<dyn Fn(&mut Context) -> Option<RawResponse> + 'static + Send + Sync>,
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

    pub fn call(&self, ctx: &mut Context) -> Option<RawResponse> {
        (self.inner)(ctx)
    }
}

impl<R, T> System<R> for T
where
    T: Fn() -> R,
    R: MaybeIntoResponse,
{
    fn run(self, _ctx: &mut Context) -> Option<RawResponse> {
        let r = self();

        r.response()
    }
}

impl<R, Arg1, T> System<(R, Arg1)> for T
where
    T: Fn(Arg1) -> R,
    Arg1: Resolve<Output = ResolveGuard<Arg1>>,
    R: MaybeIntoResponse,
{
    fn run(self, ctx: &mut Context) -> Option<RawResponse> {
        let arg1 = match Arg1::resolve(ctx) {
            ResolveGuard::Value(v) => v,
            ResolveGuard::None => return None,
            ResolveGuard::Respond(r) => return Some(r),
        };

        let r = self(arg1);

        r.response()
    }
}

impl<R, Arg1, Arg2, T> System<(R, Arg1, Arg2)> for T
where
    T: Fn(Arg1, Arg2) -> R,
    Arg1: Resolve<Output = ResolveGuard<Arg1>>,
    Arg2: Resolve<Output = ResolveGuard<Arg2>>,
    R: MaybeIntoResponse,
{
    fn run(self, ctx: &mut Context) -> Option<RawResponse> {
        let arg1 = match Arg1::resolve(ctx) {
            ResolveGuard::Value(v) => v,
            ResolveGuard::None => return None,
            ResolveGuard::Respond(r) => return Some(r),
        };

        let arg2 = match Arg2::resolve(ctx) {
            ResolveGuard::Value(v) => v,
            ResolveGuard::None => return None,
            ResolveGuard::Respond(r) => return Some(r),
        };

        let r = self(arg1, arg2);

        r.response()
    }
}
