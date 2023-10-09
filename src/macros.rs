/// This macro builds a vec of `DynSystem` from passed in systems.
#[macro_export]
macro_rules! sys {
    () => { vec![] };

    ($($x:ident),*) => {
        vec![$(foxhole::systems::DynSystem::new($x),)*]
    };
}
