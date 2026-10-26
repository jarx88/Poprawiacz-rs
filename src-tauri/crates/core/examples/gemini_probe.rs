//! Debug probe: sends the app's exact Gemini request (same body builder,
//! headers, streaming endpoint) to several models and logs every attempt.
//!
//!   GEMINI_KEY_FILE=/path cargo run -p poprawiacz-core --example gemini_probe -- [rounds] [model...]

use std::time::Instant;

use futures_util::StreamExt;
use poprawiacz_core::ai::{gemini, CorrectionRequest, Provider, ReasoningLevel};
use poprawiacz_core::Style;

const TEXT: &str = "no wiec chcialem napisac ze jutro spotkanie jest przesuniete na 15 bo szef nie moze wczesniej";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let key = std::fs::read_to_string(std::env::var("GEMINI_KEY_FILE").expect("GEMINI_KEY_FILE"))
        .expect("key file")
        .trim()
        .to_string();
    let mut args = std::env::args().skip(1);
    let rounds: usize = args.next().and_then(|r| r.parse().ok()).unwrap_or(3);
    let mut models: Vec<String> = args.collect();
    let client = poprawiacz_core::ai::build_client().unwrap();

    // Which models does this key actually see?
    let list = client
        .get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=200")
        .header("x-goog-api-key", &key)
        .send()
        .await
        .unwrap();
    let status = list.status();
    let v: serde_json::Value = list.json().await.unwrap_or_default();
    let names: Vec<String> = v["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| {
                    m["supportedGenerationMethods"]
                        .as_array()
                        .is_some_and(|s| s.iter().any(|x| x == "generateContent"))
                })
                .filter_map(|m| m["name"].as_str().map(|n| n.trim_start_matches("models/").to_string()))
                .filter(|n| n.contains("flash") || n.contains("pro"))
                .collect()
        })
        .unwrap_or_default();
    println!("models list: HTTP {status}, {} flash/pro models: {}", names.len(), names.join(", "));
    if models.is_empty() {
        models = vec!["gemini-3.8-flash".into(), "gemini-3.6-flash".into(), "gemini-2.5-flash".into()];
    }

    for model in &models {
        for stream in [true, false] {
            for round in 0..rounds {
                let req = CorrectionRequest {
                    provider: Provider::Gemini,
                    model: model.clone(),
                    api_key: key.clone(),
                    style: Style::Normal,
                    text: TEXT.into(),
                    stream,
                    reasoning_level: ReasoningLevel::Off,
                    verbosity: "medium".into(),
                    custom_system_prompt: None,
                    correction_examples: Vec::new(),
                };
                if std::env::var("E2E").is_ok() {
                    // The production path: retries, fallback model, dash stripping.
                    let t = Instant::now();
                    let cancel = tokio_util::sync::CancellationToken::new();
                    let r = if stream {
                        poprawiacz_core::ai::correct_stream(&client, &req, &cancel, |_| {}).await
                    } else {
                        poprawiacz_core::ai::correct(&client, &req, &cancel).await
                    };
                    let out = match r {
                        Ok(t) => t,
                        Err(e) => format!("ERR {e}"),
                    };
                    let out: String = out.chars().take(200).collect();
                    println!("E2E {model} stream={stream} #{round}: {} ms | {out}", t.elapsed().as_millis());
                    continue;
                }
                let url = if stream { gemini::stream_endpoint(model) } else { gemini::endpoint(model) };
                let t = Instant::now();
                let resp = client
                    .post(url)
                    .header("x-goog-api-key", &key)
                    .json(&{
                        let mut b = gemini::build_body(&req);
                        // GEMINI_THINKING=low|medium|high overrides the level; "none" drops thinkingConfig.
                        match std::env::var("GEMINI_THINKING").as_deref() {
                            Ok("none") => {
                                b["generationConfig"].as_object_mut().unwrap().remove("thinkingConfig");
                            }
                            Ok(l) => b["generationConfig"]["thinkingConfig"] = serde_json::json!({ "thinkingLevel": l }),
                            Err(_) => {}
                        }
                        b
                    })
                    .send()
                    .await;
                let resp = match resp {
                    Ok(r) => r,
                    Err(e) => {
                        println!("{model} stream={stream} #{round}: transport error {e}");
                        continue;
                    }
                };
                let status = resp.status();
                let headers_ms = t.elapsed().as_millis();
                let body = if stream {
                    let mut s = resp.bytes_stream();
                    let mut all = String::new();
                    while let Some(chunk) = s.next().await {
                        all.push_str(&String::from_utf8_lossy(&chunk.unwrap_or_default()));
                    }
                    let text: String = all
                        .lines()
                        .filter_map(|l| gemini::parse_sse_line(l).ok().flatten())
                        .collect();
                    if status.is_success() { text } else { all }
                } else {
                    let v: serde_json::Value = resp.json().await.unwrap_or_default();
                    if status.is_success() {
                        gemini::parse_response(&v).unwrap_or_else(|e| format!("PARSE ERROR {e}: {v}"))
                    } else {
                        v.to_string()
                    }
                };
                let one_line: String = body.replace('\n', " ").chars().take(std::env::var("BODY_CHARS").ok().and_then(|n| n.parse().ok()).unwrap_or(160)).collect();
                println!(
                    "{model} stream={stream} #{round}: HTTP {} headers {headers_ms} ms total {} ms | {one_line}",
                    status.as_u16(),
                    t.elapsed().as_millis()
                );
            }
        }
    }
}
