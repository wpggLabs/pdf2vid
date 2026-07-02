use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub text: String,
    pub target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResponse {
    pub translated_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSynthesisRequest {
    pub text: String,
    pub voice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSynthesisResponse {
    pub audio_base64: String,
    pub format: String,
}

pub async fn openai_translate(
    api_key: &str,
    req: TranslationRequest,
) -> Result<TranslationResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "system",
                "content": format!(
                    "You are a professional translator. Translate the user's text into {}. \
                     Preserve technical terms, named entities, and numbers. \
                     Output only the translated text, no commentary.",
                    req.target_language
                )
            },
            {"role": "user", "content": req.text}
        ],
        "temperature": 0.2,
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI error {}: {}", status, text));
    }

    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let translated = value["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "Unexpected OpenAI response shape".to_string())?
        .trim()
        .to_string();

    Ok(TranslationResponse {
        translated_text: translated,
    })
}

pub async fn google_translate(
    api_key: &str,
    req: TranslationRequest,
) -> Result<TranslationResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let lang_code = google_language_code(&req.target_language);

    let resp = client
        .post("https://translation.googleapis.com/language/translate/v2")
        .query(&[
            ("key", api_key),
            ("q", &req.text),
            ("target", lang_code),
            ("format", "text"),
        ])
        .send()
        .await
        .map_err(|e| format!("Google request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Google error {}: {}", status, text));
    }

    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let translated = value["data"]["translations"][0]["translatedText"]
        .as_str()
        .ok_or_else(|| "Unexpected Google response shape".to_string())?
        .to_string();

    Ok(TranslationResponse {
        translated_text: translated,
    })
}

fn google_language_code(language: &str) -> &'static str {
    match language {
        "English (US)" => "en",
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "Portuguese" => "pt",
        "Hindi" => "hi",
        "Japanese" => "ja",
        "Korean" => "ko",
        "Chinese (Simplified)" => "zh-CN",
        "Arabic" => "ar",
        _ => "en",
    }
}

pub async fn openai_tts(
    api_key: &str,
    req: CloudSynthesisRequest,
) -> Result<CloudSynthesisResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "tts-1",
        "input": req.text,
        "voice": req.voice,
        "response_format": "mp3",
    });

    let resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI TTS request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI TTS error {}: {}", status, text));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    Ok(CloudSynthesisResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "mp3".into(),
    })
}

pub async fn elevenlabs_tts(
    api_key: &str,
    voice_id: &str,
    req: CloudSynthesisRequest,
) -> Result<CloudSynthesisResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
    let body = serde_json::json!({
        "text": req.text,
        "model_id": "eleven_monolingual_v1",
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75,
        }
    });

    let resp = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Accept", "audio/mpeg")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("ElevenLabs request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("ElevenLabs error {}: {}", status, text));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    Ok(CloudSynthesisResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "mp3".into(),
    })
}

pub async fn marian_translate(
    _model_path: &str,
    req: TranslationRequest,
) -> Result<TranslationResponse, String> {
    // MarianMT local inference requires an ONNX runtime. The model files
    // are downloaded by models::download_model. The full ONNX inference
    // loop is the integration point for this implementation.
    //
    // Architecture supports swapping this in without changing the
    // Provider trait or the render pipeline.
    //
    // For the initial release, surface a clear message so the UI can
    // either fall back to a cloud translator or notify the user.

    if req.target_language == "English (US)" {
        return Ok(TranslationResponse {
            translated_text: req.text,
        });
    }

    Err(format!(
        "Local MarianMT inference not yet implemented for {}. \
         The model files can be downloaded, but the ONNX inference loop \
         needs ort (ONNX Runtime) integration. See cloud.rs for the integration point.",
        req.target_language
    ))
}

pub async fn piper_synthesize(
    _model_path: &str,
    req: CloudSynthesisRequest,
) -> Result<CloudSynthesisResponse, String> {
    // Piper local inference requires the piper ONNX model + ONNX runtime.
    // Same integration point as MarianMT above.
    Err(format!(
        "Local Piper TTS not yet implemented for voice '{}'. \
         The model files can be downloaded, but the ONNX inference loop \
         needs ort integration. See cloud.rs for the integration point.",
        req.voice
    ))
}
