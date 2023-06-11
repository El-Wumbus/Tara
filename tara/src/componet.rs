use std::{collections::HashMap, future::Future, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use serenity::{all::ComponentInteraction, client::Cache, futures::future::BoxFuture, http::Http};
use tokio::sync::{Mutex, RwLock};

use crate::{commands::CommandArguments, Result};

pub struct AsyncFn<Params, Returns> {
    func: Box<dyn Fn(Params) -> BoxFuture<'static, Returns> + Send + Sync + 'static>,
}

impl<P: 'static, R> AsyncFn<P, R> {
    #[inline]
    pub fn new<F>(f: fn(P) -> F) -> AsyncFn<P, R>
    where
        F: Future<Output = R> + Send + 'static,
    {
        AsyncFn {
            func: Box::new(move |params: P| Box::pin(f(params))),
        }
    }

    /// Run an asyncronous function pointer
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// async fn watch_happy_feet(x: usize) -> usize { x * 10 }
    /// # let func = AsyncFn::new(watch_happy_feet, None);
    /// let result = func.run(9).await;
    /// assert_eq!(result, 90usize);
    /// # });
    /// ```
    #[inline]
    pub(super) async fn run(&self, params: P) -> R { (self.func)(params).await }
}

pub type ComponentFn = AsyncFn<(ComponentInteraction, CommandArguments), Result<()>>;
pub type CleanupFn = AsyncFn<(CustomId, Arc<Http>, Arc<Cache>), Result<()>>;

type CustomId = String;


struct ComponentInner {
    component_map: RwLock<HashMap<CustomId, (ComponentFn, Option<CleanupFn>)>>,
    // It's a hashmap because we want the IDs to be unique and have these times associated.
    timeout_map:   Mutex<HashMap<CustomId, DateTime<Utc>>>,
}

impl ComponentInner {
    fn new() -> Self {
        Self {
            component_map: RwLock::new(HashMap::new()),
            timeout_map:   Mutex::new(HashMap::new()),
        }
    }

    async fn insert(&self, id: CustomId, f: ComponentFn, cf: Option<CleanupFn>) {
        let _ = self.component_map.write().await.insert(id.clone(), (f, cf));
        let _ = self
            .timeout_map
            .lock()
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

    // Timeout an id before it's scheduled time.
    pub(super) async fn timeout(&self, id: CustomId) {
        let _ = self.inner.timeout_map.lock().await.insert(id, Utc::now());
    }

    pub(super) async fn timeout_watcher(&self, http: Arc<Http>, cache: Arc<Cache>) -> Result<()> {
        loop {
            let now = Utc::now();

            let kill_list = {
                let timeout_map = self.inner.timeout_map.lock().await;
                let kill_list = timeout_map
                    .iter()
                    .filter(|(_, time)| **time <= now)
                    .map(|(id, _)| id.clone()) // So the lock gets dropped when this is done collecting
                    .collect::<Vec<_>>();

                kill_list
            };

            for id in kill_list {
                if let Some((_, Some(cleanup))) = self.inner.component_map.write().await.remove(&id) {
                    cleanup.run((id.clone(), http.clone(), cache.clone())).await?;
                }
                let _ = self.inner.timeout_map.lock().await.remove(&id);
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
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
