use firestore::FirestoreDb;

use crate::Status;

pub struct FirestoreApi {
    db: FirestoreDb,
}

impl FirestoreApi {
    pub async fn connect() -> Result<Self, Status> {
        Ok(FirestoreApi {
            db: FirestoreDb::new("espy-library").await?,
        })
    }

    pub fn db(&self) -> &FirestoreDb {
        &self.db
    }
}
