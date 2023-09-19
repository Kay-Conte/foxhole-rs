#[macro_export]
macro_rules! sys {
    () => { vec![] };

    ($($x:ident),*) => {
        vec![$(turnip_http::systems::DynSystem::new($x),)*]
    };
}
