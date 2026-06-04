# TMD AI

Serverless OpenAI compatible API to proxy Bedrock models.
Currently being used only with Codecompanion on Neovim.

## Tech stack
- AWS Bedrock
- AWS Lambda
- AWS APIGW V1
- AWS SSM Param Store
- Rust
- CDK


## Data flow
### v0.1
```mermaid
Client → Lambda (Auth/Model Mapping) → Bedrock Stream → OpenAI-formatted SSE
```

### v0.2
```mermaid
Client → APIGW (Auth) → Valid → Proxy Lambda
                      └ Invalid → 401
```
