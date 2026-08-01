# Checklist de publication et reproductibilité

## Sources et provenance

- [ ] SHA complets R1, R2, R3, R4 et R5 présents dans le manifest.
- [ ] R2 est régénéré depuis le commit figé ; aucun diff manuel des tables.
- [ ] R2↔R1 contrôlé ; toute divergence possède un ADR et un test.
- [ ] `CONFLICT-ABI-001` et flags RV64ILP32 documentés dans les notes de release.
- [ ] Licences des normes, oracles, bibliothèques et binaires distribués archivées.
- [x] SBOM et liste des dépendances avec versions exactes générés dans
      `docs/release/SBOM.tsv`.

## Build reproductible

- [x] Audit local reproductible disponible via `tools/release-audit.sh` ; le mode
      `--strict-oracles` ajoute la vérification GNU/LLVM sans la masquer.
- [ ] Clean checkout, toolchain Rust pinée et build offline après cache.
- [ ] `fmt`, `clippy`, unit tests, component tests et E2E verts.
- [ ] Artefacts générés identiques par hash sur deux builds séparés.
- [ ] Aucun timestamp, chemin absolu ou locale hôte dans `.luna`, snapshot, listing ou trace canonique.
- [x] Le dossier `docs/release/` contient le manifest de profil, le hash R2,
      le schema de snapshot et le manifest de provenance vérifiable.

## ISA, ABI et runtime

- [ ] Matrice extension × parse/assemble/decode/execute/debug/export publiée.
- [ ] `addi x1,x0,1` assemble, charge, step et montre `x1=1`.
- [ ] Instruction illégale, CSR absent, mauvais alignement, unmapped et quota produisent diagnostics/traps stables.
- [ ] Frontières `0x7fffffff`, `0x80000000`, `0xffffffff` et sign-extension testées.
- [ ] ELF ambigu refusé ; format `.luna` round-trip.
- [ ] Mémoire cible isolée de l’hôte et include sandbox testé.

## Flottants

- [ ] `fadd.s` et `fadd.d` motifs/flags comparés à oracle externe.
- [ ] `frm`, rm statique/dynamique, sticky `fflags`, ±0, infinis, subnormaux, NaN et payloads couverts.
- [ ] NaN-boxing valide/invalide couvert.
- [ ] binary16/32/64/128 littéraux et affichages exacts ; bfloat16 distinct.
- [ ] Résultats identiques sur hôtes supportés.
- [ ] Q/Zfh non exécutés et refusés explicitement selon capability.

## Débogueur et interaction

- [ ] Scénarios SPEC 1–14 exécutés par script.
- [ ] Code↔hex↔ASCII conserve adresse ; marks/QuickJump fonctionnent.
- [ ] Breakpoint label, watch condition, step-over/out et source↔adresse vérifiés.
- [ ] Undo transactionnel mémoire/registres et restore snapshot vérifiés.
- [ ] Boucle infinie interrompue dans la limite sans geler l’UI.
- [ ] Clavier couvre commandes et navigation principales ; contraste/accessibilité contrôlés.

## Oracle, fuzzing et qualité

- [x] GNU/LLVM exécutés avec versions et sous-ensembles documentés par
      `tools/release-audit.sh --strict-oracles` (GNU 2.44, LLVM 22.1.8,
      corpus R1 de 7 encodages).
- [ ] Sail/Spike exécutés quand applicables ; aucun oracle indisponible masqué.
- [ ] Fuzz PR/nightly/pre-release exécuté selon budgets ; crashes réduits et replayables.
- [ ] Couverture cœur ≥90 %, branches critiques ≥85 %, tous codes diagnostics testés.
- [ ] Zéro test flaky connu ; corpus de non-régression versionné.

## Performance et sécurité

- [ ] Assemblage p95 <1 s pour 64 KiB.
- [ ] Latence commande p95 <50 ms hors run.
- [ ] Snapshot sparse <200 ms pour 256 MiB.
- [ ] Mémoire UI au repos <256 MiB.
- [ ] Limites CPU/RAM/macro et chemins/inclusions testés adversarialement.

## Dossier de release

- [ ] Changelog distingue repris, modernisé, rejeté et limitations.
- [x] Format projet/snapshot/session v4 strict documenté ; les versions
      antérieures sont refusées explicitement (aucune migration automatique).
- [x] Rapport de traçabilité REQ→source→composant→test livré (`docs/TRACEABILITY.md`).
- [x] Rapport des waivers, risques résiduels et questions différées livré dans
      `docs/release/WAIVERS.md`; son approbation finale reste une décision de
      publication.
- [ ] Hashes des packages, sources, tables et fixtures publiés.
- [ ] Cas reproductible de référence exporté et rejoué depuis clean checkout.
