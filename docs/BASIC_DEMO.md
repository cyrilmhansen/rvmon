# Démonstration MiniBASIC-RV

1. Construire et démarrer QEMU selon `BASIC_BUILD.md`.
2. À `rvmonitor>`, poser le breakpoint du calcul cible :

   ```text
   riscv64-linux-gnu-nm -n target/riscv64gc-unknown-none-elf/debug/luna-guest-monitor | awk '$3 == "minibasic_divide" { print $1; exit }'
   ```

   Puis entrer `break 0x...` dans le moniteur.
3. Entrer `basic`, puis exécuter :

   ```text
   PRINT 2+3*4
   PRINT 22/7
   40 PRINT I,X
   10 PRINT "RV64 MINIBASIC"
   60 END
   30 X=I/3
   20 FOR I=1 TO 10
   50 NEXT I
   LIST
   TRACE ON
   RUN
   ```

4. Au breakpoint `minibasic_divide`, utiliser `disasm 0x... 12` et `regs`.
   Le désassemblage montre `fdiv.d`; le contexte montre les registres f et
   `fcsr`. `step` permet de passer l’instruction et `regs` d’observer le
   résultat.
5. Exécuter ensuite le programme GOTO invalide et le programme INPUT décrits
   dans `BASIC_DEMO_TRANSCRIPT.txt`.
6. La transcription versionnée est générée par QEMU, jamais écrite à la main.
