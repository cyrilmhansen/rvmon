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
