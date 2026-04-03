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
}
