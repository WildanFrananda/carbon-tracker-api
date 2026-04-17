# ---------------------------------------------------
# STAGE 1: Builder
# ---------------------------------------------------
# Kita gunakan versi rust resmi terbaru
FROM rust:1.75 as builder

WORKDIR /usr/src/app

# Salin seluruh kode proyek
COPY . .

# SQLx membutuhkan akses ke database SATAT COMPILE-TIME.
# Namun dalam Docker build, kita tidak punya akses ke DB aktif.
# SOLUSI: Menggunakan mode offline SQLx.
# Pastikan Anda menjalankan `cargo sqlx prepare` di mesin lokal (macOS) sebelum mem-build Docker ini!
ENV SQLX_OFFLINE=true

# Compile proyek dalam mode release untuk performa maksimal
RUN cargo build --release

# ---------------------------------------------------
# STAGE 2: Runtime Environment (Ubuntu/Debian Server)
# ---------------------------------------------------
FROM debian:bookworm-slim

# Install OpenSSL (dibutuhkan oleh rust-postgres/rustls) dan ca-certificates
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Salin binary hasil build dari Stage 1
COPY --from=builder /usr/src/app/target/release/carbon-tracker-api /app/carbon-tracker-api

# Salin folder migrations agar bisa dijalankan di staging
COPY --from=builder /usr/src/app/migrations /app/migrations

# Expose port yang digunakan Rocket
EXPOSE 8000

# Variabel environment default
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

# Jalankan aplikasi
CMD ["/app/carbon-tracker-api"]