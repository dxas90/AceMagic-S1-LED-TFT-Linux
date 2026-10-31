"""
Common hypercorn configuration
https://hypercorn.readthedocs.io/en/latest/how_to_guides/configuring.html
"""
import os
import multiprocessing
from pprint import pprint

# ssl opts
use_ssl = bool(os.environ.get("USE_SSL", False))
use_tcp = bool(os.environ.get("USE_TCP", True))
if not any([use_ssl, use_tcp]):
    raise ValueError("At least one of USE_SSL and USE_TCP must be set")

use_certfile = os.environ.get("CERTFILE", None)
use_ca_certs = os.environ.get("CA_CERTS", None)
use_ciphers = os.environ.get("CIPHERS", "ECDHE+AESGCM")
use_keyfile = os.environ.get("KEYFILE", None)
if use_ssl and not all([use_certfile, use_keyfile, use_ca_certs]):
    raise ValueError("USE_SSL Requires CERTFILE/KEYFILE/CA_CERTS")


# binding
host = os.environ.get("HOST", "0.0.0.0")
ssl_port = os.environ.get("SSL_PORT", "443")
tcp_port = os.environ.get("TCP_PORT", "8000")
if use_ssl and ssl_port == tcp_port:
    raise ValueError("SSL_PORT Must be different than TCP_PORT")

use_quic_bind = os.environ.get("QUIC_BIND", None)

use_insecure_bind = os.environ.get("INSECURE_BIND", None)
if use_ssl and use_tcp:
    if not use_insecure_bind:
        use_insecure_bind = "{}:{}".format(host, tcp_port)

if bool(use_insecure_bind) != all([use_ssl, use_tcp]):
    raise ValueError(
        "INSECURE_BIND Must be used only when USE_SSL and USE_TCP are both set"
    )

use_bind = os.environ.get("BIND", None)
if not use_bind:
    use_bind = "{}:{}".format(host, ssl_port if use_ssl else tcp_port)


# workers
cores = multiprocessing.cpu_count()
workers_multiplier = float(os.environ.get("WORKERS_PER_CORE", "1"))
workers_max = int(os.environ.get("MAX_WORKERS", "0"))
if workers_max:
    workers_max = int(workers_max)

default_web_concurrency = cores * workers_multiplier
web_concurrency = os.environ.get("WEB_CONCURRENCY", None)

if web_concurrency:
    use_web_concurrency = int(web_concurrency)
    assert use_web_concurrency > 0, "WEB_CONCURRENCY Must be non zero"
else:
    use_web_concurrency = max(int(default_web_concurrency), 2)
    if workers_max:
        use_web_concurrency = min(use_web_concurrency, workers_max)
        assert (
            use_web_concurrency > 0
        ), "MAX_WORKERS and WORKERS_PER_CORE Must be non zero"

use_worker_class = os.getenv("WORKER_CLASS", "asyncio")
assert use_worker_class in [
    "asyncio",
    "uvloop",
    "trio",
], "WORKER_CLASS Must be asyncio, uvloop or trio"


# others
use_graceful_timeout = os.getenv("GRACEFUL_TIMEOUT", "120")
use_errorlog = os.getenv("ERROR_LOG", "-")
use_accesslog = os.getenv("ACCESS_LOG", "-")
use_keepalive_timeout = os.getenv("KEEP_ALIVE", "5")


# conf
keep_alive_timeout = int(use_keepalive_timeout)
worker_class = use_worker_class
workers = use_web_concurrency
loglevel = os.getenv("LOG_LEVEL", "info")
accesslog = use_accesslog or None
errorlog = use_errorlog or None
graceful_timeout = int(use_graceful_timeout)
backlog = int(os.getenv("BACKLOG", "100"))

certfile = use_certfile
ca_certs = use_ca_certs
ciphers = use_ciphers
keyfile = use_keyfile

bind = use_bind
if use_ssl and use_tcp:
    insecure_bind = use_insecure_bind
if use_quic_bind:
    quic_bind = use_quic_bind


# conf/env data
conf_data = {
    "accesslog": accesslog,
    "errorlog": errorlog,
    "loglevel": loglevel,
    "backlog": backlog,
    "bind": bind,
    "insecure_bind": insecure_bind if use_tcp and use_ssl else None,
    "quic_bind": quic_bind if use_quic_bind else None,
    "graceful_timeout": graceful_timeout,
    "keep_alive_timeout": keep_alive_timeout,
    "workers": workers,
    "worker_class": worker_class,
    "env": {
        "host": host,
        "ssl_port": ssl_port if use_ssl else None,
        "tcp_port": tcp_port if use_tcp else None,
        "use_ssl": use_ssl,
        "use_tcp": use_tcp,
        "workers_multiplier": workers_multiplier,
        "cores": cores,
    },
}

pprint(conf_data)
