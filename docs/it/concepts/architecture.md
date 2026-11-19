---
source: concepts/architecture.md
source_hash: bd0fe4470a94
---
# Architettura

Un solo motore decide; adattatori sottili traducono. Il motore non chiama mai un modello, e gli
agenti non scrivono mai lo stato a mano. La regola è registrata
nell'[ADR 0001](https://salvo8519.github.io/wayloop/project/adr/0001-rust-engine-thin-adapters/).

## Cosa esiste

| Componente | Percorso | Cosa fa oggi |
| --- | --- | --- |
| Schemi | `crates/wayloop-schemas` | Libreria Rust senza I/O, sede dei tipi dei contratti che rispecchiano gli schemi JSON. Non contiene ancora tipi. |
| Core | `crates/wayloop-core` | Libreria Rust senza I/O. Fissa il vocabolario del ciclo di un'issue: i nove passi in ordine, i loro identificativi stabili e quale passo è uno stato di approvazione. Definisce l'errore di ogni rifiuto ([sotto](#gli-errori-sono-valori)). |
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
wayloop-cli        adattatore: traduce un canale, non decide nulla
     |
     v
wayloop-runtime    I/O, nessuna decisione
     |
     v
wayloop-core       decisioni, niente I/O
     |
     v
wayloop-schemas    tipi dei contratti, niente I/O
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
- **Dipendenze solo da un elenco approvato.** Ogni crate puro può dipendere solo dai crate
  della propria allowlist, approvata in revisione. Entrambi possono usare `serde` con le sue
  macro derive e `serde_json`, che non fanno I/O, con i crate che raggiungono (`serde_core`,
  `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `itoa`, `zmij`, `memchr`);
  `wayloop-core` può usare anche `wayloop-schemas`, e `wayloop-schemas` nessun crate di
  Wayloop. `scripts/check_core_io.py` risolve ogni crate puro da solo con
  `cargo tree` (dipendenze normali, ogni target, tutte le sue feature, il lock file lasciato
  invariato) e fallisce su ogni crate raggiunto fuori dall'allowlist, stampando il percorso
  che lo raggiunge e indicando i crate di I/O noti. Le allowlist impongono anche la direzione
  della tabella sopra: un crate puro non dipende mai dal runtime o da un adattatore, e gli
  schemi non dipendono mai dal core. Una feature che un altro crate del workspace attiva su una
  dipendenza condivisa non è attribuita a un crate puro. Le dipendenze di sviluppo (solo per i
  test) e quelle di build (eseguite durante la compilazione) non fanno parte del crate
  distribuito e non sono seguite. Estendere un'allowlist è una modifica dello script, soggetta
  a revisione.
- **Niente I/O nel sorgente.** Lo rifiuta il compilatore. Ogni crate puro vieta con `forbid`,
  alla propria radice, i lint di Clippy `print_stdout`, `print_stderr`, `dbg_macro`,
  `disallowed_methods` e `disallowed_types`, e il suo `clippy.toml` elenca le parti della
  libreria standard che fanno I/O o leggono lo stato dell'ambiente: file (`std::fs`), rete
  (`std::net`), processi (`std::process`), ambiente (`std::env`), stream standard, thread,
  letture dell'orologio (`SystemTime::now`, `Instant::now`) e hash map con seme casuale
  (`HashMap`, `HashSet`; al loro posto si usano `BTreeMap` e `BTreeSet`). I lint seguono la
  risoluzione dei nomi, quindi un alias o un import annidato non li aggira, e `forbid`, a
  differenza di `deny`, non può essere revocato da un `allow` più interno. `cargo clippy` li
  applica in `just check` e nella CI; `scripts/check_core_io.py` fallisce se l'attributo o una
  voce di un `clippy.toml` viene rimossa.
- **Niente panic.** Ogni crate del workspace rifiuta i lint di Clippy che permettono al codice
  di andare in panic o di fermare il processo: `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented`, `unreachable`, `indexing_slicing`, `string_slice`, `get_unwrap`,
  `panic_in_result_fn`, `exit` e `mem_forget`, oltre ai gruppi `all` e `pedantic`. Sono fissati
  nel manifest del workspace e fanno fallire `cargo clippy -D warnings` in `just check` e nella
  CI. I test possono usare unwrap, expect, panic e indici: ogni `clippy.toml` lo consente solo
  nel codice di test, quello alla radice per il runtime e la riga di comando, quello di ogni
  crate puro per il suo crate, perché il file di un crate sostituisce quello alla radice. Un
  fallimento è un valore di errore; la riga di comando lo traduce in un codice di uscita,
  anche quando non riesce a scrivere la propria uscita.

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

### Gli errori sono valori

Il motore non va mai in panic su un input errato: restituisce `wayloop_core::Error`. Il tipo è
un tipo del contratto: è definito in `wayloop-schemas` e riesportato dal core. Porta come dati
ogni campo di un rifiuto su cui un agente agisce (ADR 0009, C3), e il punto in cui è salvato il
valore errato:

| Campo | Significato |
| --- | --- |
| `code` | Identificativo stabile del rifiuto; un nuovo significato riceve un nuovo codice, mai uno rinominato (C8) |
| `message` | Che cosa non va, per una persona |
| `pointer` | Puntatore RFC 6901 dalla radice dell'input inviato; vuoto per l'intero input |
| `pointers` | Puntatori alle altre parti dell'input coinvolte, come i due lati di un conflitto; facoltativo |
| `reason` | Un valore dell'enum documentato: `syntax`, `missing`, `unknown_field`, `duplicate`, `type`, `enum`, `conflict`, `requires`, `range`, `length`, `state`, `stale` |
| `expected` | I valori accettati in `pointer`, mai un pattern; facoltativo |
| `fix` | Il campo più vicino a una chiave scritta male, o un valore corretto; facoltativo |
| `source` | Dove è salvato il valore: `{file, line}` per un file del repository, `{issue}` per una issue del tracker, `{milestone}` per una milestone del tracker; assente per un documento inviato direttamente |

Un campo facoltativo assente è omesso dalla forma JSON. Il report degli errori,
`wayloop_core::ErrorReport`, elenca in `errors` tutti gli errori trovati in un solo passaggio,
nell'ordine in cui sono stati trovati, e ripete il primo al livello superiore, così `errors[0]`
è sempre uguale al livello superiore. Il tipo `Error` implementa `Display` e
`std::error::Error`. Un codice è una costante del tipo `Code`, così un solo punto elenca tutti
i codici del motore; i codici arrivano con la specifica di ogni comando.
