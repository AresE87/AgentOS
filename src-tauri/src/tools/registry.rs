use std::collections::HashMap;
use super::trait_def::{Tool, ToolDefinition};

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| ToolDefinition {
            name: t.name().to_string(),
            description: t.description().to_string(),
            input_schema: t.input_schema(),
        }).collect()
    }

    pub fn definitions_for_tools(&self, tool_names: &[String]) -> Vec<ToolDefinition> {
        tool_names.iter().filter_map(|name| {
            self.tools.get(name).map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
        }).collect()
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_is_empty() {
        let reg = ToolRegistry::new();
        assert_eq!(reg.len(), 0);
        assert!(reg.is_empty());
        assert!(reg.definitions().is_empty());
        assert!(reg.tool_names().is_empty());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let reg = ToolRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn definitions_for_missing_tools_returns_empty() {
        let reg = ToolRegistry::new();
        let defs = reg.definitions_for_tools(&["missing".to_string()]);
        assert!(defs.is_empty());
    }
}
