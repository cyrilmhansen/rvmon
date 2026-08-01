# MiniBASIC-RV V1

## Statut

Le moteur actuel est un programme `no_std` compilé pour
`riscv64gc-unknown-none-elf` et lancé en U-mode par le moniteur. Il ne lit ni
n’interprète aucune donnée sur l’hôte. La version assembleur source, destinée à
être importée puis assemblée par `assemble-program`, reste la prochaine
tranche ; cette limite est volontairement visible et empêche de déclarer le
jalon assembleur complet terminé.

## Grammaire V1

```ebnf
program       = { numbered-line } ;
numbered-line = number , [ space , statement ] ;
statement     = "REM" , text
              | "PRINT" , print-item , { "," , print-item }
              | "INPUT" , variable
              | [ "LET" , space ] , variable , "=" , expression
              | "IF" , expression , "THEN" , number
              | "GOTO" , number
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
MiniBASIC réserve 64 variables binary64 ; une variable lue avant affectation
est créée avec la valeur zéro. Au-delà de 64 noms distincts ou de 16
caractères, l’instruction est rejetée par `BASIC-SYNTAX-001`. `LIST` montre le
texte ; `DUMP` montre slot, adresse, longueur et octets du record, puis les
variables utilisées avec leur motif binary64 et leur affichage fixe.

## Sémantique numérique

Les identifiants sont 64 variables binary64 au maximum. Les opérations `+`, `-`, `*` et `/` sont
effectuées dans le guest ; le chemin `/` contient réellement une instruction
`fdiv.d` (symbole `minibasic_divide`). Les comparaisons produisent `0.0` ou
`1.0`. L’affichage V1 est fixe à six décimales, arrondi au plus proche ; les
valeurs infinies et NaN sont affichées `INF`, `-INF` et `NAN`. Une division par
zéro produit `BASIC-ARITH-001`.

`FOR` utilise une pile de huit frames. Un STEP nul, une pile pleine, une cible
GOTO/THEN absente et une boucle interrompue produisent respectivement
`BASIC-FLOW-003`, `BASIC-FLOW-002`, `BASIC-FLOW-001` et `BASIC-RUN-001`.
Une réponse `INPUT` vide ou syntaxiquement invalide produit
`BASIC-INPUT-001`.

## Commandes directes

`NEW`, `LIST`, `RUN`, `TRACE ON`, `TRACE OFF`, `DUMP`, `PRINT`/`?`, `BYE` et
`EXIT` sont disponibles. Une ligne numérotée est insérée ou remplacée ; un
numéro seul la supprime. `TRACE ON` affiche `[numéro]` avant chaque ligne.
Ctrl-C est détecté par polling pendant `RUN`.

## ABI console cible

Les appels `ecall` utilisent `a7=x17` comme numéro, `a0=x10` comme argument ou
résultat et `a1=x11` pour la longueur : `1=write_char`, `2=read_char`
(bloquant), `3=exit`, `5=poll_char` (zéro si aucun octet, sinon octet). Le
service 4 `write_buffer` est documenté dans `docs/TUTORIAL-GUEST.md`.

## Limites

Pas de chaînes variables, tableaux, DATA/READ, fichiers, GOSUB, fonctions,
exposants, instructions séparées par `:` ni compatibilité Turbo-BASIC. Les
chaînes existent uniquement comme littéraux de PRINT.
