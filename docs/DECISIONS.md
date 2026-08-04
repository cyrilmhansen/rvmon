# Décisions d’architecture

## D-001 — Séparer ISA, ABI et environnement

**Décision :** trois objets versionnés et trois validateurs séparés. RV64 est l’ISA ; RV64ILP32D-MON-1 est l’ABI locale ; la machine U mono-hart est l’environnement. **Alternative rejetée :** appeler “RV64ILP32” un unique target triple. **Motif :** la psABI est expérimentale [R3, RV64ILP32*]. **Coût :** manifest plus verbeux, mais migration explicite.

## D-002 — Profil V1

**Décision :** RV64IMAFD_Zicsr_Zifencei, little-endian, C optionnel, A hors mono-hart, Zfh/Zfhmin/Q data/decode only, B/V/Zfa/S/M/crypto hors V1. **Alternative :** RV64GC complet ou RV64I minimal. **Motif :** couverture utile sans prétendre implémenter des extensions non validées.

## D-003 — Pointeurs ILP32 sign-extended

**Décision :** `sign_extend_32` partout pour les pointeurs ABI ; adresses ISA explicites restent 64 bits. **Alternative :** zero-extension, rejetée car contraire au snapshot R3. **Coût :** frontière `0x80000000` non intuitive, tests obligatoires.

## D-004 — Soft flags RV64ILP32

**Décision locale :** ILP32 sans F utilise `EF_RISCV_FLOAT_ABI_SOFT=0`; ILP32F=single, ILP32D=double. Toute valeur ELF contradictoire est refusée. **CONFLICT-ABI-001 :** le snapshot de développement présente des formulations de flags/named ABIs incomplètement alignées pour la famille RV64ILP32*, notamment la description du profil sans flottants ; probable coquille ou section inachevée. **Alternative :** recopier le flag observé sans contrôle, rejetée. **Coût :** compatibilité avec un outil draft fautif sacrifiée ; migration possible par manifest.

## D-005 — FLEN et binary128

**Décision :** FLEN exécuté=64 ; binary128 est un format de données/littéral, encode/decode et affichage exacts, pas une instruction Q exécutée. **Alternative :** implémenter Q implicitement via l’hôte, rejetée pour reproductibilité.

## D-006 — Encodages générés

**Décision :** R2 figé par SHA `c6edca7d8c3f92694963a0a0baeb511930fb2af4`, tables générées mask/match/champs/pseudos. **Alternative :** tables manuelles, rejetée pour divergence et maintenance.

## D-007 — Mémoire isolée et limites

**Décision :** RAM virtuelle séparée, un hart, 10 M instructions par défaut, 256 MiB. **Alternative :** exécution dans le processus hôte sans quota, rejetée pour sécurité.

## D-008 — Transactions et historique

**Décision :** toute mutation UI est transactionnelle ; historique inverse borné 100 000 mutations. **Alternative :** undo textuel seulement, rejetée car insuffisant pour mémoire/registre.

## D-009 — Commandes

**Décision :** grammaire unique, expressions à contrôle de largeur, aliases documentés. **Alternative :** compatibilité totale ASM-One, rejetée car ambiguë et destructive.

## D-010 — ELF

**Décision :** format interne canonique ; ELF32 seulement si classe, machine, flags, ABI float et relocations passent les contrôles. **Alternative :** considérer ELFCLASS32 comme preuve d’ILP32, rejetée.

## D-011 — Privilèges

**Décision :** mode U virtuel seulement, CSR nécessaires visibles, traps structurés. **Alternative :** émuler M/S immédiatement, différée pour ne pas multiplier les états.

## D-012 — C

**Décision :** C assemblable/désassemblable/exécutable seulement si activé ; émission automatique off. **Alternative :** toujours relaxer en C, rejetée car adresses, debug et taille changent implicitement.

## D-013 — Flottants

**Décision :** bits conservés dans u64, opérations hôtes remplacées par une sémantique déterministe vérifiée ; affichage hex exact et décimal shortest-round-trip. NaN payload préservée autant que l’instruction l’autorise ; signalant/quiet distingués.

## D-014 — Sources et conflits

**Décision :** R1 prime, R2 automatisable après contrôle, R3 jamais ratifiée, R4 dialecte, A interaction. Chaque divergence reçoit `CONFLICT-*`, règle locale et test.

## D-015 — Profil big-endian expérimental conforme RISC-V

**Décision :** ajouter un profil versionné `RV64IMAFD_Zicsr_Zifencei-BE` séparé
du profil little-endian actuel. L’ISA et les fetchs d’instructions restent les
mêmes ; les accès données M-mode et U-mode utilisent respectivement `MBE=1` et
`UBE=1`, sans mode mixte dans ce profil. Les adresses continuent de croître
octet par octet ; seule la correspondance entre les octets d’une valeur
multi-octets et ces adresses est inversée par rapport au profil LE. La pile
continue donc de croître vers les adresses basses dans les deux profils. Les
valeurs multi-octets, les champs de pile, les structures ABI, les snapshots de
registres et les objets ELF utilisent `ELFDATA2MSB`. Les directives numériques
se sérialisent selon l’endianess du profil ; une directive de suite d’octets
reste indépendante de l’endianess.

Le profil d’appel BE du moniteur est local et expérimental : la psABI RISC-V
actuelle ne définit pas encore de convention d’appel big-endian. Il est donc
nommé `RV64ILP32D-MON-1-BE`, et aucune compatibilité GNU/LLVM BE ne sera
annoncée sans preuve par une toolchain et un ELF interopérables.

**Alternative :** ajouter un simple drapeau d’affichage ou inverser les octets
dans QEMU depuis le moniteur, rejetée car elle ne modifie ni les loads/stores ni
la représentation ABI et donnerait un faux mode BE.

**Alternative :** rendre le profil LE/BE dynamique à chaque commande, rejetée
pour V1-BE ; le changement d’endianess est une propriété de profil/runtime et
non une mutation de l’interface utilisateur.

**Coût :** double matrice de tests, format de projet/snapshot versionné avec
endianess explicite, toolchain et ELF BE à qualifier séparément. Le profil BE
reste expérimental tant qu’un compilateur/linker produisant un ELF RISC-V BE
compatible n’est pas démontré.

## D-017 — Archiver Zfh/Zfhmin/Q sans annoncer leur exécution

**Décision :** archiver les fichiers `rv_zfhmin`, `rv_zfh`, `rv64_zfh`, `rv_q`
et `rv64_q` du commit R2 déjà figé, et les inclure dans la génération des
tables profile-aware. Le décodeur les représente comme
`GeneratedInstruction`, le désassembleur les réémet sous forme `.word` avec
leur mnémonique de diagnostic, et l’encodeur préserve les bits validés par R2.
Le moteur RV64IMAFD refuse ces instructions avec un diagnostic
`TRAP-UNSUPPORTED-EXTENSION`. Les formats binary16 et binary128 restent
disponibles dans les directives et les vues de bits ; cela ne constitue pas
une implémentation arithmétique Zfh ou Q.

**Alternative :** laisser ces mots illégaux, ou les exécuter via les types
flottants de l’hôte, rejetée : la première perd la capacité de décodage/export
prévue par le profil, la seconde viole la reproductibilité et la séparation
parse/assemble/decode/execute.

**Coût :** le registre généré augmente et les outils doivent distinguer
`GeneratedInstruction` de `Illegal`; l’assemblage des formes réelles passe par
les champs générés, tandis que les pseudo-instructions et le désassemblage
canonique des opérandes restent une tranche ultérieure explicitement versionnée.

## D-016 — Oracle sémantique F/D par probe QEMU versionné

**Décision :** utiliser QEMU user-mode RISC-V 11.0.2 comme oracle externe
indépendant pour un corpus de probes fadd.s/fadd.d. Le probe est un binaire
RISC-V autonome utilisant uniquement les syscalls Linux write et exit; il ne
partage ni le code d’opération, ni le moteur flottant, ni les fonctions Rust
de luna-machine. La comparaison porte sur le motif de résultat et les bits
fflags.

**Alternative :** utiliser les opérations flottantes de l’hôte ou comparer le
moteur Rust à lui-même, rejetée car elle ne fournit pas d’oracle indépendant.
SoftFloat, Sail et Spike restent des oracles complémentaires à qualifier, pas
des dépendances du runtime.

**Coût :** dépendance de test QEMU 11.0.2, probe Linux RV64 à maintenir et
couverture initiale limitée à huit cas RNE F/D. Une absence ou une divergence
QEMU bloque la validation sémantique et est rapportée comme unsupported, pas
convertie en succès.

## D-018 — Représentation cible des chaînes et tableaux MiniBASIC

**Décision :** les chaînes et tableaux sont des objets du runtime cible, jamais
des objets ou des pointeurs de l’hôte. Une chaîne est un descripteur aligné sur
8 octets de 24 octets :

StringDesc {
    data_addr: u64,   // adresse cible RV64, ou 0 pour la chaîne vide
    length:    u64,   // nombre d’octets ASCII
    capacity:  u64,   // capacité réservée dans le pool cible
}

data_addr est une adresse de la machine cible et non un pointeur C de
l’ABI RV64ILP32. Le runtime ne convertit donc pas silencieusement une adresse
par extension de signe ; toute conversion entre handle interne et pointeur ABI
doit être explicite et contrôlée. Les copies vérifient length <= capacity
avant toute écriture.

Un tableau est un descripteur aligné sur 8 octets de 64 octets, avec un rang
maximal de 4 :

ArrayDesc {
    data_addr:    u64,
    element_cnt:  u64,
    element_size: u32,
    rank:         u32,
    element_kind: u32,  // binary64 ou StringDesc
    flags:        u32,
    dimensions:   u64[4], // bornes supérieures inclusives
}

DIM A(10) crée onze éléments, indexés de 0 à 10, et le stockage est
row-major. Pour un tableau de chaînes, un élément est un StringDesc de
24 octets. Le produit des dimension + 1, le calcul de taille et chaque index
sont contrôlés avant l’accès. Les budgets de pool et d’éléments restent des
limites de configuration du payload et produisent un diagnostic stable plutôt
qu’une allocation hôte.

**Alternative :** stocker des pointeurs hôte, utiliser des chaînes terminées
par zéro ou choisir une représentation dépendante de l’ABI C, rejeté pour
l’isolation, les octets nuls, la reproductibilité et le futur profil BE.

**Coût :** 24 octets par variable chaîne, descripteurs de tableaux de taille
fixe et une copie explicite lors d’une affectation de chaîne ; les tableaux
dynamiques et les chaînes Unicode sont différés.

## D-019 — Compilateur MiniBASIC comme backend natif optionnel

**Décision :** planifier un compilateur explicite, postérieur au chemin
interprété, qui partage le lexer/parser mais produit un payload RV64 natif,
un runtime cible versionné, des symboles et une carte source. Il ne remplace
pas `RUN` et ne délègue aucune analyse ou évaluation à l’hôte. L’artefact
`.luna` ou payload brut contrôlé est prioritaire sur ELF externe.

**Alternative :** compiler uniquement côté hôte vers une VM, ou remplacer
l’interpréteur par un compilateur transparent ; rejetée car cela empêcherait la
preuve d’exécution cible et supprimerait l’oracle différentiel interprété /
compilé.

**Référence historique :** l’étude couvre Turbo-BASIC XL 1.5, Compiler 1.1,
Runtime, Linker, les manuels, images et désassemblage MADS. Le source original
étant déclaré perdu dans les références disponibles, aucun code historique
n’est une dépendance ou une source à recopier.

**Coût :** AST/IR, backend RV64, runtime/linker, source mapping, manifeste et
une matrice de tests distincte ; estimation initiale 20–35 journées-agent,
hors optimisations et compatibilité ELF externe.
