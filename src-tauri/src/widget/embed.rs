use serde::{Deserialize, Serialize};

/// Configuration for an embeddable AgentOS widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    /// API key used to authenticate the embedded widget
    pub api_key: String,
    /// URL of the AgentOS agent backend
    pub agent_url: String,
    /// Optional persona name to use for the widget
    pub persona: Option<String>,
    /// Theme: "light" or "dark"
    pub theme: String,
    /// Position on the page: "bottom-right", "bottom-left", "top-right", "top-left"
    pub position: String,
    /// Welcome message shown when widget opens
    pub welcome_message: String,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            agent_url: "http://localhost:8080".to_string(),
            persona: None,
            theme: "light".to_string(),
            position: "bottom-right".to_string(),
            welcome_message: "Hello! How can I help you today?".to_string(),
        }
    }
}

/// Generates embeddable HTML/JS snippets and iframe URLs for the AgentOS widget.
pub struct EmbedGenerator;

impl EmbedGenerator {
    /// Generate an HTML/JS snippet that can be pasted into any web page to embed the widget.
    pub fn generate_snippet(config: &WidgetConfig) -> String {
        let persona_attr = match &config.persona {
            Some(p) => format!("\n      data-persona=\"{}\"", p),
            None => String::new(),
        };

        format!(
            r#"<!-- AgentOS Embeddable Widget -->
<div id="agentos-widget"
     data-api-key="{api_key}"
     data-agent-url="{agent_url}"{persona}
     data-theme="{theme}"
     data-position="{position}"
     data-welcome="{welcome}">
</div>
<script>
(function() {{
  var w = document.getElementById('agentos-widget');
  var cfg = {{
    apiKey: w.getAttribute('data-api-key'),
    agentUrl: w.getAttribute('data-agent-url'),
    persona: w.getAttribute('data-persona') || null,
    theme: w.getAttribute('data-theme') || 'light',
    position: w.getAttribute('data-position') || 'bottom-right',
    welcome: w.getAttribute('data-welcome') || 'Hello!'
  }};
  var iframe = document.createElement('iframe');
  iframe.src = cfg.agentUrl + '/widget?api_key=' + encodeURIComponent(cfg.apiKey)
    + '&theme=' + cfg.theme + '&position=' + cfg.position
    + '&welcome=' + encodeURIComponent(cfg.welcome)
    + (cfg.persona ? '&persona=' + encodeURIComponent(cfg.persona) : '');
  iframe.style.cssText = 'border:none;width:380px;height:520px;position:fixed;'
    + (cfg.position.includes('bottom') ? 'bottom:20px;' : 'top:20px;')
    + (cfg.position.includes('right') ? 'right:20px;' : 'left:20px;')
    + 'z-index:99999;border-radius:12px;box-shadow:0 4px 24px rgba(0,0,0,0.15);';
  w.appendChild(iframe);
}})();
</script>"#,
            api_key = config.api_key,
            agent_url = config.agent_url,
            persona = persona_attr,
            theme = config.theme,
            position = config.position,
            welcome = config.welcome_message,
        )
    }

    /// Generate a standalone iframe URL for the widget.
    pub fn generate_iframe_url(config: &WidgetConfig) -> String {
        let mut url = format!(
            "{}/widget?api_key={}&theme={}&position={}&welcome={}",
            config.agent_url,
            urlencoded(&config.api_key),
            urlencoded(&config.theme),
            urlencoded(&config.position),
            urlencoded(&config.welcome_message),
        );
        if let Some(ref persona) = config.persona {
            url.push_str(&format!("&persona={}", urlencoded(persona)));
        }
        url
    }
}

/// Simple percent-encoding for URL query values.
fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_snippet() {
        let config = WidgetConfig {
            api_key: "test-key-123".to_string(),
            agent_url: "http://localhost:8080".to_string(),
            persona: Some("helper".to_string()),
            theme: "dark".to_string(),
            position: "bottom-right".to_string(),
            welcome_message: "Hi there!".to_string(),
        };
        let snippet = EmbedGenerator::generate_snippet(&config);
        assert!(snippet.contains("test-key-123"));
        assert!(snippet.contains("data-theme=\"dark\""));
        assert!(snippet.contains("data-persona=\"helper\""));
    }

    #[test]
    fn test_generate_iframe_url() {
        let config = WidgetConfig {
            api_key: "key-abc".to_string(),
            agent_url: "https://agent.example.com".to_string(),
            persona: None,
            theme: "light".to_string(),
            position: "bottom-left".to_string(),
            welcome_message: "Welcome".to_string(),
        };
        let url = EmbedGenerator::generate_iframe_url(&config);
        assert!(url.starts_with("https://agent.example.com/widget?"));
        assert!(url.contains("api_key=key-abc"));
        assert!(!url.contains("persona="));
    }
}
