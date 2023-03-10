use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, trace};
use url::Url;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Missing environment variable: {0}")]
    MissingEnv(#[from] env::VarError),

    #[error("Error reading app config: {0}")]
    Config(#[from] confy::ConfyError),

    #[error("No token present")]
    NoTokenPresent,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    token: Option<String>,
}

fn get_token() -> Result<Option<String>, confy::ConfyError> {
    let cfg: Config = confy::load("posters", None)?;
    Ok(cfg.token)
}

fn save_token(token: &str) -> Result<(), confy::ConfyError> {
    let token = token.to_string();
    confy::store("posters", None, Config { token: Some(token) })
}

pub fn auth() -> Result<String, AuthError> {
    if let Some(token) = get_token()? {
        debug!("token found, returning...");
        return Ok(token);
    }

    let graph_client_id = ClientId::new(env::var("CLIENT_ID")?);
    let graph_client_secret = ClientSecret::new(env::var("CLIENT_SECRET")?);
    let auth_url =
        AuthUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string())?;
    let token_url =
        TokenUrl::new("https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string())?;

    // Set up the config for the Microsoft Graph OAuth2 process.
    let client = BasicClient::new(
        graph_client_id,
        Some(graph_client_secret),
        auth_url,
        Some(token_url),
    )
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(
        "http://localhost:3003/redirect".to_string(),
    )?);

    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "openid profile email offline_access https://outlook.office.com/IMAP.AccessAsUser.All"
                .to_string(),
        ))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    trace!("opening URL:\n{authorize_url}\n");
    opener::open(authorize_url.as_str()).unwrap();

    // A very naive implementation of the redirect server.
    let listener = TcpListener::bind("127.0.0.1:3003").unwrap();
    #[allow(clippy::manual_flatten)]
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let code;
            let state;
            {
                let mut reader = BufReader::new(&stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();

                trace!("MS Graph returned the following request line: {request_line}",);

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let (ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = AuthorizationCode::new(value.into_owned());

                let state_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let (ref key, _) = pair;
                        key == "state"
                    })
                    .unwrap();

                let (_, value) = state_pair;
                state = CsrfToken::new(value.into_owned());
            }

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            trace!("MS Graph returned the following code:\n{}\n", code.secret());
            trace!(
                "MS Graph returned the following state:\n{} (expected `{}`)\n",
                state.secret(),
                csrf_state.secret()
            );

            // Exchange the code with a token.
            let token = client
                .exchange_code(code.clone())
                // Send the PKCE code verifier in the token request
                .set_pkce_verifier(pkce_code_verifier)
                .request(http_client)
                .unwrap();

            println!("token: {token:?}");

            let token = token.access_token().secret().to_string();
            save_token(&token)?;

            let client = reqwest::blocking::Client::new();
            let body = client
                .get("https://graph.microsoft.com/profile")
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .unwrap();
            println!("Body = {body:?}");
            let text = body.text().unwrap();
            println!("Text = {text:?}");

            return Ok(token);
        }
    }

    Err(AuthError::NoTokenPresent)
}
