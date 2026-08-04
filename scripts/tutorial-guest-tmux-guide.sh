#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
doc="$repo_root/docs/TUTORIAL-GUEST.md"

section_title='Tutoriel : RVMonitor exécuté dans QEMU'
section_marker="$section_title"
section_start=1
section_end=24
scroll_offset=1
scroll_window=8
scroll_interval="${TUTORIAL_GUEST_GUIDE_SCROLL_INTERVAL:-0.75}"
scroll_step="${TUTORIAL_GUEST_GUIDE_SCROLL_STEP:-1}"
active='en attente du moniteur guest'

reset_scroll() {
    scroll_offset=$section_start
}

set_section() {
    local title="$1"
    case "$title" in
        *'MONITEUR GUEST'*)
            section_marker="$title"
            section_title='2. Construire et démarrer le moniteur dans QEMU'
            section_start=40; section_end=118; reset_scroll ;;
        *'aperçu source assembleur'*)
            section_marker="$title"
            section_title='4.0.1 Chargement explicite du programme utilisateur'
            section_start=716; section_end=742; reset_scroll ;;
        *'chargement binaire explicite'*)
            section_marker="$title"
            section_title='4.0.1 Chargement explicite du programme utilisateur'
            section_start=716; section_end=774; reset_scroll ;;
        *'payload assembleur chargé'*)
            section_marker="$title"
            section_title='4.0 Ce que fait réellement basic'
            section_start=677; section_end=774; reset_scroll ;;
        *'RUST MINIBASIC'*)
            section_marker="$title"
            section_title='4.0 Ce que fait réellement basic — distinction ASM/Rust'
            section_start=677; section_end=715; reset_scroll ;;
        *'sections progressives'*)
            section_marker="$title"
            section_title='4.1–4.5 Progression MiniBASIC-RV'
            section_start=775; section_end=911; reset_scroll ;;
        *'jeu final HAMMURABI-RV'*)
            section_marker="$title"
            section_title='4.7 Jeu final : HAMMURABI-RV'
            section_start=994; section_end=1113; reset_scroll ;;
        *)
            section_marker="$title"
            section_title="$title"
            section_start=1; section_end=24; reset_scroll ;;
    esac
}

set_section_for_command() {
    local command="$1"
    case "$command" in
        help|\?) section_title='3. Commandes disponibles dans le guest'; section_start=119; section_end=184; reset_scroll ;;
        regs|registers|memory\ *|edit\ *|undo|data\ *) section_title='3. Commandes disponibles dans le guest'; section_start=119; section_end=255; reset_scroll ;;
        assemble\ *|assemble-program\ *|assemble-source|source*) section_title='Pas-à-pas instrumenté : assembler et corriger'; section_start=256; section_end=366; reset_scroll ;;
        snapshot\ *|watch\ *|info\ watch*|delete\ watch*) section_title='Snapshots et watchpoints'; section_start=470; section_end=605; reset_scroll ;;
        break\ *|delete\ *) section_title='Breakpoints logiciels'; section_start=606; section_end=665; reset_scroll ;;
        # BASIC commands intentionally do not change the range.  The explicit
        # phase notes in the controller distinguish the progressive lesson from
        # the final Hammurabi listing; otherwise `basic` or `TRACE ON` would
        # jump back to the first BASIC page in the middle of the game.
    esac
}

render() {
    printf '\033[2J\033[H'
    printf ' RVMonitor — source française synchronisée\n'
    printf ' === %s ===\n' "$section_marker"
    section_preview_end=$((scroll_offset + scroll_window - 1))
    (( section_preview_end > section_end )) && section_preview_end=$section_end
    printf ' docs/TUTORIAL-GUEST.md:%s-%s / section %s-%s\n\n' \
        "$scroll_offset" "$section_preview_end" "$section_start" "$section_end"
    awk -v start="$scroll_offset" -v end="$section_preview_end" \
        'NR >= start && NR <= end { printf "%6d  %s\n", NR, $0 }' "$doc"
    printf '\n > %s\n' "$active"
}

advance_scroll() {
    if (( scroll_offset + scroll_window <= section_end )); then
        scroll_offset=$((scroll_offset + scroll_step))
        render
    fi
}

render
while true; do
    # A timeout is intentional: while the controller streams a large payload,
    # no guide event is emitted for every 32-byte block.  The reader therefore
    # advances through the relevant documentation instead of freezing on its
    # first lines.  The next FIFO event still preempts the scroll immediately.
    if IFS= read -r -t "$scroll_interval" event; then
        case "$event" in
            N\|*)
                set_section "${event#N|}"
                active="étape : ${event#N|}" ;;
            C\|*)
                command="${event#C|}"
                set_section_for_command "$command"
                active="commande guest : $command" ;;
            E\|*)
                active='séance terminée — transcription conservée par asciinema'
                render
                sleep 2
                exit 0 ;;
            *) active="$event" ;;
        esac
        render
    else
        advance_scroll
    fi
done
