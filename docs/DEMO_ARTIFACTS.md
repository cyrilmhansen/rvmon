# Artefacts de démonstration auditables

Les sessions longues du moniteur ne sont pas commitées dans Git. Elles sont
produites localement ou par une CI dans `artifacts/audit/<commit>/`, répertoire
ignoré par Git, puis publiées comme artefacts d’un workflow ou attachées à une
release.

## Capture MiniBASIC/Hammurabi

Préparer les outils et lancer :

```sh
rustup target add riscv64gc-unknown-none-elf
bash scripts/record-minibasic-demo.sh
```

Le script :

1. construit l’image guest depuis le commit courant ;
2. résout symboliquement `minibasic_divide` ;
3. enregistre une session QEMU réelle avec `asciinema` ;
4. exécute le mode direct, les boucles, `TRACE`, le breakpoint flottant et le
   jeu Hammurabi ;
5. produit le cast, une transcription texte et `manifest.toml` ;
6. calcule les SHA-256 des deux artefacts.

Exemple de sortie :

```text
artifacts/audit/7141b4d/minibasic-hammurabi.cast
artifacts/audit/7141b4d/minibasic-hammurabi.txt
artifacts/audit/7141b4d/manifest.toml
```

Le manifeste doit être publié avec les artefacts. Il lie la capture au commit,
au profil RV64ILP32D, aux versions de QEMU/Rust/Asciinema et aux empreintes.
Une vérification minimale est :

```sh
sha256sum minibasic-hammurabi.cast minibasic-hammurabi.txt
```

Comparer les deux valeurs obtenues à `cast_sha256` et `transcript_sha256` dans
`manifest.toml`. Le cast peut être rejoué avec `asciinema play`; la
transcription est destinée aux audits rapides et aux environnements sans
lecteur Asciinema.

## Politique de conservation

- les `.cast` et transcriptions longues restent hors Git ;
- la documentation et les scripts de production sont versionnés ;
- une release peut joindre les artefacts et leur manifeste ;
- les artefacts CI doivent conserver le commit exact et ne pas être remplacés
  par une capture d’une autre révision ;
- aucune capture ne doit contenir de secret, identifiant privé ou donnée
  utilisateur réelle.

La transcription ne remplace pas les tests automatisés : elle prouve la
lisibilité et l’enchaînement d’une session interactive, tandis que les tests
QEMU et leurs assertions restent la preuve fonctionnelle mécanisée.

## Capture suivant exactement le tutoriel guest

La capture compacte précédente n’est pas une reproduction littérale du
tutoriel. Pour l’audit pédagogique, utiliser :

```sh
bash scripts/record-tutorial-guest.sh
```

Cette variante rejoue les sections de `docs/TUTORIAL-GUEST.md` dans leur ordre
et attend une seconde après chaque commande ou ligne saisie (réglable à trois
secondes avec `TUTORIAL_GUEST_PAUSE=3`). Elle commence
par environ vingt secondes d'aperçu du source assembleur à environ 500 lignes
par seconde, puis montre le transfert réel des images code et données par
`payload-load`/`payload-load-data` et relit leur métadonnée en RAM cible. Elle comprend
les diagnostics, l’assembleur, les flottants, les snapshots, les watchpoints,
le breakpoint `fdiv.d`, les exercices directs et le jeu Hammurabi final. Sa
durée attendue est de plusieurs minutes ; `TUTORIAL_GUEST_TIMEOUT` permet
d’augmenter la limite si une machine est plus lente.

Elle produit `tutorial-guest.cast`, `tutorial-guest.txt` et
`tutorial-manifest.toml` dans le même répertoire d’artefacts ignoré.
La conversion est contrôlée par `scripts/check-tutorial-transcript.sh` : une
capture contenant une commande inconnue, un état de débogueur invalide ou un
trap d’instruction illégale est refusée comme preuve de tutoriel.

Pour afficher le texte français du tutoriel au-dessus de la session QEMU,
utiliser la variante tmux :

```sh
bash scripts/record-tutorial-guest-tmux.sh
```

Le panneau supérieur montre la section et un extrait de
`docs/TUTORIAL-GUEST.md`; le panneau inférieur est le guest QEMU réel. Le cast
brut conserve les écritures des deux panneaux, tandis que la conversion texte
peut ne montrer que l’écran final à cause des effacements ANSI de tmux. Le
contrôle adapté est donc `scripts/check-tutorial-tmux-cast.sh`, qui vérifie les
marqueurs documentaires, le chargement du payload, les sorties Hammurabi et
les diagnostics interdits directement dans le cast.
Le lecteur supérieur défile maintenant automatiquement dans la plage de la
phase active (`0,75 s` et une ligne par défaut), et chaque nouvelle commande ou
annotation le préempte. Les paramètres
`TUTORIAL_GUEST_GUIDE_SCROLL_INTERVAL` et `TUTORIAL_GUEST_GUIDE_SCROLL_STEP`
permettent de ralentir ou d'accélérer ce défilement sans modifier la session
QEMU.
Le moniteur guest laisse volontairement QEMU vivant après `q` ; le wrapper
attend donc sa limite de 600 secondes et conserve le cast avant la terminaison
normale du processus QEMU. Pour seulement régénérer la transcription et le
manifeste d’un cast déjà capturé :

```sh
REUSE_CAST=1 bash scripts/record-tutorial-guest.sh
```

Les lignes `=== ... ===` du cast sont des annotations de l’orchestrateur hôte,
pas des commandes envoyées à QEMU. Elles séparent le moniteur M-mode, le
MiniBASIC assembleur lancé par `basic`, et l’ancien MiniBASIC Rust indiqué comme
référence seulement. Le payload assembleur réémet aussi les caractères lus
sur la console cible : les lignes BASIC saisies et les valeurs de `INPUT` sont
donc visibles dans la transcription au lieu de produire une succession
ambiguë de `READY>`.
