Web Image Processor Lambda

Purpose
- Entry point that serves and processes images. Uses `ImageUsecase` and adapters from `libs/infrastructure`.

Wiring
- Example wiring: `ReqwestSource` -> `ImageUsecase` in `src/http_handler/mod.rs`.
