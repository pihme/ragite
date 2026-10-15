use std::sync::OnceLock;

use model2vec_rs::model::StaticModel;

use crate::{MODEL_NAME, TECHNICAL_ID};

#[cfg(feature = "builtin-model")]
const QUERY_PREFIX: &str = "";
#[cfg(feature = "builtin-model")]
const PASSAGE_PREFIX: &str = "";

#[cfg(ragite_provider_model)]
const QUERY_PREFIX: &str = env!("RAGITE_COMPILED_QUERY_PREFIX");
#[cfg(ragite_provider_model)]
const PASSAGE_PREFIX: &str = env!("RAGITE_COMPILED_PASSAGE_PREFIX");

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
        #[cfg(all(not(feature = "builtin-model"), not(ragite_provider_model)))]
        {
            return Err("this build has no built-in model".to_string());
        }
        #[cfg(any(feature = "builtin-model", ragite_provider_model))]
        {
            let (tokenizer, weights, config) = weight_bytes();
            let model = StaticModel::from_bytes(tokenizer, weights, config, None)
                .map_err(|err| format!("built-in model failed to load: {err}"))?;
            let probe = model.encode_single("ragite");
            #[cfg(feature = "builtin-model")]
            if probe.len() != crate::DIMENSION {
                return Err(format!(
                    "built-in model dimension is {}, expected {}",
                    probe.len(),
                    crate::DIMENSION
                ));
            }
            if probe.is_empty() {
                return Err("built-in model dimension is 0".to_string());
            }
            Ok(Self {
                model,
                query_prefix: QUERY_PREFIX.to_string(),
                passage_prefix: PASSAGE_PREFIX.to_string(),
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

#[cfg(feature = "builtin-model")]
fn weight_bytes() -> (&'static [u8], &'static [u8], &'static [u8]) {
    (
        include_bytes!("../../../models/potion-retrieval-32M/tokenizer.json"),
        linked_weights(),
        include_bytes!("../../../models/potion-retrieval-32M/config.json"),
    )
}

#[cfg(ragite_provider_model)]
fn weight_bytes() -> (&'static [u8], &'static [u8], &'static [u8]) {
    (
        include_bytes!(env!("RAGITE_COMPILED_TOKENIZER")),
        linked_weights(),
        include_bytes!(env!("RAGITE_COMPILED_CONFIG")),
    )
}

// The weights are linked with the assembler's `.incbin` instead of `include_bytes!`.
// rustc holds an included file in memory several times over, which gets a 512 MB
// model OOM-killed on a 16 GB machine. build.rs writes ELF, Mach-O, or COFF syntax.
#[cfg(all(
    any(feature = "builtin-model", ragite_provider_model),
    not(all(target_os = "windows", target_arch = "x86_64"))
))]
std::arch::global_asm!(
    include_str!(concat!(env!("OUT_DIR"), "/ragite_weights.s")),
    options(raw)
);

// x86_64 Windows parses global_asm as Intel syntax unless asked for AT&T.
#[cfg(all(
    any(feature = "builtin-model", ragite_provider_model),
    target_os = "windows",
    target_arch = "x86_64"
))]
std::arch::global_asm!(
    include_str!(concat!(env!("OUT_DIR"), "/ragite_weights.s")),
    options(att_syntax, raw)
);

#[cfg(any(feature = "builtin-model", ragite_provider_model))]
fn linked_weights() -> &'static [u8] {
    unsafe extern "C" {
        static ragite_weights_start: u8;
        static ragite_weights_end: u8;
    }
    // SAFETY: both symbols bound one contiguous read-only blob emitted above.
    unsafe {
        let start = &raw const ragite_weights_start;
        let end = &raw const ragite_weights_end;
        std::slice::from_raw_parts(start, end.offset_from(start) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(not(feature = "builtin-model"), not(ragite_provider_model)))]
    #[test]
    fn tc_53_no_default_features_omits_the_embedded_file() {
        let err = builtin_model().err().expect("weights were embedded");
        assert!(err.contains("no built-in model"), "{err}");
    }

    #[cfg(ragite_provider_model)]
    #[test]
    fn tc_54_tc_55_tc_126_provider_prefixes() {
        let model = builtin_model().expect("provider model");
        assert_eq!(model.name(), "provider-demo");
        assert_eq!(model.technical_id(), "provider-demo@rev-supplied");
        let revision = model.technical_id().split_once('@').unwrap().1;
        assert_eq!(revision, "rev-supplied");
        assert_ne!(revision.len(), 40);

        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../models/potion-base-8M");
        let on_disk = model2vec_rs::model::StaticModel::from_bytes(
            std::fs::read(root.join("tokenizer.json")).unwrap(),
            std::fs::read(root.join("model.safetensors")).unwrap(),
            std::fs::read(root.join("config.json")).unwrap(),
            None,
        )
        .expect("weights on disk");
        assert_ne!(
            model.embed("hello", Role::Query),
            on_disk.encode_single("hello")
        );
        assert_eq!(
            model.embed("hello", Role::Query),
            on_disk.encode_single("query:hello")
        );
        assert_eq!(
            model.embed("hello", Role::Passage),
            on_disk.encode_single("passage:hello")
        );
        assert_eq!(model.dimension(), 256);
    }

    #[cfg(feature = "builtin-model")]
    #[test]
    fn tc_259_builtin_model_embeds_on_cpu() {
        let model = builtin_model().expect("model");
        assert_eq!(model.name(), "potion-retrieval-32M");
        assert_eq!(
            model.technical_id(),
            "potion-retrieval-32M@6fc8051fab2a1e0ee76689cf08c853792ac285e7"
        );
        let vector = model.embed("hello", Role::Passage);
        assert_eq!(model.dimension(), crate::DIMENSION);
        assert_eq!(model.dimension(), vector.len());
        assert_eq!(vector.len(), crate::DIMENSION);
        assert!(vector.iter().all(|v| v.is_finite()));
        assert_eq!(vector, model.embed("hello", Role::Query));
        assert_eq!(
            model.embed_blob("hello", Role::Passage).len(),
            crate::DIMENSION * 4
        );
    }
}
