use std::cmp::min;

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
    pub email_offset: usize,
    pub email_page_size: usize,
}

impl App {
    pub fn new() -> Self {
        App::default()
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
        self.email_offset += 10;
    }

    pub fn prev_body_page(&mut self) {
        let new_offset: i32 = self.email_offset as i32 - 10_i32;
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
        let Some(open_email) = self.open_email.clone() else {
            return;
        };
        let Some(body) = open_email.body else {
            return;
        };
        self.email_offset = body.lines().into_iter().count();
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

    pub fn selected_email(&self) -> Email {
        self.emails.get(self.selected_email).unwrap().clone()
    }
}
