# Guide pédagogique du MiniBASIC assembleur

## Statut

Le portage complet de MiniBASIC en assembleur reste en cours. Cette première
organisation modulaire porte un chemin d’expression minimal : `2+3*4`. Elle
sert de source de tutoriel et de contrat de migration, pas de déclaration que
le parser BASIC complet est terminé.

Le dialecte guest utilise `;` comme caractère unique de commentaire : tout ce
qui suit le premier `;` est ignoré, après une instruction comme sur une ligne
isolée. Ce choix reprend l’usage historique Amiga ; le manuel de développement
Amiga documente également le point-virgule comme début de commentaire ([Amiga
Developer’s Manual](https://retro-commodore.eu/files/downloads/amigamanuals-xiik.net/eBooks/AmigaOS%20Developer%27s%20Manual%20%28another%29%20-%20eBook-ENG.pdf)).

Le test assemble les mêmes instructions dans deux formes :

- [`minibasic-runtime-expression.rv`](../examples/minibasic-runtime-expression.rv),
  version monolithique pratique pour les tests unitaires ;
- [`examples/minibasic-asm/`](../examples/minibasic-asm/), version découpée
  pour la lecture pédagogique et la future extension.

## Modules et contrats

| Module | Responsabilité | Entrées | Sorties |
|---|---|---|---|
| `00_data.rv` | image des tokens et constantes | adresse de données | ASCII et binary64 en RAM cible |
| `10_entry.rv` | entrée U-mode et premier token | `x8` = base data | branche vers le parseur du premier terme |
| `20_parse_sum.rv` | niveau somme | `f1`, token `+` | second terme dans `f2` |
| `30_parse_product.rv` | niveau produit et évaluation | `f2`, token `*`, `f3` | `f4=12`, `f5=14`, mémoire résultat |
| `90_session.rv` | frontière source/moniteur | programme assemblé | run, registres et dump |

Les registres de travail sont documentés par convention locale : `x8` pointe
la zone data, `x5/x6` servent aux tokens ASCII, `f1` porte le premier terme,
`f2` le second et `f3` le troisième. `f4` est le produit intermédiaire et
`f5` le résultat. Cette convention est volontairement plus petite qu’une ABI
générale : elle évite une pile d’appel pour le chemin chaud et rend le
pas-à-pas lisible.

## Ce qui est pédagogique

Le lecteur peut suivre le calcul comme une descente de priorité :

```text
expression := terme "+" produit
produit    := nombre "*" nombre
```

`fmul.d` est exécuté avant `fadd.d`. Le résultat est inspectable dans `f4`,
`f5` et dans les huit octets écrits en RAM. La même observation est impossible
si l’expression est évaluée silencieusement par l’hôte.

La version actuelle reconnaît un corpus borné, non une grammaire générale.
Les prochaines étapes remplaceront les séquences de sélection fixes par des
boucles de lexer, une pile d’opérandes statique et des routines séparées pour
`parse_factor`, `parse_product`, `parse_sum` et `parse_comparison`.

## Premier noyau REPL chargé par le moniteur

[`payload-repl.rv`](../examples/minibasic-asm/payload-repl.rv) constitue la
première tranche verticale assembleur intégrée. Le moniteur assemble directement
la source, puis le payload U-mode assure lui-même :

- l’invite `READY> ` et la lecture UART d’une ligne ;
- le stockage target-side de `10 PRINT X+Y` ;
- `LIST`, `NEW` et `RUN` ;
- deux lectures de variables binary64, `fadd.d` et l’écriture du résultat en
  mémoire cible.

La séance reproductible est :

```text
bash scripts/test-guest-runtime-asm-repl.sh
```

Cette tranche fixe le contrat d’intégration source → assembleur du moniteur →
payload → UART → mémoire/registres. Elle ne remplace pas encore
`crates/minibasic-payload`, qui reste un binaire Rust chargé par le script de
build ; le parseur général, les affectations, le formatage décimal et le
contrôle de flot restent les prochaines étapes du portage assembleur.

Le noyau sélectionne désormais l’opérateur stocké dans `PRINT X<op>Y` et
dispatch réellement vers `fadd.d`, `fsub.d`, `fmul.d` ou `fdiv.d`. Les séances
`X+Y` et `X*Y` sont couvertes par :

```text
bash scripts/test-guest-runtime-asm-repl.sh
bash scripts/test-guest-runtime-asm-repl-mul.sh
```

Le payload décode maintenant chaque opérande de la forme bornée
`PRINT <atome><op><atome>` : `X` et `Y` sont lus dans la table binary64, tandis
que `0` à `9` sont convertis par `fcvt.d.l`. Le cas littéral est vérifié par :

```text
bash scripts/test-guest-runtime-asm-repl-literal.sh
```

Le payload remplace maintenant les positions fixes par un scanner target-side
qui avance un pointeur dans le corps. Il accepte plusieurs chiffres avant et
après le point ainsi que des espaces autour des atomes et de l’opérateur, tout
en restant limité à deux opérandes et un opérateur binaire.
Le cas `12.5+3.5` est vérifié par :

```text
bash scripts/test-guest-runtime-asm-repl-multidigit.sh
```

La descente target-side possède maintenant deux niveaux : produit (`*`/`/`)
puis somme (`+`/`-`). La séance `PRINT 2+3*4` produit `14.0` et est vérifiée
par :

```text
bash scripts/test-guest-runtime-asm-repl-precedence.sh
```

Les diagnostics de syntaxe, les comparaisons et les fonctions BASIC restent
des étapes ultérieures.

Le résultat n’est plus seulement laissé dans `f3` : le payload le convertit
dans la cible en forme fixe à six décimales, construit un buffer ASCII puis le
transmet par `ecall 4`. Le breakpoint reste placé après cette émission pour
permettre l’inspection simultanée de la sortie, des registres et de la RAM.
La conversion est actuellement bornée aux valeurs finies représentables par
la conversion entière utilisée par cette tranche ; NaN, infinis et grandes
valeurs nécessitent encore une voie dédiée.

Le cas négatif fractionnaire `PRINT -2.25+0` est couvert par
`scripts/test-guest-runtime-asm-repl-format-negative-fraction.sh` et vérifie
à la fois `-2.250000` sur la console, le motif signé de `f3` et le dump mémoire.

Le magasin target-side accepte maintenant les lignes `10` et `20` dans deux
enregistrements séparés. Les bits d’occupation et les longueurs restent en
RAM, et `LIST` teste les deux slots dans l’ordre numérique même si l’entrée est
saisie `20` puis `10` :

```text
bash scripts/test-guest-runtime-asm-repl-two-lines.sh
```

`RUN` sélectionne la ligne 10 puis enchaîne la ligne 20 si elle existe, avec un
seul arrêt au breakpoint après la dernière. Les sauts entre lignes appartiennent
encore à la prochaine tranche de contrôle de flot. `GOTO 20` est maintenant
reconnu par le dispatcher et transfère réellement l’exécution au second slot ;
la séance est vérifiée par :

```text
bash scripts/test-guest-runtime-asm-repl-goto.sh
```

La capacité assembleur du moniteur est passée à 768 lignes pour absorber ce
payload pédagogique.

Les comparaisons flottantes nécessaires au prochain dispatcher BASIC sont
désormais disponibles dans le dialecte guest. `feq.d`, `flt.d` et `fle.d`
utilisent une destination entière, conformément à l’ISA, et sont vérifiées par
`scripts/test-guest-runtime-fcmp-d.sh`.

Le dispatcher BASIC reconnaît maintenant une forme bornée
`IF <atome><comparaison><atome> THEN <10|20>`. Le chemin vrai est démontré par
`scripts/test-guest-runtime-asm-repl-if.sh`; la comparaison, la décision et le
transfert de slot se déroulent dans le payload.
Le chemin faux sans slot cible est couvert par
`scripts/test-guest-runtime-asm-repl-if-false.sh`.

Le dispatcher distingue maintenant `INPUT X`/`INPUT Y` de `IF`. Il affiche une
invite, lit la ligne UART dans la cible, réutilise le parseur décimal binary64
et reprend l’exécution vers le slot suivant. La séance `INPUT X` puis
`PRINT X*X` est couverte par `scripts/test-guest-runtime-asm-repl-input.sh`.

`TRACE ON` imprime `[10]` ou `[20]` avant chaque ligne exécutée ; la boucle
`10 GOTO 10` peut être interrompue par Ctrl-C, qui produit `BREAK` dans le
guest et rend le contrôle au moniteur. Les preuves sont respectivement
`scripts/test-guest-runtime-asm-repl-trace.sh` et
`scripts/test-guest-runtime-asm-repl-break.sh`.

`PRINT "..."` est maintenant séparé du chemin numérique : les octets de la
chaîne sont lus depuis le record en RAM cible et envoyés par `ecall 1`. Les
chaînes sont ASCII, sans échappement ni variable chaîne à ce stade ; la preuve
est `scripts/test-guest-runtime-asm-repl-string.sh`.

`END` est également dispatché target-side, émet `END\n` puis arrête la séance
sur un breakpoint contrôlable. Sa non-régression est :

```text
bash scripts/test-guest-runtime-asm-repl-end.sh
```

Le scanner reconnaît maintenant les signes `+`/`-` et une parenthèse autour
d’un atome, par exemple `PRINT (-2.5) + (+3.5)`. La négation est exécutée par
le payload au moyen de `fmv.x.d`, inversion du bit 63 et `fmv.d.x`, car le
parseur assembleur actuel ne fournit pas le pseudo-opcode `fneg.d`. La preuve
QEMU est disponible avec :

```text
bash scripts/test-guest-runtime-asm-repl-unary-paren.sh
```

Ces parenthèses restent limitées à un atome ; elles ne forment pas encore un
AST ni une expression imbriquée générale.

Le record conserve désormais la longueur réelle du corps, ce qui permet une
forme décimale bornée `d.d`. La séance `PRINT 2.5+3.5` est vérifiée par :

```text
bash scripts/test-guest-runtime-asm-repl-decimal.sh
```

Le calcul et la conversion restent entièrement target-side ; les exposants et
les formes signées ne sont pas encore acceptés.

Le même record est désormais alimenté par le mode direct `PRINT ...`, sans
numéro de ligne. La preuve minimale est :

```text
bash scripts/test-guest-runtime-asm-repl-direct.sh
```

Cette réutilisation du record est intentionnelle : elle prépare un seul
chemin d’évaluation pour le mode direct et le mode programme.

L’alias historique `?` est normalisé par le payload vers le même préfixe
`PRINT`. Sa preuve target-side est :

```text
bash scripts/test-guest-runtime-asm-repl-question.sh
```

Une affectation numérique directe `X=7` modifie désormais la table cible ; un
`PRINT X+3` ultérieur relit la valeur par `fld`. La séance est vérifiée par :

```text
bash scripts/test-guest-runtime-asm-repl-assignment.sh
```

La source du noyau dépasse désormais 256 lignes ; la séance utilise la
capacité assembleur étendue à 512 lignes du moniteur, introduite dans le
commit `54388af` sans mélanger les autres évolutions du moniteur.

## Références historiques

Le découpage suit le principe historique de tables de syntaxe, pile
d’opérandes, évaluation d’expressions et gestionnaires d’instructions observé
dans [dmsc/turbo-dis](https://github.com/dmsc/turbo-dis). Les adresses 6502,
les tokens BCD et les appels OS Atari ne sont pas copiés. Le port RV64 conserve
plutôt l’immédiateté de l’interface et rend explicites les registres, les
limites mémoire et l’ABI `RVMPAY01`. La source de préservation et les variantes
ATR sont référencées dans [AtariWiki](https://www.atariwiki.org/wiki/Wiki.jsp?page=Turbo-BASIC+XL).
