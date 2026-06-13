//! CLI-only proxy output (`edgecrab proxy` help). Status/doctor/client text lives in `proxy_hub`.

pub fn print_help() {
    println!(
        "edgecrab proxy — OpenAI-compatible local bridge for Aider, Cline, OpenAI SDK\n\
         \n\
         Quick start (Grok / xAI OAuth):\n\
           edgecrab proxy setup grok\n\
           edgecrab proxy start --provider xai\n\
         \n\
         Commands:\n\
           setup [grok|nous|xai]   guided config + token + client snippet\n\
           enable <grok|nous|xai>  add upstream to ~/.edgecrab/config.yaml\n\
           doctor                  preflight checks\n\
           client                  print env vars for OpenAI clients\n\
           start [--provider KEY]  run server (Ctrl+C to stop)\n\
           status                  config summary\n\
           upstreams               list forward providers (alias: providers)\n\
           token set|show|rotate   local client bearer (not provider OAuth)\n\
         \n\
         In TUI: /proxy  (setup wizard for Grok / Nous)\n\
         \n\
         Clients use http://127.0.0.1:11434/v1 + proxy token; the proxy attaches real upstream auth."
    );
}
