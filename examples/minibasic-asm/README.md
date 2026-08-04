# MiniBASIC-RV assembleur — première organisation modulaire

Ces fichiers sont des fragments de source acceptés par l’assembleur guest.
Le caractère `;` commence un commentaire et tout le reste de la ligne est
ignoré, comme dans le dialecte historique Amiga. Les commentaires pédagogiques
longs sont dans `docs/MINIBASIC_ASM_SOURCE_GUIDE.md`.

Ordre obligatoire :

1. `00_data.rv` — octets des tokens et constantes binary64 dans la RAM cible ;
2. `10_entry.rv` — contrat d’entrée et sélection du premier littéral ;
3. `20_parse_sum.rv` — reconnaissance de `+` et sélection du second terme ;
4. `30_parse_product.rv` — reconnaissance de `*`, `fmul.d`, `fadd.d` et faute ;
5. `90_session.rv` — fin de source et commandes du moniteur.

La séance est concaténée par `scripts/test-guest-runtime-expression-modular.sh`.
Le résultat est identique à la fixture monolithique, mais chaque module
possède une responsabilité et des labels documentés.

## Payload MiniBASIC principal

Le payload complet est désormais organisé dans `modules/` selon six fragments
ordonnés : données/bootstrap, REPL/dispatch, expressions, tableaux/fonctions,
chaînes/tables et session. La composition est déterministe :

```text
bash scripts/compose-minibasic-asm.sh target/payload-repl.composed.rv
bash scripts/check-minibasic-asm-modules.sh
```

`payload-repl.rv` reste le miroir source compatible avec les anciennes séances
UART ; le contrôle ci-dessus exige qu’il soit identique à la concaténation des
modules. Le build du payload vérifie ce contrat avant de produire le binaire.
Les fragments ne sont donc pas des copies alternatives : ils sont la vue de
travail par responsabilités du même programme utilisateur target-side.
