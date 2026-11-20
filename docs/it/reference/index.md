---
source: reference/index.md
source_hash: 470f2c9023ae
---
# Riferimento

Questa sezione raccoglie il riferimento dei comandi, dei contratti JSON e dei codici di errore
di Wayloop. Non c'è ancora una release; ciò che viene dopo è nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Comandi di una build dai sorgenti

Il binario `wayloop` costruito da `crates/wayloop-cli` risponde a due comandi. Entrambi
stampano una riga di JSON sullo standard output ed escono con codice 0. La versione è quella
del manifest del workspace.

| Comando | Uscita |
| --- | --- |
| `wayloop version` (anche `wayloop --version`) | Il nome e la versione del motore |
| `wayloop steps` | Gli identificativi dei passi del ciclo di un'issue, in ordine |

```console
$ cargo run -q -p wayloop-cli -- version
{"name":"wayloop","version":"0.0.0"}
$ cargo run -q -p wayloop-cli -- steps
{"issue_steps":["claim","reanalysis","counter_review","checklist","build","verify","review","pull_request","merge"]}
```

Qualunque altro comando, o nessun comando, stampa una riga d'uso sullo standard error ed esce
con codice 2:

```console
$ cargo run -q -p wayloop-cli
wayloop 0.0.0: pre-alpha. Commands: version, steps.
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
I passi sono descritti in [Il loop](../concepts/the-loop.md#il-ciclo-di-unissue).

## Contratti JSON

Ogni comando restituisce un solo envelope
([architettura](../concepts/architecture.md#lenvelope)). Il suo schema, l'envelope con dati
qualsiasi, è il file `schemas/envelope/v1.json` del repository, generato dai tipi Rust del motore
([ADR 0030](https://salvo8519.github.io/wayloop/project/adr/0030-contracts-generated-from-rust-types/)).
È un JSON Schema della draft 2020-12, aperto come ogni schema di output
([ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/), C5): un
client ignora i membri e accetta i valori enum che non conosce. Un successo ha `data` e non
`error`; un input rifiutato o un errore del motore ha `error` e non `data`, e lo schema richiede
`error` ogni volta che `data` è assente. Il suo `$id` è
`https://salvo8519.github.io/wayloop/schemas/envelope/v1.json`.
