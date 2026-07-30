# Questions ouvertes à fort impact

Toutes les décisions ISA/ABI nécessaires à V1 ont un défaut choisi. Une question ne bloque le plan que si le choix recommandé est refusé.

1. **Bloquante avant implémentation : SHA complet R2.** Recommandation : figer le SHA complet correspondant à `c6edca7` et l’archiver. Bloque la génération reproductible des tables.
2. **Bloquante avant import ELF : SHA complet R3.** Recommandation : figer le commit du 31-07-2026 et enregistrer le PDF/HTML. Bloque la validation d’ELF et des flags.
3. **Bloquante pour ELF externe : stratégie de flags draft.** Recommandation : appliquer D-004 et refuser toute ambiguïté. Bloque seulement l’import d’objets ambigus.
4. **Différable : activer A multi-hart.** Recommandation : conserver mono-hart V1 ; réserver `A-MH1` aux tests ultérieurs. Bloque l’exécution atomique, pas l’assemblage/décodage.
5. **Différable : inclure un frontend Q exécuté.** Recommandation : data/decode only. Bloque uniquement l’exécution Q.
6. **Différable : format ELF32 d’export.** Recommandation : export interne d’abord, ELF contrôlé ensuite. Bloque l’interopérabilité avec certains linkers.
7. **Différable : politique exacte de sauvegarde des traces.** Recommandation : journal local opt-in avec anonymisation. Bloque seulement le partage automatique.
8. **Différable : profondeur d’historique arrière.** Recommandation : 100 000 mutations ou désactivation. Bloque les longues sessions au-delà du quota.
9. **Différable : UI graphique ou terminal.** Recommandation : abstraction clavier et vues, premier frontend selon plateforme. Bloque la sélection de toolkit, pas les contrats.
10. **Différable : compatibilité GNU/LLVM ciblée.** Recommandation : versions détectées au runtime et corpus minimal documenté. Bloque seulement les garanties de compatibilité externe, pas le dialecte natif.
