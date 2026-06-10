CREATE TABLE IF NOT EXISTS check_ins (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    check_in_date   DATE NOT NULL,
    checked_in_at   TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_check_ins_user_date
    ON check_ins (user_id, check_in_date);

-- Index for streak calculation: we query consecutive dates per user
-- ordered descending. This composite index covers that query pattern.
CREATE INDEX IF NOT EXISTS idx_check_ins_user_date_desc
    ON check_ins (user_id, check_in_date DESC);