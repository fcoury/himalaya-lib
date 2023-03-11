use std::cmp::min;

use crossterm::event::{KeyCode, KeyEvent};

use crate::email::Email;

#[derive(Debug, Default, Clone)]
pub struct App {
    pub emails: Vec<Email>,
    pub selected_email: usize,
    pub open_email: Option<Email>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
}

impl App {
    pub fn new() -> Self {
        App::default()
    }

    pub fn on_down(&mut self) {
        if self.selected_email < self.emails.len() - 1 {
            self.selected_email += 1;
        }
    }

    pub fn on_up(&mut self) {
        if self.selected_email > 0 {
            self.selected_email -= 1;
        }
    }

    pub fn page_down(&mut self) {
        if self.selected_email < self.emails.len() - 1 {
            self.selected_email = min(self.selected_email + 10, self.emails.len() - 1);
        }
    }

    pub fn page_up(&mut self) {
        if self.selected_email > 0 {
            let selected_email = self.selected_email as i32 - 10_i32;
            if selected_email < 0 {
                self.selected_email = 0;
            } else {
                self.selected_email = selected_email as usize;
            }
        }
    }

    pub fn home(&mut self) {
        self.selected_email = 0;
    }

    pub fn end(&mut self) {
        self.selected_email = self.emails.len() - 1;
    }

    pub fn on_tick(&mut self, events: &Vec<AppEvent>) {
        for event in events {
            match event {
                AppEvent::Key(key_event) => match key_event.code {
                    KeyCode::Up => self.on_up(),
                    KeyCode::Down => self.on_down(),
                    KeyCode::PageUp => self.page_up(),
                    KeyCode::PageDown => self.page_down(),
                    KeyCode::Home => self.home(),
                    KeyCode::End => self.end(),
                    KeyCode::Enter => {
                        if let Some(email) = self.emails.get(self.selected_email) {
                            let mut email = email.clone();
                            email.load().unwrap();

                            self.open_email = Some(email);
                        }
                    }
                    _ => (),
                },
            }
        }
    }
}
