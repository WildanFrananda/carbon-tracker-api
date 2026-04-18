# 🌍 Personal Carbon Footprint Tracker - Backend API

Backend API ini adalah mesin cerdas (**Smart Engine**) untuk melacak, menghitung, dan menganalisis jejak karbon harian pengguna. Dibangun dengan **Rust (Rocket framework)** untuk performa tinggi dan skalabilitas, menggunakan arsitektur **Layered** yang rapi.

---

## 🛠️ Tech Stack

| Komponen | Teknologi |
|---|---|
| Language | Rust 🦀 |
| Framework | Rocket v0.5.x (Async) |
| Database | PostgreSQL 15 |
| ORM | SQLx (Compile-time checked queries) |
| Auth | JWT (jsonwebtoken) & Argon2 (Password Hashing) |
| Infrastructure | Docker & Docker Compose (Nginx Reverse Proxy for Staging) |

---

## 👨‍💻 Backend Developer Guide (Setup & Workflow)

Proyek ini dirancang agar mudah dikembangkan di **macOS** (sebagai mesin utama) dan dapat di-deploy dengan lancar ke **Ubuntu** (sebagai staging/production).

### 1. Prerequisites (macOS)

Pastikan Anda telah menginstal:

- **Rust Toolchain**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Docker Desktop for Mac**
- **SQLx CLI**
  ```bash
  cargo install sqlx-cli --no-default-features --features rustls,postgres
  ```

### 2. Initial Setup

**Clone repo & siapkan environment:**

Buat file `.env` di root direktori:

```env
DATABASE_URL=postgres://tracker_user:tracker_pass@localhost:5432/carbon_tracker_db
JWT_SECRET=rahasia_untuk_development
ROCKET_PORT=8000
ROCKET_ADDRESS=0.0.0.0
```

**Jalankan Database Lokal:**

```bash
docker compose up -d
```

**Setup Database Schema:**

```bash
sqlx database create
sqlx migrate run
```

**Seed Data (Wajib untuk Kalkulasi Engine):**

Jalankan query ini ke database PostgreSQL Anda (bisa via DBeaver atau `psql`):

```sql
INSERT INTO emission_factors (category, subcategory, factor_value, unit, source) VALUES 
('transport', 'motorcycle', 0.112, 'km', 'EPA 2023'),
('food', 'beef', 27.0, 'kg', 'IPCC 2023');
```

### 3. Running & Testing

| Perintah | Fungsi |
|---|---|
| `cargo run` | Menjalankan server |
| `cargo test` | Menjalankan integration tests |

### 4. Deployment Workflow (macOS → Ubuntu Staging)

Karena kita menggunakan `sqlx` dengan compile-time validation, Docker tidak memiliki akses ke database Anda saat proses build. Oleh karena itu, lakukan langkah ini:

**Di macOS** — simpan struktur query (wajib sebelum commit ke Git):

```bash
cargo sqlx prepare
```

**Di Ubuntu (Staging)** — pull kode dari Git, lalu jalankan Docker Compose versi staging:

```bash
docker compose -f docker-compose.staging.yml up -d --build
```

> API akan terekspos di **Port 80** via Nginx.

---

## 📱 Mobile Developer Guide (API Contract)

Semua endpoint memberikan response dalam format JSON standar:

```json
{
  "status": "success|error",
  "message": "Pesan deskriptif (Opsional)",
  "data": { ... }
}
```

- **Base URL (Local):** `http://localhost:8000`
- **Base URL (Staging):** `http://<IP_UBUNTU>`

---

## 🔐 1. Authentication

> Semua endpoint selain Auth membutuhkan Header `Authorization: Bearer <TOKEN>`.

### `POST /api/auth/register`

Mendaftarkan akun baru.

**Body Request:**

```json
{
  "email": "user@example.com",
  "password": "securepassword",
  "display_name": "Wildan Frananda"
}
```

**Response `200 OK`:** Mengembalikan `token` (JWT) dan `user_id`.

---

### `POST /api/auth/login`

Masuk untuk mendapatkan token.

**Body Request:**

```json
{
  "email": "user@example.com",
  "password": "securepassword"
}
```

**Response `200 OK`:** Mengembalikan `token` (JWT). Simpan token ini di **Secure Storage** (iOS Keychain / Android EncryptedSharedPreferences).

---

## 📝 2. Activity Logging (The Engine)

### `POST /api/activities`

Mencatat aktivitas dan memicu **Smart Engine** untuk menghitung emisi CO₂ serta mengupdate agregasi harian secara otomatis.

**Headers:** `Authorization: Bearer <TOKEN>`

**Body Request:**

```json
{
  "category": "transport",
  "subcategory": "motorcycle",
  "quantity": 15.5,
  "date": "2026-04-18"
}
```

> `category` — pilihan: `"transport"`, `"food"`, `"energy"`, `"shopping"`
> `quantity` — tipe Float/Decimal
> `date` — format `YYYY-MM-DD`

**Response `200 OK`:** Mengembalikan data aktivitas beserta `calculated_emission_kg` (hasil hitungan Engine).

---

### `GET /api/activities?date=YYYY-MM-DD`

Mengambil riwayat aktivitas pada tanggal tertentu.

**Headers:** `Authorization: Bearer <TOKEN>`

**Response `200 OK`:** Array of Activity Objects.

---

### `DELETE /api/activities/<id>`

Menghapus aktivitas.

> **Note:** Backend otomatis akan mengkalkulasi ulang (rollback) agregasi di dashboard.

**Headers:** `Authorization: Bearer <TOKEN>`

**Response `200 OK`:** Status success.

---

## 📊 3. Dashboard & Analytics

> Endpoint di bawah ini sangat cepat karena membaca tabel agregasi yang sudah dihitung oleh Engine.

### `GET /api/dashboard/daily?date=YYYY-MM-DD`

Ringkasan emisi harian (untuk Pie Chart / Donut Chart).

**Headers:** `Authorization: Bearer <TOKEN>`

**Response `200 OK`:**

```json
{
  "status": "success",
  "data": {
    "date": "2026-04-18",
    "total_emission": "1.736",
    "transport_kg": "1.736",
    "food_kg": "0.0",
    "energy_kg": "0.0",
    "shopping_kg": "0.0",
    "is_green_day": false
  }
}
```

---

### `GET /api/dashboard/weekly?end_date=YYYY-MM-DD`

Agregasi selama 7 hari ke belakang dari `end_date` (cocok untuk Bar Chart mingguan).

**Headers:** `Authorization: Bearer <TOKEN>`

---

### `GET /api/dashboard/heatmap?year=YYYY`

Data setahun penuh untuk membuat grafik ala GitHub Contribution Graph.

**Headers:** `Authorization: Bearer <TOKEN>`

**Response `200 OK`:**

```json
[
  { "date": "2026-01-01", "total_emission": "4.5" },
  ...
]
```

---

## 💡 4. Smart Insights

### `GET /api/insights/recommendations`

AI-Rules based engine. Menganalisa habit user 7 hari terakhir dan memberikan tips kustom yang bisa dilakukan.

**Headers:** `Authorization: Bearer <TOKEN>`

**Response `200 OK`:**

```json
{
  "status": "success",
  "data": {
    "dominant_category": "Transportasi",
    "message": "Emisi terbesar Anda minggu ini berasal dari transportasi.",
    "actionable_tips": [
      "Pertimbangkan untuk menggunakan transportasi umum 1-2 kali minggu depan."
    ]
  }
}
```

---

*Developed with 🦀 Rust and Rocket.*