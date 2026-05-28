-- Fix risk_rules schema mismatch: add missing 'id' column
-- The SeaORM entity expects id as BIGSERIAL PK, but DB only has user_id as PK

-- Add id column if it doesn't exist (will fail gracefully if it does)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='risk_rules' AND column_name='id') THEN
        ALTER TABLE risk_rules ADD COLUMN id BIGSERIAL;
    END IF;
END $$;

-- Drop the existing PK constraint and set id as new PK
DO $$
DECLARE
    pk_name TEXT;
BEGIN
    -- Find the primary key constraint name
    SELECT constraint_name INTO pk_name
    FROM information_schema.table_constraints
    WHERE table_name = 'risk_rules' AND constraint_type = 'PRIMARY KEY';

    -- Drop it if it exists
    IF pk_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE risk_rules DROP CONSTRAINT %I', pk_name);
    END IF;
END $$;

-- Add PK on id column (IF NOT EXISTS won't work for ALTER TABLE ADD CONSTRAINT)
ALTER TABLE risk_rules ADD CONSTRAINT risk_rules_pkey PRIMARY KEY (id);

-- Add UNIQUE constraint on user_id (since it was effectively unique before)
ALTER TABLE risk_rules ADD CONSTRAINT risk_rules_user_id_key UNIQUE (user_id);
