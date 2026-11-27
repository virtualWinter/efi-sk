#!/usr/bin/env bash
# Downloads the ISOs used to test the generic Linux fallback, verifies them
# against the published checksums and moves them into isos/. Interrupted
# transfers resume on the next run.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

mkdir -p build/dl
cd build/dl

declare -A urls=(
    [debian-live-13.8.0-amd64-standard.iso]='https://cdimage.debian.org/debian-cd/current-live/amd64/iso-hybrid/debian-live-13.8.0-amd64-standard.iso'
    [openSUSE-Tumbleweed-NET-x86_64-Current.iso]='https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-NET-x86_64-Current.iso'
    [Fedora-Workstation-Live-45-1.4.x86_64.iso]='https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Workstation/x86_64/iso/Fedora-Workstation-Live-45-1.4.x86_64.iso'
)

remote_size() {
    curl -sIL --retry 3 "$1" |
        awk 'tolower($1) == "content-length:" { n = $2 } END { print n }' |
        tr -d '\r'
}

pending=()
for name in "${!urls[@]}"; do
    if [[ -e ../../isos/$name ]]; then
        echo "$name: already in isos/"
        continue
    fi
    # An interrupted download leaves $name.part. A checksum failure leaves $name.
    if [[ -e $name && ! -e $name.part ]]; then
        mv -- "$name" "$name.part"
    fi
    pending+=("$name")
done
if (( ${#pending[@]} == 0 )); then
    exit 0
fi

# Resuming a complete file sends a range past EOF and gets HTTP 416.
todo=()
for name in "${pending[@]}"; do
    remote=$(remote_size "${urls[$name]}")
    if [[ -n $remote && -e $name.part ]] && (( $(stat -c %s "$name.part") >= remote )); then
        echo "$name: transfer complete"
        continue
    fi
    todo+=("$name")
done

pids=()
for name in "${todo[@]}"; do
    curl -fL --retry 3 --continue-at - --output "$name.part" "${urls[$name]}" &
    pids+=($!)
done
for pid in "${pids[@]}"; do
    # Verification below decides whether a transfer finished; a completed
    # file answered 416 here and must not be treated as an error.
    wait "$pid" || true
done

echo 'Fetching checksums'
curl -fL --retry 3 -o SHA256SUMS.debian \
    'https://cdimage.debian.org/debian-cd/current-live/amd64/iso-hybrid/SHA256SUMS'
curl -fL --retry 3 -o CHECKSUM.fedora \
    'https://dl.fedoraproject.org/pub/fedora/linux/releases/45/Workstation/x86_64/iso/Fedora-Workstation-45-1.4-x86_64-CHECKSUM'
curl -fL --retry 3 -o sha256.opensuse \
    'https://download.opensuse.org/tumbleweed/iso/openSUSE-Tumbleweed-NET-x86_64-Current.iso.sha256'

verify() {
    local file=$1 sums=$2 pattern=$3 expected actual
    expected=$(grep -E "$pattern" "$sums" | grep -oE '[0-9a-f]{64}' | head -1 || true)
    if [[ -z $expected ]]; then
        echo "No checksum for $file in $sums" >&2
        return 1
    fi
    actual=$(sha256sum "$file" | grep -oE '[0-9a-f]{64}')
    if [[ $actual != "$expected" ]]; then
        echo "Checksum mismatch for $file" >&2
        return 1
    fi
    echo "$file: OK"
}

status=0
for name in "${pending[@]}"; do
    case $name in
        debian-live-*)
            verify "$name.part" SHA256SUMS.debian 'debian-live-13\.8\.0-amd64-standard\.iso$' || status=1
            ;;
        Fedora-*)
            verify "$name.part" CHECKSUM.fedora 'Fedora-Workstation-Live-45-1\.4\.x86_64\.iso\) =' || status=1
            ;;
        openSUSE-*)
            verify "$name.part" sha256.opensuse 'openSUSE-Tumbleweed-NET.*\.iso$' || status=1
            ;;
    esac
done
if (( status != 0 )); then
    echo 'Checksum verification failed; re-run to resume downloads' >&2
    exit 1
fi

for name in "${pending[@]}"; do
    mv -f -- "$name.part" "../../isos/$name"
done
echo 'Installed into isos/:'
ls -lh ../../isos/
