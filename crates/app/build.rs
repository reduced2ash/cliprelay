fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=GST_PLUGIN_PATH");
    println!("cargo:rerun-if-env-changed=GST_PLUGIN_SYSTEM_PATH");

    let Ok(out_dir) = std::env::var("OUT_DIR") else { return; };

    let mut src_dirs = Vec::new();
    if let Ok(output) = std::process::Command::new("pkg-config")
        .args(["--variable=pluginsdir", "gstreamer-1.0"])
        .output()
    {
        if output.status.success() {
            let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !p.is_empty() {
                src_dirs.push(p);
            }
        }
    }
    for p in [
        "/usr/lib/gstreamer-1.0",
        "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
        "/tmp/gst_extract/usr/lib/gstreamer-1.0",
    ] {
        if std::path::Path::new(p).exists() {
            src_dirs.push(p.to_string());
        }
    }
    src_dirs.sort();
    src_dirs.dedup();
    if src_dirs.is_empty() {
        println!("cargo:warning=GStreamer pluginsdir not found, skipping bundle");
        return;
    }

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_debug = manifest_dir.join("../../target/debug/gst-plugins");
    let target_release = manifest_dir.join("../../target/release/gst-plugins");
    for dst in [&target_debug, &target_release] {
        let mut ok = false;
        for src in &src_dirs {
            let src_path = std::path::Path::new(src);
            match bundle_plugins(src_path, dst) {
                Ok(n) if n > 0 => ok = true,
                Ok(_) => {},
                Err(e) => println!("cargo:warning=Failed to bundle {} to {}: {}", src_path.display(), dst.display(), e),
            }
        }
        if ok {
            println!("cargo:warning=Bundled GStreamer plugins to {}", dst.display());
        }
    }

    let out_plugins = std::path::Path::new(&out_dir).join("gst-plugins");
    for src in &src_dirs {
        let _ = bundle_plugins(std::path::Path::new(src), &out_plugins);
    }
}

fn bundle_plugins(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(dst)?;
    let mut copied = 0usize;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let is_plugin = name.starts_with("libgst") && (name.ends_with(".so") || name.ends_with(".dylib") || name.ends_with(".dll"));
        if !is_plugin {
            continue;
        }
        let dst_path = dst.join(&file_name);
        if dst_path.exists() {
            if let (Ok(src_m), Ok(dst_m)) = (std::fs::metadata(&path), std::fs::metadata(&dst_path)) {
                if src_m.len() == dst_m.len() {
                    if let (Ok(s), Ok(d)) = (src_m.modified(), dst_m.modified()) {
                        if d >= s {
                            continue;
                        }
                    }
                }
            }
        }
        std::fs::copy(&path, &dst_path)?;
        copied += 1;
    }
    Ok(copied)
}
