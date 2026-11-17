---
source: concepts/architecture.md
source_hash: 3e5f0f431711
---
# Architettura

Un solo motore decide; adattatori sottili traducono. Il motore non chiama mai un modello, e gli
agenti non scrivono mai lo stato a mano. La regola è registrata
nell'[ADR 0001](https://salvo8519.github.io/wayloop/project/adr/0001-rust-engine-thin-adapters/).

## Cosa esiste

| Componente | Percorso | Cosa fa oggi |
| --- | --- | --- |
| Schemi | `crates/wayloop-schemas` | Libreria Rust senza I/O, sede dei tipi dei contratti che rispecchiano gli schemi JSON. Non contiene ancora tipi. |
| Core | `crates/wayloop-core` | Libreria Rust senza I/O. Fissa il vocabolario del ciclo di un'issue: i nove passi in ordine, i loro identificativi stabili e quale passo è uno stato di approvazione. |
| Runtime | `crates/wayloop-runtime` | Libreria Rust che esegue il lavoro deciso dal core ([sotto](#core-sincrono-runtime-asincrono)). Non contiene ancora codice. |
| Riga di comando | `crates/wayloop-cli` | Il binario `wayloop`, solo un adattatore: legge gli argomenti, chiama il core e stampa JSON. Comandi: `version` e `steps` ([riferimento](../reference/index.md)). |
| Contenuti per agenti | `.agents/` | Ruoli e skill nel formato aperto Agent Skills, con mirror generati per Claude Code. Servono a costruire Wayloop stesso. |
| Processo | `roadmap/`, `specs/`, `sprints/`, `scripts/` | Il piano come codice, sincronizzato con le issue di GitHub; specifiche con requisiti numerati; corsie degli sprint con percorsi disgiunti; i controlli eseguiti dalla CI. |

Wayloop non ha ancora una release. I componenti del motore, e l'ordine in cui vengono
costruiti, sono nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Struttura dei crate

Il motore è un workspace Cargo di quattro crate. Ognuno ha un solo ruolo, e le dipendenze
vanno in una sola direzione: verso i crate puri.

```text
wayloop-cli ------> wayloop-runtime ----> wayloop-core ----> wayloop-schemas
 (adattatore)         (I/O, nessuna decisione) (decisioni, niente I/O) (tipi dei contratti, niente I/O)
```

| Crate | Ruolo | Può dipendere da | Fa I/O |
| --- | --- | --- | --- |
| `wayloop-schemas` | Tipi dei contratti degli schemi JSON in `schemas/` | nessun crate Wayloop | No |
| `wayloop-core` | Tutta la logica decisionale: modello degli artefatti, macchina a stati, regole del ledger | `wayloop-schemas` | No |
| `wayloop-runtime` | Coda, worker in parallelo, chiamate al tracker, unico scrittore del ledger | `wayloop-core`, `wayloop-schemas` | Sì |
| `wayloop-cli` e gli altri adattatori | Traduzione tra un canale e il motore | tutti i precedenti | Sì |

- **Un solo punto di ingresso.** Il core espone una sola funzione a ogni canale: comando,
  input canonico e contesto in ingresso, envelope in uscita
  ([ADR 0028](https://salvo8519.github.io/wayloop/project/adr/0028-one-contract-on-every-channel/)).
  Appartiene alla radice di `wayloop-core`; gli adattatori la chiamano e si limitano a
  tradurne l'input e l'envelope.
- **Niente I/O nei crate puri.** `scripts/check_core_io.py` legge il grafo delle dipendenze
  risolto da Cargo e fallisce quando un crate di I/O noto (un runtime asincrono, un client
  HTTP, un database, una libreria per file o processi, una libreria git) è raggiunto da
  `wayloop-core` o da `wayloop-schemas` attraverso le dipendenze normali, e stampa il percorso
  che lo raggiunge. Fallisce anche sull'I/O della libreria standard nel loro sorgente:
  `std::fs`, `std::net`, `std::process`, `std::env`, gli stream standard e le macro di stampa.
  Il controllo gira in `just check` e nella CI. Le dipendenze di sviluppo, usate solo dai test,
  non sono controllate.

## Core sincrono, runtime asincrono

La divisione tra il core e il runtime intorno è l'oggetto
dell'[ADR 0006](https://salvo8519.github.io/wayloop/project/adr/0006-synchronous-core-asynchronous-runtime/). Il core è
sincrono e puro: dato uno stato e un evento, restituisce lo stato successivo e il lavoro da
fare. Tutta la concorrenza vive nel runtime.

- **Coda.** Il lavoro pronto si ricava dallo stato (dipendenze soddisfatte, corsia libera),
  non da un broker esterno. Non c'è niente da installare e niente da perdere.
- **Worker in parallelo.** Le issue di corsie diverse girano in parallelo, ciascuna nel proprio
  worktree git. Le corsie dichiarano percorsi disgiunti, verificati prima che parta lo sprint,
  così agenti in parallelo non modificano mai gli stessi file. Un limite configurabile fissa il
  numero di agenti attivi insieme.
- **Un solo scrittore.** I worker mandano i risultati a un unico scrittore che li valida e li
  aggiunge al ledger in ordine. L'esecuzione parallela non rende mai lo stato
  non deterministico.
- **Batch.** Le chiamate al tracker vengono raggruppate per rispettare i limiti di frequenza.
  I passi indipendenti e non interattivi (pareri del comitato, documentazione di un modulo,
  review) possono usare le API batch dei fornitori di modelli dove esistono, più economiche e
  più lente.

Questo repository lavora già così, a mano: le corsie degli sprint dichiarano percorsi
disgiunti, verificati da `scripts/sprint.py`, e ogni issue si lavora nel proprio worktree git.

## Contratti

Ogni ingresso e ogni uscita che attraversano il confine del motore sono JSON con uno schema
versionato. Un adattatore che ha bisogno di un nuovo comportamento lo chiede al motore; non lo
implementa mai. L'envelope comune a tutti i comandi, con i codici di errore e di uscita, è
l'oggetto dell'[ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/).
