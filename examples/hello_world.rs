use turnip_http::{systems::{Get, Resolve, ResolveGuard, DynSystem}, routing::Node, framework::Framework};

pub struct User {
    id: i32,
    permissions: u8,
}

impl Resolve for User {
    type Output = ResolveGuard<Self>;

    fn resolve(ctx: &mut turnip_http::framework::Request) -> Self::Output {
        ResolveGuard::Value(User {
            id: 123,
            permissions: 255,
        })
    }
}

fn get_user(_get: Get, user: User) {
    
}

fn post_user(_post: Post, _user: User) {
    
}

fn main() { 
    let router = Node::new(vec![]).insert("user", Node::new(vec![(get_user as fn(_, _)).into(), (post_user as fn(_)).into()]));

    let framework = Framework::new();
}
