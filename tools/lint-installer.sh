#!/usr/bin/env bash
set -euo pipefail

project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
qmllint=${QMLLINT:-}
if [[ -z $qmllint ]]; then
    qmllint=$(command -v qmllint || true)
fi
if [[ -z $qmllint && -x /usr/lib/qt6/bin/qmllint ]]; then
    qmllint=/usr/lib/qt6/bin/qmllint
fi
[[ -n $qmllint ]] || {
    printf 'Error: qmllint was not found. Install the Qt 6 declarative tools.\n' >&2
    exit 1
}

exec "$qmllint" -I /usr/lib/qt6/qml "$project_dir/installer/src/ui.qml"
