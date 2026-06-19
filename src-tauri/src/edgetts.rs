use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: String,
    pub rate: Option<String>,
    pub pitch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResponse {
    pub audio_base64: String,
    pub format: String,
}

/// Synthesize narration audio for a scene.
///
/// Backend selection:
///   - English voices: StreamElements TTS (free, public, Amazon Polly voices
///     under the hood, returns MP3, no auth needed).
///   - Non-English voices: Google Translate TTS (free, public, no auth,
///     works in 100+ languages, lower quality but reliable).
///
/// Both endpoints are public anonymous APIs that have worked for years.
/// We avoid Microsoft's edge-tts reverse-engineered WebSocket protocol
/// because Microsoft rotates tokens/versions frequently, which makes it
/// fragile to maintain.
pub async fn synthesize(req: TtsRequest) -> Result<TtsResponse, String> {
    let lang = voice_to_language(&req.voice);

    if lang == "en" {
        // Try StreamElements first (better English quality).
        match streamelements_synthesize(&req).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                log::warn!("StreamElements failed ({e}), falling back to Google TTS");
            }
        }
    }

    // Google Translate TTS for non-English or as fallback.
    google_translate_synthesize(&req, &lang).await
}

fn voice_to_language(voice: &str) -> String {
    // Microsoft voice short name -> Google TTS language code.
    // Falls back to "en" for unknown voices.
    if voice.starts_with("en-") { return "en".into(); }
    if voice.starts_with("es-") { return "es".into(); }
    if voice.starts_with("fr-") { return "fr".into(); }
    if voice.starts_with("de-") { return "de".into(); }
    if voice.starts_with("pt-") { return "pt".into(); }
    if voice.starts_with("hi-") { return "hi".into(); }
    if voice.starts_with("ja-") { return "ja".into(); }
    if voice.starts_with("ko-") { return "ko".into(); }
    if voice.starts_with("zh-") { return "zh-CN".into(); }
    if voice.starts_with("ar-") { return "ar".into(); }
    "en".into()
}

fn voice_to_streamelements(voice: &str) -> &'static str {
    // Map Microsoft voice names to StreamElements voice IDs.
    // StreamElements exposes Amazon Polly voices under friendly names.
    match voice {
        "en-US-JennyNeural" => "Amy",
        "en-US-GuyNeural" => "Brian",
        _ => "Amy",
    }
}

async fn streamelements_synthesize(req: &TtsRequest) -> Result<TtsResponse, String> {
    let voice = voice_to_streamelements(&req.voice);
    let encoded = url_encode(&req.text);
    let url = format!(
        "https://api.streamelements.com/kappa/v2/speech?voice={}&text={}",
        voice, encoded
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(&url).send().await.map_err(|e| {
        format!("StreamElements request failed: {e}")
    })?;

    if !resp.status().is_success() {
        return Err(format!(
            "StreamElements returned HTTP {}",
            resp.status()
        ));
    }

    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("StreamElements returned no audio".into());
    }

    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
        format: "audio/mpeg".into(),
    })
}

async fn google_translate_synthesize(req: &TtsRequest, lang: &str) -> Result<TtsResponse, String> {
    // Google Translate's anonymous TTS endpoint. Long text may need to be
    // split into <=200 char chunks because Google limits single requests.
    let chunks = chunk_text(&req.text, 200);
    let mut combined: Vec<u8> = Vec::new();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    for chunk in chunks {
        let encoded = url_encode(&chunk);
        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob",
            encoded, lang
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Google TTS request failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Google TTS returned HTTP {} for chunk",
                resp.status()
            ));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        combined.extend_from_slice(&bytes);
    }

    if combined.is_empty() {
        return Err("Google TTS returned no audio".into());
    }

    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&combined),
        format: "audio/mpeg".into(),
    })
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.chars().count() + word.chars().count() + 1 > max_chars {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    // If a single word exceeds max_chars (rare), hard-split it.
    let mut out = Vec::new();
    for chunk in chunks {
        if chunk.chars().count() <= max_chars {
            out.push(chunk);
        } else {
            for slice in chunk.as_bytes().chunks(max_chars) {
                if let Ok(s) = std::str::from_utf8(slice) {
                    out.push(s.to_string());
                }
            }
        }
    }
    out
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_to_language_known() {
        assert_eq!(voice_to_language("en-US-JennyNeural"), "en");
        assert_eq!(voice_to_language("es-ES-ElviraNeural"), "es");
        assert_eq!(voice_to_language("zh-CN-XiaoxiaoNeural"), "zh-CN");
    }

    #[test]
    fn voice_to_streamelements_known() {
        assert_eq!(voice_to_streamelements("en-US-JennyNeural"), "Amy");
        assert_eq!(voice_to_streamelements("en-US-GuyNeural"), "Brian");
        assert_eq!(voice_to_streamelements("unknown"), "Amy");
    }

    #[test]
    fn chunk_text_short() {
        assert_eq!(chunk_text("hello world", 200), vec!["hello world"]);
    }

    #[test]
    fn chunk_text_long() {
        let text = "a".repeat(500);
        let chunks = chunk_text(&text, 200);
        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
        assert!(chunks.iter().all(|c| c.chars().count() <= 200));
    }

    #[test]
    fn url_encode_handles_spaces_and_unicode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("café"), "caf%C3%A9");
    }
}