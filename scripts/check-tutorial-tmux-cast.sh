#!/usr/bin/env bash
set -euo pipefail

cast="${1:?usage: check-tutorial-tmux-cast.sh CAST}"
test -s "$cast"

header="$(head -n 1 "$cast")"
if [[ "$header" != *'"cols":160'* || "$header" != *'"rows":48'* ]]; then
    printf 'tmux tutorial cast has unreadable terminal size (expected 160x48)\n' >&2
    exit 1
fi

# A tmux recording contains alternate-screen control sequences.  The converted
# text is therefore only the last screen, while the cast itself retains every
# write made by both panes.  Validate the audit evidence in that raw stream.
required=(
    'RVMonitor — source française synchronisée'
    '=== MONITEUR GUEST'
    '=== ASM MINIBASIC-RV — chargement binaire explicite'
    'payload loaded address=0x0000000081000100'
    'docs/TUTORIAL-GUEST.md:716-723 / section 716-774'
    'docs/TUTORIAL-GUEST.md:719-726 / section 716-774'
    'MINIBASIC-RV ASM'
    '=== ASM MINIBASIC-RV — jeu final HAMMURABI-RV ==='
    'GOVERN SUMER'
    'docs/TUTORIAL-GUEST.md:1062-1069 / section 994-1113'
    '[10]'
    '[20]'
    '[30]'
    '[40]'
    '[600]'
    'GRAIN 1950.000000'
    'snapshot saved'
)

for marker in "${required[@]}"; do
    if ! grep -aFq -- "$marker" "$cast"; then
        printf 'tmux tutorial cast missing: %s\n' "$marker" >&2
        exit 1
    fi
done

for forbidden in 'mcause=0x0000000000000002' 'trap: illegal instruction' 'error: unknown command'; do
    if grep -aFq -- "$forbidden" "$cast"; then
        printf 'tmux tutorial cast contains forbidden failure: %s\n' "$forbidden" >&2
        exit 1
    fi
done

printf 'tmux tutorial cast passed: %s\n' "$cast"
