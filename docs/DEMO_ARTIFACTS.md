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
