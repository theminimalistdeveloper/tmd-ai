use std::env;
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message, ConverseStreamOutput};
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
    let default_model = "global.amazon.nova-2-lite-v1:0".to_string();

    let bedrock_model_id = match openai_req.model.as_str() {
        "claude-4-5-sonnet" => default_model,
        _ => default_model, // fallback default
    };

    let mut bedrock_messages: Vec<Message> = Vec::new();

    for msg in openai_req.messages {
        match msg.role.as_str() {
            "system" => {},
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
    let model_id_string = bedrock_model_id.to_string();

    let response = client_clone
        .converse_stream()
        .model_id(model_id_string)
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
                    tx.send_data(format!("data: {}\n\n", chunk).into()).await.unwrap();
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
                    tx.send_data(format!("data: {}\n\n", chunk).into()).await.unwrap();
                }
                // Triggered after generation stops; provides metrics like token usage
                ConverseStreamOutput::Metadata(_metadata_event) => {
                    // if let Some(usage) = metadata_event.usage {
                    //     tx.send_data(format!(
                    //             "[Token usage: input={}, output={}]",
                    //             usage.input_tokens, usage.output_tokens
                    //     ).into()).await.unwrap();
                    // }
                    tx.send_data("data: [DONE]\n\n".into()).await.unwrap();
                }
                _ => {
                    // Ignore other events like ContentBlockStart, ContentBlockStop, etc.
                }
            }
        }
    });

    Ok(Response::from(rx))
}
