use std::marker::PhantomData;

use foxhole::{App, Http1, Method::Post, Resolve, ResolveGuard, Router};

struct HasPermission<T>(PhantomData<T>);

impl<T> Resolve for HasPermission<T>
where
    T: Permission,
{
    type Output<'a> = Self;

    fn resolve<'a>(
        _ctx: &'a foxhole::RequestState,
        _captures: &mut foxhole::Captures,
    ) -> ResolveGuard<Self::Output<'a>> {
        // let user = auth_base.get(something);
        // T::has_permission(user.permissions)

        // Get permission
        //
        // compare the permission to T
        ResolveGuard::Value(HasPermission(PhantomData))
    }
}

trait Permission {
    fn has_permission(permissions: u8) -> bool;
}

struct Edit;

impl Permission for Edit {
    fn has_permission(permissions: u8) -> bool {
        permissions == 1
    }
}

struct Create;

impl Permission for Create {
    fn has_permission(permissions: u8) -> bool {
        permissions == 2
    }
}

fn edit_something(_: HasPermission<Edit>) {
    // We dont have to check permissions here as it was done for us already
}

fn main() {
    let router = Router::new().add_route("/", Post(edit_something));

    println!("Running on '127.0.0.1:8080'");

    App::builder(router).run::<Http1>("127.0.0.1:8080");
}
