import os
from fastapi import FastAPI, Request


app = FastAPI(
    title="ACEMAGIC S1 Mini TFT/LCD API",
    description="API for ACEMAGIC S1 Mini TFT/LCD and LED Control for Linux",
)


@app.get("/")
async def home(request: Request):
    return {
        "version": os.getenv("APP_VERSION", "v0.1.x"),
        "commit_hash": os.getenv("COMMIT_HASH", "d3faul7"),
    }


@app.get("/healthz")
def healthz() -> dict:
    return {"alive": True}


@app.get(
    "/ping",
    summary="Quick check if the service is alive",
)
def ping() -> str:
    return "pong"
