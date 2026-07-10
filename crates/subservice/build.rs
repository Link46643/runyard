use std::fs;
use std::path::Path;

fn main() {
    // Ensure the embedded assets directory exists at compile time.
    // In dev, this will contain a stub page. In production, the Svelte frontend
    // build output will populate this directory before `cargo build`.
    let build_dir = Path::new("../../apps/desktop/build");
    if !build_dir.exists() {
        fs::create_dir_all(build_dir).expect("Failed to create apps/desktop/build/");
    }

    let index_html = build_dir.join("index.html");
    if !index_html.exists() {
        fs::write(
            &index_html,
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Runyard — Build Required</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: 'JetBrains Mono', monospace;
      background: #0d0e14;
      color: #c0c8d8;
      height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 16px;
    }
    h1 { color: #7c9ef8; font-size: 1.4rem; }
    p { opacity: 0.7; font-size: 0.9rem; }
    code {
      background: #1a1d27;
      border: 1px solid #2a2d3e;
      border-radius: 6px;
      padding: 12px 20px;
      font-size: 0.85rem;
      color: #a8d8a8;
    }
  </style>
</head>
<body>
  <h1>Runyard Sub-Service</h1>
  <p>The frontend has not been built yet. Run:</p>
  <code>pnpm --filter @runyard/desktop build</code>
  <p>Then restart the sub-service.</p>
</body>
</html>
"#,
        )
        .expect("Failed to write stub index.html");
    }

    // Re-run this build script if the build dir contents change
    println!("cargo:rerun-if-changed=../../apps/desktop/build");
}
