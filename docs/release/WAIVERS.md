# Waivers et limites de publication

Ce rapport distingue les preuves acquises par l’audit local des preuves
nécessaires à une publication externe complète. Aucun item ci-dessous n’est
présenté comme validé.

| ID | Statut | Périmètre | Effet |
|---|---|---|---|
| WVR-001 | ouvert | Copies locales complètes R1/R3/R4/R5 et A1/A2/C1 non archivées ; seuls les manifestes, extraits R2 et URLs sont versionnés. | bloque une redistribution autonome des normes, pas l’exécution du dépôt. |
| WVR-002 | ouvert | Preuve Sail/Spike non capturée dans l’environnement courant. | bloque la revendication d’une validation sémantique externe complète. |
| WVR-003 | ouvert | Hash de deux builds clean séparés et build offline après cache non établi. | bloque la garantie de build bit-reproductible multi-hôte. |
| WVR-004 | ouvert | 9/9 scripts E2E guest/QEMU passent (`E2E-REPORT.md`), mais les scénarios SPEC 1–14 et la couverture seuil release ne sont pas réunis dans un rapport unique. | bloque la déclaration de release finale M9. |
| WVR-005 | accepté | Signature cryptographique et archive binaire distribuable différées. | aucune distribution signée n’est annoncée. |

Les waivers WVR-001 à WVR-004 doivent être résolus ou approuvés avant une
release publique. Le mode `tools/release-audit.sh --strict-oracles` ne les
masque pas : il ne couvre que les contrôles qu’il exécute explicitement.
