import subprocess

from pathlib import Path
from typing import Tuple


openssl_config = """
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no
[req_distinguished_name]
C = UK
ST = London
L = London
O = Small Magellanic Cloud AI
OU = Self Hosted
CN = localhost
[v3_req]
keyUsage = critical, digitalSignature, keyAgreement
extendedKeyUsage = serverAuth
subjectAltName = @alt_names
[alt_names]
DNS.1=localhost
DNS.2=inference.smallcloud.local
"""


def gen_certificate(workdir: Path) -> Tuple[Path, Path]:
    cert_dir = workdir / "cert"
    cert_dir.mkdir(parents=False, exist_ok=True)

    key_filename = cert_dir / "server.key"
    if not key_filename.exists():
        s = subprocess.Popen(f"openssl genrsa 2048 > {key_filename}", shell=True)
        s.communicate()
        if not key_filename.exists():
            raise RuntimeError(f"failed to generate ssl key")

    cert_filename = cert_dir / "server.cert"
    if not cert_filename.exists():
        alt_names_ips = {"127.0.0.1", "0.0.0.0"}
        s = subprocess.Popen(
            "hostname -I",
            stdout=subprocess.PIPE,
            shell=True)
        stdout = s.communicate()[0]
        if stdout is not None:
            alt_names_ips.update(stdout.decode("utf8").split())

        openssl_config_filename = cert_dir / "openssl.cfg"
        alt_names_suffix = "\n".join([
            f"IP.{idx + 1}={ip}"
            for idx, ip in enumerate(alt_names_ips)
        ])
        openssl_config_filename.write_text(f"{openssl_config}{alt_names_suffix}\n")

        s = subprocess.Popen(
            "openssl req -new -x509 -nodes -sha256 -days 15330 "
            f"-key {key_filename} -out {cert_filename} -config {openssl_config_filename}",
            shell=True
        )
        s.communicate()

        if not cert_filename.exists():
            raise RuntimeError(f"failed to generate certificate")

    return key_filename, cert_filename
