---
source: concepts/architecture.md
source_hash: 5f78b0f66293
---
# Architettura

Un solo motore decide; adattatori sottili traducono. Il motore non chiama mai un modello, e gli
agenti non scrivono mai lo stato a mano. La regola è registrata
nell'[ADR 0001](https://salvo8519.github.io/wayloop/project/adr/0001-rust-engine-thin-adapters/).

## Cosa esiste

| Componente | Percorso | Cosa fa oggi |
| --- | --- | --- |
| Schemi | `crates/wayloop-schemas` | Libreria Rust senza I/O, sede dei tipi dei contratti che rispecchiano gli schemi JSON: il report degli errori ([sotto](#gli-errori-sono-valori)), l'envelope con i suoi codici di uscita ([sotto](#lenvelope)), il brief e l'input e i dati di `validate`, con un tipo per ogni forma di valore (identificativi, percorsi del progetto, tag di lingua, numeri da 1). Ognuno deriva il proprio JSON Schema dentro il sottoinsieme del core ([sotto](#schemi-derivati-dai-tipi-dei-contratti)). |
| Core | `crates/wayloop-core` | Libreria Rust senza I/O. Fissa il vocabolario del ciclo di un'issue: i nove passi in ordine, i loro identificativi stabili e quale passo è uno stato di approvazione. Riesporta l'errore di ogni rifiuto e l'envelope, e calcola l'hash di un input dal suo JSON canonico ([sotto](#lenvelope)). Legge gli schemi JSON dei contratti con un proprio interprete ([sotto](#linterprete-degli-schemi)). Il suo punto di ingresso esegue il comando `validate`, che controlla i brief rispetto al loro schema ([riferimento](../reference/validate.md)). |
| Runtime | `crates/wayloop-runtime` | Libreria Rust che esegue il lavoro deciso dal core ([sotto](#core-sincrono-runtime-asincrono)). Oggi legge gli artefatti salvati di un progetto nell'input canonico di `validate` ([sotto](#lettura-degli-artefatti-salvati)). |
| Riga di comando | `crates/wayloop-cli` | Il binario `wayloop`, solo un adattatore: legge gli argomenti, chiama il core e stampa JSON. Comandi: `version` e `steps` ([riferimento](../reference/index.md)), e `validate`, che legge gli artefatti del progetto con il runtime, oppure un input canonico in JSON, e stampa l'envelope del punto di ingresso ([riferimento](../reference/validate.md)). |
| Generatore degli schemi | `crates/wayloop-schemagen` | Uno strumento del repository, non pubblicato: stampa gli snapshot dei contratti, che `uv run just generate` scrive in `schemas/` ([sotto](#snapshot-dei-contratti)). |
| Contenuti per agenti | `.agents/` | Ruoli e skill nel formato aperto Agent Skills, con mirror generati per Claude Code. Servono a costruire Wayloop stesso. |
| Processo | `roadmap/`, `specs/`, `sprints/`, `scripts/` | Il piano come codice, sincronizzato con le issue di GitHub; specifiche con requisiti numerati; corsie degli sprint con percorsi disgiunti; i controlli eseguiti dalla CI. |

Wayloop non ha ancora una release. I componenti del motore, e l'ordine in cui vengono
costruiti, sono nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Struttura dei crate

Il motore è un workspace Cargo di quattro crate. Ognuno ha un solo ruolo, e le dipendenze
vanno in una sola direzione: verso i crate puri. Un quinto crate del workspace,
`wayloop-schemagen`, è uno strumento al livello degli adattatori, mai pubblicato.

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
| `wayloop-schemagen` | Stampa gli snapshot dei contratti; non pubblicato | `wayloop-core`, `wayloop-schemas` | Sì, solo lo standard output |

- **Un solo punto di ingresso.** Il core espone una sola funzione a ogni canale: comando,
  input canonico e contesto in ingresso, envelope in uscita
  ([ADR 0028](https://salvo8519.github.io/wayloop/project/adr/0028-one-contract-on-every-channel/)).
  È `wayloop_core::run`, alla radice di `wayloop-core`; gli adattatori la chiamano e si
  limitano a tradurne l'input e l'envelope, `Envelope<Value>`, i cui dati sono il tipo dei dati
  del comando scritto in JSON in un solo punto. `wayloop_core::run_json` riceve l'input come
  testo JSON: un testo che non è JSON, contiene un numero oltre un double o ripete una chiave
  nello stesso oggetto è rifiutato con il motivo `syntax`, `range` o `duplicate` prima che il
  comando venga eseguito. Il contesto porta le impostazioni del canale: il dry run, scritto in
  `meta.dry_run`, e la versione richiesta del comando; senza, viene eseguita la versione
  corrente ed è elencata in `meta.defaults_applied`, e una versione che il motore non esegue è
  rifiutata (ADR 0028, modificato il 2026-11-20). `wayloop_core::usage_error` e
  `wayloop_core::input_unreadable` costruiscono gli envelope di un canale non usato come
  documentato e di un input che non ha potuto leggere, così che la riga di comando stampi un
  envelope per ogni esito.
- **Dipendenze solo da un elenco approvato.** Ogni crate puro può dipendere solo dai crate
  della propria allowlist, approvata in revisione. Entrambi possono usare `serde` con le sue
  macro derive e `serde_json`, che non fanno I/O, con i crate che raggiungono (`serde_core`,
  `serde_derive`, `proc-macro2`, `quote`, `syn`, `unicode-ident`, `itoa`, `zmij`, `memchr`),
  e `schemars`, che deriva il JSON Schema dei tipi dei contratti e non fa I/O, senza la sua
  feature `preserve_order`, con i crate che raggiunge (`schemars_derive`,
  `serde_derive_internals`, `dyn-clone`, `ref-cast`, `ref-cast-impl`)
  ([ADR 0030](https://salvo8519.github.io/wayloop/project/adr/0030-contracts-generated-from-rust-types/));
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
- **Niente I/O nel sorgente.** Lo rifiuta il compilatore. Il `clippy.toml` del workspace,
  l'unico, elenca le parti della libreria standard che fanno I/O o leggono lo stato
  dell'ambiente: file (`std::fs`), rete (`std::net`), processi (`std::process`), ambiente
  (`std::env`), stream standard, thread, letture dell'orologio (`SystemTime::now`,
  `Instant::now`) e hash map con seme casuale (`HashMap`, `HashSet`; al loro posto si usano
  `BTreeMap` e `BTreeSet`). I lint di Clippy `disallowed_methods` e `disallowed_types`
  applicano gli elenchi a ogni crate e a ogni target per impostazione predefinita, test
  compresi. Un crate che fa I/O, il runtime, un adattatore o il generatore degli schemi, se ne
  esclude in modo esplicito con un `allow` dei due lint alla radice di ciascuno dei suoi target
  che ne ha bisogno. Ogni crate puro vieta
  inoltre con `forbid`, alla propria radice, `print_stdout`, `print_stderr`, `dbg_macro`,
  `disallowed_methods` e `disallowed_types`: `forbid`, a differenza di `deny`, non può essere
  revocato da un `allow` più interno. I lint seguono la risoluzione dei nomi, quindi un alias o
  un import annidato non li aggira. `cargo clippy` li applica in `just check` e nella CI.
  `scripts/check_core_io.py` fallisce se l'attributo o una voce del `clippy.toml` viene
  rimossa, se un crate ha un `clippy.toml` proprio, che Clippy leggerebbe al posto di quello
  del workspace, e se un manifest consente i due lint.
- **Niente panic.** Ogni crate del workspace rifiuta i lint di Clippy che permettono al codice
  di andare in panic o di fermare il processo: `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented`, `unreachable`, `indexing_slicing`, `string_slice`, `get_unwrap`,
  `panic_in_result_fn`, `exit` e `mem_forget`, oltre ai gruppi `all` e `pedantic`. Sono fissati
  nel manifest del workspace e fanno fallire `cargo clippy -D warnings` in `just check` e nella
  CI. I test possono usare unwrap, expect, panic e indici: il `clippy.toml` del workspace lo
  consente solo nel codice di test. Un fallimento è un valore di errore; la riga di comando lo
  traduce in un codice di uscita, anche quando non riesce a scrivere la propria uscita.

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

### Lettura degli artefatti salvati

Dove sono salvati gli artefatti è una regola della specifica, quindi la decide il core; leggere
i file è I/O, quindi lo fa il runtime (piano di SPEC-0001, D1).

- **Il core decide quali file.** `wayloop_core::artifacts::locations` elenca le posizioni
  (`plan/briefs/*.yaml`, `plan/council/*.yaml`, `plan/items/*.yaml` e
  `specs/<NNNN>-<capability>/spec.md`, quindi mai `specs/_templates/`), tiene i percorsi degli
  artefatti tra quelli che riceve e li ordina per byte, con separatori `/` e relativi alla
  radice del progetto, così l'input canonico è lo stesso su ogni piattaforma. Controlla anche i
  percorsi indicati da una persona: relativi, senza `..` né `\`, `.yaml` o `.md`. Contiene i
  limiti, ciascuno definito una volta: 10.000 file per un insieme; 1 MiB per un file; 64
  livelli di annidamento e 8 MiB di byte contati per un documento analizzato; 32 MiB di byte
  contati per l'intero insieme, tra byte letti e documenti costruiti. I byte contati sono una
  stima della memoria: il testo, i puntatori delle righe, un costo fisso di 128 byte per ogni
  valore e ogni riga, e ciò che alias e ancore ripetono. Un test del runtime verifica su Linux
  che la memoria residente aggiunta da un insieme caricato resti entro questi limiti.
- **Il runtime legge e analizza, solo dentro il progetto.** `wayloop_runtime::project` elenca
  le cartelle di ogni posizione fino alla sua profondità, e indica le cartelle che esistono nel
  membro `locations` dell'input; da esse il core decide se avvisare che non ne esiste nessuna
  (SPEC-0001 R26). Una posizione la cui cartella, o una
  cartella che la contiene, è un link simbolico è trattata come assente; dentro una posizione i
  link, i file speciali e i nomi non UTF-8 sono saltati. Prima di leggere un file risolve i link
  della radice e del file e rifiuta un file fuori dalla radice o il cui nome risolto ha un'altra
  estensione, come la riga di comando rifiuta un tale percorso, e rifiuta ciò che non è un file
  regolare, così una lettura non si blocca mai su una pipe. Legge ogni file fino al limite e costruisce una voce di
  `documents` per file: un file YAML come documento JSON con la riga di ogni valore, uno
  `spec.md` come testo Markdown, che il core normalizza. Un file più grande di un limite, che
  supererebbe i byte contati di un documento o dell'insieme, non UTF-8 o che non si analizza
  diventa una voce `unreadable` con la sua riga, che il core riporta come errore `syntax` del
  file; l'envelope viene sempre scritto.
- **Lo YAML segue il piano, non il parser.** Il runtime usa il parser a eventi di
  `saphyr-parser`, che dà la posizione di ogni nodo, e costruisce da sé il valore JSON: gli
  scalari seguono lo schema core di YAML 1.2, quindi `no`, `on` e `2026-11-17` restano
  stringhe; una chiave data due volte nella stessa mappa è rifiutata; ogni valore e ogni riga
  sono contati, e un alias conta i valori e i byte che ripete prima di essere copiato, quindi
  una alias bomb o un valore grande ripetuto dagli alias è rifiutato prima di essere costruito;
  le copie tenute per le ancore sono contate a parte, quindi le ancore annidate si sommano
  invece di raddoppiare; un file contiene un solo documento, una mappa; un numero che JSON non
  può contenere e un tag fuori dallo schema core sono rifiutati. Un messaggio cita al massimo
  80 caratteri di una chiave o di un valore, e la riga di comando rende visibili sullo standard
  error i caratteri di controllo e i caratteri di formato invisibili (a larghezza zero e di
  direzione). Il parser resta fuori dai crate puri, le cui allowlist non cambiano.

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
| `pointer` | Puntatore RFC 6901 dalla radice dell'input inviato; vuoto per l'intero input; un valore del tipo `Pointer`, verificato quando è letto da un testo, oppure costruito un token alla volta con l'escape (`join`, `join_index`) |
| `pointers` | Puntatori alle altre parti dell'input coinvolte, come i due lati di un conflitto; facoltativo |
| `reason` | Un valore dell'enum documentato: `syntax`, `missing`, `unknown_field`, `duplicate`, `type`, `enum`, `conflict`, `requires`, `range`, `length`, `state`, `stale`, e `internal` per un guasto del motore stesso, che non è un rifiuto dell'input |
| `expected` | I valori accettati in `pointer`, mai un pattern; facoltativo, e `[]` quando nessun valore è accettato |
| `fix` | Il campo più vicino a una chiave scritta male, o un valore corretto; per un avviso il cui luogo di memorizzazione ha un nome sbagliato, l'identificativo da cui il luogo dovrebbe prendere il nome; facoltativo |
| `source` | Dove è salvato il valore: `{file, line}` per un file del repository, con `line` solo quando è nota, `{issue}` per una issue del tracker, `{milestone}` per una milestone del tracker, con righe e numeri a partire da 1; assente per un documento inviato direttamente |
| `document` | Indice, da 0, del documento dell'input a cui l'errore appartiene, il cui `pointer` è allora relativo a quel documento; assente per un errore dell'input stesso |
| `related` | Gli altri luoghi della stessa condizione, ciascuno in un altro documento dell'input: `{document, pointer}`, con i membri della sua `source` (`file` e `line`, `issue` o `milestone`) nella voce stessa quando quel documento è stato memorizzato, così l'envelope resta entro quattro livelli di oggetti (C4); facoltativo, e mai `[]` |

Un campo facoltativo non fornito è omesso dalla forma JSON; una lista fornita vuota è scritta
`[]`, perché assente significa non fornito e `[]` significa nessuno (ADR 0009, C2); `related` è
omesso quando non c'è un altro luogo, mai scritto `[]`. Il report degli errori,
`wayloop_core::ErrorReport`, elenca in `errors` tutti gli errori trovati in un solo passaggio, nell'ordine in cui sono stati trovati, e ripete il primo al livello superiore,
così `errors[0]` è sempre uguale al livello superiore. `Error` ed `ErrorReport` implementano
`Display` e `std::error::Error`. Un codice è una costante del tipo `Code`, così un solo punto elenca tutti
i codici del motore; i codici arrivano con la specifica di ogni comando.

### L'envelope

Ogni comando restituisce un solo envelope (ADR 0009, C1), un tipo del contratto di
`wayloop-schemas` riesportato come `wayloop_core::Envelope<D>`, generico su `D`, il tipo dei dati
del comando; i diagnostics, il meta e il report degli errori al suo interno sono definiti una
sola volta per tutti i comandi
([ADR 0030](https://salvo8519.github.io/wayloop/project/adr/0030-contracts-generated-from-rust-types/), G1):

| Campo | Significato |
| --- | --- |
| `command` | Identificativo del comando |
| `data` | Il risultato; presente solo quando il comando è riuscito |
| `error` | Il report degli errori ([sopra](#gli-errori-sono-valori)); presente solo quando il comando è stato rifiutato o è fallito, e allora `data` è assente |
| `diagnostics` | `warnings` nella forma di un errore; `not_done`, un `{check, reason}` per ogni controllo non eseguito, con la reason `not_built` (la parte del motore che lo esegue non è ancora costruita) o `input_absent` (l'input che gli serve non è stato fornito); `checks`, i controlli eseguiti. Ogni lista è sempre scritta, `[]` quando è vuota |
| `meta` | `engine_version`, `envelope_version` (1), `command_version`, `input_hash`, `dry_run` e `defaults_applied`: un `{pointer, value, reason, message, document}` per ogni valore predefinito applicato dal motore, dove `reason` è un identificativo, `message`, per le persone, è facoltativo, e `document`, facoltativo, è l'indice del documento a cui il valore predefinito appartiene, come in un errore |

Ogni identificativo nell'envelope (un controllo, una reason, un codice di errore) è un valore
`snake_case` stabile, documentato con il comando che lo scrive; i client accettano i valori che
non conoscono (ADR 0009, C5).

`Envelope::exit_code()` dà il codice di uscita dell'ADR 0009 C7: 0 per un risultato con `data`,
1 per un input rifiutato, 2 per un canale non usato come documentato (codice `usage`), 3 per un
input che non è stato possibile leggere (codice `input_unreadable`), 5 per un errore interno del
motore; restituisce
`Outcome::exit_code()`, l'unico punto in cui un esito diventa un codice di uscita. Un errore interno è scritto in `error` come un rifiuto; i suoi
errori hanno la reason `internal`, che distingue i due casi nel JSON. L'envelope non contiene nulla che dipenda dalla macchina o
dal momento: nessun orario, nome host o percorso assoluto.

Un esito si costruisce solo con costruttori che rifiutano, restituendo `None`, ciò che il
contratto non definisce: `Outcome::success` rifiuta dati scritti come null (null significa non
fornito, ADR 0009 C2) o senza forma JSON, `Outcome::refused` un report con un errore di reason
`internal`, `Outcome::failed` un report con un errore di qualsiasi altra reason,
`Outcome::unusable` e `Outcome::environment` un report i cui errori non hanno tutti i codici
`usage` e `input_unreadable`. `Outcome::internal` costruisce sempre un fallimento, per chi deve
dare un esito quando nessun altro si può costruire. Ogni envelope
che il motore può costruire è quindi uno che il suo lettore accetta. Il lettore rifiuta gli
stessi casi, e un documento con sia `data` sia `error`, o con nessuno dei due. Lo schema
richiede `error` quando `data` è assente; il sottoinsieme dell'interprete non ha negazione,
quindi solo il tipo rifiuta la presenza di entrambi. Lo schema dell'envelope con dati qualsiasi
è quello di `Envelope<Value>`, versionato come snapshot del contratto `envelope/result`; lo
schema di output di un comando è l'envelope con il tipo dei dati di quel comando. I test del
core caricano ogni snapshot con l'interprete e con l'oracolo.

`meta.input_hash` è lo SHA-256 dell'input serializzato come JSON canonico (RFC 8785), in
esadecimale minuscolo, così lo stesso input dà lo stesso hash su ogni canale e piattaforma:

- **JSON canonico** (`wayloop_core::canonical`). I membri di un oggetto sono ordinati per unità
  di codice UTF-16 delle chiavi, mai secondo l'ordine della mappa; le stringhe sono escapate
  come le escapa `JSON.stringify` di ECMAScript; i numeri sono scritti come il double IEEE 754
  che denotano, nella forma ECMAScript, quindi `4.50` diventa `4.5` e `1E30` diventa `1e+30`. Un
  intero oltre 2^53 in valore assoluto è scritto come il double più vicino, come richiede la RFC
  8785, quindi due interi così possono avere lo stesso hash. `4364.0` e `4364` denotano lo stesso
  double, quindi hanno lo stesso hash. NaN e gli infiniti non possono
  stare in un valore JSON, quindi la funzione è totale. Lo scrittore tiene una propria pila,
  così un input annidato in profondità non può esaurire lo stack delle chiamate. È verificato
  sugli esempi e sui campioni numerici della RFC 8785. Un valore JSON non può contenere chiavi
  duplicate, che la RFC 8785 rifiuta: `serde_json` tiene l'ultima di due chiavi uguali, quindi
  chi legge un testo JSON deve rifiutarle prima, con la reason `duplicate`.
- **Lettura dei numeri.** Il workspace dichiara `serde_json` una sola volta, con la feature
  `float_roundtrip`, che non aggiunge crate: ogni numero è letto come il double più vicino, come
  lo leggono gli altri parser JSON. Il parser predefinito di `serde_json` può sbagliare di
  un'unità nell'ultima cifra binaria, e lo stesso input avrebbe allora hash diversi su due
  canali. Alcuni test del core falliscono se `float_roundtrip` viene tolta o se un crate
  qualsiasi abilita `arbitrary_precision`, che conserva il testo di un numero invece del suo
  double.
- **SHA-256** (`wayloop_core::sha256`). Scritto a mano da FIPS 180-4, così il core non ha
  bisogno di un crate di hashing (piano della SPEC-0001, D3). È verificato sui vettori di test
  pubblicati e, su input casuali, contro un crate di hashing usato solo nei test (`sha2`, una
  dipendenza di sviluppo, mai distribuita e non presente nella allowlist del core). L'hash
  identifica un contenuto e non protegge segreti, quindi non cerca di avere tempo costante.

### L'interprete degli schemi

Gli schemi dei contratti sono JSON Schema (draft 2020-12), derivati dai tipi dei contratti
(vedi la sezione seguente). Il core li legge con un piccolo interprete di un sottoinsieme
documentato, `wayloop_core::schema`, così che ogni errore di un documento sia riportato in un
solo passaggio con il suo puntatore e il suo motivo, e il core non abbia bisogno di un crate
di validazione (piano della SPEC-0001, D5). Uno
schema che usa una parola chiave fuori dal sottoinsieme, o una parola chiave fuori dal suo
posto, è rifiutato quando viene caricato: nessuna regola di uno schema è ignorata in silenzio.

| Parola chiave | Significato | Motivo dell'errore |
| --- | --- | --- |
| `type` | `boolean`, `integer`, `string`, `array` o `object`; un intero è un numero finito senza parte frazionaria, quindi `2.0` lo è | `type`, e su quel valore non si controlla altro |
| `enum`, `const` | Valori accettati: stringhe, interi, booleani o `null`, confrontati per valore | `enum`, con i valori accettati in `expected` |
| `minLength`, `maxLength` | Limiti di una stringa, in code point Unicode | `length` |
| `minItems`, `maxItems` | Limiti di un array | `length` |
| `minimum`, `maximum` | Limiti interi inclusi, accanto a `type: integer` | `range` |
| `format` | Un formato con nome del core, sulle stringhe | `syntax`, con un valore corretto in `fix` quando si può ricavare |
| `items` | Schema di ogni elemento di un array | |
| `properties` | Schema di ogni membro dichiarato; con `additionalProperties: false` ogni altro membro è rifiutato, senza `additionalProperties` l'oggetto è aperto e gli altri membri sono accettati senza controllo, come negli schemi di output (ADR 0009 C5, modificato il 2026-11-20) | `unknown_field`, con la chiave dichiarata più vicina in `fix` |
| `additionalProperties` | `false`, oppure lo schema di ogni membro non dichiarato in `properties`: i valori di una mappa | i motivi di quello schema |
| `propertyNames` | Schema del nome di ogni membro, uno schema di stringhe senza `enum` né `const`, controllato come stringa e segnalato al puntatore del membro prima degli errori del suo valore: le chiavi di una mappa | i motivi di quello schema, di solito `syntax` |
| `required` | Membri dichiarati che devono essere presenti | `missing` |
| `if` e `then`, `allOf` | Requisiti condizionali: quando il valore soddisfa la condizione di `if`, i membri del `required` di `then` devono essere presenti; `allOf` elenca coppie di questo tipo | `requires`, con i membri verificati dalla condizione in `pointers` |
| `$ref`, `$defs` | Un riferimento a una definizione dello stesso file, il cui nome non contiene `/`, `~` o `%` | |
| `$schema`, `$id`, `$comment`, `title`, `description` | Annotazioni, alla radice per `$schema` e `$id` | |

La condizione di un `if` usa `type`, `properties`, `required`, `enum`, `const` e `contains`.
`contains` è vera per un valore che non è un array, quindi una condizione su una lista dichiara
`type: array`: un membro del tipo sbagliato dà allora solo il suo errore di `type`. Non c'è
`pattern`: il core non contiene un motore di espressioni regolari, e un formato con nome dà un
`fix` preciso. I formati con nome sono `date`, `utc-date-time` (con il suffisso `Z`),
`language-tag` (lingua, scrittura, regione e varianti di BCP 47), `tracker-handle`,
`project-path` (un percorso relativo alla radice del progetto in una sola grafia: segmenti uniti
da `/`, nessuno vuoto, `.` o `..`; nessun `\` e nessuna lettera di unità), `capability` (parole
minuscole di lettere e cifre unite da un solo trattino, lo slug della cartella di una specifica
viva), gli identificativi di ogni tipo di artefatto (`brief-id`, `council-id`, `gate-id`,
`epic-id`, `story-id`, `spec-id`, `change-id`, `task-id`, `milestone-id`, `handover-id`),
`brief-or-council-id` (l'uno o l'altro dei primi due, l'oggetto di una issue di gate) e gli
identificativi delle parti (`requirement-id`, `criterion-id`, `decision-id`, `question-id`), con
le forme canoniche della SPEC-0001, e `json-pointer` (un puntatore RFC 6901, la forma di ogni
`pointer` dei contratti). La documentazione del modulo elenca ogni forma e ogni correzione.
L'elenco dei formati con nome è un solo enum, `wayloop_schemas::Format`, e il controllo dei
formati del core lo esamina in modo esaustivo: un formato aggiunto all'enum non compila finché
il core non lo controlla.

- **Null.** Un membro di un oggetto impostato a `null` non è dato, esattamente come un membro
  assente (ADR 0009 C2): accettato se facoltativo, `missing` se obbligatorio, mai un campo
  sconosciuto, e assente per una condizione. Un elemento `null` di un array è un valore come
  gli altri e non supera `type`, perché nessun tipo del sottoinsieme è `null`.
- **Ordine degli errori.** Gli errori seguono l'ordine del documento: gli errori di un valore
  prima di quelli al suo interno, i membri di un oggetto nell'ordine dei byte delle chiavi, gli
  elementi di un array per indice, e i membri assenti di un oggetto dopo quelli presenti. Un
  membro che manca sia per `required` sia per un `then` è riportato una volta sola, come
  `missing`.
- **Chiave più vicina.** La correzione di una chiave sconosciuta è una chiave dichiarata non
  ancora presente, a una distanza di allineamento ottimo delle stringhe (la distanza di
  Damerau-Levenshtein ristretta) non superiore a un terzo della lunghezza della chiave
  sconosciuta; a parità vince la prima chiave nell'ordine dei byte. Una chiave più lunga di 64
  caratteri non riceve correzioni.
- **Interi.** Ogni intero controllato dall'interprete sta entro più o meno (2^53 - 1),
  l'intervallo in cui un double IEEE 754 rappresenta esattamente ogni intero (RFC 7493,
  I-JSON); un valore oltre è rifiutato con il motivo `range`, qualunque cosa dica il suo
  schema. Gli interi di uno schema rispettano lo stesso limite. I numeri si confrontano per
  valore, quindi `4364.0` è uguale a `4364`. Un numero in una parte del documento che nessuno
  schema descrive non è controllato. I numeri sono letti come double IEEE 754, come li legge
  RFC 8785, quindi le cifre oltre quella precisione non sono viste: `1.0000000000000001` è
  l'intero 1.
- **Limiti.** Uno schema ha al massimo 32 livelli di sottoschemi, contati attraverso i
  riferimenti, e un riferimento che raggiunge la propria definizione è rifiutato. L'interprete
  scende in un documento solo dove scende il suo schema, quindi la sua ricorsione è limitata
  qualunque sia il documento.
- **Oracolo.** I test verificano ogni schema e documento che usano con un crate di validazione
  JSON Schema, una dipendenza di sviluppo che non viene mai distribuita; deve concordare con
  l'interprete sul verdetto. Nell'oracolo i formati con nome sono controllati da espressioni
  regolari scritte a partire dalla SPEC-0001, indipendentemente dai parser del core, tranne
  `date` e `json-pointer`, che l'oracolo controlla con i propri formati incorporati. Due regole
  di Wayloop non sono JSON Schema, quindi l'oracolo vede il documento senza i membri impostati
  a null, e al suo verdetto si aggiunge il limite degli interi; il limite è controllato su ogni
  numero, il che coincide con l'interprete solo per i documenti che il loro schema descrive
  per intero, come i documenti dei test.

### Schemi derivati dai tipi dei contratti

Ogni tipo dei contratti di `wayloop-schemas` deriva il proprio JSON Schema con `schemars`, e i
suoi commenti di documentazione diventano le descrizioni
([ADR 0030](https://salvo8519.github.io/wayloop/project/adr/0030-contracts-generated-from-rust-types/)).
`wayloop_schemas::contract::schema_for` restituisce lo schema di un tipo per una direzione,
input o output, e non scrive alcun file; `contract::TYPES` elenca ogni tipo dei contratti il cui
schema è derivato. Ogni tipo dei contratti rilegge anche ciò che scrive: un documento che il
suo schema accetta si deserializza nel tipo, che riscrive lo stesso JSON
(`contract::reread`). Un enum chiuso è scritto con la macro `closed_enum!` di
`wayloop_schemas::ident`, con una sola grafia per valore, letta da `id()`, `from_id()` e
`Display`; due valori con la stessa grafia non compilano. Un enum chiuso di un contratto
(`Reason`, `NotDoneReason`, e `IssueStep`, lo `step` di un passaggio di mano, che il core
riesporta e su cui decide) è scritto con `contract_enum!`, così che la sua forma serializzata e
i valori del suo schema leggano la stessa grafia; `Format`, il cui valore è scritto da
`contract::rules::formatted` tramite `id()`, usa `closed_enum!`; `ExitCode`, un numero per
contratto (ADR 0009 C7), non ha grafia. Un identificativo statico (`Code`, `Check`,
`DefaultReason`) è scritto con `static_id!`: le costanti del motore prendono in prestito il
loro testo, un valore letto da un documento lo possiede. Ciò che la derive non scrive è scritto
una sola volta, nel modulo `contract`:

| Esigenza | Come è scritta |
| --- | --- |
| Un requisito condizionale | Una funzione del tipo, indicata in `#[schemars(transform = ...)]`, chiama `contract::rules::require_if_present`, `require_if_absent`, `require_if_equals`, `require_if_in` o `require_if_contains`, che aggiungono a `allOf` una coppia di `if` e `then`; `require_if_contains` verifica una lista con `type: array` e `contains`; `require_if_absent` confronta il membro con `const: null`, condizione vera quando è assente o null, sia nell'interprete sia nella draft 2020-12 |
| Un formato con nome | Una forma di valore implementa `JsonSchema` a mano con `contract::rules::formatted`, a partire da un valore di `Format` |
| Assente, non null | Un membro facoltativo, fuori da `required`, perde il tipo `null` che la derive aggiunge a un `Option` |
| Un valore di enum con il suo significato | Un `oneOf` di costanti documentate diventa un `enum`, e ogni valore è aggiunto alla descrizione come riga ``- `valore`: significato`` |
| Un qualsiasi valore JSON | Lo schema booleano `true` diventa uno schema oggetto con una descrizione |
| Formati degli interi | Un `format` come `uint32` è rimosso; il tipo e i limiti restano |
| Input chiusi, output aperti | In un input ogni oggetto che dichiara i suoi membri e non ha `additionalProperties` riceve `additionalProperties: false`; un oggetto che non ne dichiara è un oggetto libero, scritto a mano per un documento di un altro schema, e resta aperto; una mappa mantiene lo schema dei suoi valori e delle sue chiavi; un output non ha `additionalProperties`, tranne lo schema dei valori di una mappa pura |

Le righe di un commento di documentazione sono unite in paragrafi, perché vanno a capo alla
larghezza del codice. Il generatore poi verifica le regole dei contratti che non sono regole
del sottoinsieme: una descrizione sulla radice, su ogni definizione e su ogni membro (una fatta
solo di spazi non conta); il significato di ogni valore di enum nella sua descrizione, fuori da
una condizione (`if`, `then`, `contains`), che nomina valori di un membro descritto dove il
membro è dichiarato; ogni valore nominato da una condizione, accettato dallo schema dichiarato
del suo membro, e nessuna condizione che non possa mai valere (`contains` su un membro che non è
una lista, un `type` che il suo membro non dichiara); ogni valore testuale di enum in
`snake_case`; nessun `additionalProperties`
che chiuda un output (una mappa pura mantiene lo schema dei suoi valori); e al massimo quattro
livelli di oggetti, compresi i valori di una mappa (ADR 0009 C4).
Il sottoinsieme ha una sola definizione, il caricatore dell'interprete (ADR 0030 G3):
`wayloop_core::schema::contract_schema` esegue i controlli del generatore e poi
`Schema::load` su un tipo registrato, e fallisce con `ContractFault::Rule` o
`ContractFault::Subset`. Un tipo `null`, le parole chiave `pattern`, `default` e
`uniqueItems`, un formato senza nome, uno schema booleano o, in un input, una mappa i cui
valori possono essere qualsiasi cosa sono rifiutati lì, al loro posto.

Un tipo la cui forma serializzata la derive non sa descrivere nel sottoinsieme ha un tipo di
forma privato per il proprio schema e per la lettura: `Source`, le cui forme sono un solo
oggetto con membri facoltativi e `file` e `line` dati insieme, che il tipo legge come
esattamente una forma, `ErrorReport`, i membri del suo primo errore uniti a `errors`, letto
solo quando `errors` comincia con quei membri, ed `Envelope<D>`, con `data` ed `error` entrambi
facoltativi nel suo schema, letto come esattamente uno dei due. Un tipo generico non ha uno
schema finché il suo parametro non è dato, quindi `contract::TYPES` registra l'envelope come
`Envelope<Value>`, con il nome del suo schema, `Envelope`. `contract::snapshot` scrive il testo
di uno snapshot: lo schema con il suo `$id`, le chiavi in ordine di byte, rientro di due spazi e
un a capo finale; non scrive alcun file. I test del core eseguono `contract_schema` su
ogni tipo di `contract::TYPES` in entrambe le direzioni, verificano ogni schema con l'oracolo
rispetto al meta-schema della draft 2020-12, e verificano che i valori serializzati dei tipi
siano accettati e riletti nello stesso JSON.

### Snapshot dei contratti

Gli schemi dei contratti sono versionati in `schemas/` come snapshot, generati dai tipi
([ADR 0030](https://salvo8519.github.io/wayloop/project/adr/0030-contracts-generated-from-rust-types/), G5 e G6). Un solo registro,
`wayloop_schemas::contract::CONTRACTS`, elenca ogni contratto con la sua area (`artifacts`,
`inputs`, `commands`, `envelope`), il suo nome, la sua versione corrente e il tipo e la
direzione di ciascuno dei suoi file; oggi elenca l'envelope, `envelope/result`, alla versione di
`meta.envelope_version`. Il binario `wayloop-schemagen` esegue `contract_schema` su ogni file di
ogni versione corrente, ne scrive il testo con `contract::snapshot` e li stampa tutti come un
solo documento JSON; non scrive alcun file. `scripts/schema_snapshots.py` scrive quei byte senza
modificarli in `schemas/<area>/<name>/v<N>.json` (`v<N>.output.json` per l'output di un
comando, `v<N>.input.json` per l'input di un passaggio di consegne), con la storia di ogni
contratto in `schemas/<area>/<name>/index.json`.

Una versione precedente a quella corrente non cambia mai, e la versione corrente, una volta
rilasciata, cambia solo in modo additivo (ADR 0009 C5): le regole sono nelle
[convenzioni](https://salvo8519.github.io/wayloop/project/conventions/#contract-snapshots). Una
versione precedente si riproduce dal suo file versionato, non dai tipi, che descrivono solo
quella corrente.

`wayloop-schemas` incorpora il testo di ogni snapshot di ogni versione, `contract::EMBEDDED`.
Poiché `cargo package` impacchetta solo la cartella di un crate, gli snapshot sono incorporati
da copie identiche byte per byte al suo interno, in `crates/wayloop-schemas/snapshots/`,
elencate nel file generato `src/contract/embedded.rs`; il controllo fallisce quando una copia
differisce dalla sua fonte. I test di `wayloop-schemas` verificano che le versioni correnti
incorporate siano gli snapshot dei tipi, e i test del core caricano ogni snapshot incorporato
con l'interprete e con l'oracolo.
