use crate::brain::Gateway;
use crate::config::Settings;
use tracing::info;

/// Check if messages need compaction (over threshold tokens).
/// Uses rough estimate of 4 chars per token.
pub fn should_compact(messages: &[serde_json::Value], threshold_tokens: usize) -> bool {
    let total_chars: usize = messages
        .iter()
        .map(|m| serde_json::to_string(m).unwrap_or_default().len())
        .sum();
    let estimated_tokens = total_chars / 4;
    estimated_tokens > threshold_tokens
}

/// Compact old messages into a summary, preserving recent ones.
/// Calls the LLM (cheap tier) to produce a summary of older messages,
/// then returns [summary_message] + recent messages.
pub async fn compact_messages(
    messages: &[serde_json::Value],
    keep_recent: usize,
    gateway: &Gateway,
    settings: &Settings,
) -> Result<Vec<serde_json::Value>, String> {
    if messages.len() <= keep_recent + 1 {
        return Ok(messages.to_vec());
    }

    let to_summarize = &messages[..messages.len() - keep_recent];
    let to_keep = &messages[messages.len() - keep_recent..];

    // Build summary text from old messages (cap at 8000 chars to stay within cheap model limits)
    let summary_text = to_summarize
        .iter()
        .map(|m| serde_json::to_string(m).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    let truncated = &summary_text[..summary_text.len().min(8000)];

    let summary_prompt = format!(
        "Summarize this conversation history in 2-3 paragraphs, preserving key facts, decisions, and tool results:\n\n{}",
        truncated
    );

    let response = gateway
        .complete_cheap(&summary_prompt, settings)
        .await
        .unwrap_or_else(|_| crate::brain::LLMResponse {
            task_id: String::new(),
            content: "Previous conversation context was compacted.".to_string(),
            model: String::new(),
            provider: String::new(),
            tokens_in: 0,
            tokens_out: 0,
            cost: 0.0,
            duration_ms: 0,
        });

    info!(
        old_messages = to_summarize.len(),
        kept_messages = to_keep.len(),
        "context compaction applied"
    );

    let mut compacted = vec![serde_json::json!({
        "role": "user",
        "content": format!("[Context from previous conversation]\n{}", response.content)
    })];
    compacted.extend_from_slice(to_keep);

    Ok(compacted)
}
