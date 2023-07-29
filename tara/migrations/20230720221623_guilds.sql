-- Add migration script here

-- Any binary blobs are all encoded with lib.rs/bincode


CREATE TABLE IF NOT EXISTS guilds (
    id BIGINT PRIMARY KEY NOT NULL,
    name TEXT
);

-- Self-assignable roles related to their respective guilds
CREATE TABLE IF NOT EXISTS roles (
    id BIGINT PRIMARY KEY NOT NULL, -- I was *told* these are unique across servers (RoleId/u64)
    guild_id BIGINT NOT NULL,
    FOREIGN KEY(guild_id) REFERENCES guilds(id)
);

CREATE TABLE IF NOT EXISTS registered_components (
    componet_id TEXT PRIMARY KEY NOT NULL,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    expiry_date TIMESTAMP,  -- An optional chrono::DateTime<Utc> (when to delete this record)
    FOREIGN KEY(guild_id) REFERENCES guilds(id)
);