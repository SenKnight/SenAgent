//! 构建脚本：确保内嵌前端目录存在。
//!
//! `rust-embed` 在编译期读取 `frontend/dist`，目录缺失会直接编译失败。
//! 这里在缺失时写入一个占位页，保证裸 `cargo build` 依然可用；
//! 完整 Web UI 需先在 `frontend/` 执行 `npm run build`。

use std::path::PathBuf;

const PLACEHOLDER: &str = r#"<!doctype html>
<html lang="zh-CN">
<head><meta charset="utf-8"><title>SenAgent</title></head>
<body style="font-family:system-ui;max-width:40rem;margin:4rem auto;line-height:1.7">
<h1>前端资源未构建</h1>
<p>当前二进制内嵌的是占位页，请构建完整 Web UI 后重新编译 CLI：</p>
<pre style="background:#f4f4f4;padding:1rem;border-radius:6px">cd frontend &amp;&amp; npm install &amp;&amp; npm run build</pre>
<p>然后重新安装：<code>cargo install --path crates/sen-cli</code></p>
</body>
</html>
"#;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dist = manifest.join("../../frontend/dist");
    let index = dist.join("index.html");
    if !index.is_file() {
        let _ = std::fs::create_dir_all(&dist);
        if std::fs::write(&index, PLACEHOLDER).is_ok() {
            println!(
                "cargo:warning=frontend/dist 缺失，已生成占位页；完整 Web UI 请先 cd frontend && npm run build"
            );
        }
    }
    // 前端产物变化时重新编译（rust-embed 自身也会追踪，双保险）
    println!("cargo:rerun-if-changed=../../frontend/dist");
}
