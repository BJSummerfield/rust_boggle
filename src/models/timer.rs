use super::boggle::{Boggle, BoggleStateEnum};
use crate::render::Render;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, Notify};

#[derive(Debug)]
pub struct Timer {
    duration: u32,
    cancel_token: Arc<Notify>,
    tx: broadcast::Sender<String>,
    boggle_channel_tx: broadcast::Sender<BoggleStateEnum>,
}

impl Timer {
    pub fn new(
        tx: broadcast::Sender<String>,
        boggle_channel_tx: broadcast::Sender<BoggleStateEnum>,
    ) -> Arc<Mutex<Self>> {
        let cancel_token = Arc::new(Notify::new());
        Arc::new(Mutex::new(Self {
            duration: Boggle::GAME_DURATION,
            cancel_token,
            tx,
            boggle_channel_tx,
        }))
    }

    pub async fn start(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.cancel_token);
        let boggle_channel_tx = self.boggle_channel_tx.clone();
        let duration = self.duration;

        tokio::spawn(async move {
            let mut remaining = duration;

            while remaining > 0 {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        remaining -= 1;

                        let formated_time = Timer::format_time(remaining);
                        let timer_html = Render::timer(&formated_time);

                        if let Err(e) = timer_tx.send(timer_html) {
                            eprintln!("Failed to send timer update: {}", e);
                        }

                        if remaining == 0 {
                            if let Err(e) = boggle_channel_tx.send(BoggleStateEnum::GameOver) {
                                eprintln!("Failed to send game over message: {}", e);
                            }
                        }
                    },
                    _ = cancel_token.notified() => {
                        break;
                    }
                }
            }
        });
    }

    pub fn format_time(duration: u32) -> String {
        let minutes = duration / 60;
        let seconds = duration % 60;
        format!("{}:{:02}", minutes, seconds)
    }

    pub fn cancel(&self) {
        self.cancel_token.notify_one();
    }
}
