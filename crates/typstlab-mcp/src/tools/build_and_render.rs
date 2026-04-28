use base64::{Engine as _, engine::general_purpose::STANDARD};
use rmcp::model::{CallToolResult, Content};
use typstlab_app::{AppContext, BuildAction, BuildFormat};
use typstlab_proto::{Action, Entity};

pub fn execute(ctx: AppContext, paper_id: String) -> Result<CallToolResult, String> {
    use typstlab_base::driver::TypstDriver;
    let driver = TypstDriver::new(ctx.typst.path());
    let format = BuildFormat {
        pdf: false,
        png: true,
        svg: false,
        html: false,
    };
    let action = BuildAction::new(
        ctx.loaded_project,
        driver,
        Some(vec![paper_id.clone()]),
        format,
    );
    let results = action.run(&mut |_| {}, &mut |_| {}).map_err(|errors| {
        let err_msgs: Vec<_> = errors.into_iter().map(|e| e.to_string()).collect();
        format!("Build failed:\n{}", err_msgs.join("\n"))
    })?;

    let target_dist = results
        .into_iter()
        .find(|d| d.paper_id == paper_id)
        .ok_or_else(|| format!("Paper '{}' not found in build results.", paper_id))?;

    let png_paths = target_dist.png.unwrap_or_default();
    format_png_response(&paper_id, &png_paths)
}

/// png_paths と paper_id からレスポンスを整形する純粋な関数
pub(crate) fn format_png_response(
    paper_id: &str,
    png_paths: &[std::path::PathBuf],
) -> Result<CallToolResult, String> {
    if png_paths.is_empty() {
        return Err("No PNG images were generated during the build process.".to_string());
    }

    let mut contents = Vec::new();
    contents.push(Content::text(format!(
        "Successfully built paper '{}' ({} pages).",
        paper_id,
        png_paths.len()
    )));

    for path in png_paths {
        let image_data = std::fs::read(path)
            .map_err(|e| format!("Failed to read image file {}: {}", path.display(), e))?;
        let base64_data = STANDARD.encode(&image_data);
        contents.push(Content::image(base64_data, "image/png"));
    }

    Ok(CallToolResult::success(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    // --- format_png_response のユニットテスト ---

    #[test]
    fn test_format_png_response_empty_returns_error() {
        let result = format_png_response("p01", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No PNG images were generated"));
    }

    #[test]
    fn test_format_png_response_single_page_output_format() {
        let temp = TempDir::new().unwrap();
        let png_path = temp.path().join("1.png");
        let fake_png_bytes = b"\x89PNG\r\n\x1a\n"; // 偽の PNG バイト（中身は問わない）
        std::fs::write(&png_path, fake_png_bytes).unwrap();

        let result = format_png_response("my-paper", &[png_path]);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());

        let tool_result = result.unwrap();
        let actual = serde_json::to_value(&tool_result).unwrap();

        let expected_base64 = base64::engine::general_purpose::STANDARD.encode(fake_png_bytes);

        // MCP の実際の出力フォーマットに近い形で検証
        assert_eq!(
            actual["content"][0],
            json!({"type": "text", "text": "Successfully built paper 'my-paper' (1 pages)."}),
        );
        assert_eq!(
            actual["content"][1],
            json!({"type": "image", "data": expected_base64, "mimeType": "image/png"}),
        );
        assert_eq!(actual["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_format_png_response_multi_page_output_format() {
        let temp = TempDir::new().unwrap();
        let paths: Vec<_> = (1..=3)
            .map(|i| {
                let p = temp.path().join(format!("{}.png", i));
                std::fs::write(&p, b"\x89PNG\r\n\x1a\n").unwrap();
                p
            })
            .collect();

        let result = format_png_response("demo", &paths);
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        let actual = serde_json::to_value(&tool_result).unwrap();

        // text(1件) + image(3件) = 4件
        assert_eq!(
            actual["content"][0],
            json!({"type": "text", "text": "Successfully built paper 'demo' (3 pages)."}),
        );
        assert_eq!(actual["content"].as_array().unwrap().len(), 4);
        // 全ての画像コンテンツが image/png であること
        for i in 1..=3 {
            assert_eq!(actual["content"][i]["type"], json!("image"));
            assert_eq!(actual["content"][i]["mimeType"], json!("image/png"));
        }
    }
}
