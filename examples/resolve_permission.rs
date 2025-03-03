use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use foxhole::{
    App, Http1, IntoResponse, Method::Post, Resolve, ResolveGuard, Router, TypeCache, TypeCacheKey,
};

struct User {
    permission: u8,
}

struct UserBase {
    map: HashMap<String, User>,
}

impl Default for UserBase {
    fn default() -> Self {
        let mut map = HashMap::new();

        map.insert(
            String::from("Admin"),
            User {
                permission: u8::MAX,
            },
        );

        Self { map }
    }
}

impl TypeCacheKey for UserBase {
    type Value = Arc<UserBase>;
}

struct HasPermission<T>(PhantomData<T>);

impl<T> Resolve for HasPermission<T>
where
    T: Permission,
{
    type Output<'a> = Self;

    fn resolve<'a>(
        ctx: &'a foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> ResolveGuard<Self::Output<'a>> {
        let user_base = ctx.global_cache.get::<UserBase>().unwrap();

        let Some(user_name) = ctx.request.headers().get("user") else {
            return ResolveGuard::Respond(401u16.response());
        };

        let Some(user) = user_base.map.get(user_name.to_str().unwrap()) else {
            return ResolveGuard::Respond(401u16.response());
        };

        if !T::has_permission(user.permission) {
            return ResolveGuard::Respond(401u16.response());
        }

        ResolveGuard::Value(HasPermission(PhantomData))
    }
}

trait Permission {
    fn has_permission(permissions: u8) -> bool;
}

struct Edit;

impl Permission for Edit {
    fn has_permission(permissions: u8) -> bool {
        permissions & 0b00000001 != 0
    }
}

fn edit_something(_: HasPermission<Edit>) -> u16 {
    // We dont have to check permissions here as it was done for us already
    200
}

fn main() {
    let mut cache = TypeCache::new();
    cache.insert::<UserBase>(Arc::new(UserBase::default()));

    let router = Router::new().add_route("/", Post(edit_something));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router)
        .cache(cache)
        .run::<Http1>("127.0.0.1:8080");
}
