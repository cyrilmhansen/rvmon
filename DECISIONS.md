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
