use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../frontend/dist");

    let dist_dir = Path::new("../frontend/dist");
    if !dist_dir.exists() {
        let _ = fs::create_dir_all(dist_dir);
    }

    let index_file = dist_dir.join("index.html");
    if !index_file.exists() {
        let placeholder = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>Atlas Desktop Companion</title>
  </head>
  <body>
    <div id="root">
      <h1>Atlas Desktop</h1>
      <p>Frontend assets not built. Run <code>npm run build</code> in <code>atlas-desktop/frontend</code>.</p>
    </div>
  </body>
</html>"#;
        let _ = fs::write(&index_file, placeholder);
    }
}
