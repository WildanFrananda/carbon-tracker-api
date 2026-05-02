-- migrations/<timestamp>_add_google_auth_to_users.sql

-- 1. Tambahkan kolom google_id
ALTER TABLE users ADD COLUMN google_id VARCHAR(255);

-- 2. Buat kolom google_id menjadi unik
ALTER TABLE users ADD CONSTRAINT users_google_id_key UNIQUE (google_id);

-- 3. Ubah password_hash menjadi opsional (Nullable)
-- Ini agar user yang login via Google tidak wajib punya password lokal
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
