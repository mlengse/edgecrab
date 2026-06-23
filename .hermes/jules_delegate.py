import os, json, subprocess, sys, time

api_key = os.environ.get("JULES_API_KEY", "")
base_url = os.environ.get("JULES_BASE_URL", "https://jules.googleapis.com/v1alpha")

def create_session(prompt, title):
    """Create a repoless Jules session."""
    body = json.dumps({"prompt": prompt, "title": title, "requirePlanApproval": False})
    r = subprocess.run(
        ["curl", "-s", "-X", "POST", f"{base_url}/sessions",
         "-H", f"x-goog-api-key: {api_key}",
         "-H", "Content-Type: application/json",
         "-d", body],
        capture_output=True, text=True, timeout=30
    )
    try:
        data = json.loads(r.stdout)
        sid = data.get("name", "ERROR").split("/")[-1]
        state = data.get("state", "?")
        print(f"  {title}: session={sid} state={state}")
        return sid
    except:
        print(f"  {title}: FAILED - {r.stdout[:200]}")
        return None

# ── Task 2: Buang AWS Bedrock default ─────────────────────────────────
# (3 baris Cargo.toml — risk-free)
prompt_task2 = """Buka file Cargo.toml di Rust workspace berikut. Lakukan 3 perubahan:

REPO: github.com/raphaelmansuy/edgecrab
BRANCH: main

FILE 1: /Cargo.toml (workspace root)
- Cari baris: edgequake-llm = { version = "0.6.25", features = ["bedrock"] }
- Ganti jadi: edgequake-llm = "0.6.25"
- Hapus juga komentar sebelumnya tentang Bedrock jika ada

FILE 2: /crates/edgecrab-core/Cargo.toml
- Cari: default = ["bedrock-model-discovery"]
- Ganti jadi: default = []

FILE 3: /crates/edgecrab-cli/Cargo.toml
- Cari: default = ["bedrock-model-discovery"]
- Ganti jadi: default = []

Output: print full content dari ketiga file setelah diubah.
Tidak perlu compile — cukup print file contents."""
# ── Task 1: Buang wreq/DuckDuckGo ──────────────────────────────────────
prompt_task1 = """Saya punya Rust workspace (github.com/raphaelmansuy/edgecrab) yang perlu di-refactor untuk menghapus dependency wreq (HTTP client) dan DuckDuckGo search backend. Ini untuk Windows cross-compile — wreq narik BoringSSL (boring-sys2) yang butuh cmake + nasm + libclang.

FILE 1: /Cargo.toml (workspace root)
- Hapus baris: wreq = { version = "5", default-features = false, ... }
- Hapus komentar tentang BoringSSL / wreq (baris 91-94)

FILE 2: /crates/edgecrab-tools/Cargo.toml
- Hapus baris: wreq = { workspace = true }

FILE 3: /crates/edgecrab-tools/src/tools/web/search/http.rs
- Hapus import: use super::backends::ddgs::{ImpersonateProfile, build_wreq_client, pick_random_profile};
- Ganti fungsi build_chrome_client:
  BEFORE:
    pub fn build_chrome_client(timeout_secs: u64) -> Result<wreq::Client, SearchError> {
        build_chrome_client_with_headers(timeout_secs, None, None, None)
    }
  AFTER:
    pub fn build_chrome_client(timeout_secs: u64) -> Result<reqwest::Client, SearchError> {
        build_api_client(timeout_secs)
    }
- Ganti fungsi build_chrome_client_with_headers — return type dari wreq::Client ke reqwest::Client, implementasinya panggil build_api_client(timeout_secs)
- Hapus pub fn validate_url_legacy jika masih ada

FILE 4: /crates/edgecrab-tools/src/tools/web/extract_crawl.rs
- Cari fungsi build_chrome_client (sekitar line 1782-1793)
- Ganti return type dari Result<wreq::Client, ToolError> ke Result<reqwest::Client, ToolError>
- Ganti implementasi: crate::tools::web::search::http::build_api_client(15)
- Update semua import: use wreq::header::CONTENT_TYPE → use reqwest::header::CONTENT_TYPE
- Cari referensi wreq::Proxy — reqwest juga punya Proxy::all() dengan fitur socks

FILE 5: Hapus folder /crates/edgecrab-tools/src/tools/web/search/backends/ddgs/
- Hapus seluruh folder ddgs/ (fingerprint.rs dll)

FILE 6: /crates/edgecrab-tools/src/tools/web/search/backends/mod.rs
- Hapus baris: mod ddgs;
- Hapus semua referensi ke ddgs di enum/routing

Output: print full content dari semua file yang diubah.
Lalu jalankan: cargo check -p edgecrab-cli --no-default-features 2>&1
Print hasilnya."""

# ── Task 3: Buang AV1 (rav1e) via conditional clipboard ────────────────
prompt_task3 = """Saya punya Rust workspace (github.com/raphaelmansuy/edgecrab). Perlu menghapus rav1e (AV1 encoder) dari transitive dependencies. Rav1e masuk via arboard (clipboard) → image (default features) → ravif → rav1e.

FILE 1: /crates/edgecrab-cli/Cargo.toml
- Cari baris: arboard = "3"
- Hapus baris itu dari [dependencies]
- Tambah di bagian bawah file (sebelum [dev-dependencies]):
[target.'cfg(not(target_os = "windows"))'.dependencies]
arboard = "3"

FILE 2: Cari semua file Rust di crates/edgecrab-cli/src/ yang pake arboard
- grep -r "use arboard" atau grep -r "arboard::"
- Untuk setiap file, tambah #[cfg(not(target_os = "windows"))] sebelum baris use arboard::...
- Juga tambah cfg guard di semua fungsi yang panggil arboard

FILE 3: Cari crate edgeparse-core di Cargo.lock — cek apakah dia narik image dengan default features
- kalau edgeparse-core narik image tanpa default-features=false, kita perlu fork/patch
- TAPI untuk sekarang, cukup cek dan print: apa edgeparse-core tergantung pada image dengan fitur apa

Output: print:
1. Isi /crates/edgecrab-cli/Cargo.toml setelah diubah
2. Semua file Rust yang diubah, dengan perubahan cfg guard
3. Hasil grep dari Cargo.lock tentang image + rav1e
4. Hasil cargo check -p edgecrab-cli --no-default-features 2>&1"""

print("=== Creating 3 Jules sessions ===")
sid1 = create_session(prompt_task2, "EdgeCrab Trimmer - Task 2: Bedrock (3 lines)")
sid2 = create_session(prompt_task1, "EdgeCrab Trimmer - Task 1: wreq/DDGS refactor")
sid3 = create_session(prompt_task3, "EdgeCrab Trimmer - Task 3: rav1e/arboard")

print(f"\n=== Sessions created ===")
print(f"Task 2 (Bedrock): {sid1 or 'FAILED'}")
print(f"Task 1 (wreq/DDGS): {sid2 or 'FAILED'}")
print(f"Task 3 (rav1e): {sid3 or 'FAILED'}")
