# MiniBASIC-RV V1

## Statut

Le moteur actuel est un programme assembleur chargé dans l’espace cible puis
assemblé par `assemble-program` depuis le moniteur, et lancé en U-mode. Il ne
lit ni n’interprète aucune donnée sur l’hôte. La source est donc testée par le
chemin réel source → assemblage cible → chargement → exécution ; l’image ELF
résidente reste un artefact de démarrage et non l’oracle de ces tests.

## Grammaire V1

```ebnf
program       = { numbered-line } ;
numbered-line = number , [ space , statement-list ] ;
statement-list = statement , { space , ":" , space , statement } ;
statement     = "REM" , text
              | "PRINT" , print-item , { "," , print-item }
              | "INPUT" , variable
              | [ "LET" , space ] , variable , "=" , expression
              | "IF" , expression , "THEN" , number
              | "GOTO" , number
              | "GOSUB" , number
              | "RETURN"
              | "DATA" , expression , { "," , expression }
              | "READ" , variable
              | "RESTORE"
              | "FOR" , variable , "=" , expression , "TO" , expression , [ "STEP" , expression ]
              | "NEXT" , variable
              | "END" ;
print-item    = string | expression ;
expression    = comparison ;
comparison    = sum , [ ( "=" | "<>" | "<" | "<=" | ">" | ">=" ) , sum ] ;
sum           = product , { ( "+" | "-" ) , product } ;
product       = factor , { ( "*" | "/" ) , factor } ;
factor        = number | variable | "(" , expression , ")" | ( "+" | "-" ) , factor ;
variable      = identifier ;
identifier    = letter , { letter | digit | "_" } ;
letter        = "A" .. "Z" | "a" .. "z" ;
digit         = "0" .. "9" ;
```

Les lignes sont des octets ASCII, numérotées de 0 à 65535, stockées dans 256
enregistrements fixes de 96 octets et triées par numéro. Une ligne vide après
son numéro la supprime. Un identifiant comporte de 1 à 16 caractères ASCII
alphanumériques ou `_`, commence par une lettre et est insensible à la casse.
MiniBASIC réserve 64 variables binary64 dans le dialecte général ; une
variable lue avant affectation est créée avec la valeur zéro. Le payload
assembleur actuel expose 26 emplacements historiques `A`..`Z` et 32 entrées
nommées de 2 à 16 caractères. Au-delà de la capacité de la tranche ou de 16
caractères, l’instruction est rejetée par `BASIC-SYNTAX-001`. `LIST` montre le
texte ; `DUMP` montre slot, adresse, longueur et octets du record, puis les
variables utilisées avec leur motif binary64 et leur affichage fixe.

Les variables chaînes portent le suffixe `$` : `S$` et les noms de 2 à 16
caractères suivis de `$` sont distincts des variables numériques. Le payload
assembleur réserve 26 cellules courtes et 32 cellules nommées longues ; chaque
cellule longue contient un nom canonique de 16 octets, une longueur `u64` et
120 octets ASCII. Les opérations `LET`, `PRINT` et `INPUT` qui ciblent un nom
chaîne long sont exécutées dans la cible. Les chaînes dépassant 120 octets
produisent une erreur cible sans écriture partielle.

## Sémantique numérique

Les identifiants sont 64 variables binary64 au maximum dans le langage ; la
tranche assembleur utilise les 26 slots courts et 32 slots nommés décrits
ci-dessus. Les opérations `+`, `-`, `*` et `/` sont
effectuées dans le guest ; le chemin `/` contient réellement une instruction
`fdiv.d` (symbole `minibasic_divide`). Les comparaisons produisent `0.0` ou
`1.0`. L’affichage V1 est fixe à six décimales, arrondi au plus proche ; les
valeurs infinies et NaN sont affichées `INF`, `-INF` et `NAN`. Une division par
zéro produit `BASIC-ARITH-001`.

`FOR` et `GOSUB` utilisent chacun une pile cible fixe de huit frames. Un STEP
nul, une pile pleine, une cible GOTO/THEN/GOSUB absente et une boucle
interrompue produisent respectivement `BASIC-FLOW-003`, `BASIC-FLOW-002`,
`BASIC-FLOW-001` et `BASIC-RUN-001`. `RETURN` sans appel actif est une erreur
de flot et rend la main à l’invite sans modifier le programme.
Une réponse `INPUT` vide ou syntaxiquement invalide produit
`BASIC-INPUT-001`.

`DATA` et `READ` utilisent un curseur séquentiel conservé dans la mémoire
cible. La tranche actuelle accepte des valeurs numériques binary64 et des
chaînes littérales séparées par des virgules ; les espaces autour des virgules
sont ignorés. `DATA` ne produit aucune sortie et `READ` consomme la prochaine
valeur du type demandé dans l’ordre des lignes du programme. Une chaîne est
copiée dans la variable cible avec une capacité maximale de 120 octets. Une
lecture au-delà des données disponibles ou un type incompatible est une erreur
de flot. `RESTORE` remet ce curseur au début des lignes `DATA`.

## Commandes directes

`NEW`, `LIST`, `RUN`, `TRACE ON`, `TRACE OFF`, `DUMP`, `PRINT`/`?`, `BYE` et
`EXIT` sont disponibles. Une ligne numérotée est insérée ou remplacée ; un
numéro seul la supprime. `TRACE ON` affiche `[numéro]` avant chaque ligne.
Ctrl-C est capturé par le pilote UART interrupt-driven puis consommé par
polling coopératif pendant `RUN`.

## ABI console cible

Les appels `ecall` utilisent `a7=x17` comme numéro, `a0=x10` comme argument ou
résultat et `a1=x11` pour la longueur : `1=write_char`, `2=read_char`
(bloquant), `3=exit`, `5=poll_char` (zéro si aucun octet, sinon octet). Le
service 4 `write_buffer` est documenté dans `docs/TUTORIAL-GUEST.md`.

## Limites du jalon courant et extensions conservées

Le jalon actuellement exécuté fournit les variables chaînes courtes et longues,
les tableaux numériques et de chaînes 1D/2D et `DATA/READ`
numérique et chaîne. Il ne fournit pas encore les fichiers, les fonctions
utilisateur ou les exposants. Cette absence est une limite d’implémentation
intermédiaire, pas un rejet
du produit : les chaînes et les tableaux complets restent des fonctionnalités
obligatoires de la trajectoire MiniBASIC-RV.

Les instructions d’une ligne peuvent être chaînées par `:`. Le séparateur est
traité dans la cible : le payload copie le reste de la ligne dans un record
scratch, puis réutilise le même dispatcher. Le record source et la table des
lignes restent inchangés ; une ligne sans séparateur conserve exactement le
chemin d’exécution précédent. Le record scratch est borné par la longueur de
la ligne et ne peut pas lire au-delà de son terminateur NUL.

La cible de conception conserve donc : chaînes littérales et variables,
affectation et affichage de chaînes, tableaux numériques et tableaux de
chaînes, avec stockage et opérations exécutés dans la machine RV64. Le layout
est maintenant fixé par D-018 ; le payload assembleur est chargé et assemblé
par le moniteur dans les tests QEMU, et aucune conversion ou évaluation n’est
déléguée à l’hôte.

## Contrat retenu pour les chaînes et tableaux

Une chaîne est représentée en mémoire cible par
{data_addr:u64, length:u64, capacity:u64}. Une affectation copie les octets
dans le pool cible après contrôle de capacité et ne partage aucune adresse
hôte. La chaîne vide a length=0 et peut avoir data_addr=0.

La syntaxe actuellement disponible est S$="TEXT", PRINT S$, DIM A(10),
A(I), DIM LONGNUM(10), LONGNUM(I), DIM LONGGRID(2,3), LONGGRID(I,J), DIM
A$(10), A$(I), DIM LONGARRAY$(2) et LONGARRAY$(I). DIM réserve les indices 0 à
N inclus pour chaque dimension. Les tableaux numériques contiennent des
binary64 ; les
tableaux de chaînes contiennent des cellules ASCII de capacité fixe. Les
tableaux de chaînes nommés longs utilisent 32 descripteurs dans la RAM cible à
`0x82010000` et leurs cellules sont à `0x82020000 + slot*4096 + index*128`
pour une dimension. En deux dimensions, l’index ligne-major est `i*dim2+j`;
la dimension 2 est stockée dans le descripteur à `+24` et la même zone de slot
est utilisée, avec au plus 32 cellules.
Les dimensions sont fixes après `DIM`, les tableaux numériques longs peuvent
avoir une ou deux dimensions, et tout index hors bornes produit une erreur
cible avant mutation. Les tableaux de chaînes courts restent unidimensionnels
dans cette tranche ; les tableaux de chaînes longs acceptent également deux
dimensions. Les variantes `LET` de ces affectations sont prises en charge dans
les lignes de programme et en mode direct.

Le payload assembleur accepte aussi, pour les tableaux numériques longs à une
ou deux dimensions, un index calculé de la forme `index [+|-] entier` : par
exemple `LONGNUM(10+1)`, `LONGNUM(I-1)` et `LONGGRID(1+0,2-0)`. Le calcul, la
conversion en entier et le contrôle de borne sont exécutés dans la cible avant
l’accès. Les expressions générales, les parenthèses imbriquées et cette forme
calculée pour les autres familles de tableaux restent des extensions à porter
séparément.

Les tableaux numériques à nom long (2 à 16 caractères ASCII alphanumériques ou
`_`) utilisent 32 descripteurs de 32 octets à `0x82011000`. Chaque descripteur
contient `name[16]`, puis `dimension:u64` (le nombre d’éléments est donc
`N+1`). Les éléments binary64 sont à
`0x82040000 + slot*512 + index*8` pour une dimension, avec une capacité
maximale de 64 éléments par tableau. Pour deux dimensions, le descripteur
ajoute `dim2:u64` à `+24` et les éléments sont en ordre ligne-major à
`0x82050000 + slot*4096 + (i*dim2+j)*8`, avec au plus 512 éléments par tableau.
Les zones sont exclusivement dans la mémoire cible ; une résolution absente,
un produit de dimensions trop grand ou un index hors limites produit une
erreur avant écriture. Les noms sont normalisés en majuscules comme les
variables courtes.

Le même index calculé `base [+|-] entier` est également disponible pour les
tableaux numériques courts `A(…)`, en lecture, affichage et affectation, y
compris sur leurs deux dimensions. Le parseur partagé restaure le contexte de
l’évaluateur avant de revenir au chemin de stockage court.

Les tableaux de chaînes courts acceptent cette même forme calculée en une
dimension, par exemple `A$(I+0)`, en lecture, affichage et affectation. Les
tableaux de chaînes longs l'acceptent pour leur première dimension, y compris
dans la forme à deux dimensions (`LONGGRID$(I+0,J)`) ; la seconde dimension
reste limitée à un entier ou à une variable simple dans cette tranche. Le
calcul d'index et les contrôles de bornes restent exécutés exclusivement dans
la cible.
