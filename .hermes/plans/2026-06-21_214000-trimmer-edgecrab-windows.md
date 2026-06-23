# EdgeCrab Windows Build — Dependency Trimmer Plan

> **Goal:** Trim 3 problematic native dependencies (boring-sys2, aws-lc-sys, rav1e) dan retain rusqlite bundled, lalu rebuild EdgeCrab untuk Windows target dari Docker cross-compile.

**Mengapa ini mungkin:**
- Setelah trimming, native dep yang tersisa cuma **C compiler (rusqlite)** — dari sebelumnya 4 toolchain (cmake + nasm + libclang + golang)
- C compiler via `gcc-mingw-w64-x86-64` adalah paket Debian standar
- Ini memungkinkan cross-compile yang sebelumnya gagal 4x karena spiral dep

**Tek Stack:** Rust 1-slim, mingw-w64 cross toolchain, reqwest (replacement wreq), Docker buildx multiarch

---

## Ringkasan Perubahan

| Dep | File yg diubah | Sifat |
|---|---|---|
| **boring-sys2** (wreq) | 4 file | Hapus DuckDuckGo backend, ganti fallback scraping pake reqwest |
| **aws-lc-sys** (Bedrock) | 3 file | Hapus feature flag `bedrock` dari default |
| **rav1e** (image/arboard) | 2 file + fork arboard | Minimal clipboard, patch image dep |
| **rusqlite** | 0 file | TETAP — tambah C compiler ke Docker toolchain |
| **Dockerfile** | 1 file | Tambah mingw-w64 + cross-compile config |

---

## Task 1: Buang DuckDuckGo Search Backend + wreq (boring-sys2)

**Objective:** Hapus seluruh dependency `wreq` yang narik `boring2` → `boring-sys2` (cmake + nasm + libclang). Ganti dengan `reqwest` biasa + User-Agent spoof.

**Files:**
- Modify: `Cargo.toml` workspace — hapus baris `wreq`
- Modify: `crates/edgecrab-tools/Cargo.toml` — hapus dep `wreq`
- Modify: `crates/edgecrab-tools/src/tools/web/search/http.rs` — hapus fungsi build_chrome_client yg pake wreq
- Modify: `crates/edgecrab-tools/src/tools/web/extract_crawl.rs` — ganti `build_chrome_client` pake reqwest
- Delete: `crates/edgecrab-tools/src/tools/web/search/backends/ddgs/` — seluruh folder (fingerprint.rs, mod.rs, dll)

**Step 1: Hapus wreq dari workspace Cargo.toml**

```
Cargo.toml:95 — hapus entire line: wreq = { version = "5", default-features = false, ... }
```

Juga hapus komentar baris 91-94 (tentang BoringSSL).

**Step 2: Hapus wreq dari edgecrab-tools Cargo.toml**

```
crates/edgecrab-tools/Cargo.toml:41 — hapus "wreq = { workspace = true }"
```

**Step 3: Hapus folder ddgs backend**
```
rm -rf crates/edgecrab-tools/src/tools/web/search/backends/ddgs/
```

Buka `crates/edgecrab-tools/src/tools/web/search/backends/mod.rs`, hapus baris `mod ddgs;` dan semua referensi ke ddgs.

**Step 4: Refactor http.rs — ganti build_chrome_client pake reqwest**

Di `crates/edgecrab-tools/src/tools/web/search/http.rs`:
- Hapus import `backends::ddgs::*`
- Ganti fungsi `build_chrome_client(timeout_secs) → Result<wreq::Client>` dengan:
  ```rust
  pub fn build_chrome_client(timeout_secs: u64) -> Result<reqwest::Client, SearchError> {
      build_api_client(timeout_secs)
  }
  ```
- Ganti `build_chrome_client_with_headers(...)` sama — cukup panggil `build_api_client`
- Hapus baris `pub fn validate_url_legacy` kalau cuma dipake extract_crawl (cek dulu)

**Step 5: Refactor extract_crawl.rs — ganti wreq::Client → reqwest::Client**

Di `crates/edgecrab-tools/src/tools/web/extract_crawl.rs`:
- Baris 1782-1793: fungsi `build_chrome_client` return type ganti dari `wreq::Client` ke `reqwest::Client`
- Ganti implementasi panggil `crate::tools::web::search::http::build_api_client(15)`
- Update semua import: `use wreq::...` → hapus, tambah `use reqwest::Client` dan `crate::tools::web::search::http::build_api_client`
- Cek semua referensi `wreq::header::CONTENT_TYPE` (line 1665) — ganti jadi `reqwest::header::CONTENT_TYPE`
- Cek `wreq::Proxy` — reqwest punya fitur `socks` yg sama

**Step 6: Hapus wreq dari Cargo.lock**

Jalankan:
```bash
cargo update -p wreq 2>/dev/null || cargo generate-lockfile
```

Atau kalo cargo gak available, jalankan clean install:
```bash
cargo fetch
```

**Step 7: Update search backend mod.rs**

Buka `crates/edgecrab-tools/src/tools/web/search/backends/mod.rs`, hapus modul `ddgs` dan semua enum variant/routing yang referensi DuckDuckGo.

**Verifikasi:**
```bash
# Compile check Linux dulu (masih bisa)
cargo check -p edgecrab-cli --no-default-features 2>&1 | grep -i error
```
Expected: 0 errors.

---

## Task 2: Buang AWS Bedrock feature (aws-lc-sys)

**Objective:** Hapus fitur `bedrock` dari default features, karena narik `aws-sdk-bedrock` → `aws-lc-rs` → `aws-lc-sys` (cmake + golang).

**Files:**
- Modify: `Cargo.toml` workspace — hapus `features = ["bedrock"]` dari edgequake-llm dep
- Modify: `crates/edgecrab-core/Cargo.toml` — ganti `default = ["bedrock-model-discovery"]` jadi `default = []`
- Modify: `crates/edgecrab-cli/Cargo.toml` — ganti `default = ["bedrock-model-discovery"]` jadi `default = []`

**Step 1: Workspace Cargo.toml**

Baris 73:
```toml
# Before:
edgequake-llm = { version = "0.6.25", features = ["bedrock"] }
# After:
edgequake-llm = "0.6.25"
```

**Step 2: edgecrab-core Cargo.toml**

Baris 10-11:
```toml
# Before:
default = ["bedrock-model-discovery"]
bedrock-model-discovery = ["dep:aws-config", "dep:aws-sdk-bedrock"]
# After:
default = []
# Keep the feature definition so it still exists for those who want it:
bedrock-model-discovery = ["dep:aws-config", "dep:aws-sdk-bedrock"]
```

**Step 3: edgecrab-cli Cargo.toml**

Baris 15-16:
```toml
# Before:
default = ["bedrock-model-discovery"]
bedrock-model-discovery = ["edgecrab-core/bedrock-model-discovery"]
# After:
default = []
bedrock-model-discovery = ["edgecrab-core/bedrock-model-discovery"]
```

**Step 4: Cek apakah ada conditional code yang perlu di-fix**

Search untuk `#[cfg(feature = "bedrock-model-discovery")]` di codebase — pastikan semua kode di-gate dengan feature flag. Kalo ada yang tidak di-gate, tambahin `#[cfg]`.

```bash
grep -r "bedrock" crates/ --include="*.rs" -l
```

**Verifikasi:**
```bash
cargo check -p edgecrab-cli --no-default-features
```
Expected: 0 errors, tidak ada aws-lc-sys di compile log.

---

## Task 3: Buang AV1/rav1e dari transitive deps

**Objective:** Hentikan `image` crate narik `ravif` → `rav1e` (nasm). Dua sumber: `arboard` (clipboard) dan `edgeparse-core` (PDF/image parsing).

**Files:**
- Modify: `crates/edgecrab-cli/Cargo.toml` — ganti `arboard = "3"` jadi optional + conditional compile
- Create/Modify: `crates/edgecrab-cli/src/clipboard.rs` — clipboard stub untuk non-Windows + minimal arboard config
- OR: Fork `arboard` secara lokal

**Opsi A — Minimal (recommended): Conditional clipboard**

Ganti `arboard` di `crates/edgecrab-cli/Cargo.toml`:
```toml
# Sebelum:
arboard = "3"
# Sesudah:
[target.'cfg(not(target_os = "windows"))'.dependencies]
arboard = "3"
```

Ini artinya clipboard cuma dikompil di Linux/macOS. Di Windows, clipboard gak aktif.
Tapi kita tetap perlu handle compile error — cek kode yg pake `use arboard::...` dan tambah `#[cfg(not(target_os = "windows"))]`.

**Opsi B — Fork arboard (lebih proper tapi lebih besar)**

1. Buat `vendor/arboard-fork/`
2. Copy source arboard, patch Cargo.toml: `image = { version = "0.25", default-features = false, features = ["png"] }`
3. Di `Cargo.toml` workspace, tambah:
```toml
[patch.crates-io]
arboard = { path = "vendor/arboard-fork" }
```

**Untuk edgeparse-core:**

Cek apakah edgeparse-core pake fitur image yg benar-benar butuh binary:
```bash
grep -r "ravif\|avif\|av1" Cargo.lock | head -5
# Jika hanya di lock sebagai transitive, kita butuh patch crates-io
```

Opsi:
1. Fork edgeparse-core di vendor/ — patch image dep
2. Atau buat wrapper tipis — edgecrab gak perlu image parsing yg AV1

**Untuk plan ini, rekomendasi: Opsi A (clipboard non-Windows) + cari test image untuk edgeparse-core.**

**Verifikasi:**
```bash
cargo check -p edgecrab-cli --no-default-features 2>&1 | grep -c rav1e
```
Expected: 0 (tidak ada referensi rav1e).

---

## Task 4: rusqlite — retain + konfigurasi C compiler

**Objective:** rusqlite bundled tetap dipakai. Untuk cross-compile ke Windows dari Docker, kita perlu `gcc-mingw-w64` sebagai C compiler.

**Files:**
- No change to Cargo.toml — rusqlite bundled tetap
- Modify: Dockerfile — tambah mingw-w64 toolchain

**Step 1: Verifikasi bahwa edgecrab-state + edgecrab-migrate pake bundled**

Sudah: di Cargo.toml workspace:
```toml
rusqlite = { version = "0.32", features = ["bundled", "functions"] }
```

Tidak perlu diubah.

---

## Task 5: Docker cross-compile setup

**Objective:** Tambah cross-compile stage ke Dockerfile untuk target `x86_64-pc-windows-gnu`.

**Files:**
- Modify: Dockerfile — tambah stage cross-compile
- Create: `.cargo/config.toml` project-level — tambah target linker config

**Step 1: Dockerfile — Stage cross-compile baru**

```dockerfile
# ─── Stage 1b: Cross-compile for Windows ─────────────────────────────────────
FROM rust:1-slim-bookworm AS builder-win

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    git \
    make \
    # C compiler for rusqlite bundled (the ONE remaining native dep)
    gcc-mingw-w64-x86-64 \
    # Required for the target
    libc6-dev-i386 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-pc-windows-gnu

WORKDIR /build

# Same copy pattern as Stage 1
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY skills/ skills/
COPY sdks/ sdks/

# Build with --no-default-features to skip Bedrock
RUN cargo build --release \
    --target x86_64-pc-windows-gnu \
    --no-default-features \
    -p edgecrab-cli

# ─── Windows runtime stage ──────────────────────────────────────────────────
# For native Windows, just use the .exe from Stage 1b
# Output: target/x86_64-pc-windows-gnu/release/edgecrab.exe
```

**Step 2: Project-level cargo config untuk linker**

Buat `.cargo/config.toml`:
```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

[target.x86_64-pc-windows-msvc]
# Default MSVC linker — only if VS Build Tools available
```

---

## Task 6: Cargo.lock cleanup + fresh resolve

**Objective:** Regenerate Cargo.lock tanpa wreq, aws-lc-sys, rav1e.

```bash
# Hapus Cargo.lock yang ada
rm Cargo.lock

# Resolve ulang tanpa default features
cargo generate-lockfile --no-default-features -p edgecrab-cli

# Verifikasi dependency tree
cargo tree -p edgecrab-cli --no-default-features 2>&1 | grep -E "(rav1e|boring|aws-lc|wreq)"
```
Expected: empty output (tidak ada match).

---

## Task 7: Build + verify Windows binary

**Step 1: Build Docker image**
```bash
docker buildx build \
  --target builder-win \
  --output type=local,dest=./dist/ \
  --platform linux/amd64 \
  .
```

Atau langsung:
```bash
docker build -f Dockerfile.win -t edgecrab-win .
docker create --name tmp edgecrab-win
docker cp tmp:/build/target/x86_64-pc-windows-gnu/release/edgecrab.exe ./dist/
docker rm tmp
```

**Step 2: Verifikasi binary**
```bash
file dist/edgecrab.exe
# Expected: PE32+ executable (console) x86-64, for MS Windows

# cek dependency Windows
x86_64-w64-mingw32-objdump -p dist/edgecrab.exe | grep "DLL Name"
# Expected: kernel32.dll, ntdll.dll, ws2_32.dll, userenv.dll, bcrypt.dll
# TIDAK expected: libssl-3.dll, libcrypto-3.dll (OpenSSL)
```

**Step 3: Test di Windows native**
```bash
# Copy ke Windows, jalankan
edgecrab.exe --help
edgecrab.exe --version
edgecrab.exe gateway start --foreground
```

---

## Expected Outcome

| Sebelum | Sesudah |
|---|---|
| 4 native toolchains (cmake + nasm + clang + golang) | 1 native toolchain (C compiler via mingw-w64) |
| wreq (BoringSSL TLS) | reqwest (rustls TLS) |
| AWS Bedrock default ON | Bedrock masi ada sbg optional feature |
| rav1e + 5 format decoder lain | PNG + JPEG only |
| Cross-compile gagal 4x | Cross-compile berhasil |
| Binary ~49MB | Binary ~40MB (estimasi) |

## Risk Register

| Risiko | Mitigasi |
|---|---|
| `extract_crawl.rs` pake wreq utk bypass bot-detection, ganti reqwest kena block | Test dengan user-agent Chrome yg uptodate; fallback ke error message explicit |
| `arboard` gak ada di Windows → clipboard tool error | Conditional compile; Windows user tinggal ada error message "clipboard not supported on Windows" |
| `edgeparse-core` tetap narik rav1e via image | Fork crate atau `[patch.crates-io]`; alternatif parse PDF pake library lain |
| mingw-w64 linker error (MSYS2 path) | Set `MSYS2_ARG_CONV_EXCL=*` env var; pakai absolute paths |
| rusqlite bundled gagal cross-compile | Sudah proven pattern — crate ini mature dan stable |

## Execution Order

1. ✅ **Task 2** dulu — Bedrock feature (paling mudah, 3 baris, risiko rendah)
2. ✅ **Task 1** — wreq/DuckDuckGo (medium, butuh refactor kode)
3. ✅ **Task 3** — rav1e/arboard (conditional compile)
4. ✅ **Task 4** — rusqlite (retain, C compiler toolchain)
5. ✅ **Task 5** — Dockerfile cross-compile
6. ✅ **Task 6** — Cargo.lock fresh resolve
7. ✅ **Task 7** — Build + verify

Setiap task independent — bisa di-paralel (1&2 bisa bersamaan, 3 bisa sendiri).
