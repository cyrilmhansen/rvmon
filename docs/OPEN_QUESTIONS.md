# Questions ouvertes à fort impact

Toutes les décisions ISA/ABI nécessaires à V1 ont un défaut choisi. Une question ne bloque le plan que si le choix recommandé est refusé.

1. **Différable : stratégie de flags draft et ELF BE externe.** Recommandation : appliquer D-004, refuser toute ambiguïté et ne pas faire dépendre la release du profil ELF big-endian externe. Bloque uniquement l’interopérabilité ELF externe ; le runtime interne LE/BE, l’assembleur natif et les fixtures restent indépendants.
2. **Différable : activer A multi-hart.** Recommandation : conserver mono-hart V1 ; réserver `A-MH1` aux tests ultérieurs. Bloque l’exécution atomique, pas l’assemblage/décodage.
3. **Différable : inclure un frontend Q exécuté.** Recommandation : data/decode only. Bloque uniquement l’exécution Q.
4. **Différable : format ELF32 d’export.** Recommandation : export interne d’abord, ELF contrôlé ensuite. Bloque l’interopérabilité avec certains linkers.
5. **Différable : politique exacte de sauvegarde des traces.** Recommandation : journal local opt-in avec anonymisation. Bloque seulement le partage automatique.
6. **Différable mais requise avant P2 : profondeur d’historique arrière.** Recommandation : 100 000 mutations ou désactivation explicite, avec indication du quota atteint. Bloque les longues sessions au-delà du quota et la sélection du format de trace inverse.
7. **Différable mais requise avant P2 : toolkit graphique.** Recommandation : conserver le modèle de panneaux et d’événements indépendant du toolkit, puis choisir une bibliothèque native de plateforme après un prototype. Bloque l’implémentation UI, pas les contrats du moniteur.
8. **Différable : compatibilité GNU/LLVM ciblée.** Recommandation : versions détectées au runtime et corpus minimal documenté. Bloque seulement les garanties de compatibilité externe, pas le dialecte natif.
9. **Résolue localement pour le source assembleur guest.** Décision : 256
   lignes de 128 caractères, 64 labels et scratch statique ; metadata borné à
   32 Kio. La capacité du magasin de lignes BASIC reste distincte et sera
   décidée dans `BASIC-SOURCE-001`. La pile M-mode ne porte pas ces tableaux.
10. **Résolue par D-018 : layout des chaînes et tableaux (budgets seulement).**
    la représentation D-018 : descripteurs de chaînes dans une zone cible
    bornée, tableaux row-major à dimensions fixes, éléments numériques
    binary64 ou descripteurs de chaînes, et erreurs de pool/bornes stables.
    Les budgets de configuration restent différables ; aucune donnée ne doit
    être évaluée par l’hôte. La représentation ne reste plus ouverte.
