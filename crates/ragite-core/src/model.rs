use std::sync::OnceLock;

use model2vec_rs::model::StaticModel;

use crate::{DIMENSION, MODEL_NAME, TECHNICAL_ID};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Query,
    Passage,
}

pub struct BuiltinModel {
    model: StaticModel,
    query_prefix: String,
    passage_prefix: String,
    dimension: usize,
}

static BUILTIN: OnceLock<Result<BuiltinModel, String>> = OnceLock::new();

pub fn builtin_model() -> Result<&'static BuiltinModel, String> {
    BUILTIN
        .get_or_init(BuiltinModel::load)
        .as_ref()
        .map_err(|err| err.clone())
}

impl BuiltinModel {
    fn load() -> Result<Self, String> {
        #[cfg(not(feature = "builtin-model"))]
        {
            return Err("this build has no built-in model".to_string());
        }
        #[cfg(feature = "builtin-model")]
        {
            let model = StaticModel::from_bytes(
                include_bytes!("../../../models/potion-base-8M/tokenizer.json").as_slice(),
                include_bytes!("../../../models/potion-base-8M/model.safetensors").as_slice(),
                include_bytes!("../../../models/potion-base-8M/config.json").as_slice(),
                None,
            )
            .map_err(|err| format!("built-in model failed to load: {err}"))?;
            let probe = model.encode_single("ragite");
            if probe.len() != DIMENSION {
                return Err(format!(
                    "built-in model dimension is {}, expected {DIMENSION}",
                    probe.len()
                ));
            }
            Ok(Self {
                model,
                query_prefix: String::new(),
                passage_prefix: String::new(),
                dimension: probe.len(),
            })
        }
    }

    pub fn name(&self) -> &'static str {
        MODEL_NAME
    }

    pub fn technical_id(&self) -> &'static str {
        TECHNICAL_ID
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn embed(&self, text: &str, role: Role) -> Vec<f32> {
        let prefix = match role {
            Role::Query => self.query_prefix.as_str(),
            Role::Passage => self.passage_prefix.as_str(),
        };
        if prefix.is_empty() {
            self.model.encode_single(text)
        } else {
            self.model.encode_single(&format!("{prefix}{text}"))
        }
    }

    pub fn embed_blob(&self, text: &str, role: Role) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.dimension * 4);
        for value in self.embed(text, role) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_model_embeds_on_cpu() {
        let model = builtin_model().expect("model");
        assert_eq!(model.name(), "potion-base-8M");
        assert_eq!(
            model.technical_id(),
            "potion-base-8M@bf8b056651a2c21b8d2565580b8569da283cab23"
        );
        let vector = model.embed("hello", Role::Passage);
        assert_eq!(vector.len(), DIMENSION);
        assert!(vector.iter().all(|v| v.is_finite()));
        assert_eq!(vector, model.embed("hello", Role::Query));
        assert_eq!(model.embed_blob("hello", Role::Passage).len(), DIMENSION * 4);
    }
}
