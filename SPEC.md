# Moniteur-assembleur RV64ILP32 — spécification fonctionnelle et technique

**Statut :** proposition normative V1, gel architectural. **Version :** 1.0.0-draft, 2026-07-31. Les mots *doit*, *ne doit pas*, *devrait* et *peut* ont leur sens d’exigence.

## 1. Résumé et vision

Le produit est une application locale, mono-utilisateur et hors ligne qui assemble, charge, inspecte et exécute un programme dans une machine virtuelle RISC-V déterministe. Il reprend le cycle immédiat d’ASM-One — ligne de commande centrale, éditeur, assemblage en mémoire, désassemblage, moniteur hexadécimal/ASCII, exécution et débogage — tout en ajoutant isolation, transactions, snapshots, diagnostics structurés et export reproductible. La référence historique confirme la séparation assembleur, désassembleur, éditeur, debugger et moniteur [Aminet ASM-One, description, lignes 26–50; A1, ch. 6–9].

Le produit distingue strictement :

* **ISA** : instructions et état architectural RV64 sélectionnés ;
* **ABI** : convention de représentation et d’appel RV64ILP32 locale, explicitement expérimentale ;
* **environnement d’exécution** : mémoire, harts, privilèges, traps, MMIO et appels d’environnement de la machine virtuelle.

## 2. Objectifs, non-objectifs, utilisateurs

### Objectifs mesurables

* **REQ-PROD-001** : un utilisateur doit pouvoir éditer, assembler, charger puis exécuter un programme de moins de 64 KiB en moins de trois actions principales et en moins de 250 ms sur une machine de référence ;
* **REQ-PROD-002** : toute exécution identique, depuis le même snapshot et avec le même profil, doit produire le même état final et la même trace ;
* **REQ-PROD-003** : toute modification mémoire ou registre doit être annulable au niveau transactionnel ;
* **REQ-PROD-004** : toute erreur d’assemblage doit pointer fichier, ligne, colonne, code stable, gravité, cause et remède ;
* **REQ-PROD-005** : le clavier doit couvrir 100 % des commandes et des opérations de navigation principales ;
* **REQ-PROD-006** : une session doit exporter un cas reproductible autonome contenant profil, sources, image, symboles, état initial, limites et journal.

### Non-objectifs V1

Pas de cible distante/native, de système d’exploitation complet, de MMU/S-mode/M-mode complet, de multi-utilisateur, de compilation C, de linker ELF généraliste, de V exécuté, ni de certification d’un ABI ratifié RV64ILP32. L’export ELF32 est limité et optionnel ; le format interne est canonique.

### Utilisateurs

Débutant qui apprend l’ISA ; programmeur assembleur qui veut un cycle edit–test rapide ; développeur de simulateur qui veut un oracle inspectable ; enseignant qui veut des états et traces reproductibles.

## 3. Glossaire strict

**ISA** : contrat des instructions, encodages, registres et effets observables. **ABI** : contrat de représentation des objets et des appels entre unités compilées. **XLEN** : largeur des registres entiers et de l’adresseur architectural, 64 en V1. **FLEN** : largeur maximale des registres flottants du profil, 64 en V1 même si binary128 est manipulable comme donnée. **Adresse** : entier RV64 utilisé par la mémoire virtuelle ; **pointeur** : valeur ABI de 32 bits représentée par extension de signe dans un registre RV64. **Mot** : 32 bits ; **double mot** : 64 bits. **Format flottant** : format IEEE 754 binary16/32/64/128 ; bfloat16 est distinct et non exécuté. **Moniteur** : vues et opérations directes sur mémoire/état. **Débogueur** : contrôles d’exécution, arrêts et corrélation source-adresse. **Backend** : implémentation de cible exposant l’état et les opérations d’exécution au frontend.

## 4. Sources, versions et conflits

Ordre d’autorité : R1 ISA ratifiée v20260120 ; R2 encodages figés ; R3 psABI de développement figée ; R4 dialecte ; R5 oracle ; A1/A2 interaction ; C1 index uniquement. Les liens de référence sont [R1](https://docs.riscv.org/reference/home/index.html), [R2](https://github.com/riscv/riscv-opcodes/tree/c6edca7), [R3](https://github.com/riscv-non-isa/riscv-elf-psabi-doc), [R4](https://github.com/riscv-non-isa/riscv-asm-manual), [R5](https://github.com/riscv/sail-riscv), [A1](https://archive.org/stream/AsmOne1.02Manual/Asm-One1.02Manual_djvu.txt), [Aminet](https://aminet.net/package/dev/asm/ASM-One).

R2 est gelé sur le commit `c6edca7d8c3f92694963a0a0baeb511930fb2af4` (11-07-2026) ; les tables sont générées, jamais recopiées. R3 est gelé sur `76b837ec964509f4bac11c66e5d7106b6a1e626a` (snapshot du 31-07-2026), et reste une spécification de développement expérimentale. R4 est une spécification non ratifiée de dialecte. R5 ne remplace jamais R1.

Une divergence est enregistrée dans `DECISIONS.md`, reçoit un identifiant `CONFLICT-*`, une règle locale versionnée et un test. Aucun changement silencieux. La règle locale est réversible par migration de projet.

## 5. Profil ISA et ABI

### 5.1 Profil gelé

`rv64imafd_zicsr_zifencei`, little-endian, un hart, mode U virtuel, `XLEN=64`, `FLEN=64`, ABI `RV64ILP32D-MON-1`. Les instructions F et D s’exécutent ; Q, Zfh et Zfhmin sont représentables et décodables mais non exécutés en V1. `C` est assemblable/désassemblable et exécutable si activé par projet ; émission automatique désactivée par défaut. `A` est décodable et assemblable mais son exécution exige le profil multi-hart optionnel, absent du profil par défaut. B, V, Zfa, privilèges S/M complets et crypto sont hors profil.

### 5.2 ISA/ABI/environnement

R1 définit l’ISA et XLEN, pas la taille des pointeurs de langage. R3 indique que RV64ILP32* est expérimental et que les pointeurs scalaires 32 bits, étant plus étroits que XLEN, sont sign-extended vers XLEN [R3, `riscv-cc.adoc`, section “RV64ILP32* Calling Convention”]. La psABI ratifiée v1.0 de 2022 ne doit donc pas être citée comme ratifiant RV64ILP32.

Types ABI V1 : `char=1/1`, `short=2/2`, `int=4/4`, `long=4/4`, `long long=8/8`, pointeur=4/4, `size_t=4/4`, `float=4/4`, `double=8/8`, `long double=16/16` comme objet de données seulement ; notation taille/alignement en octets. Les agrégats suivent leur alignement naturel, padding explicite, et alignement de pile 16 octets. Un pointeur transmis en registre ou pile est la valeur 32 bits signée étendue : `0x80000000` devient `0xffffffff80000000` dans x-register.

L’espace pointeur visible est `0x00000000..0xffffffff` modulo la convention de représentation. La traduction locale est `logical32 -> rv64 = sign_extend_32(logical32)`, donc deux fenêtres RV64 disjointes sont possibles ; V1 n’autorise que la fenêtre canonique basse `0x00000000..0x7fffffff` et la fenêtre signée haute `0xffffffff80000000..0xffffffffffffffff`. `0x80000000..0xffffffff` est accepté comme valeur logique mais toute allocation ou accès doit être déclaré dans la fenêtre haute correspondante. Aucun cast pointeur→adresse ne tronque silencieusement.

Conventions : `a0..a7` arguments, `a0..a1` retours, `s0..s11` callee-saved, `sp` aligné à 16, pile décroissante ; avec `ILP32D`, les arguments flottants nommés admissibles utilisent `fa0..fa7`, les valeurs plus étroites sont NaN-boxées, mais les variadiques utilisent la convention entière [R3, `riscv-cc.adoc`, sections Integer/Hardware Floating-Point Calling Convention]. Les valeurs de 64 bits occupent deux mots ABI selon l’ordre low/high ; les structures suivent leur layout mémoire ; les retours indirects utilisent un pointeur caché en premier argument. Les outils GNU/LLVM actuels ne sont pas supposés compatibles : ils peuvent accepter RV64ILP32 selon version/patch, mais la chaîne V1 doit vérifier `-mabi` et refuser tout ELF ou relocation non conforme.

ELF : V1 interne utilise un conteneur `LUNA-RV64ILP32`; l’import ELF doit accepter seulement les identifiants cohérents avec `ELFCLASS32`/flags observés et l’ABI déclaré, sinon diagnostic. Un ELF32 ne prouve pas à lui seul l’ABI : `EI_CLASS`, `e_flags`, machine, ABI float et relocations doivent concorder.

### 5.3 Matrice de conformité

| Extension | Parse | Assemble | Désassemble | Exécute | Débogue | Teste | Exporte |
|---|---|---|---|---|---|---|---|
| I | oui | oui | oui | oui | oui | oui | brut/ELF interne |
| M | oui | oui | oui | oui | oui | oui | idem |
| F/D | oui | oui | oui | oui | oui, fcsr/flags | motifs IEEE | idem |
| Zicsr/Zifencei | oui | oui | oui | oui | oui | oui | idem |
| C | oui | oui explicite | oui | oui si `C=on` | oui | oui | brut |
| A | oui | oui | oui | non V1 mono-hart; oui profil `A-MH1` | oui | oui différé | idem |
| Zfh/Zfhmin | oui | oui données/instructions | oui | non V1 | registres exacts | encode/decode | données |
| Q | oui | oui données | oui | non V1 | affichage exact | encode/decode | données |
| Zfa/B/V | erreur structurée ou mode expérimental isolé | non V1 | non V1 | non | non | corpus de rejet | non |

Le mot “support” est interdit sans les colonnes ci-dessus.

## 6. Environnement d’exécution

Un hart virtuel est constitué de `x[32]:u64`, `pc:u64`, `f[32]:u64`, `fcsr:u32`, mémoire, périphériques et compteur d’instructions. `x0` reste zéro. Les CSR visibles V1 sont `fflags`, `frm`, `fcsr` et les CSR nécessaires à `Zicsr`; les CSR privilégiés sont absents et toute tentative produit `TRAP_CSR_UNIMPLEMENTED`. Mode U seulement ; pas de MMU, interruptions ou concurrence V1. Les traps sont des arrêts structurés avec cause, PC, adresse fautive et instruction.

Mémoire little-endian, pages logiques de 4 KiB, taille configurable par projet mais bornée à 256 MiB. RAM `0x00000000..ram_end`; pile réservée au sommet ; MMIO `0xffff0000..0xffffffff`, lecture/écriture via périphériques déterministes explicitement déclarés. Adresse non mappée, accès hors limites, instruction mal alignée et accès mal aligné suivent R1, chapitre Load/Store et exceptions ; V1 lève un trap plutôt que de simuler une extension hôte.

Le temps est un compteur d’instructions exécutées ; aucun wall-clock, thread ou valeur aléatoire n’est visible. `ecall` expose uniquement `exit`, `write-console` et `read-input` avec entrée enregistrée ; les autres appels sont des traps. Chaque run a une limite configurable par défaut de 10 millions d’instructions et 256 MiB, puis `TRAP_RESOURCE_LIMIT`.

## 7. Carte et adressage 32/64 bits

| Valeur logique | Registre RV64 après ABI | Interprétation |
|---|---|---|
| `0x7fffffff` | `0x000000007fffffff` | dernière valeur basse |
| `0x80000000` | `0xffffffff80000000` | première valeur signée haute |
| `0xffffffff` | `0xffffffffffffffff` | -1, pas `0x00000000ffffffff` |

Une adresse ISA RV64 écrite explicitement (`ld t0, 0xffffffff80000000`) n’est pas une conversion ABI : elle est utilisée telle quelle. Une expression `ptr32(symbol)` vérifie que le symbole est dans une fenêtre et produit la sign-extension ; `addr64(symbol)` conserve 64 bits. Les additions de pointeurs sont calculées en 32 bits signés puis vérifiées avant conversion. Exemple erroné : `la a0, 0x0000000080000000` pour un pointeur ILP32 ; le résultat doit être obtenu par symbole logique ou refusé.

## 8. Modèle d’état

```text
MachineState = {
  profile: ProfileId, hart: HartState[1], memory: MemoryMap,
  devices: DeviceState, symbols: SymbolTable, breakpoints: BreakpointSet,
  watches: WatchSet, history: HistoryId, instruction_count: u64
}
HartState = { x: u64[32], pc: u64, f: u64[32], fcsr: u32, csrs: map<u12,u64> }
```

Invariant : `x[0]=0`, `fcsr=(frm<<5)|fflags`, toutes les écritures passent par validation, snapshot immuable et transaction. Les registres f sont conservés comme bits 64 ; la vue interprète binary16/32/64 selon la demande. Symboles contiennent nom, portée, section, adresse, taille et source. Un breakpoint contient adresse ou symbole, condition, compteur de hits et action. Un watchpoint contient plage, lecture/écriture et condition.

## 9. Architecture et contrats

Frontend appelle `CommandService`; Editor produit `SourceDocument`; Assembler produit `ObjectImage + Diagnostics`; Loader résout sections/symboles/relocations et produit `LoadedImage`; Disassembler consomme bytes+profile et produit `DecodedItem`; Backend expose `read_state`, `write_transaction`, `run`, `pause`, `step`, `snapshot`, `restore`; Debugger orchestre arrêts ; MemoryView ne mutile jamais directement la RAM ; FileService vérifie les chemins.

Le backend V1 est `DeterministicRv64Backend`. Le contrat futur interdit d’exposer des types UI : toute cible doit déclarer capabilities, endianness, XLEN, extensions, latence et garanties de snapshot. Une opération de mutation retourne `CommitId` ou diagnostic, jamais un état partiellement écrit.

## 10. Ligne de commande

EBNF :

```ebnf
line = [ command , { ws , argument } ] ;
command = identifier ; argument = range | expression | string | identifier ;
range = expression , [ ".." , expression ] ;
expression = unary , { ("+"|"-"|"*"|"/"|"<<"|">>"|"&"|"|"|"^") , unary } ;
unary = ["+"|"-"|"~"] , atom ;
atom = number | symbol | register | "(" , expression , ")" ;
```

Expressions sont entières signées 128 bits pendant l’évaluation, puis contrôlées par contexte. Commandes : `asm [file]`, `build`, `load image`, `run [until expr]`, `pause`, `si`, `so`, `su`, `reset`, `regs [x|f|csr]`, `set x5=expr`, `mem [b|h|w|d] range`, `edit addr=value`, `dis addr[..end]`, `break label|addr [if expr]`, `watch range [r|w]`, `trace on|off`, `snapshot name`, `restore name`, `undo`, `redo`, `symbols`, `find`, `fill`, `copy`, `save`, `open`, `help`, `quit`. Alias : `s=si`, `r=run`, `b=break`, `m=mem`, `u=undo`.

Exemples : `asm main.s`; `break add`; `run until a0 == 42`; `mem w 0x1000..0x103f`; `set a0=ptr32(0x80000000)`; `dis pc..pc+32`. Erreurs : commande inconnue `CMD-001`, argument manquant `CMD-002`, expression invalide `CMD-003`, plage inversée `CMD-004`, mutation pendant run `CMD-005`. `help` affiche syntaxe, exemples, profil et codes sans effet de bord.

## 11. Dialecte assembleur

ASCII Unicode NFC seulement dans commentaires, chaînes et identifiants quotés ; mots-clés et registres ASCII insensibles à la casse ; commentaires `#` et `//` jusqu’à fin de ligne. Registres `x0..x31`, `f0..f31` et aliases ABI exacts ; `zero`, `ra`, `sp`, `gp`, `tp`, `t0..t6`, `s0..s11`, `a0..a7`, `ft0..ft11`, `fs0..fs11`, `fa0..fa7`.

Labels globaux : `[A-Za-z_.$][A-Za-z0-9_.$]*:` ; locaux `.Lname` ont portée fichier/section jusqu’au prochain global. Littéraux `0x`, `0b`, décimal, apostrophes séparateurs, chaînes échappées. Précédence : `|`, `^`, `&`, shifts, `+/-`, `*/%`, unaire. Les valeurs hors largeur sont erreurs sauf troncature explicitement demandée (`lo12`, `hi20`, `u8`), jamais wrap implicite.

Instructions réelles et pseudo-instructions R4 sont importées depuis les tables R2 ; la forme canonique est stockée. Pseudos V1 : `nop`, `mv`, `li`, `la`, `call`, `ret`, `j`, `jr`, branches de comparaison zéro et CSR usuels. Une pseudo-instruction est toujours listée avec expansion et bytes.

Directives : `.text`, `.rodata`, `.data`, `.bss`, `.section name, flags`, `.byte`, `.half`, `.word`, `.dword`, `.binary16`, `.float`, `.double`, `.binary128`, `.ascii`, `.asciz`, `.string`, `.align`, `.balign`, `.equ`, `.set`, `.include`, `.macro/.endm`, `.if/.elseif/.else/.endif`, `.global`, `.local`, `.extern`, `.option rvc/norvc`, `.profile`. `.float`=binary32, `.double`=binary64 ; `.binary16` et `.binary128` sont obligatoires pour lever toute ambiguïté ; bfloat16 s’appelle `.bfloat16` et n’est pas confondu.

Deux passes minimum : collecte sections/symboles puis émission/relocations ; une passe de relaxation contrôlée peut remplacer une expansion seulement si les bytes et la carte des relocations restent validés. Les symboles non résolus sont autorisés uniquement avec relocation exportable. Les tables d’encodage et champs sont générées de R2 (`mask`, `match`, champs variables, pseudo/import) puis comparées à R1.

Compatibilité GNU/LLVM : dialecte GNU accepté en mode `gnu-compat` avec avertissements ; syntaxe non standard est préfixée `luna.` ou refusée. Les différences de relocations, RV64ILP32 et flags ABI sont documentées et testées contre `as/objdump` et `llvm-mc`, jamais déduites d’un succès de parsing.

### 11.1 Littéraux et sémantique flottante

Les quatre formats sont des formats IEEE 754 distincts : binary16 (1/5/10 bits), binary32 (1/8/23), binary64 (1/11/52) et binary128 (1/15/112). `bfloat16` (1/8/7) n’est jamais accepté par `.binary16` et reste une donnée non exécutable. Chaque directive accepte une liste de valeurs séparées par virgule :

* décimal (`1.5`, `-0.0`, `1e-40`) converti dans le format demandé ;
* hexadécimal numérique (`0x1.8p+1`) avec exposant binaire ;
* motif exact (`bits16(0x8000)`, `bits32(0x7fc00001)`, `bits64(...)`, `bits128(0x...)`).

Les motifs ont priorité sur toute conversion et exigent exactement la largeur (les zéros initiaux sont permis). Les décimaux utilisent l’arrondi `RNE` par défaut ; `.round rtz|rne|rdn|rup|rmm` est local au littéral et n’altère pas `frm`. Une valeur finie non représentable émet `FP-ROUND-001` avec le motif produit et le mode ; elle n’est pas silencieusement tronquée. Les valeurs `±0`, `±inf`, subnormaux, sNaN et qNaN sont acceptées par nom (`inf`, `nan`, `snan`) avec payload facultative ; pour une garantie de payload, `bitsN` est obligatoire.

Le moteur conserve le résultat comme bits et calcule les exceptions `NV`, `DZ`, `OF`, `UF`, `NX` dans `fflags` selon R1, chapitre F “Floating-Point Control and Status Register”. `frm` sélectionne RNE, RTZ, RDN, RUP ou RMM ; les valeurs réservées de `frm` sont `FP-CSR-001`. Un encodage statique (`rm` dans l’instruction) et dynamique (`rm=111`, `frm`) sont distingués dans la trace. `fflags` est sticky : une opération ajoute ses flags, une écriture CSR peut les effacer.

Une valeur binary32 rangée dans un registre f64 doit être NaN-boxée par bits hauts à 1 ; une valeur non boxée est invalide pour les opérations étroites et produit le résultat/flag défini par R1, jamais une interprétation dépendante de l’hôte. Les conversions entier↔flottant et format↔format sont testées pour signe, saturation IEEE, arrondi et flags. binary128 peut être chargé dans la mémoire, affiché et exporté comme 16 octets little-endian, mais aucune instruction Q n’est exécutable V1.

Affichage : colonne hex exacte toujours disponible ; colonne décimale en shortest-round-trip pour les valeurs finies, avec `-0`, `inf`, `nan` et indication sNaN/qNaN/payload. L’affichage ne modifie jamais les bits. Les résultats doivent être identiques sur hôte x86_64 et arm64 ; les bibliothèques hôte ne peuvent décider seules de l’arrondi ou du payload. Erreurs et avertissements sont `FP-LITERAL-*`, `FP-ROUND-*`, `FP-CSR-*`, `FP-BOX-*`.

## 12. Formats

Source UTF-8 avec manifest de profil ; projet `.luna` versionné, JSON canonique UTF-8 sans horodatage ; binaire brut avec adresse de base et endian explicites ; image mémoire sparse ; symboles TSV/JSON ; listing avec source, adresse, bytes, expansion et relocation ; map ; snapshot compressé et hashé ; export reproductible contenant tous les éléments. ELF32 éventuel est lecture/écriture limitée, avec refus explicite si l’ABI ou relocations divergent.

## 13. Désassembleur

Le décodeur essaie 16 bits si `C=on` et bits bas indiquent une instruction compressée, sinon 32 bits ; longueurs non admises produisent `DIS-ILLEGAL-LENGTH`. Il applique `mask/match` R2 et les contraintes de profil. Un opcode illégal est un item visible, jamais des données silencieuses. Affichage par défaut canonique réel ; option `--pseudo` propose un alias seulement si expansion bijective et annotée. Les symboles sont affichés sans changer l’adresse. Les zones données sont déterminées par sections/marks ; code et données mêlés restent navigables. Les instructions C 16 bits et 32 bits partagent le même curseur d’octets.

## 14. Moniteur mémoire

Vues désassemblée, hexadécimale et ASCII partagent un curseur d’adresse et une sélection. `QuickJump` accepte symbole, registre ou expression. Marks nommés sont persistants dans le projet. Édition, remplissage, copie et recherche sont transactions ; une sélection illégale n’est jamais partiellement appliquée. ASCII affiche octets imprimables Unicode-safe mais édite des octets, pas des caractères hôte. `undo` restaure la transaction mémoire et ses métadonnées.

## 15. Registres

Groupes `x`, `f`, `csr`; alias ABI affichés avec `xN`; changements depuis le dernier arrêt sont surlignés. Édition accepte entier hexadécimal exact, signé, ou vue flottante explicitement choisie et valide NaN-boxing. `fcsr`, `frm`, `fflags` sont montrés en champs et en hex. x0 est non éditable ; écriture d’un CSR absent est rejetée.

## 16. Débogueur

`run`, pause coopérative à chaque instruction, `step-into` une instruction, `step-over` saute l’appel courant, `step-out` surveille la profondeur `ra/sp`. Breakpoints adresse/symbole/ligne, watchpoints RAM/MMIO, conditions sans effets de bord, compteurs de hits. Un trap arrête avant l’instruction suivante et montre cause/PC/adresse. Source↔adresse vient de la table de lignes. Pile : heuristique `sp/fp`, avec avertissement si absence de frame. Historique arrière est retenu V1 sous forme de journal inverse borné à 100 000 mutations ; il est désactivable pour performance.

## 17. Éditeur et workspace

Jusqu’à 16 documents ouverts, numéros de ligne stables, navigation vers diagnostics, sauvegarde automatique transactionnelle, restauration après crash via journal. Raccourcis : `Ctrl+Enter` assemble/charge, `F5` run, `F10` step-over, `F11` step-into, `Shift+F11` step-out, `Ctrl+Z/Y`, `Ctrl+G` QuickJump, `Ctrl+F` find. L’état UI n’altère jamais l’état cible sans commande explicite.

## 18. Isolation

La mémoire cible n’est jamais un pointeur ou buffer de l’hôte. Chaque run dispose de limites CPU, RAM, profondeur de commande et récursion macro. Les includes restent dans le workspace ou une allowlist ; chemins absolus, traversée `..`, liens sortants et fichiers spéciaux sont refusés par défaut. Après faute, la machine est arrêtée mais le snapshot précédent reste restaurable. Une commande hôte n’est jamais exécutable depuis le code cible.

## 19. Erreurs et diagnostics

```text
Diagnostic = { code:string, severity:error|warning|info, file?:string,
 line?:u32, column?:u32, span?:u32, message:string,
 cause?:string, remedy?:string, related:[Location], profile:string }
```

Codes réservés : `PARSE-*`, `ASM-*`, `REL-*`, `DIS-*`, `EXEC-*`, `TRAP-*`, `MEM-*`, `ABI-*`, `CMD-*`, `IO-*`, `SEC-*`. Les codes sont stables ; le texte est localisable mais le code, champs et gravité ne changent pas en patch release.

## 20. Exigences non fonctionnelles

Assemblage médian <250 ms et p95 <1 s pour 64 KiB ; latence commande/affichage <50 ms p95 hors run ; snapshot <200 ms pour 256 MiB sparse ; déterminisme bit-à-bit sur Linux/macOS/Windows x86_64 et arm64 ; empreinte installée <100 MiB hors symboles ; RAM UI <256 MiB au repos ; exécution limitée sans gel hôte ; contraste WCAG AA, navigation clavier complète, taille de police configurable. Les nombres sont des critères d’acceptation, non des promesses pour corpus pathologique.

## 21. Observabilité

Journal append-only des commandes avec résultat, snapshot, profil et hash ; trace optionnelle par instruction avec PC, bytes, changements x/f/CSR, mémoire et trap ; export d’un cas reproductible. Les entrées utilisateur et chemins peuvent être anonymisés, mais l’anonymisation est visible dans le manifest.

## 22. Compatibilité et évolution

Le projet porte `schema_version`, `profile_id`, versions R1–R5 et hashes de tables. Migration mineure automatique, majeure explicite avec sauvegarde. Un nouveau profil ne modifie jamais la sémantique d’un profil existant. Le statut d’une extension est `ratified`, `draft`, `experimental-local`, `unsupported`.

## 23. Vérification

Tests unitaires lexer/parser/expressions, génératifs encode↔decode, masques R2, relocations, macros et transactions ; différentiels GNU `as/objdump` et LLVM `llvm-mc` quand ces outils acceptent le même profil ; comparaison Sail/Spike lorsque la configuration couvre l’instruction ; tests de traps, limites, C et opcode illégal ; fuzzing parser, désassembleur et commandes ; corpus de non-régression versionné. Flottants testés par motifs bits, IEEE 754 limites, flags, NaN-boxing et hôte multiple.

## 24. Scénarios d’acceptation bout-en-bout

1. `addi a0, zero, 40; addi a0,a0,2` assemble, charge, exécute et affiche `a0=42`.
2. Modifier l’instruction à `pc` via moniteur, constater le nouveau résultat, `undo`, constater les bytes et résultat initiaux.
3. Naviguer code→hex→ASCII→code en conservant l’adresse et la sélection.
4. `break loop`, run, arrêt sur label et surlignage de `a0` modifié.
5. `fadd.s` avec binary32 et `fadd.d` avec binary64 produit bits et `fflags` exacts.
6. Charger un operand non NaN-boxé dans une opération F étroite déclenche le comportement architectural spécifié et un diagnostic visible.
7. `.binary128 0x...` conserve 128 bits, listing et snapshot sans conversion hôte.
8. Le pointeur logique `0x80000000` s’affiche `0xffffffff80000000`, traverse la convention ABI et n’est pas tronqué.
9. Une instruction C 16 bits suivie d’une instruction 32 bits est correctement décodée et pas-à-pas.
10. Un opcode avec aucun `mask/match` actif produit `DIS-ILLEGAL-*` et un trap à l’exécution.
11. Une boucle infinie s’arrête à la limite d’instructions avec `TRAP_RESOURCE_LIMIT`, UI responsive et snapshot intact.
12. Réouvrir un projet restaure source, profil, symboles, bytes, breakpoints et état cible identiques par hash.
13. `fcsr.frm` dynamique et statique donnent le même arrondi demandé ; `fflags` est accumulé puis effacé uniquement par écriture explicite.
14. Une include hors workspace est refusée `SEC-INCLUDE-001` sans lire le fichier.

## 25. Repris, modernisé, rejeté; risques

Repris d’ASM-One : ligne de commande centrale, éditeur rapide, assembleur en mémoire, désassembleur, vues hex/ASCII/binaire, marks, debugger source, run/step/watch/breakpoints et sauvegarde de zones [A1 ch. 6–9; Aminet]. Modernisé : snapshots, undo transactionnel, profil ISA explicite, diagnostics structurés, isolation, determinisme, limites, traces et formats versionnés. Rejeté : accès direct au matériel Amiga, secteurs/pistes, dépendance graphique native, exécution PPC/68k, commandes ambiguës destructives et toute cohabitation ISA non déclarée.

Risques : psABI RV64ILP32 instable ; outils GNU/LLVM partiellement compatibles ; divergences R2/R1 ; complexité IEEE 754/NaN-boxing ; performance de l’historique ; ambiguïtés ELF32 ; diagnostic de code/données mêlés. Dettes assumées : V1 ne simule pas V/Q/Zfa/V ; ELF externe limité ; pas de multi-hart par défaut ; mapping haut de pointeurs documenté mais peu portable.

### Audit de couverture

Estimation au gel : **88 % des décisions fondamentales figées**, **6 % des exigences sans source normative directe** (UI, quotas, format interne), **4 % sans test détaillé** (migration/portabilité UI), **2 % d’incohérences restantes** concentrées dans le snapshot psABI et la compatibilité ELF32. Les conflits et choix locaux sont listés dans `DECISIONS.md`; les décisions à fort impact restantes dans `OPEN_QUESTIONS.md`.

### Registre de références de section

R1 : Volume I, chapitres “Introduction”, RV64I, M, A, F, D, Q, C, Zicsr, Zifencei, Loads/Stores, Memory Model et Exceptions. R2 : README, “Encoding Syntax”, “mask/match”, pseudo-ops/imports et génération d’artefacts. R3 : `riscv-cc.adoc`, “Register Convention”, “Integer Calling Convention”, “Hardware Floating-Point Calling Convention”, “RV64ILP32*”; `riscv-elf.adoc`, ELF flags/classes. R4 : sections pseudo-instructions/directives/relocations. R5 : README et configuration de l’émulateur. A1 : chapitres 6–9.
