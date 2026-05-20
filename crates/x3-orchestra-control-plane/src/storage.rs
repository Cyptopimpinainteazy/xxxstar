use crate::error::{ControlPlaneError, Result};
use crate::service::OrchestraState;
use std::path::Path;

const STATE_KEY: &[u8] = b"orchestra_state_v1";

pub struct PersistentStateStore {
    db: sled::Db,
}

impl PersistentStateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path).map_err(|err| ControlPlaneError::Persistence(err.to_string()))?;
        Ok(Self { db })
    }

    pub(crate) fn load_state(&self) -> Result<Option<OrchestraState>> {
        let raw = self
            .db
            .get(STATE_KEY)
            .map_err(|err| ControlPlaneError::Persistence(err.to_string()))?;
        raw.map(|bytes| {
            serde_json::from_slice::<OrchestraState>(&bytes)
                .map_err(|err| ControlPlaneError::Persistence(err.to_string()))
        })
        .transpose()
    }

    pub(crate) fn save_state(&self, state: &OrchestraState) -> Result<()> {
        let bytes = serde_json::to_vec(state)
            .map_err(|err| ControlPlaneError::Persistence(err.to_string()))?;
        self.db
            .insert(STATE_KEY, bytes)
            .map_err(|err| ControlPlaneError::Persistence(err.to_string()))?;
        self.db
            .flush()
            .map_err(|err| ControlPlaneError::Persistence(err.to_string()))?;
        Ok(())
    }
}
