use chrono::{DateTime, Local};
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
    pub async fn load(&mut self) -> anyhow::Result<()> {
        let token = auth::auth()?;
        let access_token = token.access_code;

        let body: String = reqwest::Client::new()
            .get(format!(
                "http://localhost:3001/api/emails/{}",
                &self.internal_id
            ))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?
            .json()
            .await?;

        self.body = Some(body);

        Ok(())
    }

    pub async fn move_to(&self, folder: &str) -> anyhow::Result<()> {
        let token = auth::auth()?;
        let access_token = token.access_code;

        reqwest::Client::new()
            .put(format!(
                "http://localhost:3001/api/emails/{id}/move/{folder}",
                id = &self.internal_id
            ))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;
        Ok(())
    }

    pub fn toggle_select(&mut self) {
        self.selected = !self.selected;
    }
}

pub async fn get_emails() -> anyhow::Result<Vec<Email>> {
    let token = auth::auth().unwrap();
    let access_token = token.access_code;
    let emails = reqwest::Client::new()
        .get("http://localhost:3001/api/emails")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?
        .json()
        .await?;

    info!("Emails: {emails:#?}");

    Ok(emails)
}
