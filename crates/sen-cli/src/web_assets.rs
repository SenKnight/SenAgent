//! 内嵌前端资源：把 `frontend/dist` 编译进二进制（`sen web` 单文件提供 Web UI）。

use rust_embed::Embed;
use sen_server::WebAssets;

#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../frontend/dist"]
struct FrontendDist;

/// 内嵌资源访问器（供 `sen-server` 的 [`WebAssets`] 使用）。
pub struct EmbeddedWeb;

impl WebAssets for EmbeddedWeb {
    fn get(&self, path: &str) -> Option<(Vec<u8>, String)> {
        let file = FrontendDist::get(path)?;
        Some((file.data.into_owned(), file.metadata.mimetype().to_string()))
    }
}

/// 内嵌资源是否为真实前端产物（vite 构建会生成 `assets/` 目录；占位页没有）。
pub fn is_built() -> bool {
    FrontendDist::iter().any(|f| f.starts_with("assets/"))
}
