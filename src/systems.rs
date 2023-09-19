use http::{Method, Response};

use crate::framework::Context;

pub type RawResponse = Response<Vec<u8>>;

pub trait Resolve: Sized {
    type Output;

    fn resolve(ctx: &mut Context) -> Self::Output;
}

trait MaybeIntoResponse {
    fn response(self) -> Option<RawResponse>;
}

impl MaybeIntoResponse for () {
    fn response(self) -> Option<RawResponse> {
        None
    }
}

impl MaybeIntoResponse for u16 {
    fn response(self) -> Option<RawResponse> {
        Some(
            Response::builder()
                .status(self)
                .header("Test", "Header")
                .body(Vec::new())
                .expect("Failed to build request"),
        )
    }
}

pub struct Query<T>
where
    T: Resolve,
{
    pub inner: Option<T>,
}

pub enum ResolveGuard<T> {
    /// Succesful value, run the system
    Value(T),
    // Todo make this an actual response struct
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

impl<T> Resolve for Query<T>
where
    T: Resolve<Output = Option<T>>,
{
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut Context) -> ResolveGuard<Self> {
        ResolveGuard::Value(Query {
            inner: T::resolve(ctx),
        })
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

impl<Arg1, R> From<fn(Arg1) -> R> for DynSystem
where
    Arg1: Resolve<Output = ResolveGuard<Arg1>> + 'static,
    R: MaybeIntoResponse + 'static,
{
    fn from(value: fn(Arg1) -> R) -> Self {
        DynSystem::new(value)
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
