# Oracle de portage Rust → assembleur MiniBASIC

Ce document fixe la méthode utilisée pour porter les routines du MiniBASIC
assembleur. Le Rust n’est pas le moteur d’exécution du payload assembleur : il
sert d’implémentation de référence, de source de contrats et d’oracle de
comparaison.

## Générer l’artefact d’étude

Depuis la racine du dépôt :

```sh
cargo rustc -p luna-minibasic-payload \
  --target riscv64gc-unknown-none-elf --release -- --emit=asm
```

Le fichier `.s` est produit dans
`target/riscv64gc-unknown-none-elf/release/deps/`. Il est un artefact de build,
pas une source commitée : les noms manglés et les offsets peuvent changer avec
la version du compilateur.

## Correspondance des routines

| Contrat Rust | Routine assembleur cible | Observation indépendante |
|---|---|---|
| `ExprParser::parse_factor` | `parse_atom` | motifs binary64 et `fdiv.d` |
| `ExprParser::parse_product` | `parse_product` | précédence `*`/`/` |
| `ExprParser::parse_sum` | `eval_expression` / `sum_loop` | `+`/`-` |
| `execute_for` | `for_statement` / `next_statement` | pile FOR statique |
| `execute_print` | `print_statement` / `print_result` | sortie produite par le guest |
| `run_program` | `run_command_generic` / `run_line_generic` | lignes et branchements |

La correspondance est sémantique, pas instructionnelle. Le compilateur Rust
peut employer une pile d’appels et des conventions ABI générales ; le portage
assembleur conserve au contraire des buffers statiques et des contrats de
registres locaux lorsque cela rend le pas-à-pas et l’enseignement plus lisibles.

## Procédure de portage d’une routine

1. écrire ou sélectionner un corpus BASIC minimal couvrant le contrat ;
2. exécuter le corpus avec le chemin Rust et enregistrer sortie, variables,
   motifs binary64 et flags flottants ;
3. repérer dans l’assembly Rust les opérations de contrôle et D pertinentes ;
4. porter une seule routine dans le payload assembleur avec un contrat d’entrée
   et de sortie écrit dans les commentaires de la routine ;
5. exécuter le même corpus dans QEMU avec le payload assembleur ;
6. comparer les observations depuis l’hôte, sans appeler le moteur Rust pendant
   l’exécution cible ;
7. conserver le test et le motif de régression avant de passer à la routine
   suivante.

Une comparaison Rust contre Rust est rejetée comme preuve. Les résultats
doivent être observés dans la cible RV64 et, pour les encodages, contrôlés par
le désassembleur/assembleur indépendants du moniteur.

## Limites actuelles

L’assembly Rust est actuellement un oracle de sémantique et de structure. Il
ne constitue pas encore un backend automatique qui transforme le payload
assembleur. La génération d’un compilateur BASIC RV est donc différée jusqu’à
la stabilisation du lexer, de l’IR et des contrats runtime décrits dans
`BASIC_COMPILER_ROADMAP.md`.
