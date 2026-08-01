# Rapport E2E guest/QEMU

Profil : `guest-qemu-mmode-umode`

Checkout : `d172936`

QEMU : `11.0.2`

Rust : `1.96.0`
Commande : `bash tools/e2e-release-smoke.sh --strict`

| Script | Résultat | Couverture principale |
|---|---|---|
| `test-guest-monitor.sh` | PASS | boot M→U, assembleur, mémoire, registres, FP, breakpoint, undo, diagnostics |
| `test-guest-ecall.sh` | PASS | ABI ecall console, interruption et sortie cible |
| `test-guest-run-at.sh` | PASS | lancement d’un payload assemblé dans le workspace |
| `test-guest-fdiv.sh` | PASS | exécution `fdiv.d`, résultat et `fflags` |
| `test-minibasic.sh` | PASS | MiniBASIC-RV cible, REPL, expressions, contrôle de flot |
| `test-hammurabi.sh` | PASS | programme HAMMURABI-RV complet, calculs D et interaction |
| `test-guest-run.sh` | PASS | exécution bornée, budget épuisé et budget invalide |
| `test-guest-ld-sd.sh` | PASS | calcul d’adresse, `sd`, `ld`, inspection mémoire |
| `test-guest-source.sh` | PASS | source persistante, correction, réassemblage |
| `test-guest-watchpoint.sh` | PASS | watchpoint logiciel sur écriture |
| `test-guest-snapshot.sh` | PASS | snapshot/projet guest, restore et CRC |
| `test-guest-snapshot-binary.sh` | PASS | patch binaire TCP |
| `test-guest-snapshot-export.sh` | PASS | export snapshot/projet TCP vers l’hôte |
| `test-qemu-gdb-backend.sh` | PASS | GDB RSP, chargement, step et breakpoint |

Résultat global : **14/14 PASS, 0 skip, 0 échec**.

Le nouveau smoke `test-guest-payload-skeleton.sh` passe également seul, mais
n’est pas inclus dans ce rapport de campagne historique ; une nouvelle
campagne complète devra être enregistrée après stabilisation du smoke guest
historique.

Ce rapport ne prétend pas couvrir à lui seul tous les scénarios SPEC 1–14,
la couverture seuil release ni les preuves Sail/Spike. Ces écarts restent
référencés dans `WAIVERS.md`.
