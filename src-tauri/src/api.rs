use crate::storage::Settings;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::time::Duration;

pub const MODEL: &str = "microsoft/mai-transcribe-2";
const BASE: &str = "https://openrouter.ai/api/v1";
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(65))
        .connect_timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("HTTP client configuration")
}
pub fn transcription_body(wav: &[u8], settings: &Settings) -> Value {
    let mut body = json!({ "model":MODEL, "input_audio": { "data":STANDARD.encode(wav), "format":"wav" }, "provider": { "options": { "azure": { "enhancedMode": { "enabled":true, "model":"MAI-Transcribe-2", "modelOptions": { "transcribeStyle":settings.style } }, "phraseList": { "phrases":settings.vocabulary } } } } });
    if settings.language != "auto" {
        body["language"] = json!(settings.language);
    }
    body
}
fn status_message(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => "OpenRouter denied this request. Check your API key and model access.",
        402 => "Your OpenRouter account needs credits before it can transcribe.",
        408 | 504 => "Transcription timed out. Try a shorter recording.",
        429 => "OpenRouter is limiting requests. Wait a moment, then try again.",
        400 | 404 | 422 => "OpenRouter could not accept this transcription request. Check MAI Transcribe 2 access on your account.",
        _ => "The transcription provider is unavailable. Try again shortly."
    }.into()
}
async fn response_json(
    response: Result<reqwest::Response, reqwest::Error>,
) -> Result<Value, String> {
    let response = response.map_err(|e| {
        if e.is_timeout() {
            "The request timed out. Try a shorter recording."
        } else {
            "Could not reach OpenRouter. Check your internet connection."
        }
    })?;
    if !response.status().is_success() {
        return Err(status_message(response.status()));
    }
    response
        .json()
        .await
        .map_err(|_| "Bol could not read the provider response. Please try again.".into())
}
pub async fn test_key(client: &reqwest::Client, key: &str) -> Result<(), String> {
    response_json(
        client
            .get(format!("{BASE}/key"))
            .bearer_auth(key)
            .send()
            .await,
    )
    .await?;
    Ok(())
}
pub async fn transcribe(
    client: &reqwest::Client,
    key: &str,
    wav: &[u8],
    settings: &Settings,
) -> Result<(String, Option<f64>), String> {
    let result = response_json(
        client
            .post(format!("{BASE}/audio/transcriptions"))
            .bearer_auth(key)
            .header("X-OpenRouter-Title", "Bol")
            .json(&transcription_body(wav, settings))
            .send()
            .await,
    )
    .await?;
    let text = result["text"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .ok_or("No transcript was returned. Check your microphone and try again.")?;
    Ok((text.trim().to_owned(), result["usage"]["cost"].as_f64()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_detection_uses_only_mai() {
        let body = transcription_body(&[1, 2, 3], &Settings::default());
        assert!(body.get("language").is_none());
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["input_audio"]["data"], "AQID");
        assert_eq!(
            body["provider"]["options"]["azure"]["enhancedMode"]["modelOptions"]["transcribeStyle"],
            "clean"
        );
    }
    #[test]
    fn language_hints_are_sent_while_style_is_preserved() {
        let languages: Value =
            serde_json::from_str(include_str!("../../src/languages.json")).unwrap();
        for language in languages
            .as_object()
            .unwrap()
            .keys()
            .filter(|code| *code != "auto")
        {
            let settings: Settings =
                serde_json::from_value(json!({"language": language, "style": "verbatim"})).unwrap();
            let body = transcription_body(&[], &settings);
            assert_eq!(body["language"].as_str(), Some(language.as_str()));
            assert_eq!(
                body["provider"]["options"]["azure"]["enhancedMode"]["modelOptions"]
                    ["transcribeStyle"],
                "verbatim"
            );
        }
    }
    #[tokio::test]
    #[ignore = "Makes paid MAI transcription calls using existing synthetic fixtures and the protected key"]
    async fn live_model_checks() {
        let local = std::env::var("LOCALAPPDATA").unwrap();
        let key = crate::platform::read_key(
            &std::path::Path::new(&local).join("local.bol.desktop/openrouter.key"),
        )
        .expect("Saved key must exist");
        let client = client();
        for name in ["short", "hindi-hinglish"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../artifacts/live-audio/{name}.wav"));
            let wav = std::fs::read(path).unwrap();
            let started = std::time::Instant::now();
            let (text, _) = transcribe(&client, &key, &wav, &Settings::default())
                .await
                .expect("Live MAI transcription");
            println!(
                "MAI {name} ({:.2}s): {text}",
                started.elapsed().as_secs_f64()
            );
            if name == "short" {
                assert!(text.to_lowercase().contains("meeting"));
            } else {
                assert!(text.chars().any(|c| ('\u{0900}'..='\u{097f}').contains(&c)));
            }
        }
    }
    #[tokio::test]
    async fn provider_errors_do_not_expose_response_body() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let body = "private-transcript-and-secret-key";
            write!(stream,"HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        let result = response_json(client().get(format!("http://{address}")).send().await)
            .await
            .unwrap_err();
        assert!(result.contains("limiting requests"));
        assert!(!result.contains("secret"));
        assert!(!result.contains("private-transcript"));
        server.join().unwrap();
    }
}
