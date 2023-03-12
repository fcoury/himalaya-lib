use std::{borrow::Cow, fs};

use chrono::{DateTime, Local};
use himalaya_lib::{
    AccountConfig, Backend, Envelope, ImapBackend, ImapBackendBuilder, ImapConfig,
    ShowTextPartsStrategy, Tpl,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::auth;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Email {
    pub folder: String,
    pub internal_id: String,
    pub date: DateTime<Local>,
    pub from_name: Option<String>,
    pub from_addr: String,
    pub subject: String,
    pub body: Option<String>,
    pub selected: bool,
}

impl Email {
    pub fn load(&mut self) -> anyhow::Result<()> {
        let config = AccountConfig {
            email: "felipe.coury@methodiq.com".into(),
            ..AccountConfig::default()
        };

        let backend = backend()?;
        let emails = backend.get_emails(&self.folder, vec![&self.internal_id])?;
        let mut glue = "";
        let mut bodies = String::default();

        for email in emails.to_vec() {
            bodies.push_str(glue);

            let tpl = email
                .to_read_tpl_builder(&config)?
                .show_headers(config.email_reading_headers())
                .show_text_parts_only(false)
                .sanitize_text_parts(false)
                .use_show_text_parts_strategy(ShowTextPartsStrategy::HtmlOtherwisePlain)
                .build();

            bodies.push_str(&<Tpl as Into<String>>::into(tpl));
            glue = "\n\n";
        }

        info!("Body:\n{}", bodies);
        fs::write(format!("data/{}.html", self.internal_id), bodies.clone())?;
        self.body = Some(bodies);

        Ok(())
    }

    pub fn move_to(&self, folder: &str) -> anyhow::Result<()> {
        let backend = backend().unwrap();
        Ok(backend.move_emails(&self.folder, folder, vec![&self.internal_id])?)
    }

    pub fn toggle_select(&mut self) {
        self.selected = !self.selected;
    }

    fn from(folder: &str, envelope: Envelope) -> Self {
        Email {
            folder: folder.to_string(),
            internal_id: envelope.internal_id,
            date: envelope.date,
            from_name: envelope.from.name.clone(),
            from_addr: envelope.from.addr.clone(),
            subject: envelope.subject,
            ..Default::default()
        }
    }
}

pub fn backend<'a>() -> anyhow::Result<ImapBackend<'a>> {
    let access_token = auth::auth().unwrap();

    let account = AccountConfig {
        name: "Felipe Coury".to_string(),
        email: "felipe.coury@methodiq.com".to_string(),
        ..Default::default()
    };

    let config = ImapConfig {
        host: "outlook.office365.com".to_string(),
        port: 993,
        ssl: Some(true),
        login: "felipe.coury@methodiq.com".to_string(),
        access_token: Some(access_token),
        ..Default::default()
    };

    Ok(ImapBackendBuilder::new().build(Cow::Owned(account), Cow::Owned(config))?)
}

pub fn get_emails(force: bool) -> anyhow::Result<Vec<Email>> {
    if !force && fs::metadata("data/emails.json").is_ok() {
        let json = fs::read_to_string("data/emails.json")?;
        let emails: Vec<Email> = serde_json::from_str(&json)?;
        return Ok(emails);
    }

    let backend = backend()?;
    let envelopes = backend.list_envelopes("INBOX", 0, 10)?;
    let emails = envelopes
        .iter()
        .map(|e| Email::from("INBOX", e.clone()))
        .collect();

    let json = serde_json::to_string_pretty(&emails);
    fs::write("data/emails.json", json.unwrap())?;

    Ok(emails)
}
