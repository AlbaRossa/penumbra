use async_trait::async_trait;
use cnidarium::TempStorage;
use penumbra_app::app::App;
use penumbra_genesis::AppState;
use std::ops::Deref;

#[async_trait]
pub trait TempStorageExt: Sized {
    async fn apply_genesis(self, genesis: AppState) -> anyhow::Result<Self>;
    async fn apply_default_genesis(self) -> anyhow::Result<Self>;
}

#[async_trait]
impl TempStorageExt for TempStorage {
    async fn apply_genesis(self, genesis: AppState) -> anyhow::Result<Self> {
        // Check that we haven't already applied a genesis state:
        if self.latest_version() != u64::MAX {
            anyhow::bail!("database already initialized");
        }

        // Apply the genesis state to the storage
        let mut app = App::new(self.latest_snapshot()).await?;
        app.init_chain(&genesis).await;
        app.commit(self.deref().clone()).await;

        Ok(self)
    }

    async fn apply_default_genesis(self) -> anyhow::Result<Self> {
        self.apply_genesis(Default::default()).await
    }
}
