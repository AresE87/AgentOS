use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an on-device AI model that can run without internet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDeviceModel {
    pub name: String,
    pub path: String,
    pub size_mb: u64,
    pub quantization: String,
    pub loaded: bool,
}

/// Status of the on-device model engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub available_models: usize,
    pub loaded_models: Vec<String>,
    pub total_size_mb: u64,
}

/// Engine managing on-device AI models (ONNX / llama.cpp stubs).
pub struct OnDeviceEngine {
    models: HashMap<String, OnDeviceModel>,
}

impl OnDeviceEngine {
    pub fn new() -> Self {
        let mut models = HashMap::new();

        // Pre-register available models (downloaded on-demand)
        models.insert(
            "distilbert-classifier".to_string(),
            OnDeviceModel {
                name: "distilbert-classifier".to_string(),
                path: "models/distilbert-classifier.onnx".to_string(),
                size_mb: 65,
                quantization: "fp16".to_string(),
                loaded: false,
            },
        );
        models.insert(
            "minilm-embeddings".to_string(),
            OnDeviceModel {
                name: "minilm-embeddings".to_string(),
                path: "models/all-MiniLM-L6-v2.onnx".to_string(),
                size_mb: 80,
                quantization: "fp16".to_string(),
                loaded: false,
            },
        );
        models.insert(
            "ppocr-v4".to_string(),
            OnDeviceModel {
                name: "ppocr-v4".to_string(),
                path: "models/ppocr-v4.onnx".to_string(),
                size_mb: 15,
                quantization: "fp32".to_string(),
                loaded: false,
            },
        );
        models.insert(
            "phi-3-mini".to_string(),
            OnDeviceModel {
                name: "phi-3-mini".to_string(),
                path: "models/phi-3-mini-q4.gguf".to_string(),
                size_mb: 2048,
                quantization: "Q4_K_M".to_string(),
                loaded: false,
            },
        );

        Self { models }
    }

    /// List all available on-device models.
    pub fn list_models(&self) -> Vec<OnDeviceModel> {
        self.models.values().cloned().collect()
    }

    /// Load a model into memory by name.
    pub fn load_model(&mut self, name: &str) -> Result<OnDeviceModel, String> {
        let model = self
            .models
            .get_mut(name)
            .ok_or_else(|| format!("Model '{}' not found", name))?;

        if model.loaded {
            return Err(format!("Model '{}' is already loaded", name));
        }

        // Stub: mark as loaded (real impl would load ONNX/GGUF into memory)
        model.loaded = true;
        Ok(model.clone())
    }

    /// Unload a model from memory.
    pub fn unload_model(&mut self, name: &str) -> Result<OnDeviceModel, String> {
        let model = self
            .models
            .get_mut(name)
            .ok_or_else(|| format!("Model '{}' not found", name))?;

        if !model.loaded {
            return Err(format!("Model '{}' is not loaded", name));
        }

        model.loaded = false;
        Ok(model.clone())
    }

    /// Run inference on a loaded model (stub).
    pub fn infer(&self, model_name: &str, prompt: &str) -> Result<String, String> {
        let model = self
            .models
            .get(model_name)
            .ok_or_else(|| format!("Model '{}' not found", model_name))?;

        if !model.loaded {
            return Err(format!(
                "Model '{}' is not loaded — call load_model first",
                model_name
            ));
        }

        // Stub response — real implementation would call ONNX Runtime or llama.cpp
        Ok(format!(
            "On-device inference not yet available (model={}, prompt_len={})",
            model_name,
            prompt.len()
        ))
    }

    /// Get current engine status.
    pub fn get_status(&self) -> ModelStatus {
        let loaded: Vec<String> = self
            .models
            .values()
            .filter(|m| m.loaded)
            .map(|m| m.name.clone())
            .collect();
        let total_size: u64 = self.models.values().map(|m| m.size_mb).sum();
        ModelStatus {
            available_models: self.models.len(),
            loaded_models: loaded,
            total_size_mb: total_size,
        }
    }
}
