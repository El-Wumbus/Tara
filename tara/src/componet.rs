use std::{collections::HashMap, future::Future, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use serenity::{all::ComponentInteraction, client::Cache, futures::future::BoxFuture, http::Http};
use tokio::sync::RwLock;

use crate::{commands::CommandArguments, Result};

pub struct AsyncFn<Params, Returns> {
    func: Box<dyn Fn(Params) -> BoxFuture<'static, Returns> + Send + Sync + 'static>,
}

impl<P: 'static, R> AsyncFn<P, R> {
    pub fn new<F>(f: fn(P) -> F) -> AsyncFn<P, R>
    where
        F: Future<Output = R> + Send + 'static,
    {
        AsyncFn {
            func: Box::new(move |params: P| Box::pin(f(params))),
        }
    }

    pub(super) async fn run(&self, params: P) -> R { (self.func)(params).await }
}

pub type ComponentFn = AsyncFn<(ComponentInteraction, CommandArguments), Result<()>>;
pub type CleanupFn = AsyncFn<(CustomId, Arc<Http>, Arc<Cache>), Result<()>>;

type CustomId = String;


struct ComponentInner {
    component_map: RwLock<HashMap<CustomId, (ComponentFn, Option<CleanupFn>)>>,
    timeout_map:   RwLock<HashMap<CustomId, DateTime<Utc>>>,
}

impl ComponentInner {
    fn new() -> Self {
        Self {
            component_map: RwLock::new(HashMap::new()),
            timeout_map:   RwLock::new(HashMap::new()),
        }
    }

    async fn insert(&self, id: CustomId, f: ComponentFn, cf: Option<CleanupFn>) {
        let _ = self.component_map.write().await.insert(id.clone(), (f, cf));
        let _ = self
            .timeout_map
            .write()
            .await
            .insert(id, Utc::now() + Duration::minutes(10));
    }

    // Returns `None` if there's nothing ran
    pub async fn run(
        &self,
        id: &CustomId,
        params: (ComponentInteraction, CommandArguments),
    ) -> Option<Result<()>> {
        let lock = self.component_map.read().await;
        let (f, _) = lock.get(id)?;
        Some(f.run(params).await)
    }
}

#[derive(Clone)]
pub struct ComponentMap {
    inner: Arc<ComponentInner>,
}

impl ComponentMap {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(ComponentInner::new()),
        }
    }

    pub(super) async fn timeout_watcher(&self, http: Arc<Http>, cache: Arc<Cache>) -> Result<()> {
        loop {
            let now = Utc::now();
            for (id, time) in self.inner.timeout_map.read().await.iter() {
                if *time <= now {
                    if let Some((_, Some(cleanup))) = self.inner.component_map.write().await.remove(id) {
                        cleanup.run((id.clone(), http.clone(), cache.clone())).await?;
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    }

    pub async fn insert(&self, id: CustomId, f: ComponentFn, cf: Option<CleanupFn>) {
        self.inner.insert(id, f, cf).await;
    }

    // Returns `None` if there's nothing ran
    pub async fn run(
        &self,
        id: &CustomId,
        params: (ComponentInteraction, CommandArguments),
    ) -> Option<Result<()>> {
        self.inner.run(id, params).await
    }
}
