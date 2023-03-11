use std::{borrow::Cow, fs};

use chrono::{DateTime, Local};
use himalaya_lib::{AccountConfig, Backend, Envelope, ImapBackendBuilder, ImapConfig};
use serde::{Deserialize, Serialize};

use crate::auth;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Email {
    pub message_id: String,
    pub date: DateTime<Local>,
    pub from_name: Option<String>,
    pub from_addr: String,
    pub subject: String,
}

pub fn get_emails() -> anyhow::Result<Vec<Email>> {
    if fs::metadata("data/emails.json").is_ok() {
        let json = fs::read_to_string("data/emails.json")?;
        let emails: Vec<Email> = serde_json::from_str(&json)?;
        return Ok(emails);
    }

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

    let backend =
        ImapBackendBuilder::new().build(Cow::Borrowed(&account), Cow::Borrowed(&config))?;

    let envelopes = backend.list_envelopes("INBOX", 0, 10)?;
    let emails = envelopes.iter().map(|e| e.clone().into()).collect();

    let json = serde_json::to_string_pretty(&emails);
    fs::write("data/emails.json", json.unwrap())?;

    Ok(emails)
}

impl From<Envelope> for Email {
    fn from(envelope: Envelope) -> Self {
        Email {
            message_id: envelope.message_id,
            date: envelope.date,
            from_name: envelope.from.name.clone(),
            from_addr: envelope.from.addr.clone(),
            subject: envelope.subject,
        }
    }
}
