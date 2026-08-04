#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
doc="$repo_root/docs/TUTORIAL-GUEST.md"

section_title='Tutoriel : RVMonitor exécuté dans QEMU'
section_marker="$section_title"
section_start=1
section_end=24
active='en attente du moniteur guest'

set_section() {
    local title="$1"
    case "$title" in
        *'MONITEUR GUEST'*)
            section_marker="$title"
            section_title='2. Construire et démarrer le moniteur dans QEMU'
            section_start=40; section_end=118 ;;
        *'aperçu source assembleur'*)
            section_marker="$title"
            section_title='4.0.1 Chargement explicite du programme utilisateur'
            section_start=716; section_end=742 ;;
        *'chargement binaire explicite'*)
            section_marker="$title"
            section_title='4.0.1 Chargement explicite du programme utilisateur'
            section_start=716; section_end=774 ;;
        *'payload assembleur chargé'*)
            section_marker="$title"
            section_title='4.0 Ce que fait réellement basic'
            section_start=677; section_end=774 ;;
        *'RUST MINIBASIC'*)
            section_marker="$title"
            section_title='4.0 Ce que fait réellement basic — distinction ASM/Rust'
            section_start=677; section_end=715 ;;
        *'sections progressives'*)
            section_marker="$title"
            section_title='4.1–4.5 Progression MiniBASIC-RV'
            section_start=775; section_end=911 ;;
        *'jeu final HAMMURABI-RV'*)
            section_marker="$title"
            section_title='4.7 Jeu final : HAMMURABI-RV'
            section_start=994; section_end=1113 ;;
        *)
            section_marker="$title"
            section_title="$title"
            section_start=1; section_end=24 ;;
    esac
}

set_section_for_command() {
    local command="$1"
    case "$command" in
        help|\?) section_title='3. Commandes disponibles dans le guest'; section_start=119; section_end=184 ;;
        regs|registers|memory\ *|edit\ *|undo|data\ *) section_title='3. Commandes disponibles dans le guest'; section_start=119; section_end=255 ;;
        assemble\ *|assemble-program\ *|assemble-source|source*) section_title='Pas-à-pas instrumenté : assembler et corriger'; section_start=256; section_end=366 ;;
        snapshot\ *|watch\ *|info\ watch*|delete\ watch*) section_title='Snapshots et watchpoints'; section_start=470; section_end=605 ;;
        break\ *|delete\ *) section_title='Breakpoints logiciels'; section_start=606; section_end=665 ;;
        basic) section_title='4. MiniBASIC-RV : progression guidée'; section_start=677; section_end=774 ;;
        TRACE\ ON|TRACE\ OFF) section_title='4.3 Ajouter une boucle et observer TRACE'; section_start=817; section_end=849 ;;
        INPUT*|*'INPUT '*|*' IF '*|*' GOTO '*) section_title='4.4 Faire intervenir INPUT et le contrôle de flot'; section_start=850; section_end=873 ;;
    esac
}

render() {
    printf '\033[2J\033[H'
    printf ' RVMonitor — source française synchronisée\n'
    printf ' === %s ===\n' "$section_marker"
    printf ' docs/TUTORIAL-GUEST.md:%s-%s\n\n' "$section_start" "$section_end"
    # Limit sed directly instead of piping through head: with `pipefail`, head
    # would close the pipe early and terminate this long-lived guide pane.
    section_preview_end=$((section_start + 7))
    awk -v start="$section_start" -v end="$section_preview_end" \
        'NR >= start && NR <= end { printf "%6d  %s\n", NR, $0 }' "$doc"
    printf '\n > %s\n' "$active"
}

render
while IFS= read -r event; do
    case "$event" in
        N\|*)
            set_section "${event#N|}"
            active="étape : ${event#N|}" ;;
        C\|*)
            command="${event#C|}"
            set_section_for_command "$command"
            active="commande guest : $command" ;;
        *) active="$event" ;;
    esac
    render
done

active='séance terminée — transcription conservée par asciinema'
render
sleep 2
