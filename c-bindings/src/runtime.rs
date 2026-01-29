use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

static CSPOT_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .enable_all()
        .thread_name("cspot-runtime")
        .build()
        .expect("cspot: failed to build tokio runtime")
});

pub(crate) fn runtime() -> &'static Runtime {
    &CSPOT_RUNTIME
}
