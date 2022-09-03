use std::thread;

use actix_rt::System;
use tokio::sync::mpsc;

use crate::{peer::RtcConfig, GgrsSocket, WebRTCSocket};

pub struct BlockingWebRTCSocket {
    ggrs_rx: mpsc::Receiver<GgrsSocket>,
    tx: mpsc::UnboundedSender<()>,
}

impl BlockingWebRTCSocket {
    pub fn connect(rtc_config: RtcConfig) -> anyhow::Result<Self> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (ggrs_tx, ggrs_rx) = mpsc::channel(1);
        thread::spawn(move || {
            let system = System::new();
            system.block_on(async move {
                let mut s = WebRTCSocket::new(rtc_config).await.unwrap();
                let _ = rx.recv().await;
                let ggrs_socket = GgrsSocket::new(&mut s);
                let _ = ggrs_tx.send(ggrs_socket).await;
                s.run().await.unwrap();
            });
        });

        Ok(Self { tx, ggrs_rx })
    }

    pub fn ggrs_socket(&mut self) -> GgrsSocket {
        let _ = self.tx.send(());
        let ggrs_socket = self.ggrs_rx.blocking_recv().unwrap();
        ggrs_socket
    }
}
