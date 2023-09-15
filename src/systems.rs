use crate::framework::Context;

pub trait Resolve: Sized {
    type Output;

    fn resolve(ctx: &mut Context) -> Self::Output;
}

trait IntoResponse {
    fn response(self) -> Option<()>;
}

impl IntoResponse for () {
    fn response(self) -> Option<()> {
        println!("Response not correctly implemented for tuple");
        Some(())
    }
}

pub struct Query<T> where T: Resolve {
    inner: Option<T>
}

pub enum ResolveGuard<T> {
    /// Succesful value, run the system
    Value(T),
    // Todo make this an actual response struct
    /// Don't run this system or any others, respond early with this response
    Respond(()),
    /// Don't run this system, but continue routing to other systems
    None,
}

impl<T> ResolveGuard<T> {
    fn map<N>(self, f: fn(T) -> N) -> ResolveGuard<N> {
        match self {
            ResolveGuard::Value(v) => ResolveGuard::Value(f(v)),
            ResolveGuard::Respond(v) => ResolveGuard::Respond(v),
            ResolveGuard::None => ResolveGuard::None,
        }
    }
}

impl<T> Resolve for Query<T> where T: Resolve<Output = Option<T>> {
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
        // Todo actually check if request is a get request
        ResolveGuard::Value(Get)
    }
}

pub trait System<T> {
    fn run(self, ctx: &mut Context);
}

pub struct DynSystem {
    inner: Box<dyn Fn(&mut Context) + 'static + Send + Sync>,
}

impl DynSystem {
    pub fn new<T, A>(system: T) -> Self where T: System<A> + 'static + Copy + Send + Sync {
        DynSystem {
            inner: Box::new(move |ctx| system.run(ctx)),
        }
    }

    pub fn call(&self, ctx: &mut Context) {
        (self.inner)(ctx)
    }
}

impl<Arg, R> From<fn(Arg) -> R> for DynSystem where Arg: Resolve + 'static, R: IntoResponse + 'static {
    fn from(value: fn(Arg) -> R) -> Self {
        DynSystem::new(value)
    }
}

impl<Arg1, Arg2, R> From<fn(Arg1, Arg2) -> R> for DynSystem where Arg1: Resolve<Output = ResolveGuard<Arg1>> + 'static, Arg2: Resolve<Output = ResolveGuard<Arg2>> + 'static, R: IntoResponse + 'static {
    fn from(value: fn(Arg1, Arg2) -> R) -> Self {
        DynSystem::new(value)
    }
}

impl<R, Arg, T> System<(R, Arg)> for T where T: Fn(Arg) ->  R, Arg: Resolve {
    fn run(self, ctx: &mut Context) {
        todo!()
    }
}

impl<R, T> System<(R, )> for T where T: Fn() -> R {
    fn run(self, ctx: &mut Context) {
        todo!()
    }
}

impl<R, Arg1, Arg2, T> System<(R, Arg1, Arg2)> for T where T: Fn(Arg1, Arg2) ->  R, Arg1: Resolve<Output = ResolveGuard<Arg1>> {
    fn run(self, ctx: &mut Context) {
        todo!()
    }
}
