-- Persistent metadata repair jobs and field-level provenance.

ALTER TABLE songs ADD COLUMN musicbrainz_recording_id TEXT;
ALTER TABLE songs ADD COLUMN metadata_revision INTEGER NOT NULL DEFAULT 0;

CREATE TABLE song_metadata_fields (
    song_id     INTEGER NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    field_name  TEXT NOT NULL CHECK (field_name IN ('title', 'artist', 'album', 'cover', 'lyrics')),
    source      TEXT NOT NULL DEFAULT 'legacy',
    locked      INTEGER NOT NULL DEFAULT 0 CHECK (locked IN (0, 1)),
    updated_at  DATETIME NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (song_id, field_name)
);

CREATE TABLE metadata_jobs (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('local', 'online', 'full')),
    status          TEXT NOT NULL CHECK (status IN ('queued', 'running', 'completed', 'cancelled', 'failed')),
    scope           TEXT NOT NULL CHECK (scope IN ('library', 'songs')),
    song_ids_json   TEXT NOT NULL DEFAULT '[]',
    force           INTEGER NOT NULL DEFAULT 0 CHECK (force IN (0, 1)),
    total           INTEGER NOT NULL DEFAULT 0,
    processed       INTEGER NOT NULL DEFAULT 0,
    matched         INTEGER NOT NULL DEFAULT 0,
    needs_review    INTEGER NOT NULL DEFAULT 0,
    failed          INTEGER NOT NULL DEFAULT 0,
    error           TEXT NOT NULL DEFAULT '',
    created_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    finished_at     DATETIME
);

CREATE TABLE metadata_job_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id          TEXT NOT NULL REFERENCES metadata_jobs(id) ON DELETE CASCADE,
    song_id         INTEGER NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    status          TEXT NOT NULL CHECK (status IN ('queued', 'running', 'updated', 'skipped', 'needs_review', 'failed', 'cancelled')),
    stage           TEXT NOT NULL DEFAULT 'queued',
    message         TEXT NOT NULL DEFAULT '',
    candidates_json TEXT NOT NULL DEFAULT '[]',
    attempts        INTEGER NOT NULL DEFAULT 0,
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now')),
    UNIQUE (job_id, song_id)
);

CREATE INDEX idx_metadata_jobs_status ON metadata_jobs(status, created_at);
CREATE INDEX idx_metadata_job_items_job_status ON metadata_job_items(job_id, status, id);
CREATE INDEX idx_song_metadata_fields_locked ON song_metadata_fields(song_id, locked);
