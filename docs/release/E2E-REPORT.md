# Rapport E2E guest/QEMU

Profil : `guest-qemu-mmode-umode`

Checkout : `b2592104d90cd264002362f373063eedb81d1d68`

QEMU : `11.0.2`

Rust : `1.96.0`
Commande : `bash tools/e2e-release-smoke.sh --strict`

| Script | Résultat | Couverture principale |
|---|---|---|
| `test-guest-monitor.sh` | PASS | boot M→U, assembleur, mémoire, registres, FP, breakpoint, undo, diagnostics |
| `test-guest-run.sh` | PASS | exécution bornée, budget épuisé et budget invalide |
| `test-guest-ld-sd.sh` | PASS | calcul d’adresse, `sd`, `ld`, inspection mémoire |
| `test-guest-source.sh` | PASS | source persistante, correction, réassemblage |
| `test-guest-watchpoint.sh` | PASS | watchpoint logiciel sur écriture |
| `test-guest-snapshot.sh` | PASS | snapshot/projet guest, restore et CRC |
| `test-guest-snapshot-binary.sh` | PASS | patch binaire TCP |
| `test-guest-snapshot-export.sh` | PASS | export snapshot/projet TCP vers l’hôte |
| `test-qemu-gdb-backend.sh` | PASS | GDB RSP, chargement, step et breakpoint |

Résultat global : **9/9 PASS, 0 skip, 0 échec**.

Ce rapport ne prétend pas couvrir à lui seul tous les scénarios SPEC 1–14,
la couverture seuil release ni les preuves Sail/Spike. Ces écarts restent
référencés dans `WAIVERS.md`.
