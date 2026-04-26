-- Menambahkan data dummy yang komprehensif ke tabel emission_factors
INSERT INTO emission_factors (category, subcategory, factor_value, unit, source) VALUES 
('transport', 'car', 0.192, 'km', 'EPA 2023'),
('transport', 'bus', 0.089, 'km', 'EPA 2023'),
('transport', 'flight', 0.254, 'km', 'IPCC 2023'),
('food', 'chicken', 6.9, 'kg', 'IPCC 2023'),
('food', 'rice', 4.0, 'kg', 'IPCC 2023'),
('energy', 'electricity', 0.85, 'kWh', 'PLN 2023'),
('shopping', 'clothes', 15.0, 'kg', 'Global 2023')
ON CONFLICT (category, subcategory) DO NOTHING;
