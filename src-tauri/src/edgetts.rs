use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
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

const WSS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?trustedclient=true&Authorization=";

pub async fn synthesize(req: TtsRequest) -> Result<TtsResponse, String> {
    let rate = req.rate.unwrap_or_else(|| "+0%".into());
    let pitch = req.pitch.unwrap_or_else(|| "+0Hz".into());

    // Step 1: Get the WebSocket auth token from the trusted client endpoint
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36 Edg/127.0.0.0")
        .build()
        .map_err(|e| e.to_string())?;

    let token_resp = client
        .get("https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?trustedclient=true")
        .header("Authority", "speech.platform.bing.com")
        .header("Pragma", "no-cache")
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch edge-tts token: {e}"))?;

    let token = token_resp
        .headers()
        .get("token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "edge-tts token missing from response".to_string())?
        .to_string();

    // Step 2: Open WebSocket
    let url = format!("{WSS_URL}{token}");
    let mut ws_request = url.into_client_request().map_err(|e| e.to_string())?;
    let headers = ws_request.headers_mut();
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());
    headers.insert(
        "Origin",
        "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse().unwrap(),
    );

    let (mut ws, _) = tokio_tungstenite::connect_async(ws_request)
        .await
        .map_err(|e| format!("edge-tts WebSocket connect failed: {e}"))?;

    // Step 3: Send speech config
    let config_id = format!("{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().format("%a %b %d %Y %H:%M:%S GMT+0000").to_string();
    let config_msg = format!(
        "X-Timestamp:{now}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"audio-24khz-48kbitrate-mono-mp3\"}}}}}}}}"
    );
    ws.send(Message::Text(config_msg))
        .await
        .map_err(|e| format!("edge-tts config send failed: {e}"))?;

    // Step 4: Send SSML speak request
    let ssml = format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{}'><prosody pitch='{}' rate='{}'>{}</prosody></voice></speak>",
        req.voice,
        pitch,
        rate,
        xml_escape(&req.text)
    );
    let speak_msg = format!(
        "X-RequestId:{config_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{now}\r\nPath:ssml\r\n\r\n{ssml}"
    );
    ws.send(Message::Text(speak_msg))
        .await
        .map_err(|e| format!("edge-tts speak send failed: {e}"))?;

    // Step 5: Collect binary audio frames
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
}