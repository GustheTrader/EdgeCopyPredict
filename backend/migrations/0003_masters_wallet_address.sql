-- Add wallet_address to masters for on-chain trade ingestion

ALTER TABLE masters ADD COLUMN IF NOT EXISTS wallet_address TEXT;
