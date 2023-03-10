use dotenvy::dotenv;
use native_tls::TlsConnector;
use tracing::trace;

mod auth;

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

fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let access_token = auth::auth().unwrap();

    let domain = "outlook.office365.com";
    let tls = TlsConnector::builder().build().unwrap();
    let client = imap::connect((domain, 993), domain, &tls).unwrap();

    let auth = OAuth2 {
        user: String::from("felipe.coury@methodiq.com"),
        access_token,
    };
    match client.authenticate("XOAUTH2", &auth) {
        Ok(mut session) => {
            trace!("Authenticated successfully!");
            session.select("INBOX").unwrap();

            let messages = session.fetch("1", "RFC822").unwrap();

            for message in messages.iter() {
                if let Some(body) = message.body() {
                    use mailparse::*;
                    let email = parse_mail(body).unwrap();
                    println!("{}", email.headers.get_first_value("Subject").unwrap());
                    println!("{}", email.get_body().unwrap());
                    println!("------------------");
                } else {
                    println!("Message didn't have a body!");
                }
            }

            session.logout().unwrap();
        }
        Err(e) => println!("Authentication failed: {e:?}"),
    }
}
