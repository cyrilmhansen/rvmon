# Construire et lancer MiniBASIC-RV

Depuis un checkout propre :

```text
rustup target add riscv64gc-unknown-none-elf
cargo build -p luna-guest-monitor --target riscv64gc-unknown-none-elf
qemu-system-riscv64 -M virt -m 64M -bios none \
  -kernel target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor -nographic
```

À `rvmonitor>`, entrer `basic`. Le programme cible affiche `READY>`. Le smoke
réel complet est :

```text
bash scripts/test-minibasic.sh
```

Pour produire une transcription issue de QEMU :

```text
MINIBASIC_TRANSCRIPT=docs/BASIC_DEMO_TRANSCRIPT.txt bash scripts/test-minibasic.sh
```

Le build utilise le guest QEMU et les services `ecall` du moniteur ; aucun
interpréteur BASIC hôte n’est invoqué.

Pour inspecter le contrat du futur payload depuis la cible :

```text
rvmonitor> info payload
```

Cette commande est informative et ne modifie ni les registres ni la mémoire.

## Statut du chargement

Dans cette version, MiniBASIC est lié dans l’ELF guest et `basic` saute vers
`minibasic_entry`. Il s’exécute bien en U-mode, mais n’est pas encore chargé
depuis le workspace par l’assembleur du moniteur.

La première brique du futur chemin utilisateur est maintenant disponible :

```text
rvmonitor> assemble-program 0x81000100
source> addi x10,x0,65
source> addi x17,x0,1
source> ecall
source> addi x10,x0,0
source> addi x17,x0,3
source> ecall
source> end
rvmonitor> run-at 0x81000100
```

`run-at` lance ce payload U-mode déjà assemblé. Le contrat est décrit dans
[`GUEST_PAYLOAD_ABI.md`](GUEST_PAYLOAD_ABI.md) ; le remplacement de MiniBASIC
résident par un payload BASIC assembleur est une étape ultérieure.
