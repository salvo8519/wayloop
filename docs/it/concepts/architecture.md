---
source: concepts/architecture.md
source_hash: 5f9418c97255
---
# Architettura

Un solo motore decide; adattatori sottili traducono. Il motore non chiama mai un modello, e gli
agenti non scrivono mai lo stato a mano.

| Componente | Responsabilità | Tecnologia |
| --- | --- | --- |
| Core | Modello degli artefatti, schemi, validazione, grafo delle dipendenze, macchina a stati, ledger, corsie, analisi di complessità, prossimo task, logica di riconciliazione | Rust, senza I/O |
| Runtime | Coda del lavoro pronto, insieme di worker, limiti, raggruppamento delle chiamate al tracker, esecutori | Rust, asincrono (tokio) |
| CLI | `init`, `validate`, `plan`, `issues sync`, `run`, `next`, `status`, uscita JSON | Rust, binario unico |
| SDK Python | Le stesse funzioni per script, notebook e agenti in Python | PyO3 e maturin |
| SDK TypeScript | Integrazione in Node, `npx` | napi-rs |
| Server MCP | Il motore per qualunque agente compatibile | Rust |
| Contenuti per agenti | Ruoli, skill, costituzione, template | `.agents/`, formato aperto Agent Skills, mirror generati per ogni agente |
| Adattatori del tracker | Prima GitHub, poi GitLab | Rust |

## Core sincrono, runtime asincrono

Il core è sincrono e puro: dato uno stato e un evento, restituisce lo stato successivo e il
lavoro da fare. Tutta la concorrenza vive nel runtime intorno.

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

Vedi [ADR 0006](https://salvo8519.github.io/wayloop/project/adr/0006-synchronous-core-asynchronous-runtime/).

## Contratti

Ogni ingresso e ogni uscita che attraversano il confine del motore sono JSON con uno schema
versionato. Un adattatore che ha bisogno di un nuovo comportamento lo chiede al motore; non lo
implementa mai.
