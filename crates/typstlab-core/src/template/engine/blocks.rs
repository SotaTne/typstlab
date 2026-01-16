//! Block matching utilities (e.g., finding closing tags for loops)

use super::tokenize::{TokenKind, TokenStream};

/// Find matching closing tag for a block (e.g., "each" → "/each")
///
/// Returns (position, length) of the closing tag token.
///
/// # Nesting
///
/// Respects nested blocks of the same type and escape sequences.
///
/// # Performance
///
/// Uses TokenStream for true O(n) performance - single pass over the input.
pub(crate) fn find_block_end(
    text: &str,
    start_keyword: &str,
    end_keyword: &str,
) -> Option<(usize, usize)> {
    let tokens = TokenStream::new(text);
    let mut depth = 0;

    // Strip '/' prefix from end_keyword (e.g., "/each" → "each")
    // TokenKind::BlockEnd stores keyword without '/'
    let end_keyword_stripped = end_keyword.strip_prefix('/').unwrap_or(end_keyword);

    for token in tokens {
        // Skip escaped tokens
        if token.is_escaped() {
            continue;
        }

        // Pattern match on TokenKind for clean, type-safe parsing
        match &token.kind {
            TokenKind::BlockStart { keyword, .. } if keyword.as_str() == start_keyword.trim() => {
                // Found nested start tag, increase depth
                depth += 1;
            }
            TokenKind::BlockEnd { keyword } if keyword.as_str() == end_keyword_stripped => {
                if depth == 0 {
                    // Found matching closing tag at depth 0
                    return Some((token.start, token.length));
                } else {
                    // This is closing a nested block, decrease depth
                    depth -= 1;
                }
            }
            _ => {
                // Other tokens (placeholders, unrelated blocks) - continue scanning
            }
        }
    }

    // No matching closing tag found
    None
}

/// Find matching {{/each}} considering nested loops and escape sequences
///
/// Returns (position, length) of the closing {{/each}}.
/// Respects backslash escaping: \{{/each}} is treated as literal, not a closing tag.
///
/// This is a thin wrapper around `find_block_end()`.
pub(crate) fn find_each_end(text: &str) -> Option<(usize, usize)> {
    find_block_end(text, "each ", "/each")
}
