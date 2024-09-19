use crate::{api::FirestoreApi, resolver::ResolveApi, util};
use std::{convert::Infallible, sync::Arc};
use warp::{self, Filter};

pub fn with_firestore(
    firestore: Arc<FirestoreApi>,
) -> impl Filter<Extract = (Arc<FirestoreApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&firestore))
}

pub fn with_resolver(
    resolver: Arc<ResolveApi>,
) -> impl Filter<Extract = (Arc<ResolveApi>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&resolver))
}

pub fn with_keys(
    keys: Arc<util::keys::Keys>,
) -> impl Filter<Extract = (Arc<util::keys::Keys>,), Error = Infallible> + Clone {
    warp::any().map(move || Arc::clone(&keys))
}
