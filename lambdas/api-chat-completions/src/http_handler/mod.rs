use aws_lambda_events::event::apigw::{
    ApiGatewayProxyRequest,
};
use aws_sdk_bedrockruntime::types::{
    ContentBlock,
    ConversationRole,
    ConverseStreamOutput,
    Message,
    SystemContentBlock,
};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use lambda_runtime::{ LambdaEvent, Error, streaming::{channel, Body, Response} };
use serde::Deserialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

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
const OBJ_TYPE: &str = "chat.completion.chunk";
const CHAT_ID: &str = "chatcmpl-bedrock";

pub(crate) async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>, 
    bedrock_client: &BedrockClient,
) -> Result<Response<Body>, Error> {
    let payload = event.payload;
    tracing::info!("Payload: {:?}", payload);

    let (mut tx, rx) = channel();
    let mut bedrock_messages: Vec<Message> = Vec::new();
    let mut system_prompt = String::new();

    let body_bytes = match payload.body {
        Some(b) => b.into_bytes(),
        None => { return Err("Invalid body".into()) },
    };

    let openai_req: OpenAiChatRequest = serde_json::from_slice(&body_bytes)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    for msg in openai_req.messages {
        match msg.role.as_str() {
            "system" => {
                system_prompt.push_str(&msg.content);
                system_prompt.push('\n');
            },
            "user" => {
                bedrock_messages.push(
                    Message::builder()
                    .role(ConversationRole::User)
                    .content(ContentBlock::Text(msg.content))
                    .build()?
                );
            }
            "assistant" => {
                bedrock_messages.push(
                    Message::builder()
                    .role(ConversationRole::Assistant)
                    .content(ContentBlock::Text(msg.content))
                    .build()?
                );
            }
            other => {
                tracing::warn!("Unexpected role: {}", other);
                continue;
            }
        }
    }

    tracing::info!("OpenAI {:?}", bedrock_messages);

    let system_blocks = if !system_prompt.is_empty() {
        Some(vec![SystemContentBlock::Text(system_prompt)])
    } else {
        None
    };

    let response = bedrock_client
        .converse_stream()
        .model_id(openai_req.model)
        .set_system(system_blocks)
        .set_messages(Some(bedrock_messages))
        .send() 
        .await?;

    tracing::info!("Response {:?}", response);

    let mut stream = response.stream;
    let created_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::from(format!("Time error: {}", e)))? 
        .as_secs(); 

    tokio::spawn(async move {
        while let Some(event) = stream.recv().await.unwrap() {
            match event {
                ConverseStreamOutput::MessageStart(_) => {
                    let chunk = json!({
                        "id": CHAT_ID,
                        "object": OBJ_TYPE,
                        "created": created_timestamp,
                        "choices": [{"index":0, "delta":{"role":"assistant"}}]
                    });
                    tx.send_data(format!("data: {}\n\n", chunk).into())
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!("error sending message: {:?}", e);
                        });
                }
                ConverseStreamOutput::ContentBlockDelta(delta_event) => {
                    if let Some(Ok(text)) = delta_event.delta.as_ref().map(|d| d.as_text())
                        && tx.send_data(
                            format!("data: {}\n\n", json!({
                                "id": CHAT_ID,
                                "object": OBJ_TYPE,
                                "created": created_timestamp,
                                "choices": [{"index":0, "delta":{"content": text}}]
                            })).into()).await.is_err() {
                            tracing::warn!("Disconnected");
                            break;
                    }
                }
                ConverseStreamOutput::MessageStop(stop_event) => {
                    let finish_reason = match stop_event.stop_reason() {
                        aws_sdk_bedrockruntime::types::StopReason::EndTurn => "stop",
                        aws_sdk_bedrockruntime::types::StopReason::MaxTokens => "length",
                        _ => "stop",
                    };
                    let chunk = json!({
                        "id":CHAT_ID,
                        "object": OBJ_TYPE,
                        "created": created_timestamp,
                        "choices": [{"index":0, "delta":{}, "finish_reason": finish_reason}]
                    });
                    tx.send_data(format!("data: {}\n\n", chunk).into())
                        .await
                        .unwrap_or_else(|e| tracing::warn!("error sending message: {:?}", e));
                    }
                ConverseStreamOutput::Metadata(_metadata_event) => {
                    tx.send_data("data: [DONE]\n\n".into())
                        .await
                        .unwrap_or_else(|e| tracing::warn!("error sending message: {:?}", e));
                    }
                _ => {}
            }
        }
    });

    Ok(Response::from(rx))
}
