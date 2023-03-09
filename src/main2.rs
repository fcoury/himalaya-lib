use native_tls::TlsConnector;

fn main() {
    let domain = "outlook.office365.com";
    let tls = TlsConnector::builder().build().unwrap();
    let client = imap::connect((domain, 993), domain, &tls).unwrap();
    let mut imap_session = client
        .login("felipe.coury@methodiq.com", "NXgFUoVDJMyTjUPR3qCU")
        .unwrap();
    imap_session.select("INBOX").unwrap();

    let messages = imap_session.fetch("1,2,3,4,5", "RFC822").unwrap();

    for message in messages.iter() {
        if let Some(body) = message.body() {
            println!("{}", std::str::from_utf8(body).unwrap());
        } else {
            println!("Message didn't have a body!");
        }
    }

    imap_session.logout().unwrap();
}
