use std::{cmp::min, fs};

use crate::email::Email;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum AppFocus {
    #[default]
    EmailList,
    EmailBody,
}

#[derive(Debug, Default, Clone)]
pub struct App {
    pub loading: bool,
    pub focus: AppFocus,
    pub emails: Vec<Email>,
    pub selected_email: usize,
    pub open_email: Option<Email>,
    pub max_width: usize,
    pub email_offset: usize,
    pub email_page_size: usize,
    pub last_update: Option<std::time::Instant>,
}

impl App {
    pub fn new() -> Self {
        App::default()
    }

    pub fn dump_emails(&self) {
        fs::write(
            "data/processed.json",
            serde_json::to_string_pretty(&self.emails).unwrap(),
        )
        .unwrap();
    }

    pub fn toggle_spam(&mut self) {
        let email = self.emails.get_mut(self.selected_email).unwrap();
        email.toggle_spam();
    }

    pub fn move_to_spam(&mut self) {
        let email = self.emails.get(self.selected_email).unwrap();
        email.move_to_spam().unwrap();

        let emails = self.emails.clone();
        let (_, kept) = emails
            .into_iter()
            .partition(|e| e.internal_id == email.internal_id);
        self.emails = kept;
    }

    pub fn archive(&mut self) {
        let email = self.emails.get(self.selected_email).unwrap();
        email.archive().unwrap();

        let emails = self.emails.clone();
        let (_, kept) = emails
            .into_iter()
            .partition(|e| e.internal_id == email.internal_id);
        self.emails = kept;
    }

    pub fn show_email(&mut self, email: Email) {
        self.open_email = Some(email);
        self.email_offset = 0;
        self.focus = AppFocus::EmailBody;
    }

    pub fn down(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.next_email(),
            AppFocus::EmailBody => self.next_line(),
        }
    }

    pub fn up(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.prev_email(),
            AppFocus::EmailBody => self.prev_line(),
        }
    }

    pub fn page_down(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.next_email_page(),
            AppFocus::EmailBody => self.next_body_page(),
        }
    }

    pub fn page_up(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.prev_email_page(),
            AppFocus::EmailBody => self.prev_body_page(),
        }
    }

    pub fn home(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.first_email(),
            AppFocus::EmailBody => self.first_line(),
        }
    }

    pub fn end(&mut self) {
        match self.focus {
            AppFocus::EmailList => self.last_email(),
            AppFocus::EmailBody => self.last_line(),
        }
    }

    pub fn next_line(&mut self) {
        self.email_offset += 1;
    }

    pub fn prev_line(&mut self) {
        if self.email_offset > 0 {
            self.email_offset -= 1;
        }
    }

    pub fn next_body_page(&mut self) {
        self.email_offset += self.email_page_size;
        if self.email_offset > self.email_line_count().unwrap() {
            self.email_offset = self.email_line_count().unwrap();
        }
    }

    pub fn prev_body_page(&mut self) {
        let new_offset: i32 = self.email_offset as i32 - self.email_page_size as i32;
        if new_offset < 0 {
            self.email_offset = 0;
        } else {
            self.email_offset = new_offset as usize;
        }
    }

    pub fn first_line(&mut self) {
        self.email_offset = 0;
    }

    pub fn last_line(&mut self) {
        let new_line: i32 = self.email_line_count().unwrap() as i32 - self.email_page_size as i32;
        if new_line < 0 {
            self.email_offset = 0;
        } else {
            self.email_offset = new_line as usize;
        }
    }

    pub fn next_email(&mut self) {
        if self.selected_email < self.emails.len() - 1 {
            self.selected_email += 1;
        }
    }

    pub fn prev_email(&mut self) {
        if self.selected_email > 0 {
            self.selected_email -= 1;
        }
    }

    pub fn next_email_page(&mut self) {
        if self.selected_email < self.emails.len() - 1 {
            self.selected_email = min(self.selected_email + 10, self.emails.len() - 1);
        }
    }

    pub fn prev_email_page(&mut self) {
        if self.selected_email > 0 {
            let selected_email = self.selected_email as i32 - 10_i32;
            if selected_email < 0 {
                self.selected_email = 0;
            } else {
                self.selected_email = selected_email as usize;
            }
        }
    }

    pub fn first_email(&mut self) {
        self.selected_email = 0;
    }

    pub fn last_email(&mut self) {
        self.selected_email = self.emails.len() - 1;
    }

    pub fn focus_next(&mut self) {
        if self.open_email.is_none() {
            return;
        }

        match self.focus {
            AppFocus::EmailList => self.focus = AppFocus::EmailBody,
            AppFocus::EmailBody => self.focus = AppFocus::EmailList,
        }
    }

    pub fn selected_email(&self) -> Email {
        self.emails.get(self.selected_email).unwrap().clone()
    }

    pub fn email_body(&self) -> Option<String> {
        let Some(email) = &self.open_email else {
            return None;
        };

        let Some(body) = &email.body else {
            return None;
        };

        Some(html2text::from_read(body.as_bytes(), self.max_width))
    }

    pub fn email_line_count(&self) -> Option<usize> {
        let Some(body) = self.email_body() else {
            return None;
        };
        Some(body.lines().count())
    }

    pub fn email_viewport(&self) -> Option<String> {
        let Some(body) = self.email_body() else {
            return None;
        };

        let lines: Vec<&str> = body.lines().collect();
        let start = self.email_offset;
        let end = min(
            start + self.email_page_size + 1,
            self.email_line_count().unwrap(),
        );
        Some(lines[start..end].join("\n"))
    }
}
