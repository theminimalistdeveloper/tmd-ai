use std::env;
use aws_sdk_bedrockruntime::types::{
    ContentBlock,
    ConversationRole,
    Message,
    ConverseStreamOutput,
    SystemContentBlock,
};
use std::collections::HashMap;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use serde::Deserialize;
use lambda_runtime::{ LambdaEvent, Error, streaming::{channel, Body, Response} };
use aws_lambda_events::event::lambda_function_urls::LambdaFunctionUrlRequest;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Deserialize, Debug)]
struct OpenAiMessage {
    role: String,
    content: String,
}

fn get_model_id(model_id: &str) -> String {
    let default_model = "nova-micro-v1";
    let mut models = HashMap::new();

    models.insert("claude-3-haiku", "us.anthropic.claude-3-haiku-20240307-v1:0");
    models.insert("claude-3-sonnet", "us.anthropic.claude-3-sonnet-20240229-v1:0");
    models.insert("claude-3.5-haiku", "us.anthropic.claude-3-5-haiku-20241022-v1:0");
    models.insert("claude-4-opus", "us.anthropic.claude-opus-4-20250514-v1:0");
    models.insert("claude-4-sonnet", "us.anthropic.claude-sonnet-4-20250514-v1:0");
    models.insert("claude-4-sonnet-global", "global.anthropic.claude-sonnet-4-20250514-v1:0");
    models.insert("claude-4.1-opus", "us.anthropic.claude-opus-4-1-20250805-v1:0");
    models.insert("claude-4.5-haiku", "us.anthropic.claude-haiku-4-5-20251001-v1:0");
    models.insert("claude-4.5-haiku-global", "global.anthropic.claude-haiku-4-5-20251001-v1:0");
    models.insert("claude-4.5-opus", "us.anthropic.claude-opus-4-5-20251101-v1:0");
    models.insert("claude-4.5-opus-global", "global.anthropic.claude-opus-4-5-20251101-v1:0");
    models.insert("claude-4.5-sonnet", "us.anthropic.claude-sonnet-4-5-20250929-v1:0");
    models.insert("claude-4.5-sonnet-global", "global.anthropic.claude-sonnet-4-5-20250929-v1:0");
    models.insert("claude-4.6-opus", "us.anthropic.claude-opus-4-6-v1");
    models.insert("claude-4.6-opus-global", "global.anthropic.claude-opus-4-6-v1");
    models.insert("claude-4.6-sonnet", "us.anthropic.claude-sonnet-4-6");
    models.insert("claude-4.6-sonnet-global", "global.anthropic.claude-sonnet-4-6");
    models.insert("claude-4.7-opus", "us.anthropic.claude-opus-4-7");
    models.insert("claude-4.7-opus-global", "global.anthropic.claude-opus-4-7");
    models.insert("cohere-embed-v4", "us.cohere.embed-v4:0");
    models.insert("cohere-embed-v4-global", "global.cohere.embed-v4:0");
    models.insert("deepseek-r1", "us.deepseek.r1-v1:0");
    models.insert("llama-3.1-70b", "us.meta.llama3-1-70b-instruct-v1:0");
    models.insert("llama-3.1-8b", "us.meta.llama3-1-8b-instruct-v1:0");
    models.insert("llama-3.2-11b", "us.meta.llama3-2-11b-instruct-v1:0");
    models.insert("llama-3.2-1b", "us.meta.llama3-2-1b-instruct-v1:0");
    models.insert("llama-3.2-3b", "us.meta.llama3-2-3b-instruct-v1:0");
    models.insert("llama-3.2-90b", "us.meta.llama3-2-90b-instruct-v1:0");
    models.insert("llama-3.3-70b", "us.meta.llama3-3-70b-instruct-v1:0");
    models.insert("llama-4-maverick-17b", "us.meta.llama4-maverick-17b-instruct-v1:0");
    models.insert("llama-4-scout-17b", "us.meta.llama4-scout-17b-instruct-v1:0");
    models.insert("marengo-embed-2.7", "us.twelvelabs.marengo-embed-2-7-v1:0");
    models.insert("marengo-embed-3.0", "us.twelvelabs.marengo-embed-3-0-v1:0");
    models.insert("nova-2-lite", "us.amazon.nova-2-lite-v1:0");
    models.insert("nova-2-lite-global", "global.amazon.nova-2-lite-v1:0");
    models.insert("nova-lite", "us.amazon.nova-lite-v1:0");
    models.insert("nova-micro", "us.amazon.nova-micro-v1:0");
    models.insert("nova-premier", "us.amazon.nova-premier-v1:0");
    models.insert("nova-pro", "us.amazon.nova-pro-v1:0");
    models.insert("palmyra-x4", "us.writer.palmyra-x4-v1:0");
    models.insert("palmyra-x5", "us.writer.palmyra-x5-v1:0");
    models.insert("pegasus-1.2", "us.twelvelabs.pegasus-1-2-v1:0");
    models.insert("pegasus-1.2-global", "global.twelvelabs.pegasus-1-2-v1:0");
    models.insert("pixtral-large", "us.mistral.pixtral-large-2502-v1:0");
    models.insert("stable-conservative-upscale", "us.stability.stable-conservative-upscale-v1:0");
    models.insert("stable-control-sketch", "us.stability.stable-image-control-sketch-v1:0");
    models.insert("stable-control-structure", "us.stability.stable-image-control-structure-v1:0");
    models.insert("stable-creative-upscale", "us.stability.stable-creative-upscale-v1:0");
    models.insert("stable-erase-object", "us.stability.stable-image-erase-object-v1:0");
    models.insert("stable-fast-upscale", "us.stability.stable-fast-upscale-v1:0");
    models.insert("stable-inpaint", "us.stability.stable-image-inpaint-v1:0");
    models.insert("stable-outpaint", "us.stability.stable-outpaint-v1:0");
    models.insert("stable-remove-bg", "us.stability.stable-image-remove-background-v1:0");
    models.insert("stable-search-recolor", "us.stability.stable-image-search-recolor-v1:0");
    models.insert("stable-search-replace", "us.stability.stable-image-search-replace-v1:0");
    models.insert("stable-style-guide", "us.stability.stable-image-style-guide-v1:0");
    models.insert("stable-style-transfer", "us.stability.stable-style-transfer-v1:0");

    match models.get(model_id) {
        Some(m) => { 
            tracing::info!("Model selected: {:?}", m);
            m.to_string()
        },
        None => { 
            tracing::info!("Reverted to the default model: {:?}", default_model);
            models.get(default_model).unwrap().to_string()
        }
    }
}

pub(crate) async fn function_handler(event: LambdaEvent<LambdaFunctionUrlRequest>, bedrock_client: &BedrockClient) -> Result<Response<Body>, Error> {
    let payload = event.payload;
    let (mut tx, rx) = channel();

    tracing::info!("Payload: {:?}", payload);

    let custom_secret = match env::var("CUSTOM_SECRET") {
        Ok(cs) => format!("Bearer {}", cs),
        Err(_) => {
            tracing::error!("CUSTOM_SECRET not set");
            return Err("INTERNAL ERROR".into());
        }
    };

    tracing::info!("Custom secret {:?}", custom_secret);

    let auth_header = payload.headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    tracing::info!("Authorization header {:?}", auth_header);

    if auth_header != custom_secret {
        return Err("Unauthorized".into());
    }

    let body_bytes = match payload.body {
        Some(b) => b.into_bytes(),
        None => { return Err("Invalid body".into()) },
    };

    let openai_req: OpenAiChatRequest = serde_json::from_slice(&body_bytes).unwrap();

    let mut bedrock_messages: Vec<Message> = Vec::new();
    let mut system_prompt = String::new();

    for msg in openai_req.messages {
        match msg.role.as_str() {
            "system" => {
                system_prompt.push_str(&msg.content);
                system_prompt.push('\n');
            },
            "user" => {
                let message = Message::builder()
                    .role(ConversationRole::User)
                    .content(ContentBlock::Text(msg.content))
                    .build()
                    .unwrap();
                bedrock_messages.push(message);
            }
            "assistant" => {
                let message = Message::builder()
                    .role(ConversationRole::Assistant)
                    .content(ContentBlock::Text(msg.content))
                    .build()
                    .unwrap();
                bedrock_messages.push(message);
            }
            _ => {
                tracing::info!("Empty");
            }
        }
    }

    let client_clone = bedrock_client.clone();
    let model_id_string = get_model_id(&openai_req.model);
    let system_blocks = if !system_prompt.is_empty() {
        Some(vec![SystemContentBlock::Text(system_prompt)])
    } else {
        None
    };

    let response = client_clone
        .converse_stream()
        .model_id(model_id_string)
        .set_system(system_blocks)
        .set_messages(Some(bedrock_messages))
        .send() 
        .await?;

    let mut stream = response.stream;

    tokio::spawn(async move {
        while let Some(event) = stream.recv().await.unwrap() {
            match event {
                // Triggered when the model begins its reply
                ConverseStreamOutput::MessageStart(_) => {
                    let chunk = json!({
                        "id":"chatcmpl-bedrock",
                        "object": "chat.completion.chunk",
                        "choices": [{"index":0, "delta":{"role":"assistant"}}]
                    });
                    tx.send_data(format!("data: {}\n\n", chunk).into())
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!("error sending message: {:?}", e);
                        });
                }
                // This event triggers when a new chunk of text arrives
                ConverseStreamOutput::ContentBlockDelta(delta_event) => {
                    if let Some(Ok(text)) = delta_event.delta.as_ref().map(|d| d.as_text())
                        && tx.send_data(
                            format!("data: {}\n\n", json!({
                                "id":"chatcmpl-bedrock",
                                "object": "chat.completion.chunk",
                                "choices": [{"index":0, "delta":{"content": text}}]
                            })).into()).await.is_err() {
                            tracing::warn!("Disconnected");
                            break;
                    }
                }
                // Triggered when the stream successfully finishes or is cut off
                ConverseStreamOutput::MessageStop(stop_event) => {
                    let finish_reason = match stop_event.stop_reason() {
                        aws_sdk_bedrockruntime::types::StopReason::EndTurn => "stop",
                        aws_sdk_bedrockruntime::types::StopReason::MaxTokens => "length",
                        _ => "stop",
                    };
                    let chunk = json!({
                        "id":"chatcmpl-bedrock",
                        "object": "chat.completion.chunk",
                        "choices": [{"index":0, "delta":{}, "finish_reason": finish_reason}]
                    });
                    tx.send_data(format!("data: {}\n\n", chunk).into())
                        .await
                        .unwrap_or_else(|e| tracing::warn!("error sending message: {:?}", e));
                    }
                // Triggered after generation stops; provides metrics like token usage
                ConverseStreamOutput::Metadata(_metadata_event) => {
                    tx.send_data("data: [DONE]\n\n".into())
                        .await
                        .unwrap_or_else(|e| tracing::warn!("error sending message: {:?}", e));
                    }
                _ => {
                }
            }
        }
    });

    Ok(Response::from(rx))
}
