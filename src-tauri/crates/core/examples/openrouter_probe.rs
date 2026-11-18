//! Live check of the OpenRouter route through the production code path.
//!
//!   OPENROUTER_API_KEY=… cargo run -p poprawiacz-core --example openrouter_probe
//!
//! Prints the model counts per panel, then corrects one sentence with every
//! panel's default model and with one model that cannot switch reasoning off
//! (it must succeed too: the first refusal is learned and retried).

use std::time::Instant;

use poprawiacz_core::ai::{self, openrouter, CorrectionRequest, Provider, ReasoningLevel};
use poprawiacz_core::Style;
use tokio_util::sync::CancellationToken;

const TEXT: &str = "ala ma kota i dwa psy ktore lubia biegac po ogrodzie";

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("set OPENROUTER_API_KEY");
    let client = ai::build_client().expect("http client");

    match openrouter::list_models(&client).await {
        Ok(lists) => {
            for p in Provider::ALL {
                let list = &lists[&p];
                let has_default = list.iter().any(|m| m == openrouter::default_model(p));
                println!("{}: {} models, default listed: {has_default}", p.key(), list.len());
            }
        }
        Err(e) => println!("models list failed: {e}"),
    }

    let mut models: Vec<(Provider, String)> =
        Provider::ALL.into_iter().map(|p| (p, openrouter::default_model(p).to_string())).collect();
    models.push((Provider::Anthropic, "anthropic/claude-sonnet-5.5".into()));
    models.push((Provider::OpenAI, "openai/nie-ma-takiego".into()));

    for (provider, model) in models {
        for stream in [true, false] {
            let req = CorrectionRequest {
                provider,
                model: model.clone(),
                api_key: api_key.clone(),
                style: Style::Normal,
                text: TEXT.into(),
                stream,
                reasoning_level: ReasoningLevel::Off,
                verbosity: "medium".into(),
                custom_system_prompt: None,
                correction_examples: Vec::new(),
                openrouter: true,
            };
            let started = Instant::now();
            let cancel = CancellationToken::new();
            let mut chunks = 0;
            let result = if stream {
                ai::correct_stream(&client, &req, &cancel, |_| chunks += 1).await
            } else {
                ai::correct(&client, &req, &cancel).await
            };
            let ms = started.elapsed().as_millis();
            match result {
                Ok(text) => println!("{model} stream={stream}: {ms} ms, {chunks} chunks: {text}"),
                Err(e) => println!("{model} stream={stream}: {ms} ms, ERROR: {}", e.user_message()),
            }
        }
    }
}
