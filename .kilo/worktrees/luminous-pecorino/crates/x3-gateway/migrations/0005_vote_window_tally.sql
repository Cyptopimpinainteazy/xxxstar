ALTER TABLE vote_windows
ADD COLUMN IF NOT EXISTS tally JSONB NOT NULL DEFAULT '{"approvals":0,"rejections":0,"abstentions":0}'::jsonb;
