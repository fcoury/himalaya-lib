use dirs::data_dir;
use log::{debug, warn};
use std::{borrow::Cow, fs, io, result};
use thiserror::Error;

use crate::{envelope, folder, AccountConfig, Backend, MaildirBackendBuilder, MaildirConfig};

use super::maildir;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirError,
    #[error("cannot create sync directories")]
    CreateXdgDataDirsError(#[source] io::Error),

    #[error(transparent)]
    MaildirError(#[from] maildir::Error),
    #[error(transparent)]
    SyncFoldersError(#[from] folder::sync::Error),
    #[error(transparent)]
    SyncEnvelopesError(#[from] envelope::sync::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait ThreadSafeBackend: Backend + Send + Sync {
    fn sync(&self, account: &AccountConfig, dry_run: bool) -> Result<()> {
        debug!("starting synchronization");

        if !account.sync {
            debug!(
                "synchronization not enabled for account {}, exiting",
                account.name
            );
            return Ok(());
        }

        let sync_dir = match account.sync_dir.as_ref().filter(|dir| dir.is_dir()) {
            Some(dir) => dir.clone(),
            None => {
                warn!("sync dir not set or invalid, falling back to $XDG_DATA_HOME/himalaya");
                let sync_dir = data_dir()
                    .map(|dir| dir.join("himalaya"))
                    .ok_or(Error::GetXdgDataDirError)?;
                fs::create_dir_all(&sync_dir).map_err(Error::CreateXdgDataDirsError)?;
                sync_dir
            }
        };

        let local = MaildirBackendBuilder::new()
            .url_encoded_folders(true)
            .build(
                Cow::Borrowed(account),
                Cow::Owned(MaildirConfig {
                    root_dir: sync_dir.join(&account.name),
                }),
            )?;

        let cache = folder::sync::Cache::new(Cow::Borrowed(account), &sync_dir)?;
        let folders = folder::sync_all(&cache, &local, self, dry_run)?;

        let cache = envelope::sync::Cache::new(Cow::Borrowed(account), &sync_dir)?;
        for folder in &folders {
            envelope::sync_all(folder, &cache, &local, self, dry_run)?;
        }

        Ok(())
    }
}
