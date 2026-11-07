#!/usr/bin/env bash
# Bash entry point; Python's standard library keeps UDP portable across platforms.
set -euo pipefail

if [[ -n "${PYTHON:-}" ]]; then
    interpreter="$PYTHON"
else
    interpreter=""
    for candidate in python3 python; do
        if command -v "$candidate" >/dev/null 2>&1 &&
            "$candidate" -c 'import sys; sys.exit(sys.version_info.major != 3)' >/dev/null 2>&1; then
            interpreter="$candidate"
            break
        fi
    done
fi
if [[ -z "$interpreter" ]]; then
    echo "Python 3 requis : installer Python 3 ou utiliser PYTHON=python bash control.sh ..." >&2
    exit 1
fi

exec "$interpreter" - "$@" <<'PYTHON'
import argparse
import ipaddress
import json
import re
import socket
import sys
import time

PORT = 42423
parser = argparse.ArgumentParser(
    prog="bash control.sh",
    description="Écouter les annonces E-outlet ou commander GPIO23 par UDP.",
)
actions = parser.add_subparsers(dest="action", required=True)
listen = actions.add_parser("listen", help="Écouter les annonces (Ctrl+C pour quitter)")
listen.add_argument("--seconds", type=float, default=0, help="Durée, 0 = illimité")
for name in ("get", "up", "down"):
    command = actions.add_parser(name, help=f"Envoyer {name.upper()} à une carte")
    command.add_argument("ip", type=ipaddress.IPv4Address, help="IP affichée par listen")
    command.add_argument("device_id", help="Nom exact, ex. outlet-0123456789ab")
args = parser.parse_args()


def decode(data):
    try:
        status = json.loads(data)
    except (ValueError, UnicodeDecodeError):
        return None
    if (isinstance(status, dict)
            and status.get("protocol") == "EOUTLET/1"
            and isinstance(status.get("id"), str)
            and status.get("gpio") == 23
            and status.get("state") in ("up", "down")):
        return status
    return None


def show(peer, status):
    print(f"{peer[0]} {json.dumps(status, separators=(',', ':'))}", flush=True)


def run():
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as udp:
        if args.action == "listen":
            if args.seconds < 0:
                parser.error("--seconds doit être positif ou nul")
            udp.bind(("0.0.0.0", PORT))
            deadline = time.monotonic() + args.seconds if args.seconds else None
            print(f"Écoute UDP {PORT} — Ctrl+C pour quitter", file=sys.stderr)
            while deadline is None or time.monotonic() < deadline:
                udp.settimeout(min(1, max(0.001, deadline - time.monotonic())) if deadline else 1)
                try:
                    data, peer = udp.recvfrom(2048)
                except socket.timeout:
                    continue
                status = decode(data)
                if status:
                    show(peer, status)
            return 0

        if not re.fullmatch(r"outlet-[0-9a-f]{12}", args.device_id):
            parser.error("Nom attendu : outlet- suivi de 12 caractères hexadécimaux minuscules")
        address = str(args.ip)
        payload = f"EOUTLET/1 {args.device_id} {args.action.upper()}".encode("ascii")
        udp.bind(("0.0.0.0", 0))  # Reply goes back to this ephemeral source port.
        for attempt in range(1, 4):
            udp.sendto(payload, (address, PORT))
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                udp.settimeout(max(0.001, deadline - time.monotonic()))
                try:
                    data, peer = udp.recvfrom(2048)
                except socket.timeout:
                    break
                status = decode(data)
                if (peer == (address, PORT) and status
                        and status["id"] == args.device_id
                        and (args.action == "get" or status["state"] == args.action)):
                    show(peer, status)
                    return 0
            print(f"Pas de réponse correspondante ({attempt}/3)", file=sys.stderr)
        print("Échec : vérifier IP, nom, Wi-Fi et pare-feu.", file=sys.stderr)
        return 1


try:
    sys.exit(run())
except KeyboardInterrupt:
    sys.exit(130)
except OSError as error:
    print(f"Erreur réseau : {error}", file=sys.stderr)
    sys.exit(1)
PYTHON
