use std::time::{Duration, SystemTime};

use crate::Status;
use firestore::errors::FirebaseError;
use firestore_db_and_auth as firestore;
use firestore_db_and_auth::{documents, dto, Credentials, ServiceSession};
use serde::{Deserialize, Serialize};
use tracing::error;

pub struct FirestoreApi {
    credentials_file: String,
    session: ServiceSession,
    next_refresh: SystemTime,
}

impl FirestoreApi {
    /// Returns a Firestore session created from input credentials.
    pub fn from_credentials(
        credentials_file: String,
    ) -> Result<Self, firestore::errors::FirebaseError> {
        let mut cred = Credentials::from_file(&credentials_file).expect("Read credentials file");
        cred.download_google_jwks()
            .expect("Failed to download public keys");

        Ok(FirestoreApi {
            credentials_file,
            session: ServiceSession::new(cred).expect("Create a service account session"),
            next_refresh: SystemTime::now()
                .checked_add(Duration::from_secs(30 * 60))
                .unwrap(),
        })
    }

    pub fn validate(&mut self) {
        if self.next_refresh <= SystemTime::now() {
            let mut cred =
                Credentials::from_file(&self.credentials_file).expect("Read credentials file");
            cred.download_google_jwks()
                .expect("Failed to download public keys");
            self.session = ServiceSession::new(cred).expect("Create a service account session");
            self.next_refresh = SystemTime::now()
                .checked_add(Duration::from_secs(30 * 60))
                .unwrap();
        }
    }

    /// Returns a document based on its id.
    pub fn read<T>(&self, path: &str, doc_id: &str) -> Result<T, Status>
    where
        for<'a> T: Deserialize<'a>,
    {
        match documents::read(&self.session, path, doc_id) {
            Ok(doc) => Ok(doc),
            Err(FirebaseError::APIError(404, _, _)) => Err(Status::not_found(format!(
                "Firestore document {path}/{doc_id} not found."
            ))),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    /// Writes a document given an optional id. If None is provided for doc_id
    /// a new document is created with a generated id. Returns the document id.
    pub fn write<T>(&self, path: &str, doc_id: Option<&str>, doc: &T) -> Result<String, Status>
    where
        T: Serialize,
    {
        match documents::write(
            &self.session,
            path,
            doc_id,
            doc,
            documents::WriteOptions::default(),
        ) {
            Ok(result) => Ok(result.document_id),
            Err(e) => Err(Status::new("Firestore.write: ", e)),
        }
    }

    /// Updates a document with fields present in the struct. For Option fields
    /// that are not set the existing document value is preserved. Fails is the
    /// document does not already exist.
    pub fn update<T>(&self, path: &str, doc_id: &str, doc: &T) -> Result<String, Status>
    where
        T: Serialize,
    {
        match documents::write(
            &self.session,
            path,
            Some(doc_id),
            doc,
            documents::WriteOptions { merge: true },
        ) {
            Ok(result) => Ok(result.document_id),
            Err(e) => Err(Status::new("Firestore.update: ", e)),
        }
    }

    /// Deletes a document in the specified path. Silently returns if no such
    /// document exists.
    pub fn delete(&self, path: &str) -> Result<(), Status> {
        match documents::delete(&self.session, path, false) {
            Ok(_) => Ok(()),
            Err(e) => Err(Status::new("Firestore.delete: ", e)),
        }
    }

    /// Returns all Firestore documents in the specified path.
    pub fn list<T>(&self, path: &str) -> Result<Vec<T>, Status>
    where
        for<'a> T: Deserialize<'a> + Default,
    {
        let collection: documents::List<T, _> = documents::list(&self.session, path);
        collection
            .into_iter()
            .map(|result| match result {
                Ok((doc, _metadata)) => Ok(doc),
                Err(e) => {
                    // Err(Status::new("Firestore.list: ", e))
                    error!("Failed to parse doc: {e}");
                    Ok(T::default())
                }
            })
            .collect()
    }

    /// Returns all Firestore documents in the specified path that satisfy the
    /// matching condition.
    pub fn query<T>(
        &self,
        path: &str,
        field_name: &str,
        value: serde_json::Value,
    ) -> Result<Vec<T>, Status>
    where
        for<'a> T: Deserialize<'a>,
    {
        let result = match documents::query(
            &self.session,
            path,
            value,
            dto::FieldOperator::EQUAL,
            field_name,
        ) {
            Ok(result) => result,
            Err(e) => return Err(Status::new("Firestore.query: ", e)),
        };

        result
            .into_iter()
            .map(
                |metadata| match documents::read_by_name(&self.session, &metadata.name) {
                    Ok(doc) => Ok(doc),
                    Err(e) => Err(Status::new("Firestore.query.read_by_name: ", e)),
                },
            )
            .collect()
    }
}
