#[cfg(feature = "imap-backend")]
use himalaya_lib::backend::{Backend, ImapBackend};

#[cfg(feature = "imap-backend")]
#[test]
fn test_imap_backend() {
    use std::collections::HashMap;

    use himalaya_lib::config::{
        AccountConfig, BackendConfig, Config, EmailSender, ImapConfig, SmtpConfig,
    };

    let imap_config = ImapConfig {
        host: "localhost".into(),
        port: 3993,
        starttls: Some(false),
        insecure: Some(true),
        login: "inbox@localhost".into(),
        passwd_cmd: "echo 'password'".into(),
        ..ImapConfig::default()
    };

    let config = Config {
        accounts: HashMap::from_iter([(
            String::new(),
            AccountConfig {
                default: Some(true),
                email_sender: EmailSender::Internal(SmtpConfig {
                    host: "localhost".into(),
                    port: 3465,
                    starttls: Some(false),
                    insecure: Some(true),
                    login: "inbox@localhost".into(),
                    passwd_cmd: "echo 'password'".into(),
                }),
                backend: BackendConfig::Imap(imap_config.clone()),
                ..AccountConfig::default()
            },
        )]),
        ..Config::default()
    };

    let mut imap = ImapBackend::new(&config, &imap_config);
    imap.connect().unwrap();

    // set up mailboxes
    if let Err(_) = imap.add_mbox("Mailbox1") {};
    if let Err(_) = imap.add_mbox("Mailbox2") {};
    imap.del_msg("Mailbox1", "1:*").unwrap();
    imap.del_msg("Mailbox2", "1:*").unwrap();

    // check that a message can be added
    let msg = include_bytes!("./emails/alice-to-patrick.eml");
    let id = imap.add_msg("Mailbox1", msg, "seen").unwrap().to_string();

    // check that the added message exists
    let msg = imap.get_msg("Mailbox1", &id).unwrap();
    assert_eq!("alice@localhost", msg.from.clone().unwrap().to_string());
    assert_eq!("patrick@localhost", msg.to.clone().unwrap().to_string());
    assert_eq!("Ceci est un message.", msg.fold_text_plain_parts());

    // check that the envelope of the added message exists
    let envelopes = imap.get_envelopes("Mailbox1", 10, 0).unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.sender);
    assert_eq!("Plain message", envelope.subject);

    // check that the message can be copied
    imap.copy_msg("Mailbox1", "Mailbox2", &envelope.id.to_string())
        .unwrap();
    let envelopes = imap.get_envelopes("Mailbox1", 10, 0).unwrap();
    assert_eq!(1, envelopes.len());
    let envelopes = imap.get_envelopes("Mailbox2", 10, 0).unwrap();
    assert_eq!(1, envelopes.len());

    // check that the message can be moved
    imap.move_msg("Mailbox1", "Mailbox2", &envelope.id.to_string())
        .unwrap();
    let envelopes = imap.get_envelopes("Mailbox1", 10, 0).unwrap();
    assert_eq!(0, envelopes.len());
    let envelopes = imap.get_envelopes("Mailbox2", 10, 0).unwrap();
    assert_eq!(2, envelopes.len());
    let id = envelopes.first().unwrap().id.to_string();

    // check that the message can be deleted
    imap.del_msg("Mailbox2", &id).unwrap();
    assert!(imap.get_msg("Mailbox2", &id).is_err());

    // check that disconnection works
    imap.disconnect().unwrap();
}
