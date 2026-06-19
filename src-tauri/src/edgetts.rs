use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

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

// Public trusted-client token used by Edge browser and reverse-engineered clients.
// Microsoft's protocol accepts this token when paired with a valid Sec-MS-GEC value.
const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

// Edge Chromium version string used to mint Sec-MS-GEC-Version. This is a fixed
// stable value — Microsoft accepts a range of recent versions.
const SEC_MS_GEC_VERSION: &str = "1-130.0.2849.68";

/// Generate the Sec-MS-GEC token required by the current edge-tts protocol.
///
/// Algorithm (matches the Python `edge-tts` reference implementation):
///   1. Take current Unix time in milliseconds.
///   2. Round down to the nearest 5-minute window (`ticks = ms / 300_000`).
///   3. Hash: `SHA256(str(ticks) + trusted_client_token)`.
///   4. Base64-encode the digest.
///   5. URL-encode the result for use in the WSS query string.
fn generate_sec_ms_gec() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let ticks = now / 300_000;
    let input = format!("{}{}", ticks, TRUSTED_CLIENT_TOKEN);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
    // URL-encode so it can be safely used as a query parameter value.
    url_encode(&b64)
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

pub async fn synthesize(req: TtsRequest) -> Result<TtsResponse, String> {
    let rate = req.rate.unwrap_or_else(|| "+0%".into());
    let pitch = req.pitch.unwrap_or_else(|| "+0Hz".into());

    let sec_ms_gec = generate_sec_ms_gec();
    let wss_url = format!(
        "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?trustedclient=true&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
        sec_ms_gec, SEC_MS_GEC_VERSION
    );

    let mut ws_request = wss_url
        .into_client_request()
        .map_err(|e| format!("edge-tts request build failed: {e}"))?;
    let headers = ws_request.headers_mut();
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());
    headers.insert(
        "Origin",
        "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Accept-Encoding",
        "gzip, deflate, br".parse().unwrap(),
    );
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0"
            .parse()
            .unwrap(),
    );

    let (mut ws, _) = tokio_tungstenite::connect_async(ws_request)
        .await
        .map_err(|e| format!("edge-tts WebSocket connect failed: {e}"))?;

    // Step 1: speech config
    let now = chrono::Utc::now().format("%a %b %d %Y %H:%M:%S GMT+0000").to_string();
    let config_msg = format!(
        "X-Timestamp:{now}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}"
    );
    ws.send(Message::Text(config_msg))
        .await
        .map_err(|e| format!("edge-tts config send failed: {e}"))?;

    // Step 2: SSML speak
    let request_id = format!("{}", uuid::Uuid::new_v4());
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{}'><prosody pitch='{}' rate='{}'>{}</prosody></voice></speak>",
        req.voice,
        pitch,
        rate,
        xml_escape(&req.text),
    );
    let speak_msg = format!(
        "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{now}\r\nPath:ssml\r\n\r\n{ssml}"
    );
    ws.send(Message::Text(speak_msg))
        .await
        .map_err(|e| format!("edge-tts speak send failed: {e}"))?;

    // Step 3: collect audio
    let mut audio_bytes: Vec<u8> = Vec::new();
    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| format!("edge-tts WebSocket error: {e}"))?;
        match msg {
            Message::Binary(data) => {
                if data.len() < 2 {
                    continue;
                }
                let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                if data.len() < header_len + 2 {
                    continue;
                }
                let header = String::from_utf8_lossy(&data[2..2 + header_len]).to_string();
                let audio = &data[2 + header_len..];
                if header.contains("Path:audio") {
                    audio_bytes.extend_from_slice(audio);
                }
            }
            Message::Text(text) => {
                if text.contains("Path:turn.end") {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    let _ = ws.close(None).await;

    if audio_bytes.is_empty() {
        return Err("edge-tts returned no audio data".into());
    }

    Ok(TtsResponse {
        audio_base64: base64::engine::general_purpose::STANDARD.encode(&audio_bytes),
        format: "audio-24khz-48kbitrate-mono-mp3".into(),
    })
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escapes_specials() {
        assert_eq!(xml_escape("a < b & c"), "a &lt; b &amp; c");
        assert_eq!(xml_escape("say \"hi\""), "say &quot;hi&quot;");
    }

    #[test]
    fn sec_ms_gec_is_deterministic_within_window() {
        let a = generate_sec_ms_gec();
        std::thread::sleep(Duration::from_millis(10));
        let b = generate_sec_ms_gec();
        // Within a 5-minute window the token must be identical.
        assert_eq!(a, b);
    }

    #[test]
    fn sec_ms_gec_is_url_safe() {
        let token = generate_sec_ms_gec();
        assert!(!token.contains('+'));
        assert!(!token.contains('/'));
        assert!(!token.contains('='));
    }

    #[test]
    fn url_encode_handles_specials() {
        assert_eq!(url_encode("a+b/c=d"), "a%2Bb%2Fc%3Dd");
        assert_eq!(url_encode("hello-world_1.0~"), "hello-world_1.0~");
    }
}