use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serenity::{all::ComponentInteraction, client::Cache, http::Http};
use tokio::sync::RwLock;

use crate::commands::CommandArguments;

type DynComponent = &'static (dyn Component + Send + Sync);

#[async_trait]
pub trait Component {
    /// Runs whenever Discord sends a component interaction with the ID matching that
    /// registered for this [`Component`] on insertion.
    async fn run(&self, interaction: ComponentInteraction, args: CommandArguments) -> anyhow::Result<()>;
    /// Runs whenever the the [`Component`] is removed from the map after its timeout or
    /// after evoking [`ComponentMap::timeout`] with the ID matching that registered for
    /// this [`Component`] on insertion. By default it's a no-op.
    #[allow(unused_variables)]
    async fn cleanup(&self, id: String, http: Arc<Http>, cache: Arc<Cache>) -> anyhow::Result<()> { Ok(()) }
}

struct ComponentInner {
    component_map: RwLock<HashMap<String, (DynComponent, DateTime<Utc>)>>,
}

impl ComponentInner {
    #[inline]
    fn new() -> Self {
        Self {
            component_map: RwLock::new(HashMap::new()),
        }
    }

    async fn insert(&self, id: String, f: DynComponent, timeout_duration: Option<Duration>) {
        let when = Utc::now() + timeout_duration.unwrap_or(Duration::minutes(5));
        let _ = self.component_map.write().await.insert(id.clone(), (f, when));
    }

    // Returns `None` if there's nothing ran
    pub(self) async fn run(
        &self,
        id: &String,
        interaction: ComponentInteraction,
        args: CommandArguments,
    ) -> Option<anyhow::Result<()>> {
        let lock = self.component_map.read().await;
        let (f, _) = lock.get(id)?;
        Some(f.run(interaction, args).await)
    }
}

#[derive(Clone)]
pub struct ComponentMap {
    inner: Arc<ComponentInner>,
}

impl ComponentMap {
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(ComponentInner::new()),
        }
    }

    #[inline]
    // Timeout an id before it's scheduled time. Returns wether it worked
    pub(super) async fn timeout(&self, id: String) -> anyhow::Result<()> {
        let mut component_map = self.inner.component_map.write().await;
        let (_, timeout) = component_map
            .get_mut(&id)
            .context(format!("'{id}' wasn't found in the component map"))?;
        *timeout = Utc::now();
        Ok(())
    }

    pub(super) async fn timeout_watcher(&self, http: Arc<Http>, cache: Arc<Cache>) -> anyhow::Result<()> {
        loop {
            let now = Utc::now();

            let kill_list = {
                let map = self.inner.component_map.read().await;
                map.iter()
                    .filter(|(_, (_, time))| *time <= now)
                    .map(|(id, _)| id.clone()) // So the lock gets dropped when this is done collecting
                    .collect::<Vec<_>>()
            };

            for id in kill_list {
                if let Some((f, _)) = self.inner.component_map.write().await.remove(&id) {
                    tracing::debug!("Removed component listener: {id}");
                    f.cleanup(id.clone(), http.clone(), cache.clone()).await?;
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    #[inline]
    pub async fn insert(&self, id: String, f: DynComponent, timeout_duration: Option<Duration>) {
        self.inner.insert(id, f, timeout_duration).await;
    }

    #[inline]
    // Returns `None` if there's nothing ran
    pub async fn run(
        &self,
        id: &String,
        interaction: ComponentInteraction,
        args: CommandArguments,
    ) -> Option<anyhow::Result<()>> {
        self.inner.run(id, interaction, args).await
    }
}
