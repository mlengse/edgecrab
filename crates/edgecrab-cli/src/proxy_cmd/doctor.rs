//! Preflight checks before `proxy start`.

use anyhow::Result;

use crate::proxy_cmd::context::ProxySession;
use crate::proxy_hub;

pub fn run_doctor() -> Result<()> {
    let session = ProxySession::load()?;
    let (_, text) = proxy_hub::format_doctor(&session);
    print!("{text}");
    Ok(())
}
