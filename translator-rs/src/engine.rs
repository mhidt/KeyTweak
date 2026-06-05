use ct2rs::tokenizers::sentencepiece::Tokenizer as SpTokenizer;
use ct2rs::{Config, TranslationOptions, Translator};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

use crate::protocol::{ErrorInfo, MAX_TEXT_LENGTH};
use crate::sbd;

#[derive(Debug, Deserialize)]
struct Metadata {
    from_code: String,
    to_code: String,
}

struct LoadedModel {
    translator: Translator<SpTokenizer>,
}

pub struct Engine {
    models: std::collections::HashMap<(String, String), LoadedModel>,
    languages: Vec<String>,
}

impl Engine {
    pub fn load(packages_dir: &Path) -> Result<Self, String> {
        if !packages_dir.exists() {
            return Err(format!(
                "ARGOS_PACKAGES_DIR does not exist: {}",
                packages_dir.display()
            ));
        }

        let mut models = std::collections::HashMap::new();
        let mut lang_codes = HashSet::new();
        let mut errors = Vec::new();

        let entries = std::fs::read_dir(packages_dir)
            .map_err(|e| format!("Failed to read packages dir: {e}"))?;

        for entry in entries.flatten() {
            let pkg_dir = entry.path();
            if !pkg_dir.is_dir() {
                continue;
            }

            let metadata_path = pkg_dir.join("metadata.json");
            if !metadata_path.exists() {
                continue;
            }

            let metadata_content = match std::fs::read_to_string(&metadata_path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(format!(
                        "Failed to read {}: {e}",
                        metadata_path.display()
                    ));
                    continue;
                }
            };

            let metadata: Metadata = match serde_json::from_str(&metadata_content) {
                Ok(m) => m,
                Err(e) => {
                    errors.push(format!(
                        "Failed to parse {}: {e}",
                        metadata_path.display()
                    ));
                    continue;
                }
            };

            let model_dir = pkg_dir.join("model");
            let sp_path = pkg_dir.join("sentencepiece.model");

            if !model_dir.exists() || !sp_path.exists() {
                errors.push(format!(
                    "Missing model/ or sentencepiece.model in {}",
                    pkg_dir.display()
                ));
                continue;
            }

            log::info!(
                "Loading model {}→{} from {}",
                metadata.from_code,
                metadata.to_code,
                pkg_dir.display()
            );

            let sp_path_str = sp_path.to_string_lossy().to_string();
            let tokenizer = match SpTokenizer::from_file(&sp_path_str, &sp_path_str) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(format!(
                        "Failed to load sentencepiece from {}: {e:?}",
                        sp_path.display()
                    ));
                    continue;
                }
            };

            let translator = match Translator::with_tokenizer(
                &model_dir,
                tokenizer,
                &Config::default(),
            ) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(format!(
                        "Failed to load ctranslate2 model from {}: {e:?}",
                        model_dir.display()
                    ));
                    continue;
                }
            };

            lang_codes.insert(metadata.from_code.clone());
            lang_codes.insert(metadata.to_code.clone());

            let key = (metadata.from_code.clone(), metadata.to_code.clone());
            log::info!("Model {}→{} loaded successfully", key.0, key.1);
            models.insert(key, LoadedModel { translator });
        }

        for err in &errors {
            log::warn!("Model load warning: {err}");
        }

        let mut languages: Vec<String> = lang_codes.into_iter().collect();
        languages.sort();

        log::info!(
            "Engine loaded: {} models, {} languages {:?}",
            models.len(),
            languages.len(),
            languages
        );

        Ok(Engine { models, languages })
    }

    pub fn languages(&self) -> &[String] {
        &self.languages
    }

    pub fn is_ready(&self) -> bool {
        let codes: HashSet<&str> = self.languages.iter().map(String::as_str).collect();
        codes.contains("en") && codes.contains("ru")
    }

    pub fn translate(
        &self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<String, ErrorInfo> {
        if text.len() > MAX_TEXT_LENGTH {
            return Err(ErrorInfo::text_too_long(text.len()));
        }

        let key = (source.to_string(), target.to_string());
        let model = self
            .models
            .get(&key)
            .ok_or_else(|| ErrorInfo::model_not_found(source, target))?;

        let sentences = sbd::split_sentences(text);
        if sentences.is_empty() {
            return Ok(String::new());
        }

        let options = TranslationOptions {
            beam_size: 1,
            length_penalty: 0.2,
            replace_unknowns: true,
            ..Default::default()
        };

        let results = model
            .translator
            .translate_batch(&sentences, &options, None)
            .map_err(|e| ErrorInfo::translation_error(&e.to_string()))?;

        let translated_parts: Vec<String> = results
            .into_iter()
            .map(|(text, _score)| {
                let mut t = text;
                if t.starts_with(' ') {
                    t.remove(0);
                }
                t
            })
            .collect();

        Ok(translated_parts.join(" "))
    }
}
