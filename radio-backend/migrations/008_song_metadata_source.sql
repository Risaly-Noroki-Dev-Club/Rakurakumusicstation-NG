-- 记录外部元数据匹配结果，避免重复搜索，并为后续手动修正保留来源。
ALTER TABLE songs ADD COLUMN ncm_song_id INTEGER;
ALTER TABLE songs ADD COLUMN metadata_source TEXT NOT NULL DEFAULT '';
ALTER TABLE songs ADD COLUMN metadata_matched_at DATETIME;

CREATE INDEX IF NOT EXISTS idx_songs_ncm_song_id ON songs(ncm_song_id);
