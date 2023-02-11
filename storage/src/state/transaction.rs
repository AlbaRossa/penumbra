use std::{any::Any, future::Future, pin::Pin};

use anyhow::Result;
use async_trait::async_trait;
use futures::{FutureExt, Stream};
use tendermint::abci;

use crate::State;

use super::{
    read::{nonconsensus_prefix_raw_with_cache, prefix_keys_with_cache, prefix_raw_with_cache},
    Cache, StateRead, StateWrite,
};

/// A set of pending changes to a [`State`] instance, supporting both writes and reads.
pub struct Transaction<'a> {
    /// The `State` instance this transaction will modify.
    ///
    /// Holding on to a &mut reference ensures there can only be one live transaction at a time.
    state: &'a mut State,
    cache: Cache,
}

impl<'a> Transaction<'a> {
    pub(crate) fn new(state: &'a mut State) -> Self {
        Self {
            state,
            cache: Default::default(),
        }
    }

    /// Applies this transaction's writes to its parent [`State`], completing the transaction.
    ///
    /// Returns a list of all the events that occurred while building the transaction.
    pub fn apply(mut self) -> Vec<abci::Event> {
        let events = std::mem::take(&mut self.cache.events);

        self.state.cache.merge(self.cache);

        events
    }

    /// Aborts this transaction, discarding its writes.
    ///
    /// Convienence method for [`std::mem::drop`].
    pub fn abort(self) {}
}

impl<'a> StateWrite for Transaction<'a> {
    fn put_raw(&mut self, key: String, value: jmt::OwnedValue) {
        self.cache.unwritten_changes.insert(key, Some(value));
    }

    fn delete(&mut self, key: String) {
        self.cache.unwritten_changes.insert(key, None);
    }

    fn nonconsensus_delete(&mut self, key: Vec<u8>) {
        self.cache.nonconsensus_changes.insert(key, None);
    }

    fn nonconsensus_put_raw(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.cache.nonconsensus_changes.insert(key, Some(value));
    }

    fn object_put<T: Any + Send + Sync>(&mut self, key: &'static str, value: T) {
        self.cache
            .ephemeral_objects
            .insert(key, Some(Box::new(value)));
    }

    fn object_delete(&mut self, key: &'static str) {
        self.cache.ephemeral_objects.insert(key, None);
    }

    fn record(&mut self, event: abci::Event) {
        self.cache.events.push(event)
    }
}

#[async_trait]
impl<'tx> StateRead for Transaction<'tx> {
    fn get_raw(
        &self,
        key: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>>> + Send + 'static>> {
        // We want to return a 'static future, so we need to get all our references
        // to &self done upfront, before we bundle the results into a future.

        // If the key is available in the unwritten_changes cache, extract it now,
        // so we can move it into the future we'll return.
        let cached_value = self.cache.unwritten_changes.get(key).cloned();
        // Prepare a query to the state; this won't start executing until we poll it.
        let state_value = self.state.get_raw(key);

        async move {
            match cached_value {
                // If the key is available in the unwritten_changes cache, return it.
                Some(v) => Ok(v),
                // Otherwise, if the key is available in the state, return it.
                None => state_value.await,
            }
        }
        .boxed()
    }

    async fn nonconsensus_get_raw(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // If the key is available in the nonconsensus cache, return it.
        if let Some(v) = self.cache.nonconsensus_changes.get(key) {
            return Ok(v.clone());
        }

        // Otherwise, if the key is available in the state, return it.
        self.state.nonconsensus_get_raw(key).await
    }

    fn prefix_raw<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<(String, Vec<u8>)>> + Sync + Send + 'a>> {
        prefix_raw_with_cache(self.state, &self.cache.unwritten_changes, prefix)
    }

    fn prefix_keys<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Pin<Box<dyn Stream<Item = Result<String>> + Sync + Send + 'a>> {
        prefix_keys_with_cache(self.state, &self.cache.unwritten_changes, prefix)
    }

    fn nonconsensus_prefix_raw<'a>(
        &'a self,
        prefix: &'a [u8],
    ) -> Pin<Box<dyn Stream<Item = Result<(Vec<u8>, Vec<u8>)>> + Sync + Send + 'a>> {
        nonconsensus_prefix_raw_with_cache(self.state, &self.cache.nonconsensus_changes, prefix)
    }

    fn object_get<T: Any + Send + Sync>(&self, key: &'static str) -> Option<&T> {
        if let Some(v_or_deletion) = self.cache.ephemeral_objects.get(key) {
            return v_or_deletion.as_ref().and_then(|v| v.downcast_ref());
        }
        self.state.object_get(key)
    }
}
