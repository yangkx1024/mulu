use std::time::Duration;

use futures::StreamExt;
use gpui::*;
use tokio::sync::mpsc;

use super::client::list_devices;
use super::runtime::tokio_rt;
use crate::mtp_browser::MtpBrowser;

const DEBOUNCE: Duration = Duration::from_millis(300);

pub fn watch_hotplug(cx: &mut Context<MtpBrowser>) {
    // Capacity 1 with try_send drops bursts — we coalesce in the consumer instead.
    let (tx, mut rx) = mpsc::channel::<()>(1);

    let producer = tokio_rt().spawn(async move {
        let Ok(watch) = nusb::watch_devices() else {
            return;
        };
        futures::pin_mut!(watch);
        while watch.next().await.is_some() {
            let _ = tx.try_send(());
        }
    });

    cx.spawn(async move |this, cx| {
        loop {
            if rx.recv().await.is_none() {
                break;
            }
            // Trailing-edge debounce: wait, then drain bursts that piled up.
            cx.background_executor().timer(DEBOUNCE).await;
            while rx.try_recv().is_ok() {}

            let Ok(result) = tokio_rt().spawn(async { list_devices() }).await else {
                continue;
            };

            if this
                .update(cx, |this, cx| this.apply_device_list(result, cx))
                .is_err()
            {
                break;
            }
        }
        producer.abort();
    })
    .detach();
}
