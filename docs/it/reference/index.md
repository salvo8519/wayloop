---
source: reference/index.md
source_hash: 7ff328828692
---
# Riferimento

Questa sezione raccoglie il riferimento dei comandi, dei contratti JSON e dei codici di errore
di Wayloop. Non c'è ancora una release; ciò che viene dopo è nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Comandi di una build dai sorgenti

Il binario `wayloop` costruito da `crates/wayloop-cli` risponde ai comandi qui sotto. Ognuno
stampa una riga di JSON sullo standard output. `version` e `steps` escono con codice 0; la
versione è quella del manifest del workspace. `validate` controlla gli artefatti rispetto ai loro
schemi ed è descritto in una pagina propria, [wayloop validate](validate.md).

| Comando | Uscita |
| --- | --- |
| `wayloop version` (anche `wayloop --version`) | Il nome e la versione del motore |
| `wayloop steps` | Gli identificativi dei passi del ciclo di un'issue, in ordine |
| `wayloop validate --input <file\|->` | L'envelope della validazione di un insieme di artefatti ([wayloop validate](validate.md)) |

```console
$ cargo run -q -p wayloop-cli -- version
{"name":"wayloop","version":"0.0.0"}
$ cargo run -q -p wayloop-cli -- steps
{"issue_steps":["claim","reanalysis","counter_review","checklist","build","verify","review","pull_request","merge"]}
```

Qualunque altro comando, o nessun comando, stampa sullo standard output l'envelope di un errore
d'uso (codice `usage`, descritto in [wayloop validate](validate.md#errori)), stampa una riga
d'uso sullo standard error ed esce con codice 2:

```console
$ cargo run -q -p wayloop-cli 2>&1 >/dev/null
wayloop: No command was given. Commands: version, steps, validate --input <file|-> [--command-version <n>].
```

Quando lo standard output non si può scrivere, per esempio perché l'estremità in lettura di
una pipe è chiusa, il comando stampa la causa sullo standard error ed esce con codice 3, un
errore d'ambiente. I codici di uscita seguono
l'[ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/) (C7).

Gli identificativi dei passi sono valori `snake_case` del contratto, come ogni valore
enumerato dei contratti JSON
(l'[ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/), C4),
e si scrivono allo stesso modo in ogni canale. Le altre grafie, come le etichette del tracker
delle issue, sono prodotte dagli adattatori a partire da questi valori. Aggiungere un passo è una
modifica additiva; rinominare o rimuovere un passo è una modifica incompatibile del contratto.
Sono i valori di `step` nel record e nell'input di passaggio di mano. I passi sono descritti in
[Il loop](../concepts/the-loop.md#il-ciclo-di-unissue).

## Contratti JSON

Gli schemi dei contratti sono pubblicati con questo sito, ciascuno all'URL del suo `$id`, sotto
`https://salvo8519.github.io/wayloop/schemas/`. La storia di ogni contratto, con i file di ogni
versione e il loro SHA-256, è in `schemas/index.json` allo stesso indirizzo, e in `index.json`
nella cartella di ciascun contratto.

| Contratto | Storia, con la versione corrente e il file di ogni versione |
| --- | --- |
| Brief ([wayloop validate](validate.md)) | `schemas/artifacts/brief/index.json` |
| Verbale del comitato ([wayloop validate](validate.md)) | `schemas/artifacts/council_report/index.json` |
| Issue di gate ([wayloop validate](validate.md)) | `schemas/artifacts/gate/index.json` |
| Milestone ([wayloop validate](validate.md)) | `schemas/artifacts/milestone/index.json` |
| Epic ([wayloop validate](validate.md)) | `schemas/artifacts/epic/index.json` |
| User story ([wayloop validate](validate.md)) | `schemas/artifacts/story/index.json` |
| Task ([wayloop validate](validate.md)) | `schemas/artifacts/task/index.json` |
| Specifica viva, in forma normalizzata ([wayloop validate](validate.md)) | `schemas/artifacts/specification/index.json` |
| Modifica a delta ([wayloop validate](validate.md)) | `schemas/artifacts/change/index.json` |
| Record e input di passaggio di mano ([wayloop validate](validate.md)) | `schemas/artifacts/handover/index.json` |
| Input e risultato di `wayloop validate` ([wayloop validate](validate.md)) | `schemas/commands/validate/index.json` |
| Envelope del risultato di ogni comando ([ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/), C1) | `schemas/envelope/result/index.json` |

Una versione, una volta rilasciata, cambia solo in modo additivo: un nuovo membro facoltativo
di input, un nuovo membro di output, un nuovo valore di `enum`. Ogni altra modifica è una nuova
versione, pubblicata accanto alla precedente, che non cambia mai. I client accettano membri e
valori di `enum` che non conoscono ([ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/), C5).
