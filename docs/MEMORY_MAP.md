# Carte mémoire de référence — RVMonitor + MiniBASIC-RV

Ce document est le contrat de placement mémoire de la cible de référence
`RV64ILP32D-MON-1`. Il couvre l’image du moniteur M-mode, le payload U-mode
MiniBASIC-RV et les zones de données qu’ils partagent. Toute nouvelle routine
assembleur doit réserver ses cellules ici avant d’être ajoutée au payload.

Une adresse indiquée comme `x8+N` est relative à la base de données MiniBASIC
contenue dans `x8`. Dans l’entrée actuelle, `x8 = 0x82000000`, mais le code
doit conserver cette propriété par calcul et ne pas remplacer `x8+N` par une
adresse hôte.

## Régions globales

| Région | Intervalle semi-ouvert | Propriétaire | Durée de vie | Règle |
|---|---:|---|---|---|
| RAM physique QEMU | `0x80000000..0x84000000` | linker du moniteur | toute la machine | 64 MiB ; les sous-régions ci-dessous sont les seules adresses publiques du contrat guest |
| Image moniteur | `0x80000000.._target_workspace_start` | M-mode Rust/ASM | boot | ne jamais utiliser depuis MiniBASIC comme stockage persistant |
| Pile privée M-mode | `_stack_bottom.._stack_top` | trap/IRQ M-mode | boot | 64 KiB ; non accessible au payload et non adressée par constante |
| Workspace code guest | `0x81000000..0x81010000` | assembleur, listing, payload | session | 64 KiB ; `assemble-program` et `run-at` valident cette fenêtre |
| Données guest | `0x82000000..0x82100000` | directives `data`, MiniBASIC | session | 1 MiB ; toutes les variables et tous les scratchs MiniBASIC résident ici |
| Reste RAM | autres adresses de `0x80000000..0x84000000` | réservé | — | aucune allocation implicite ; une extension doit obtenir une décision avant usage |

La pile U-mode fournie par le moniteur est distincte de ces régions. Son
adresse est donnée par `x2` au lancement et n’est pas une constante du
payload. Le moniteur initialise également `x8..x31` selon le contrat du
payload ; MiniBASIC réinitialise ensuite `x8` à `0x82000000` par une adresse
PC-relative, afin d’être relogeable dans le workspace.

La carte n’est pas une carte d’adresses décroissantes : les adresses croissent
normalement de gauche à droite. Le sens big-endian éventuel modifie l’ordre des
octets dans une valeur multi-octets, pas le sens de croissance des adresses ni
celui de la pile.

## Layout du workspace MiniBASIC (`x8 = 0x82000000`)

Les offsets ci-dessous sont des contrats de cellules ou de sous-régions. Une
routine peut réutiliser une cellule temporaire uniquement pendant la durée
indiquée ; elle ne peut pas la conserver à travers un appel imbriqué qui la
possède.

| Offset relatif | Adresse de référence | Contenu | Propriétaire / durée |
|---:|---:|---|---|
| `0..479` | `0x82000000..0x820001df` | données initiales, invite, messages et constantes courtes | payload ; statique |
| `480` | `0x820001e0` | pointeur vers la table des longueurs de lignes | éditeur ; session |
| `512..583` | `0x82000200..0x82000247` | état de `RUN`, drapeaux d’exécution et continuation de `PRINT` | exécuteur ; appel courant |
| `584..767` | `0x82000248..0x820002ff` | descripteurs des tableaux numériques courts | tableaux ; session |
| `768..799` | `0x82000300..0x8200031f` | table des variables numériques courtes, accessible par `x20` | variables ; session |
| `800..1023` | `0x82000320..0x820003ff` | descripteurs des tableaux de chaînes courts et cellules temporaires | chaînes/tableaux ; session |
| `1024..2047` | `0x82000400..0x820007ff` | buffer de saisie et cadres locaux du parseur | REPL et évaluateur ; ligne/appel |
| `2048..2071` | `0x82000800..0x82000817` | profondeur et index de la pile `WHILE/WEND` | contrôle de flot ; session |
| `2072..2127` | `0x82000818..0x8200084f` | mode et cadres auxiliaires des découpes/concaténations | chaînes ; appel |
| `2128..2199` | `0x82000850..0x820008b7` | profondeur et index de la pile `REPEAT/UNTIL` | contrôle de flot ; session |
| `2200..2399` | `0x82000898..0x8200095f` | profondeur et cadres de la pile unifiée (`IF`, `FOR`, `GOSUB`, etc.) | contrôle de flot ; session |
| `2400..2407` | `0x82000960..0x82000967` | pointeur de la copie gauche d'une comparaison chaîne `IF` | contrôle `IF` chaîne ; appel |
| `2408..2415` | `0x82000968..0x8200096f` | longueur de la copie gauche d'une comparaison chaîne `IF` | contrôle `IF` chaîne ; appel |
| `2416..2423` | `0x82000970..0x82000977` | opérateur de comparaison chaîne (`1..6`) | contrôle `IF` chaîne ; appel |
| `2424..2431` | `0x82000978..0x8200097f` | cellule binary64 temporaire conservée pour l'évaluateur/`PRINT` | évaluateur/PRINT ; appel, non persistante |
| `2432..4095` | `0x82000980..0x82000fff` | réserve non allouée dans V1, sauf `x8+2432` pour le retour `x31` de `VAL`, `x8+2440` pour le contexte de `PRINT` concaténé, `x8+2448` pour le début d'une expression `PRINT LEFT$/RIGHT$/MID$` concaténée, `x8+2456` pour le drapeau de transaction `RENUM`, `x8+2464` pour le retour `x31` d'une découpe directe, `x8+2488` pour la profondeur de la pile de contextes chaîne, `x8+2560..2600` pour la sauvegarde transactionnelle du reconnaisseur de statements, `x8+2608` pour la profondeur temporaire de `FOR` pendant l'analyse de `TO/STEP`, `x8+2648` pour l'ID du handler partagé `GOTO/GOSUB/RETURN`, `x8+2656` pour le début source du token statement, `x8+2664` pour sa longueur et `x8+2672` pour son type (`1=statement`) | réservé ; les cellules du reconnaisseur sont restaurées avant le dispatch legacy et la profondeur FOR est publiée après analyse ; le token est invalidé au début d'une nouvelle reconnaissance |
| `2680` | `0x82000a78` | code du dernier diagnostic (`0` ou `1` en V1) | diagnostic target-side ; session |
| `2688` | `0x82000a80` | numéro de la dernière ligne exécutée au moment du diagnostic (`0` hors `RUN`) | diagnostic target-side ; session |
| `4096..12287` | `0x82001000..0x82002fff` | table des longueurs et cellules de variables/chaînes courtes selon le payload | MiniBASIC ; session |
| `12288..` | `0x82003000..` | magasin fixe des 256 records de lignes, 128 octets chacun | éditeur ; session |

Le buffer d’entrée commence à `x18 = x8+1024` et sa longueur maximale doit
être vérifiée avant écriture. Il est interdit d’y placer un scratch numérique,
même temporaire : les quatre cellules binary64 dédiées à cet effet sont
`x8+2400`, `x8+2408`, `x8+2416` et `x8+2424`.

Les offsets `1024..2047` contiennent de nombreux cadres historiques du
parseur. Ils ne forment pas une zone libre. En particulier :

- `x8+1152`, `1176`, `1584`, `1600..1680` et `1808` servent à des sauvegardes
  de contexte numérique ou de chaîne ;
- `x8+1816..1824` servent au formateur numérique ;
- `x8+1888` sert au contexte de `VAL` ;
- `x8+1944..2040` servent aux sources, longueurs, retours imbriqués et au
  début de l’expression RHS des affectations chaîne ;
- `x8+2000` est un retour de découpe imbriquée et ne peut pas être utilisé par
  les wrappers `string_assign_*` ;
- `x8+2016` est le retour des routines spécialisées d’affectation de découpe ;
- `x8+2024` est un retour de résolution de source chaîne.

La cellule `x8+2432` est réservée à la sauvegarde de `x31` par
`atom_val_function`, car `VAL` imbrique un second `eval_expression` qui
utilise lui-même `x31` comme adresse de retour.

La cellule `x8+2440` est réservée à la sauvegarde de `x31` par
`print_string_concat`, car le concaténateur réutilise `x31` et `if_false` doit
retrouver le contexte direct ou `RUN` après le `PRINT`.

La cellule `x8+2448` conserve le pointeur initial d'une expression de découpe
dans `PRINT`. Après validation de `LEFT$`, `RIGHT$` ou `MID$`, le chemin de
sortie peut ainsi réévaluer toute l'expression par `string_concat_assign`, au
lieu de détourner une routine d'affectation qui attend une destination.

Ces cellules ont une durée de vie d’appel. Une routine qui appelle
`eval_expression`, `resolve_string_source`, `string_concat_assign` ou une
fonction numérique doit supposer que les cadres documentés comme « appel »
peuvent être réécrits.

## Zones globales MiniBASIC dans la région données

| Intervalle | Contenu | Capacité / formule | État |
|---|---|---|---|
| `0x82000000..0x82000100` | messages et données initiales du payload | invite, lignes de fixture et constantes | réservé au payload |
| `0x82010000..0x82010fff` | constantes binary64 des fonctions mathématiques | `pi`, coefficients, limites et constantes `LOG/EXP` | statique |
| `0x82011000..0x820117ff` | descripteurs des tableaux numériques longs | 32 descripteurs de 32 octets | session |
| `0x82020000..0x8203ffff` | cellules des tableaux de chaînes longs | `slot*4096 + index*128`, borné par slot | session |
| `0x82040000..0x8204ffff` | éléments des tableaux numériques longs 1D | `slot*512 + index*8`, 64 éléments max/slot | session |
| `0x82050000..0x8205ffff` | éléments des tableaux numériques longs 2D | `slot*4096 + (i*dim2+j)*8`, 512 éléments max/slot | session |
| `0x82060000..0x820606ff` | buffers temporaires chaîne | concat copie à `+512`, découpes à `+768`, `CHR$` à `+1024`, formatage à `+1280`, résultat `PRINT` concaténé à `+1536` | appel ; pas de conservation après retour |
| `0x82060700..0x82060707` | retour du wrapper de concaténation | une adresse de retour | appel |
| `0x82060710..0x82060717` | retour persistant de `string_concat_assign` | une adresse de retour | appel |
| `0x82060728..0x8206072f` | retour des wrappers `string_assign_*` | une adresse de retour | appel |
| `0x82060730..0x820607a7` | copie source de `INSTR` | 120 octets maximum ; évite l’écrasement par le second littéral | appel `INSTR` |
| `0x820607a8..0x820607af` | réserve globale chaîne | non allouée dans V1 | réservé |
| `0x820607b0..0x82060827` | source temporaire d’expression chaîne | 120 octets maximum plus NUL, fourni au concaténateur target-side | fonctions chaîne |
| `0x82060828..0x8206082f` | longueur de sortie d’expression chaîne | cellule `u64` du descripteur temporaire | fonctions chaîne |
| `0x82060830..0x820608a7` | sortie temporaire d’expression chaîne | 120 octets maximum | fonctions chaîne |
| `0x820608a8..0x8206091f` | copie gauche de comparaison chaîne `IF` | 120 octets maximum, ASCII et NUL logique hors longueur | contrôle `IF` chaîne ; appel |
| `0x82060920..0x82060bff` | réserve globale chaîne | non allouée dans V1 | réservé |
| `0x82060c00..0x82060dff` | scratch de réécriture des cibles après `RENUM` | 512 octets ; un record à la fois, avant publication dans le record source | commande `RENUM` |
| `0x82060e00..0x820615ff` | copie transactionnelle des 256 numéros de lignes | 256 valeurs `u64`, utilisée pour restaurer `RENUM` avant publication en cas de dépassement de record | commande `RENUM` |
| `0x82061600..0x82061eff` | pile des concaténateurs chaîne | 8 cadres de 288 octets : retour, total, pointeurs de buffers et destination, buffer de concaténation, buffer source d'expression et retour de découpe à `+272` | appel target-side ; profondeur maximale 8 |
| `0x82061f00..0x82061f7f` | métadonnées des résolveurs d'expressions chaîne | 8 cadres de 16 octets : retour et curseur appelant | appel target-side ; indexé par la profondeur de concaténation |
| `0x82061f80..0x82061fff` | réserve globale chaîne | non allouée dans V1 | réservé |
| `0x82062000..0x8206212f` | table de reconnaissance des fonctions numériques | 19 entrées de 16 octets : longueur, identifiant, nom ASCII et remplissage ; fonctions chaîne, mathématiques, `SQR` et `DEC` | lexer target-side ; statique |
| `0x820622e0` | profondeur temporaire du lexer d'expressions | `u64`, non imbriqué pendant la validation | lexer target-side ; appel |
| `0x820622f0` | nombre de tokens d'expression publiés | `u64`, 0..32 | lexer target-side ; appel |
| `0x82062300..0x820625ff` | flux de tokens d'expression | 32 records de 24 octets : type, adresse source, longueur | lexer target-side ; appel |
| `0x82062600..0x8206271f` | piles statiques du parseur tokenisé intégré | 32 valeurs binary64 à `+0`, 32 opérateurs à `+256` | parseur target-side ; appel |
| `0x82062720` | marqueur de chemin tokenisé | `u64=1` après une expression entièrement réduite par le parseur intégré | diagnostic/test ; session |
| `0x82062728` | sauvegarde du pointeur `x18` | une adresse du buffer d’entrée pendant le parseur tokenisé intégré | parseur target-side ; appel |
| `0x82062730` | sauvegarde du retour `x31` | continuation de `eval_expression` autour du parseur tokenisé intégré | parseur target-side ; appel |
| `0x82062738` | sauvegarde de la longueur `x9` | longueur de la source attendue par les dispatchers `PRINT` après retour | parseur target-side ; appel |
| `0x82062740` | signe unaire différé | `0` ou `1`, appliqué au prochain littéral tokenisé puis effacé | parseur target-side ; appel |

Les cellules de retour globales sont nécessaires parce que les appels de
découpe réutilisent des cellules relatives à `x8`. Elles ne doivent pas être
fusionnées, même si deux chemins semblent non imbriqués dans un exemple.

## Règles d’adressage et d’alias

1. Une adresse de code appartient à `0x81000000..0x8100ffff`; une adresse de
   données appartient à `0x82000000..0x820fffff`.
2. Le payload ne doit jamais déréférencer la mémoire de l’hôte, la pile
   M-mode, une adresse ELF calculée à partir d’un symbole hôte ou une adresse
   hors de ces fenêtres.
3. Les adresses de pointeurs sont RV64 dans les registres. MiniBASIC ne
   pratique aucun alias implicite entre une adresse 32 bits et une adresse
   haute ; une conversion doit être explicite et vérifiée.
4. Un calcul d’adresse qui dépasse une sous-région est une erreur avant
   mutation. Les cellules de longueur et les données d’un tableau sont
   validées séparément.
5. Les constructions `lui/slli/srli` destinées à une adresse globale doivent
   être comparées à cette carte et testées sur QEMU. Pour une adresse relative,
   `x8+offset` est préféré lorsque l’immédiat est représentable.
6. Toute nouvelle cellule doit apparaître dans ce document, dans un commentaire
   proche de l’assembleur et dans au moins un test de non-régression qui expose
   le chevauchement interdit.

## Invariants testables

- écriture d’un nombre pendant `PRINT` ne modifie jamais les octets du buffer
  d’entrée courant (`x8+1024..`) ;
- une concaténation imbriquée ne modifie ni le retour du wrapper, ni le retour
  de `string_concat_assign` ;
- une comparaison chaîne `IF` copie au plus 120 octets dans
  `0x820608a8..0x8206091f`, puis compare dans la cible sans modifier le texte
  source ;
- une affectation `RIGHT$`, `LEFT$` ou `MID$` conserve son retour même si elle
  délègue à la concaténation ;
- les tables `0x82011000`, `0x82020000`, `0x82040000` et `0x82050000` restent
  disjointes des buffers `0x82060000..0x82060fff` ;
- un snapshot et `DUMP` couvrent les régions publiques indiquées par l’ABI,
  mais ne donnent pas accès à la pile M-mode.

## Sources de vérité

- placement physique : [`crates/guest-monitor/linker.ld`](../crates/guest-monitor/linker.ld) ;
- ABI des fenêtres guest : [`GUEST_PAYLOAD_ABI.md`](GUEST_PAYLOAD_ABI.md) ;
- layout des tableaux : [`BASIC_LANGUAGE.md`](BASIC_LANGUAGE.md), section
  « Contrat retenu pour les chaînes et tableaux » ;
- allocation effective des cadres MiniBASIC :
  [`examples/minibasic-asm/payload-repl.rv`](../examples/minibasic-asm/payload-repl.rv).

En cas de divergence entre ce document et le source assembleur, le changement
est incomplet : il faut corriger les deux et ajouter le test associé avant de
commiter.
Pour une découpe consommée par un résolveur chaîne englobant, chaque cadre de
concaténation réserve `+272` au retour de reprise et `+280` au `x31` de
l'évaluateur numérique englobant. Le pointeur de résultat est conservé dans
un registre temporaire pendant le calcul de l'adresse du cadre ; il ne doit
pas être confondu avec le curseur de reprise.
