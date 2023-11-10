use firestore::FirestoreDb;

use crate::Status;

pub struct FirestoreApi {
    db: FirestoreDb,
    // credentials_file: String,
    // session: ServiceSession,
    // next_refresh: SystemTime,
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

    /// Returns a Firestore session created from input credentials.
    // pub fn from_credentials(credentials_file: String) -> Result<Self, FirebaseError> {
    //     let mut cred = Credentials::from_file(&credentials_file).expect("Read credentials file");
    //     cred.download_google_jwks()
    //         .expect("Failed to download public keys");

    //     Ok(FirestoreApi {
    //         credentials_file,
    //         session: ServiceSession::new(cred).expect("Create a service account session"),
    //         next_refresh: SystemTime::now()
    //             .checked_add(Duration::from_secs(30 * 60))
    //             .unwrap(),
    //     })
    // }

    pub fn validate(&mut self) {
        // if self.next_refresh <= SystemTime::now() {
        //     let mut cred =
        //         Credentials::from_file(&self.credentials_file).expect("Read credentials file");
        //     cred.download_google_jwks()
        //         .expect("Failed to download public keys");
        //     self.session = ServiceSession::new(cred).expect("Create a service account session");
        //     self.next_refresh = SystemTime::now()
        //         .checked_add(Duration::from_secs(30 * 60))
        //         .unwrap();
        // }
    }
}
