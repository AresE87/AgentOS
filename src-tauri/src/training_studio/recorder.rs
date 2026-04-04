use super::pack::*;

pub struct TrainingRecorder {
    active_recording: Option<RecordingSession>,
}

pub struct RecordingSession {
    pub pack: TrainingPack,
    pub current_example: Option<TrainingExample>,
    pub started_at: String,
}

impl TrainingRecorder {
    pub fn new() -> Self {
        Self {
            active_recording: None,
        }
    }

    pub fn start_recording(
        &mut self,
        title: &str,
        description: &str,
        category: &str,
        creator_id: &str,
        creator_name: &str,
    ) -> String {
        let pack = TrainingPack::new(title, description, category, creator_id, creator_name);
        let id = pack.id.clone();
        self.active_recording = Some(RecordingSession {
            pack,
            current_example: None,
            started_at: chrono::Utc::now().to_rfc3339(),
        });
        id
    }

    pub fn start_example(&mut self, input: &str) -> Result<(), String> {
        let session = self
            .active_recording
            .as_mut()
            .ok_or("No active recording")?;
        session.current_example = Some(TrainingExample {
            input: input.into(),
            expected_output: String::new(),
            tool_calls: vec![],
            corrections: vec![],
        });
        Ok(())
    }

    pub fn record_tool_call(
        &mut self,
        tool_name: &str,
        input: serde_json::Value,
        output: &str,
        success: bool,
    ) -> Result<(), String> {
        let session = self
            .active_recording
            .as_mut()
            .ok_or("No active recording")?;
        let example = session
            .current_example
            .as_mut()
            .ok_or("No active example")?;
        example.tool_calls.push(ToolCallCapture {
            tool_name: tool_name.into(),
            input,
            output: output.into(),
            success,
        });
        Ok(())
    }

    pub fn finish_example(&mut self, output: &str) -> Result<(), String> {
        let session = self
            .active_recording
            .as_mut()
            .ok_or("No active recording")?;
        let mut example = session
            .current_example
            .take()
            .ok_or("No active example")?;
        example.expected_output = output.into();
        session.pack.add_example(example);
        Ok(())
    }

    pub fn add_correction(&mut self, correction: &str) -> Result<(), String> {
        let session = self
            .active_recording
            .as_mut()
            .ok_or("No active recording")?;
        if let Some(example) = session.current_example.as_mut() {
            example.corrections.push(correction.into());
        } else if let Some(last) = session.pack.examples.last_mut() {
            last.corrections.push(correction.into());
        }
        Ok(())
    }

    pub fn stop_recording(&mut self) -> Result<TrainingPack, String> {
        let session = self.active_recording.take().ok_or("No active recording")?;
        Ok(session.pack)
    }

    pub fn is_recording(&self) -> bool {
        self.active_recording.is_some()
    }
}
