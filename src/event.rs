use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, trace};

use crate::{app::App, email::get_emails};

#[derive(Debug)]
pub enum EventType {
    StartLoading,
    FinishLoading,
    LoadEmails,
    Archive,
    MoveToSpam,
    ToggleSpam,
    Down,
    Up,
    PageDown,
    PageUp,
    Home,
    End,
    FocusNext,
    // SetFocus(AppFocus),
    SetMaxWidth(usize),
    // SetEmailOffset(usize),
    SetEmailPageSize(usize),
    OpenEmail,
}

pub struct EventHandler {
    app: Arc<RwLock<App>>,
}

impl EventHandler {
    pub fn new(app: Arc<RwLock<App>>) -> Self {
        Self { app }
    }

    pub async fn execute(&self, event: EventType) {
        trace!("Executing event: {:?}", event);

        // async events
        match event {
            EventType::LoadEmails => {
                let app = self.app.clone();
                tokio::spawn(async move {
                    info!("Started loading emails...");
                    let emails = get_emails().unwrap();
                    info!("Done loading emails...");
                    info!("Got {} emails", emails.len());

                    let mut app = app.write().await;
                    app.emails = emails;
                    app.last_update = Some(std::time::Instant::now());
                })
                .await
                .unwrap();

                return;
            }
            EventType::OpenEmail => {
                let app = self.app.clone();
                tokio::spawn(async move {
                    let email = {
                        let app = app.read().await;
                        let mut email = app.selected_email();
                        // FIXME handle error
                        email.load().unwrap();
                        email
                    };

                    let mut app = app.write().await;
                    app.show_email(email);
                    app.last_update = Some(std::time::Instant::now());
                })
                .await
                .unwrap();

                return;
            }
            _ => {}
        }

        // sync events
        let mut app = self.app.write().await;
        match event {
            EventType::StartLoading => app.loading = true,
            EventType::FinishLoading => app.loading = false,
            EventType::Archive => app.archive(),
            EventType::MoveToSpam => app.move_to_spam(),
            EventType::ToggleSpam => app.toggle_spam(),
            EventType::Up => app.up(),
            EventType::Down => app.down(),
            EventType::PageUp => app.page_up(),
            EventType::PageDown => app.page_down(),
            EventType::Home => app.home(),
            EventType::End => app.end(),
            // EventType::SetFocus(focus) => app.focus = focus,
            EventType::SetMaxWidth(width) => app.max_width = width,
            // EventType::SetEmailOffset(offset) => app.email_offset = offset,
            EventType::SetEmailPageSize(size) => app.email_page_size = size,
            EventType::FocusNext => app.focus_next(),
            EventType::OpenEmail => {}
            EventType::LoadEmails => {}
        }
        app.last_update = Some(std::time::Instant::now());
    }
}
