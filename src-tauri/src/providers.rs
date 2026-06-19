use crate::types::{ProviderCategory, ProviderKind, ProviderList, ProviderOption};

pub fn provider_list() -> ProviderList {
    ProviderList {
        translation: vec![
            ProviderOption {
                id: "marian".into(),
                label: "MarianMT".into(),
                kind: ProviderKind::Local,
                detail: "Free · Helsinki-NLP · Runs on this device".into(),
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
                detail: "Free · Microsoft Neural · Uses Microsoft online synthesis".into(),
                implemented: true,
                online: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProviderKind;

    #[test]
    fn free_translation_default_is_local() {
        let list = provider_list();
        assert_eq!(list.translation[0].id, "marian");
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
        assert_eq!(edge_voice_for_language("Chinese (Simplified)"), "zh-CN-XiaoxiaoNeural");
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
        for lang in ["English (US)", "Spanish", "French", "German", "Hindi", "Japanese", "Korean", "Chinese (Simplified)", "Arabic", "Portuguese"] {
            assert!(list.languages.contains(&lang.to_string()), "missing {lang}");
        }
    }
}