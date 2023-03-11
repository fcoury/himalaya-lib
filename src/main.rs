use std::time::Duration;

use dotenvy::dotenv;
use tokio::task;
use tracing::error;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use ui::run;

mod app;
mod auth;
mod email;
mod ui;

struct OAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for OAuth2 {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "posters.log");
    tracing_subscriber::fmt().with_writer(file_appender).init();

    task::block_in_place(move || {
        let res = auth::auth();
        if let Err(err) = res {
            error!("Auth error: {err}");
        }
    });

    let tick_rate = Duration::from_millis(80);
    run(tick_rate).await?;

    Ok(())
}
