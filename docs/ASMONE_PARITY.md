# Comparaison ASM-One / RVMonitor

Ce document utilise la référence locale ASM-One v1.48 convertie en Markdown,
conservée sous `docs/dontcommit/ASM-One_V1.4x.Guide.md`. Cette source est une
référence d’interaction et de fonctions ; elle n’est pas une spécification de
l’ISA RISC-V et son contenu local n’est pas une dépendance versionnée du build.

## Correspondances déjà présentes

| ASM-One v1.48 | RVMonitor | État |
|---|---|---|
| `A` / Assemble | `assemble`, `assemble-program`, `assemble-source` | présent, syntaxe moderne structurée |
| `D` / `@D` | `disasm`, `disasm-mixed` | présent ; `@D` n’est pas encore un alias texte |
| `G` / `J` | `run`, `run-at`, `continue` | présent ; budgets et arrêts sont bornés |
| `K` | `step`, `step-over`, `step-out` | présent, avec support d’appels |
| `M`, `H`, `N`, `@H`, `@N` | `memory`, `edit`, `ascii`, `cp437` | vues et édition présentes, modes legacy non aliasés |
| `S`, `F`, `C`, `Q` mémoire | `find`, `fill`, `copy`, diagnostics de comparaison à préciser | les trois premiers présents |
| breakpoints | `break`, `delete`, `info break` | présent, conditions en mode hôte |
| watchpoints | `watch`, `rwatch`, `awatch` | présent |
| `=R` / registres | `regs`, `set`, `setf`, `setcsr` | présent, affichage exact des flottants |
| `P` / paramètres | `help`, diagnostics structurés | modernisé, pas de commande homonyme |
| `WB` / binaire | snapshot/export et payload | présent sous contrats reproductibles différents |

Les références ASM-One correspondantes se trouvent dans les sections `General
MENU Structure`, `@D`, `@H`, `@N`, `G`, `K`, `M`, `N`, `S`, `F`, `C`, `Q` et
`WB` du document local.

## Différences intentionnelles

- Le moniteur moderne privilégie des mots complets et des diagnostics stables;
  les abréviations à une lettre sont ajoutées seulement lorsqu’elles ne
  créent pas de collision (`s`, `n`, `c`, `d`, `b`, `e`, `u`).
- L’édition mémoire est transactionnelle et annulable. La possibilité ASM-One
  de modifier directement une zone n’autorise pas une écriture qui contourne
  les bornes ou les points d’arrêt du backend.
- Les vues mémoire regroupent hexadécimal, ASCII et CP437 dans un modèle de
  navigation unique; elles ne reproduisent pas les écrans Amiga séparés.
- Le débogueur utilise des budgets explicites, des watchpoints et un historique
  borné afin de rester déterministe et isolé de l’hôte.

## Contrôle de flot BASIC et écart restant

La documentation Turbo-BASIC XL 1.5 décrit `POP` comme le moyen d’abandonner
proprement un `GOSUB` ou une boucle structurée lorsqu’un `GOTO` sort de son
corps. Le payload assembleur MiniBASIC-RV dispose actuellement de piles
spécialisées pour `FOR`, `WHILE`, `REPEAT` et `GOSUB`, ainsi que d’une petite
pile unifiée `{kind, line}`. `POP` est maintenant implémenté côté cible et
retire le cadre le plus récent avant de poursuivre une instruction séparée
par `:`. Le test QEMU est
`scripts/test-guest-runtime-asm-repl-pop.sh`.

`EXIT` reste différé : il doit retirer le bon cadre et scanner jusqu’au
`NEXT`, `WEND`, `UNTIL` ou `LOOP` correspondant. Une implémentation qui viderait
une pile arbitraire serait incorrecte. La prochaine tranche de contrôle de
flot doit donc soit :

1. étendre la pile unifiée avec les informations de structure nécessaires puis
   implémenter `EXIT` ;
2. soit consigner explicitement `EXIT` comme différé, sans accepter une
   implémentation qui masque les cadres actifs.

La pile unifiée reste le choix recommandé, car elle rend vérifiables les
erreurs de nesting et la sortie anticipée de `WHILE/WEND`, `REPEAT/UNTIL` et
des futures structures `DO/LOOP`. Cette évolution reste target-side et ne
réintroduit pas l’interpréteur Rust comme oracle.
