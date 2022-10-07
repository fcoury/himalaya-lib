//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use imap;
use rfc2047_decoder;

use crate::{
    backend::imap::{Error, Result},
    domain::flag::imap::flags,
    Envelope,
};

/// Represents the raw envelope returned by the `imap` crate.
pub type RawEnvelope = imap::types::Fetch;

pub fn from_raw(raw: &RawEnvelope) -> Result<Envelope> {
    let envelope = raw
        .envelope()
        .ok_or_else(|| Error::GetEnvelopeError(raw.message))?;

    let id = raw.message.to_string();

    let flags = flags::from_raws(raw.flags());

    let subject = envelope
        .subject
        .as_ref()
        .map(|subj| {
            rfc2047_decoder::decode(subj).map_err(|err| Error::DecodeSubjectError(err, raw.message))
        })
        .unwrap_or_else(|| Ok(String::default()))?;

    let sender = envelope
        .sender
        .as_ref()
        .and_then(|addrs| addrs.get(0))
        .or_else(|| envelope.from.as_ref().and_then(|addrs| addrs.get(0)))
        .ok_or_else(|| Error::GetSenderError(raw.message))?;
    let sender = if let Some(ref name) = sender.name {
        rfc2047_decoder::decode(&name.to_vec())
            .map_err(|err| Error::DecodeSenderNameError(err, raw.message))?
    } else {
        let mbox = sender
            .mailbox
            .as_ref()
            .ok_or_else(|| Error::GetSenderError(raw.message))
            .and_then(|mbox| {
                rfc2047_decoder::decode(&mbox.to_vec())
                    .map_err(|err| Error::DecodeSenderNameError(err, raw.message))
            })?;
        let host = sender
            .host
            .as_ref()
            .ok_or_else(|| Error::GetSenderError(raw.message))
            .and_then(|host| {
                rfc2047_decoder::decode(&host.to_vec())
                    .map_err(|err| Error::DecodeSenderNameError(err, raw.message))
            })?;
        format!("{}@{}", mbox, host)
    };

    let date = raw
        .internal_date()
        .map(|date| date.naive_local().to_string());

    Ok(Envelope {
        id: id.clone(),
        internal_id: id,
        flags,
        subject,
        sender,
        date,
    })
}
