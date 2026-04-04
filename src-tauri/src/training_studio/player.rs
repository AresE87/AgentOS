use super::pack::*;
use crate::brain::Gateway;
use crate::config::Settings;

pub struct TrainingPlayer;

impl TrainingPlayer {
    /// Execute a training pack on a given input — builds a few-shot enriched prompt
    /// from the pack's examples, workflow steps, and system prompt additions,
    /// then calls the LLM gateway for completion.
    pub async fn execute(
        pack: &TrainingPack,
        user_input: &str,
        gateway: &Gateway,
        settings: &Settings,
    ) -> Result<String, String> {
        // Build system prompt with training knowledge
        let mut system = format!(
            "You are trained to: {}\n\n{}\n\n",
            pack.description, pack.system_prompt_additions
        );

        // Add few-shot examples
        if !pack.examples.is_empty() {
            system.push_str("## Examples from training:\n\n");
            for (i, ex) in pack.examples.iter().enumerate().take(3) {
                system.push_str(&format!(
                    "Example {}:\nInput: {}\nOutput: {}\n\n",
                    i + 1,
                    ex.input,
                    ex.expected_output
                ));
            }
        }

        // Add workflow steps as guidance
        if !pack.workflow_steps.is_empty() {
            system.push_str("## Workflow steps:\n");
            for step in &pack.workflow_steps {
                system.push_str(&format!("{}. {}\n", step.order, step.description));
            }
        }

        // Call LLM with the enriched prompt
        let response = gateway
            .complete_with_system(user_input, Some(&system), settings)
            .await?;
        Ok(response.content)
    }
}
