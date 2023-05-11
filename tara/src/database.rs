//! Contains methods for managing guild preferences ([`Guilds`] and [`GuildPreferences`])
//! that are stored in memory and on the file system. This system is suitable for the
//! lightweight data currently stored but an actual database would need to be made to
//! store any real data if Tara were to begin to support more complex features that
//! require more data.
//!
//! [self] uses a single ron file that contains a Vec of [`GuildPreferences`] that
//! gets read into memory as a [`HashMap<GuildId, GuildPreferences>`]. This gets stored in
//! a single [`Guilds`], which is a [`Arc<RwLock<HashMap<GuildId, GuildPreferences>>>`].
//! [Guilds::get]ing a value clones it. [Guilds::save]ing clones everything before writing
//! it out.

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    path::PathBuf,
    sync::Arc,
};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serenity::{
    all::Role,
    http::Http,
    model::prelude::{GuildId, RoleId},
};
use tokio::{fs::File, sync::RwLock, task};

use crate::{defaults, error::Result};

static DATABASE_DIR: Lazy<PathBuf> = Lazy::new(|| crate::paths::database_directory().unwrap());
static GUILD_PREFERENCES_PATH: Lazy<PathBuf> = Lazy::new(|| DATABASE_DIR.join("GuildPreferences.ron"));


#[derive(Debug, Clone)]
pub struct Guilds(Arc<RwLock<HashMap<GuildId, GuildPreferences>>>);

impl Guilds {
    /// Create a new, empty [`Guilds`].
    ///
    /// # Usage
    ///     
    /// ```no_run
    /// # use tara::database::Guilds;
    /// # tokio_test::block_on(async {
    /// let guild_preferences_map = Guilds::create().await.unwrap();
    /// dbg!(guild_preferences_map);
    /// # });
    /// ```
    ///
    /// # Errors
    ///
    /// Will error if saving ([`Self::save()`]) fails
    pub async fn create() -> Result<Self> {
        let empty_guilds = Self(Arc::new(RwLock::new(HashMap::new())));
        empty_guilds.save().await?;
        Ok(empty_guilds)
    }

    /// Insert a [`GuildPreferences`] into `self`. This is only applied to the in-memory
    /// database, so it may be appropriate to [`Self::save()`].
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use tara::database::{Guilds, GuildPreferences};
    /// # use serenity::model::prelude::*;
    /// # tokio_test::block_on(async {
    /// # let guild_id = GuildId::new(1);
    /// # let guilds = Guilds::create().await.unwrap();
    /// let preferences = GuildPreferences::default(guild_id);
    /// guilds.insert(preferences);
    /// assert!(guilds.get(guild_id).await.is_some());
    /// # });
    /// ```
    pub async fn insert(&self, preferences: GuildPreferences) {
        self.0.write().await.insert(preferences.id, preferences);
    }

    /// Modify an existing [`GuildPreferences`].
    ///
    /// # Usage
    ///
    /// The case in which the [`GuildId`] provided isn't present should be handled inside
    /// the provided function. The intended use-case is to pass in a closure:
    ///
    /// ```no_run
    /// # use tara::database::{Guilds, GuildPreferences};
    /// # use serenity::model::prelude::*;
    /// # tokio_test::block_on(async {
    /// # let guild_id = GuildId::new(1);
    /// # let guilds = Guilds::create().await.unwrap();
    /// guilds
    ///     .modify(guild_id, |preferences| {
    ///         if let Some(preferences) = preferences {
    ///             preferences.content_character_limit = 1700;
    ///         };
    ///     })
    ///     .await;
    /// # });
    /// ```
    ///
    /// A return value can emitted from the closure and it will be passed along.
    pub async fn modify<Ret, F: FnOnce(Option<&mut GuildPreferences>) -> Ret>(
        &self,
        id: GuildId,
        f: F,
    ) -> Ret {
        let mut guild_write_lock = self.0.write().await;
        let prefs = guild_write_lock.get_mut(&id);
        f(prefs)
    }

    /// Check if `self` contains a [`GuildPreferences`] associated with the [`GuildId`]
    /// provided.
    pub async fn contains(&self, id: GuildId) -> bool { self.0.read().await.contains_key(&id) }

    pub async fn get(&self, id: GuildId) -> Option<GuildPreferences> {
        self.0.read().await.get(&id).map(|x| x.to_owned()) // TODO: don't clone, it
                                                           // sucks!!!
    }

    async fn read() -> Result<HashMap<GuildId, GuildPreferences>> {
        // Create a BufReader and a desearializer
        let guild_preferences_reader = std::io::BufReader::new(
            File::open(GUILD_PREFERENCES_PATH.as_path())
                .await?
                .into_std()
                .await,
        );

        task::spawn_blocking(move || -> Result<_> {
            let mut guild_preferences_map = HashMap::new();
            for guild_preferences in
                ron::de::from_reader::<_, Vec<GuildPreferences>>(guild_preferences_reader)?
            {
                guild_preferences_map
                    .entry(guild_preferences.id)
                    .or_insert(guild_preferences);
            }
            Ok(guild_preferences_map)
        })
        .await?
    }

    /// Load the Guild Preferences from the file system creating a new `Guilds`
    pub async fn load() -> Result<Self> { Ok(Self(Arc::new(RwLock::new(Self::read().await?)))) }

    /// Reload the Guild preferences from the file system modifying an existing `Guilds`
    pub async fn _reload(&self) -> Result<()> {
        *self.0.write().await = Self::read().await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        // Create a BufWriter and a serializer
        let guild_preferences_writer = std::io::BufWriter::new(
            File::create(GUILD_PREFERENCES_PATH.as_path())
                .await?
                .into_std()
                .await,
        );
        let guilds = self.0.read().await;
        let preferences = guilds.clone().into_values().collect::<Vec<_>>();
        task::spawn_blocking(move || -> Result<()> {
            ron::ser::to_writer(guild_preferences_writer, &preferences)?;
            Ok(())
        })
        .await?
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GuildPreferences {
    pub id: GuildId,

    #[serde(default = "defaults::content_character_limit_default")]
    /// The charater limit on content retrived from external sources
    pub content_character_limit: usize,

    #[serde(default)]
    /// Roles that may be self-assigned by a guild member
    assignable_roles: HashSet<SelfAssignableRole>,
}

impl GuildPreferences {
    pub fn default(id: GuildId) -> Self {
        Self {
            id,
            content_character_limit: defaults::content_character_limit_default(),
            assignable_roles: Default::default(),
        }
    }

    pub async fn all_assignable_discord_roles(&self, http: &Http) -> Option<Vec<Role>> {
        // We can unwrap because this command cannot run in DMs
        let guild = self.id.to_partial_guild(http).await.ok()?;
        let guild_roles = guild.roles;
        Some(
            self.assignable_roles
                .iter()
                .filter_map(|role_id| guild_roles.get(&role_id.id()))
                .map(|x| x.to_owned())
                .collect::<Vec<_>>(),
        )
    }

    pub fn _all_assignable_roles(&self) -> Vec<&SelfAssignableRole> {
        self.assignable_roles.iter().collect::<Vec<_>>()
    }

    pub fn _all_assignable_discord_role_ids(&self) -> Vec<RoleId> {
        self.assignable_roles.iter().map(|x| x.id()).collect::<Vec<_>>()
    }

    pub fn get_assignable_roles_mut(&mut self) -> &mut HashSet<SelfAssignableRole> {
        &mut self.assignable_roles
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, Hash, PartialEq, Eq)]
/// A role that may be self-assigned by a user
pub struct SelfAssignableRole(RoleId);

impl SelfAssignableRole {
    pub fn new(id: RoleId) -> Self { Self(id) }

    pub const fn id(&self) -> RoleId { self.0 }
}
