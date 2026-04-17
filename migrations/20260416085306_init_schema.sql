-- Create Users Table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    daily_target_kg NUMERIC(10, 2) NOT NULL DEFAULT 8.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create Emission Factors (Master Data)
CREATE TABLE emission_factors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category VARCHAR(50) NOT NULL,
    subcategory VARCHAR(50) NOT NULL,
    factor_value NUMERIC(10, 4) NOT NULL,
    unit VARCHAR(20) NOT NULL,
    source VARCHAR(255) NOT NULL,
    UNIQUE(category, subcategory)
);

-- Create Activities (Log Harian)
CREATE TABLE activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category VARCHAR(50) NOT NULL,
    subcategory VARCHAR(50) NOT NULL,
    quantity NUMERIC(10, 2) NOT NULL,
    unit VARCHAR(20) NOT NULL,
    calculated_emission_kg NUMERIC(10, 2) NOT NULL,
    date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create Daily Summaries (Agregasi)
CREATE TABLE daily_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    total_emission NUMERIC(10, 2) NOT NULL DEFAULT 0,
    transport_kg NUMERIC(10, 2) NOT NULL DEFAULT 0,
    food_kg NUMERIC(10, 2) NOT NULL DEFAULT 0,
    energy_kg NUMERIC(10, 2) NOT NULL DEFAULT 0,
    shopping_kg NUMERIC(10, 2) NOT NULL DEFAULT 0,
    is_green_day BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(user_id, date)
);

-- Create Achievements
CREATE TABLE achievements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_type VARCHAR(50) NOT NULL,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, badge_type)
);

-- Indexes for performance
CREATE INDEX idx_activities_user_date ON activities(user_id, date);
CREATE INDEX idx_daily_summaries_user_date ON daily_summaries(user_id, date);