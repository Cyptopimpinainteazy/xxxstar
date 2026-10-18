-- Keep the original source document digest distinct from interpretation version.
-- Version 1 represents snapshots made before structured agency-name precedence.
-- Preserve those records when version 2 reinterprets the same source bytes.
ALTER TABLE funding_source_snapshots
    ADD COLUMN normalization_version INTEGER NOT NULL DEFAULT 1
    CHECK (normalization_version > 0);
ALTER TABLE funding_source_snapshots
    DROP CONSTRAINT funding_source_snapshots_grant_id_content_sha256_key;
ALTER TABLE funding_source_snapshots
    ADD CONSTRAINT funding_snapshot_document_version_unique
    UNIQUE (grant_id, content_sha256, normalization_version);
