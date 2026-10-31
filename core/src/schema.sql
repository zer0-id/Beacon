-- the hash cache for all the files in a specific transfer
-- the cache is cleared if there was no tap in the last 7 days
CREATE TABLE IF NOT EXISTS hash_cache (
    file_path TEXT PRIMARY KEY,
    file_size INTEGER NOT NULL,
    mtime INTEGER NOT NULL,
    file_hash BLOB NOT NULL,
    ctime INTEGER NOT_NULL, -- time of insertion into cache
);

-- peers lose trusted privileges after 30 days
CREATE TABLE IF NOT EXISTS trusted_devices (
    fingerprint BLOB PRIMARY KEY,
    capability BLOB NOT NULL,
    last_seen INTEGER NOT NULL
);