use crate::types::{ProviderCategory, ProviderKind, ProviderList, ProviderOption};

pub fn provider_list() -> ProviderList {
    ProviderList {
        translation: vec![
            ProviderOption {
                id: "argos".into(),
                label: "Argos Translate".into(),
                kind: ProviderKind::Local,
                detail: "Free · Offline · Requires Python + argostranslate".into(),
                implemented: true,
                online: false,
                key_label: None,
                category: ProviderCategory::Translation,
            },
            ProviderOption {
                id: "openai".into(),
                label: "OpenAI".into(),
                kind: ProviderKind::Api,
                detail: "Use your API key · GPT translation".into(),
                implemented: true,
                online: true,
                key_label: Some("OpenAI API key".into()),
                category: ProviderCategory::Translation,
            },
            ProviderOption {
                id: "google".into(),
                label: "Google Cloud Translation".into(),
                kind: ProviderKind::Api,
                detail: "Use your API key".into(),
                implemented: true,
                online: true,
                key_label: Some("Google Cloud API key".into()),
                category: ProviderCategory::Translation,
            },
            ProviderOption {
                id: "deepl".into(),
                label: "DeepL".into(),
                kind: ProviderKind::Api,
                detail: "Coming soon".into(),
                implemented: false,
                online: true,
                key_label: Some("DeepL API key".into()),
                category: ProviderCategory::Translation,
            },
        ],
        voice: vec![
            ProviderOption {
                id: "edge".into(),
                label: "edge-tts".into(),
                kind: ProviderKind::Local,
                detail: "Free · Microsoft Neural · Requires Python + edge-tts".into(),
                implemented: true,
                online: true,
                key_label: None,
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "chatterbox".into(),
                label: "Chatterbox".into(),
                kind: ProviderKind::Local,
                detail: "Premium · Multilingual · Requires Python + chatterbox-tts (GPU)".into(),
                implemented: true,
                online: false,
                key_label: None,
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "kokoro".into(),
                label: "Kokoro".into(),
                kind: ProviderKind::Local,
                detail: "Free · Offline · Requires Python + kokoro".into(),
                implemented: true,
                online: false,
                key_label: None,
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "piper".into(),
                label: "Piper".into(),
                kind: ProviderKind::Local,
                detail: "Free · Offline ONNX voices".into(),
                implemented: true,
                online: false,
                key_label: None,
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "elevenlabs".into(),
                label: "ElevenLabs".into(),
                kind: ProviderKind::Api,
                detail: "Use your API key · Premium neural voices".into(),
                implemented: true,
                online: true,
                key_label: Some("ElevenLabs API key".into()),
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "openai".into(),
                label: "OpenAI TTS".into(),
                kind: ProviderKind::Api,
                detail: "Use your API key".into(),
                implemented: true,
                online: true,
                key_label: Some("OpenAI API key".into()),
                category: ProviderCategory::Voice,
            },
            ProviderOption {
                id: "azure".into(),
                label: "Azure Speech".into(),
                kind: ProviderKind::Api,
                detail: "Coming soon".into(),
                implemented: false,
                online: true,
                key_label: Some("Azure Speech key".into()),
                category: ProviderCategory::Voice,
            },
        ],
        visual: vec![
            ProviderOption {
                id: "pages".into(),
                label: "PDF pages".into(),
                kind: ProviderKind::Local,
                detail: "Free · Original document + Ken Burns".into(),
                implemented: true,
                online: false,
                key_label: None,
                category: ProviderCategory::Visual,
            },
            ProviderOption {
                id: "higgsfield".into(),
                label: "Higgsfield".into(),
                kind: ProviderKind::Api,
                detail: "Coming soon".into(),
                implemented: false,
                online: true,
                key_label: Some("Higgsfield API key".into()),
                category: ProviderCategory::Visual,
            },
        ],
        languages: vec![
            "English (US)".into(),
            "Spanish".into(),
            "French".into(),
            "German".into(),
            "Portuguese".into(),
            "Hindi".into(),
            "Japanese".into(),
            "Korean".into(),
            "Chinese (Simplified)".into(),
            "Arabic".into(),
        ],
    }
}

pub fn languages() -> Vec<String> {
    provider_list().languages
}

pub fn edge_voice_for_language(language: &str) -> &'static str {
    match language {
        "Spanish" => "es-ES-ElviraNeural",
        "French" => "fr-FR-DeniseNeural",
        "German" => "de-DE-KatjaNeural",
        "Portuguese" => "pt-PT-RaquelNeural",
        "Hindi" => "hi-IN-SwaraNeural",
        "Japanese" => "ja-JP-NanamiNeural",
        "Korean" => "ko-KR-SunHiNeural",
        "Chinese (Simplified)" => "zh-CN-XiaoxiaoNeural",
        "Arabic" => "ar-EG-SalmaNeural",
        _ => "en-US-JennyNeural",
    }
}

/// Argos/ISO language code for a UI language name. Empty string when the
/// language isn't in our advertised set.
pub fn argos_lang_code(language: &str) -> &'static str {
    match language {
        "English (US)" => "en",
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "Portuguese" => "pt",
        "Hindi" => "hi",
        "Japanese" => "ja",
        "Korean" => "ko",
        "Chinese (Simplified)" => "zh",
        "Arabic" => "ar",
        _ => "",
    }
}

/// Kokoro language code for a UI language name. Kokoro covers 8
/// languages; the ones it does not support return an empty string so the
/// caller can fall back to another provider.
pub fn kokoro_lang_code(language: &str) -> &'static str {
    match language {
        "English (US)" => "a",
        "Spanish" => "e",
        "French" => "f",
        "Portuguese" => "p",
        "Hindi" => "h",
        "Japanese" => "j",
        "Chinese (Simplified)" => "z",
        // German, Korean, Arabic are not supported by Kokoro.
        _ => "",
    }
}

/// A sensible default Kokoro voice id per language.
pub fn kokoro_voice_for_language(language: &str) -> &'static str {
    match language {
        "Spanish" => "ef_dora",
        "French" => "ff_siwis",
        "Portuguese" => "pf_dora",
        "Hindi" => "hf_alpha",
        "Japanese" => "jf_alpha",
        "Chinese (Simplified)" => "zf_xiaobei",
        _ => "af_heart",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProviderKind;

    #[test]
    fn free_translation_default_is_local() {
        let list = provider_list();
        assert_eq!(list.translation[0].id, "argos");
        assert_eq!(list.translation[0].kind, ProviderKind::Local);
        assert!(list.translation[0].implemented);
    }

    #[test]
    fn free_voice_default_is_local() {
        let list = provider_list();
        assert_eq!(list.voice[0].id, "edge");
        assert_eq!(list.voice[0].kind, ProviderKind::Local);
        assert!(list.voice[0].implemented);
    }

    #[test]
    fn voice_provider_ids_are_unique() {
        let list = provider_list();
        let ids: Vec<_> = list.voice.iter().map(|v| v.id.clone()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn translation_provider_ids_are_unique() {
        let list = provider_list();
        let ids: Vec<_> = list.translation.iter().map(|v| v.id.clone()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn voice_provider_for_english_is_jenny() {
        assert_eq!(edge_voice_for_language("English (US)"), "en-US-JennyNeural");
    }

    #[test]
    fn voice_provider_for_chinese_is_xiaoxiao() {
        assert_eq!(
            edge_voice_for_language("Chinese (Simplified)"),
            "zh-CN-XiaoxiaoNeural"
        );
    }

    #[test]
    fn free_voice_uses_edge_tts() {
        let list = provider_list();
        let free = list.voice.iter().find(|p| p.id == "edge").unwrap();
        assert_eq!(free.label, "edge-tts");
        assert!(
            free.detail.contains("Microsoft Neural"),
            "Provider description should mention Microsoft Neural"
        );
        assert!(free.online, "edge-tts is online (Microsoft endpoint)");
    }

    #[test]
    fn stub_providers_are_marked_not_implemented() {
        let list = provider_list();
        let deepl = list.translation.iter().find(|p| p.id == "deepl").unwrap();
        assert!(!deepl.implemented);
        let azure = list.voice.iter().find(|p| p.id == "azure").unwrap();
        assert!(!azure.implemented);
    }

    #[test]
    fn languages_include_advertised_set() {
        let list = provider_list();
        for lang in [
            "English (US)",
            "Spanish",
            "French",
            "German",
            "Hindi",
            "Japanese",
            "Korean",
            "Chinese (Simplified)",
            "Arabic",
            "Portuguese",
        ] {
            assert!(list.languages.contains(&lang.to_string()), "missing {lang}");
        }
    }
}
