use std::future::Future;
use std::sync::OnceLock;

use gpui::*;
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn tokio_rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

pub fn spawn_mtp<V: 'static, T, Fut, Done>(cx: &mut Context<V>, fut: Fut, done: Done)
where
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
    Done: FnOnce(&mut V, T, &mut Context<V>) + 'static,
{
    let handle = tokio_rt().spawn(fut);
    cx.spawn(async move |this, cx| {
        if let Ok(v) = handle.await {
            this.update(cx, |this, cx| done(this, v, cx)).ok();
        }
    })
    .detach();
}
