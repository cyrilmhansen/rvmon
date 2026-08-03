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
              | [ "LET" , space ] , string-target , "=" , string-assignment-rhs
              | [ "LET" , space ] , variable , "=" , expression
              | "IF" , expression , "THEN" , number
              | "IF" , expression , "THEN" , end-of-line , block-body , "ENDIF"
              | "GOTO" , number
              | "GOSUB" , number
              | "ON" , expression , ( "GOTO" | "GOSUB" ) , number , { "," , number }
              | "RETURN"
              | "POP"
              | "EXIT"
              | "WHILE" , expression
              | "WEND"
              | "REPEAT"
              | "UNTIL" , expression
              | "DO"
              | "LOOP"
              | "DATA" , expression , { "," , expression }
              | "READ" , variable
              | "RESTORE"
              | "FOR" , variable , "=" , expression , "TO" , expression , [ "STEP" , expression ]
              | "NEXT" , variable
              | "END" ;
print-item    = string | expression | string-function ;
string-function = ( "LEFT$" | "RIGHT$" ) , "(" , string-source , "," , expression , ")"
                | "MID$" , "(" , string-source , "," , expression , "," , expression , ")" ;
string-assignment-function = ( "LEFT$" | "RIGHT$" ) , "(" , string-source , "," , expression , ")"
                           | "MID$" , "(" , string-source , "," , expression , "," , expression , ")" ;
string-assignment-rhs = string-assignment-function | string-expression ;
string-expression = string-term , { "+" , string-term } ;
string-term = string-source | string-assignment-function | string-constructor ;
string-constructor = "CHR$" , "(" , expression , ")"
                   | "STR$" , "(" , expression , ")" ;
expression    = comparison ;
comparison    = sum , [ ( "=" | "<>" | "<" | "<=" | ">" | ">=" ) , sum ] ;
sum           = product , { ( "+" | "-" ) , product } ;
product       = factor , { ( "*" | "/" ) , factor } ;
factor        = number | variable | "(" , expression , ")"
              | "LEN" , "(" , string-reference , ")"
              | "ASC" , "(" , string-source , ")"
              | "VAL" , "(" , string-source , ")"
              | "INSTR" , "(" , string-source , "," , string-source , ")"
              | "ABS" , "(" , expression , ")"
              | "SGN" , "(" , expression , ")"
              | "INT" , "(" , expression , ")"
              | "TRUNC" , "(" , expression , ")"
              | "FRAC" , "(" , expression , ")"
              | "MOD" , "(" , expression , "," , expression , ")"
              | "SQR" , "(" , expression , ")"
              | "SIN" , "(" , expression , ")"
              | "COS" , "(" , expression , ")"
              | "TAN" , "(" , expression , ")"
              | "LOG" , "(" , expression , ")"
              | "EXP" , "(" , expression , ")"
              | "ATN" , "(" , expression , ")"
              | "RND" | "RND" , "(" , ")"
              | ( "+" | "-" ) , factor ;
variable      = identifier ;
string-variable = identifier , "$" ;
string-reference = string-variable , [ "(" , index , [ "," , index ] , ")" ] ;
string-target = string-reference ;
string-source = string-reference | string ;
index         = number | variable | number , ( "+" | "-" ) , number
              | variable , ( "+" | "-" ) , number ;
identifier    = letter , { letter | digit | "_" } ;
letter        = "A" .. "Z" | "a" .. "z" ;
digit         = "0" .. "9" ;
end-of-line   = ? no non-space byte before the numbered line ends ? ;
block-body    = { numbered-line } , [ "ELSE" , { numbered-line } ] ;
```

Les lignes sont des octets ASCII, numérotées de 10 à 2560 par pas de 10,
stockées dans 256 enregistrements fixes de 128 octets et triées par numéro. Une ligne vide après
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
`1.0`. L’affichage V1 est fixe à six décimales, tronqué de façon déterministe
après conversion binary64 ; les
valeurs infinies et NaN sont affichées `INF`, `-INF` et `NAN`. Une division par
zéro produit `BASIC-ARITH-001`.

Les fonctions numériques `ABS(expr)`, `SGN(expr)`, `INT(expr)`,
`TRUNC(expr)`, `FRAC(expr)`, `MOD(a,b)`, `SQR(expr)`, `SIN(expr)`,
`COS(expr)`, `TAN(expr)`, `LOG(expr)` et `EXP(expr)` sont
évaluées dans le guest. `TRUNC` convertit vers l’entier signé en arrondissant
vers zéro puis reconvertit en binary64 ; `FRAC(x)` vaut `x-TRUNC(x)` ;
`MOD(a,b)` vaut `a-TRUNC(a/b)*b` et refuse `b=0`. Les scratchs target-side sont
distincts par fonction, ce qui permet des appels imbriqués comme
`MOD(INT(-3.9),5)`. `ABS(x)` efface uniquement le bit de signe du motif
binary64 et conserve donc les valeurs particulières et leurs charges utiles.
`SGN(x)` renvoie `-1.0`, `0.0` ou `1.0` selon le signe de `x` ; les
comparaisons ordonnées de l’ISA donnent `1.0` pour un NaN. `INT(x)` renvoie le
plancher mathématique : il est distinct de `TRUNC` pour les valeurs négatives
non entières (`INT(-3.9)=-4.0`). Les valeurs hors domaine de conversion entière
suivent
la politique RISC-V de conversion implémentée par le moteur et restent une
limite V1. Le format décimal fixe peut afficher `FRAC(-3.9)` comme `-0.899999`
après la troncature binary64 et le formateur à six chiffres ; ce résultat est
déterministe et n’est pas une valeur décimale exacte.

`SQR(x)` utilise l’instruction RISC-V `fsqrt.d` sur un opérande binary64
non-négatif. `SQR(0)` renvoie `0.0`; une valeur négative est refusée par le
diagnostic target-side au lieu de laisser une valeur NaN de domaine se
propager silencieusement. La syntaxe exige exactement une expression et une
parenthèse fermante. Le résultat et les éventuels flags flottants restent
ceux de l’exécution D dans la cible ; aucun calcul de racine n’est effectué
par l’hôte.

`SIN(x)` et `COS(x)` utilisent une réduction d’intervalle target-side en
radians, suivie d’un polynôme binary64 évalué par les instructions D. La
réduction ramène l’opérande dans `[-pi/2,pi/2]`; les constantes et les
coefficients sont des motifs binary64 stockés dans la mémoire cible. Les
points canoniques `0`, `±pi/2` et `pi` bénéficient d’un résultat exact dans la
limite de la représentation retenue. Les autres résultats sont déterministes
et leur précision utile V1 est vérifiée à six décimales sur le domaine borné
par les tests QEMU. Aucun appel à une bibliothèque mathématique de l’hôte
n’est effectué. Les valeurs infinies, NaN et les opérandes hors domaine de
conversion entière suivent la limite numérique V1 documentée pour la
réduction ; elles ne constituent pas encore une promesse de précision
transcendante sur toute la plage binary64.

`TAN(x)` évalue `SIN(x)/COS(x)` dans la cible en réutilisant le même noyau
de réduction et de polynôme, sans repasser par le parseur ou l’hôte. Un
cosinus nul est refusé par le diagnostic arithmétique cible au lieu de
produire silencieusement une division par zéro. Le résultat est binary64 et
est soumis à la même précision pratique et au même affichage fixe que
`SIN`/`COS`.

`ATN(x)` calcule l’arc tangente en radians entièrement dans le guest. Pour
`|x|>1`, l’implémentation évalue la réciproque et applique l’identité avec
`pi/2`; pour les valeurs réduites au-delà de `sqrt(2)-1`, elle utilise la
transformation vers l’intervalle de `pi/4`. Le résultat final est obtenu par
un polynôme binary64 évalué par les instructions D. `ATN(0)`, `ATN(1)` et
les signes opposés sont couverts par les tests QEMU ; la précision V1 est
définie par l’affichage déterministe à six décimales et les valeurs de
référence du corpus. Les appels imbriqués sont supportés jusqu’à deux cadres
`ATN`; un troisième niveau est refusé par le diagnostic target-side afin de
préserver les zones statiques. Un NaN est refusé ; aucune conversion ni
évaluation de l’opérande n’est déléguée à l’hôte.

`LOG(x)` est le logarithme naturel. Il exige un opérande strictement positif,
normalisé et fini ; zéro, les valeurs négatives, les NaN et les sous-normaux
sont refusés par le diagnostic target-side V1. Le guest extrait l’exposant et
la mantisse binary64, puis évalue `log(m)` par la série en `z=(m-1)/(m+1)` et
ajoute `exposant*ln(2)`. `EXP(x)` utilise une réduction par `ln(2)`, un
polynôme de degré 10 et une reconstruction de l’exposant binary64. V1 accepte
la plage bornée `-708 <= x <= 708`; les opérandes hors plage sont refusés pour
éviter un résultat subnormal ou infini non documenté. Ces approximations sont
déterministes et exécutées uniquement par les instructions D dans la cible.
À six décimales tronquées, une composition comme `EXP(LOG(10))` peut donc
afficher `9.999999` plutôt que `10.000000`; ce n’est pas une délégation hôte.

`LEN(string-source)` renvoie dans le guest la longueur d’un littéral ASCII, de
la variable chaîne ou de l’élément de tableau chaîne fourni. La résolution
accepte les noms courts/longs et les tableaux 1D/2D déjà disponibles ; le
résultat est converti en binary64 pour rester utilisable dans une expression
numérique. Les expressions chaîne générales restent hors de cette tranche.

`PRINT LEFT$(string-variable,n)` et `PRINT RIGHT$(string-variable,n)` sont
disponibles dans la tranche target-side actuelle pour une variable chaîne
scalaire courte ou longue. `n` doit être entier, compris entre 0 et 120 ; une
valeur supérieure à la longueur source est ramenée à cette longueur. Le
résultat est copié dans un buffer temporaire de la RAM cible puis émis par
`write_buffer`. Elles peuvent aussi être utilisées dans les affectations et
comme termes d’une concaténation target-side, selon les limites décrites
ci-dessous.

`PRINT MID$(string-variable,start,n)` utilise une position `start` 1-based,
comme le BASIC traditionnel. `start=0` et les valeurs négatives sont rejetés ;
un début supérieur à la longueur source produit une chaîne vide. Une longueur
nulle produit également une chaîne vide, et une longueur supérieure au restant
disponible est ramenée à ce restant. Le résultat est copié et affiché dans la
RAM cible. Comme `LEFT$` et `RIGHT$`, `MID$` peut être utilisée comme terme
d’une concaténation target-side ; son affectation directe accepte les sources
littérales, scalaires et tableaux décrites ci-dessous.

La tranche assembleur accepte également
`LET destination$=LEFT$(source$,n)`, `RIGHT$` et `MID$` (avec `LET` facultatif).
La source peut être un littéral ASCII, une variable chaîne scalaire ou un
élément de tableau chaîne 1D/2D déjà résolu par le guest ; la destination peut
être une variable scalaire ou un élément de tableau. Pour
`LEFT$`/`RIGHT$`, `n` est évalué dans le guest, doit être entier et compris entre
0 et 120 ; une valeur supérieure à la source est ramenée à sa longueur. Pour
`MID$`, la position est 1-based, strictement positive, et la longueur suit les
mêmes bornes ; une position au-delà de la source produit une chaîne vide. La
copie passe par un scratch de la RAM cible afin que l’auto-affectation et les
recouvrements restent sûrs, puis écrit la longueur et les octets de destination
sans intervention de l’hôte.

Une affectation peut également concaténer plusieurs termes chaîne avec `+` :
chaque terme est un littéral ASCII, une variable, un élément de tableau chaîne,
une fonction `LEFT$`/`RIGHT$`/`MID$` ou un constructeur `CHR$`/`STR$`, par exemple
`LET TITLE$="RV "+LEFT$(TEXT$,4)+"!"`. Le résultat est assemblé dans un buffer
cible borné à 120 octets avant d’être copié vers la destination. Les
conversions numériques implicites et les opérateurs chaîne autres que `+`
restent rejetés ; `CHR$` et `STR$` sont des conversions explicites évaluées
dans le guest.

`ASC(string-source)` renvoie dans le guest le premier octet ASCII de la source
et refuse la chaîne vide. `CHR$(expression)` convertit dans le guest une valeur
entière de 0 à 255 en un terme chaîne d’un octet ; les valeurs négatives,
supérieures à 255, fractionnaires ou sans argument sont rejetées. En `PRINT`,
le terme est émis comme un octet puis suivi d’un saut de ligne.

`VAL(string-source)` copie la source dans un buffer target-side puis applique le
parseur numérique binary64 du guest. Les espaces et le format décimal accepté
par l’expression numérique sont conservés ; toute source vide, non numérique ou
contenant une traîne non consommée est rejetée.

`INSTR(haystack,needle)` recherche `needle` dans `haystack` entièrement dans la
RAM cible et renvoie une position 1-based, ou `0.0` si aucune occurrence n’est
trouvée. Une aiguille vide renvoie `1.0`. Les deux opérandes acceptent les
littéraux, variables et éléments de tableaux chaîne ; les formes numériques ou
les arguments manquants sont rejetés.

`STR$(expression)` utilise dans le guest le même format fixe à six décimales
que `PRINT` et produit un terme chaîne réutilisable dans une affectation ou une
concaténation. Le format conserve le signe et les zéros de fraction ; les
valeurs particulières suivent la limite du formateur V1. `PRINT STR$(...)`
émet directement ce buffer target-side puis un saut de ligne.

`RND` et `RND()` renvoient un nombre pseudo-aléatoire binary64 dans `[0,1)`.
Le générateur est un LCG 32 bits target-side de paramètres `1664525` et
`1013904223`, initialisé à la graine `1` au chargement et réinitialisé par
`NEW`. Cette séquence est volontairement reproductible ; les arguments comme
`RND(1)` sont rejetés en V1.

`FOR` et `GOSUB` utilisent chacun une pile cible fixe de huit frames. Un STEP
nul, une pile pleine, une cible GOTO/THEN/GOSUB absente et une boucle
interrompue produisent respectivement `BASIC-FLOW-003`, `BASIC-FLOW-002`,
`BASIC-FLOW-001` et `BASIC-RUN-001`. `RETURN` sans appel actif est une erreur
de flot et rend la main à l’invite sans modifier le programme.
Une réponse `INPUT` vide ou syntaxiquement invalide produit
`BASIC-INPUT-001`.

`WHILE expression` et `WEND` sont exécutés dans la cible et partagent une pile
de huit niveaux dédiée. Une expression numérique est vraie si elle est
différente de `0.0`; les comparaisons utilisent la même sémantique que `IF`.
Quand la condition est fausse, le guest recherche le `WEND` correspondant en
comptant les `WHILE`/`WEND` imbriqués et reprend à la ligne suivante. `WEND`
sans boucle active et une imbrication dépassant huit niveaux sont des erreurs
de flot. La résolution structurelle porte sur le premier statement de chaque
ligne ; les formes imbriquées dans une même ligne séparée par `:` restent
exclues de cette tranche.

`IF expression THEN` sans numéro ouvre un bloc structuré ; `ELSE` et `ENDIF`
doivent être les premières instructions de leurs lignes numérotées. Une
condition différente de `0.0` exécute la branche `THEN`, puis saute jusqu’à
`ENDIF` ; une condition fausse recherche `ELSE` ou `ENDIF` et reprend dans la
branche appropriée. Le cadre de bloc utilise le type 6 de la pile de contrôle
unifiée. Les blocs peuvent être imbriqués jusqu’à huit niveaux, avec recherche
bornée à 256 enregistrements et portant sur la première instruction de chaque
ligne ; les formes `IF ... THEN` à numéro et les blocs séparés par `:` ne
doivent donc pas être utilisés dans cette tranche. Un `ELSE` ou `ENDIF`
orphelin, un terminateur absent ou une profondeur excessive produit une erreur
de flot et rend la main à l’invite.

`REPEAT` et `UNTIL expression` utilisent une seconde pile cible fixe de huit
niveaux. `REPEAT` exécute toujours son corps au moins une fois ; `UNTIL` quitte
la boucle si son expression numérique est différente de `0.0`, sinon revient à
la ligne `REPEAT`. Les comparaisons (`=`, `<>`, `<`, `<=`, `>`, `>=`) suivent la
même sémantique que `IF` et `WHILE`. Un `UNTIL` sans `REPEAT` actif et une
imbrication dépassant huit niveaux produisent une erreur de flot. Comme pour
`WHILE`, la résolution structurée porte sur le premier statement de la ligne ;
les formes imbriquées dans une même ligne séparée par `:` ne sont pas admises.

`POP` retire le cadre de contrôle le plus récent d’une pile cible unifiée. Il
est valide pour `GOSUB`, `FOR`, `WHILE`, `REPEAT` et `DO` et permet notamment la forme
`POP:GOTO numéro` pour sortir explicitement d’une structure. Un `POP` sans
cadre actif produit une erreur de flot. Le payload conserve également les
métadonnées spécialisées de chaque mécanisme afin que `NEXT`, `RETURN`,
`WEND`, `UNTIL` et `LOOP` continuent à valider leur type de cadre.

`EXIT` quitte la boucle la plus récente lorsque son cadre est de type `FOR`,
`WHILE`, `REPEAT` ou `DO`. Le guest retire le cadre typé, puis effectue une
recherche bornée du `NEXT`, `WEND`, `UNTIL` ou `LOOP` correspondant en comptant
les ouvertures et
fermetures du même type. L’exécution reprend à la ligne suivant le terminateur.
Un `EXIT` sans boucle, dans un `GOSUB` actif au sommet de la pile, ou sans
terminateur correspondant produit une erreur de flot. Comme les autres
recherches structurées de cette tranche, le scan inspecte le premier statement
de chaque ligne ; les structures imbriquées dans une même ligne séparée par `:`
ne sont pas admises.

`DO` et `LOOP` forment une boucle inconditionnelle. `DO` pousse un cadre de
type 5 uniquement lors de la première entrée ; le retour de `LOOP` vers la
ligne `DO` reconnaît ce cadre et ne le duplique pas. `LOOP` sans `DO` actif et
les variantes conditionnelles (`DO WHILE`, `DO UNTIL`, `LOOP WHILE` ou `LOOP
UNTIL`) sont rejetés dans cette tranche.

`ON expression GOTO n1,n2,...` et `ON expression GOSUB n1,n2,...` évaluent
`expression` dans la cible, la convertissent en index entier 1-based et
sélectionnent le numéro correspondant dans la liste. `GOTO` transfère
directement le contrôle ; `GOSUB` empile la ligne suivante et utilise le même
contrat de `RETURN` que l’instruction simple. Un index nul, négatif, hors liste,
une liste vide ou une cible absente produit une erreur de flot. Les listes sont
volontairement limitées aux numéros de lignes, sans expressions dans les
cibles.

`DATA` et `READ` utilisent un curseur séquentiel conservé dans la mémoire
cible. La tranche actuelle accepte des valeurs numériques binary64 et des
chaînes littérales séparées par des virgules ; les espaces autour des virgules
sont ignorés. `DATA` ne produit aucune sortie et `READ` consomme la prochaine
valeur du type demandé dans l’ordre des lignes du programme. Une chaîne est
copiée dans la variable cible avec une capacité maximale de 120 octets. Une
lecture au-delà des données disponibles ou un type incompatible est une erreur
de flot. `RESTORE` remet ce curseur au début des lignes `DATA`.

## Commandes directes

`NEW`, `LIST`, `RUN`, `RENUM`, `TRACE ON`, `TRACE OFF`, `DUMP`, `PRINT`/`?`, `DEL` et
`BYE` sont disponibles ; `EXIT` est une instruction du programme. Une ligne
numérotée est insérée ou remplacée ; un numéro seul la supprime. `DEL n`
supprime une ligne et `DEL n,m` supprime l’intervalle inclusif de lignes
numérotées `n` à `m`. Les bornes doivent être comprises entre 10 et 2560 et
être des multiples de dix ; une borne finale inférieure à la borne initiale
est rejetée sans écriture partielle. `TRACE ON` affiche `[numéro]` avant chaque
ligne. `RENUM new,old,step` renumérote les lignes actives à partir de `old`
vers `new`, avec l’incrément `step`. V1 exige des paramètres décimaux
multiples de dix dans 10..2560, `new >= old` et une ligne finale dans la plage.
L’opération est prévalidée avant écriture. Les références textuelles `GOTO`,
`GOSUB`, `THEN` et `ON` restent dans les corps : chaque record renuméroté
conserve en `record+8` son numéro précédent, et les résolveurs acceptent le
numéro courant ou cet alias. L’alias est remplacé lors d’un RENUM ultérieur ;
les chaînes de références antérieures à la dernière renumérotation ne sont
pas garanties. Un numéro cible absent reste une erreur à l’exécution.
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

La carte mémoire opérationnelle du moniteur et du payload est la référence
unique [`MEMORY_MAP.md`](MEMORY_MAP.md). Toute nouvelle routine doit y réserver
ses scratchs et ses cadres avant modification du source assembleur.

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
cible avant mutation. Les tableaux de chaînes courts acceptent une ou deux
dimensions ; la forme 2D utilise le même layout de descripteurs borné que les
tableaux de chaînes longs. Les variantes `LET` de ces affectations sont prises
en charge dans les lignes de programme et en mode direct.

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

Les tableaux de chaînes courts acceptent cette même forme calculée en une ou
deux dimensions, par exemple `A$(I+0)` et `A$(I+0,J)`, en lecture, affichage et
affectation. Les tableaux de chaînes longs l'acceptent pour leur première
dimension, y compris dans la forme à deux dimensions
(`LONGGRID$(I+0,J)`) ; la seconde dimension reste limitée à un entier ou à une
variable simple dans cette tranche. Le calcul d'index et les contrôles de
bornes restent exécutés exclusivement dans la cible.
