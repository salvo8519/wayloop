---
source: reference/validate.md
source_hash: ba2cd5b35f5d
---
# wayloop validate

`wayloop validate` controlla un insieme di artefatti (brief, verbali del comitato, task e gli
altri tipi del modello degli artefatti) e di input (pull request, commit, passaggi di mano)
rispetto ai loro schemi, in un solo passaggio, e riporta ogni errore con il puntatore del valore
errato e il luogo in cui è memorizzato. Non modifica nulla.

Oggi solo i brief sono validati rispetto al loro schema. Un documento di qualunque altro tipo,
una specifica viva inviata in Markdown e un input sono accettati senza controllo, e i controlli
non eseguiti sono elencati in `diagnostics.not_done`; così anche i controlli tra documenti
(identificativi, collegamenti, parti, stati, nomi).

Questa pagina descrive la versione 1 del comando, compilata dai sorgenti. Il suo contratto è la
coppia di snapshot di schema `schemas/commands/validate/v1.json` (input) e
`schemas/commands/validate/v1.output.json` (output), generati dai tipi di `wayloop-schemas`
(ADR 0030); lo schema di un brief è `schemas/artifacts/brief/v1.json`. Questa pagina li spiega.

## Uso

```console
$ wayloop validate [<path>...] [--command-version <n>]
$ wayloop validate --input <file> [--command-version <n>]
$ wayloop validate --input - [--command-version <n>]
```

Senza `--input`, il comando legge il progetto nella cartella corrente, che deve essere la
radice del progetto: eseguito da una sottocartella, non trova alcuna posizione e valida un
insieme vuoto, con codice di uscita 0 (issue #320). Senza percorsi legge ogni artefatto
salvato: `plan/briefs/*.yaml`, `plan/council/*.yaml`, `plan/items/*.yaml` e
`specs/<NNNN>-<capability>/spec.md` (quindi non `specs/_templates/`). Una posizione raggiunta
attraverso un link simbolico è trattata come assente, e dentro una posizione i link, i file
speciali e i nomi non UTF-8 sono saltati. Con dei percorsi legge solo quei file, come un unico
insieme: ogni percorso è relativo alla cartella corrente, non ha segmenti `..` né `\`, e indica
un file YAML (`.yaml`) o un file Markdown (`.md`); un percorso il cui file, risolti i link, è
fuori dal progetto o ha un'altra estensione è rifiutato come errore d'uso, e un percorso che non
è un file regolare (una cartella, una pipe, un dispositivo) non può essere letto (codice di
uscita 3). Un insieme di più di 10.000 file, trovati o indicati, è rifiutato come errore d'uso.
In entrambi i casi i file sono letti nell'ordine per byte dei loro percorsi, ognuno in una voce
di `documents` con il suo percorso come origine `file` e, per lo YAML, la riga di ogni valore
in `lines`; così ogni errore indica in `source` il file e la riga del valore errato.

I file YAML sono letti con lo schema core di YAML 1.2: `no`, `on`, `yes` e le date come
`2026-11-17` sono stringhe, e un valore tra virgolette è sempre una stringa. Un file è
rifiutato, come voce `unreadable` con la riga del difetto (codice `artifact_schema`, motivo
`syntax`), quando non si analizza, dà una chiave due volte nella stessa mappa, contiene più di
un documento o un documento che non è una mappa, contiene un numero che JSON non può contenere
(`.inf`, `.nan`, `1e400`) o un tag fuori dallo schema core, non è UTF-8, supera 1 MiB, annida
valori oltre 64 livelli, o occuperebbe più di 8 MiB di byte contati una volta analizzato. I
byte contati sono una stima della memoria: il testo del file, i puntatori delle sue righe, 128
byte fissi per ogni valore e ogni riga, ogni valore ripetuto dai suoi alias e ogni copia tenuta
per un'ancora. Tutti i file di un insieme possono occupare al massimo 32 MiB di byte contati,
tra byte letti e documenti costruiti; un file oltre questo limite è rifiutato allo stesso modo.
Un file così non viene caricato, quindi una alias bomb o un valore grande ripetuto dagli alias è
rifiutato prima di essere costruito, e l'envelope viene sempre stampato. Un messaggio cita al
massimo 80 caratteri di una chiave o di un valore.

`--input` indica un file che contiene l'input canonico in JSON, oppure `-` per lo standard
input; non accetta percorsi. `--command-version` richiede una versione del comando; senza, viene
eseguita la versione corrente, e `meta.defaults_applied` lo dice. Il comando stampa un envelope
su una riga dello standard output, esattamente come il motore l'ha scritto, ed esce con il
codice di uscita dell'envelope. Stampa un envelope anche quando la riga di comando non è
compresa, compreso un percorso fuori dal progetto, e quando l'input o un file non può essere
letto; il messaggio per una persona, con il percorso, va sullo standard error.

## Cosa viene controllato

| Controllo | Cosa fa |
| --- | --- |
| `document_kind` | Ogni documento ha un `kind` tra `brief`, `council_report`, `gate`, `milestone`, `epic`, `story`, `specification`, `change`, `task` e `handover`, oppure è un input con `input_kind` |
| `schema_version` | Lo `schema_version` di ogni documento è una versione che il suo tipo conosce; un documento di una versione precedente viene migrato alla versione corrente dopo aver superato lo schema della propria versione |
| `schema_brief` | Ogni brief supera lo schema della sua versione (`schemas/artifacts/brief/v1.json`): membri obbligatori, membri sconosciuti, tipi, valori ammessi, lunghezze, formati |
| `identifier_format` | L'`id` è nella forma canonica del suo tipo, per esempio `BRIEF-0001` |

Ogni schema è un controllo, `schema_<kind>`: in `diagnostics.checks` quando è stato eseguito, in
`diagnostics.not_done` con il motivo `not_built` quando non è in questa versione. Un controllo
che il motore non ha eseguito non è mai saltato in silenzio:

| Controllo | Non eseguito su |
| --- | --- |
| `schema_<kind>`, per esempio `schema_task` | Un documento di un tipo il cui schema non è in questa versione |
| `schema_<input_kind>`, per esempio `schema_commit` | Un input il cui schema non è in questa versione |
| `markdown` | Una specifica viva inviata in Markdown |
| `identifier_duplicate` | L'insieme: due artefatti con lo stesso identificativo |
| `links` | L'insieme: collegamenti tra artefatti, gerarchia, citazioni di ADR |
| `parts` | L'insieme: parti degli artefatti e operazioni delle modifiche a delta |
| `statuses` | L'insieme: transizioni e stati iniziali rispetto a `base` |
| `naming` | L'insieme: titoli, nomi dei branch, intestazioni dei commit e collegamenti delle pull request |

Quando l'input stesso è rifiutato, o la riga di comando non è compresa, o l'input non può
essere letto, nessun controllo viene eseguito: `diagnostics.checks` e `diagnostics.not_done`
sono allora vuoti.

## Input

L'input canonico è un oggetto JSON. Un membro impostato a `null` non è dato, esattamente come
un membro assente. Lo schema di input è chiuso: un membro che non elenca è rifiutato.

| Membro | Tipo | Obbligatorio | Significato |
| --- | --- | --- | --- |
| `documents` | lista di oggetti | sì | Una voce per ogni artefatto o input memorizzato; `[]` significa nessuno |
| `base` | lista di `{id, status}` | no | Stati sul branch predefinito degli artefatti dell'insieme |
| `policy` | oggetto `{types, scopes, areas}`, ciascuno una lista di nomi della policy, identificativi di almeno un carattere | no | La parte della policy del progetto che il modello legge |
| `adrs` | lista di interi da 1 | no | Numeri degli ADR del progetto |
| `language` | tag BCP 47, per esempio `it` o `en-GB` | no | La lingua del progetto. Nessun controllo di questa versione la legge, quindi quando è assente nessun default viene applicato né riportato; un controllo che ne ha bisogno elenca il default in `meta.defaults_applied` |

Una voce di `documents`:

| Membro | Tipo | Obbligatorio | Significato |
| --- | --- | --- | --- |
| `format` | `document`, `markdown` o `unreadable` | sì | `document`: un valore JSON nella forma normalizzata del suo tipo; `markdown`: il testo di una specifica viva; `unreadable`: un file che non è stato possibile analizzare |
| `document` | oggetto | con `document` | Il documento |
| `lines` | oggetto | no; solo con `document` e un'origine `file` | La riga, da 1, di ogni valore del documento, indicizzata dal suo puntatore RFC 6901 nel documento |
| `text` | stringa | con `markdown` | Il contenuto del file |
| `error` | oggetto `{line, message}` | con `unreadable` | Dove il parser si è fermato, dalla riga 1, e il suo messaggio |
| `input_kind` | `pull_request`, `commit` o `handover_input` | no; solo con `document` | Il tipo di un input, che non ha un membro `kind` |
| `origin` | oggetto, esattamente uno tra `{file}`, `{issue}`, `{milestone}` | no; `{file}` con `unreadable` | Dove il documento è stato letto; assente per un documento inviato direttamente |

`origin` non ha un membro che ne nomini la forma: lo dice il membro presente, come per `source`
negli errori. Questo si discosta di proposito dalla lettera dell'ADR 0009 C4 (una scelta è un
tipo più i suoi campi), così che l'input e il report degli errori scrivano un luogo nello stesso
modo. `file` è un percorso relativo alla radice del progetto in una sola grafia: segmenti uniti
da `/`, nessuno vuoto, `.` o `..`; nessuna `/` iniziale, nessun `\` e nessuna lettera di unità,
così che non esca mai dal progetto e nomini un file in un solo modo.

Il comando rifiuta una voce che dà un membro di un altro formato (per esempio `text` con il
formato `document`), `lines` o il formato `unreadable` senza un'origine `file`, un `origin`
senza membri o con più di uno, una chiave di `lines` che non è un puntatore RFC 6901 e una riga
che non è un intero da 1.

Il testo JSON dell'input non deve ripetere una chiave nello stesso oggetto: i parser JSON
tengono uno dei due valori, quindi il motore rifiuta il testo con il motivo `duplicate` invece di
scegliere. Un numero oltre l'intervallo di un double, come `1e400`, è rifiutato con il motivo
`range`.

## Output

L'envelope dell'[ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/),
versione 1 di `envelope/result`, descritto in [Architettura](../concepts/architecture.md#lenvelope).
Quando l'insieme è accettato, `data` è:

| Membro | Significato |
| --- | --- |
| `documents` | Un riepilogo per ogni voce dell'input, nel suo ordine: `kind`, `input_kind`, `id`, `status` (dopo la migrazione) e `origin`, ciascuno omesso quando il documento non lo dà come stringa |

Lo schema di output è aperto (ADR 0009 C5, modificato il 2026-11-20): un client ignora i membri
che non conosce e accetta i valori di enum che non conosce.

- `diagnostics.checks` elenca i controlli eseguiti e `diagnostics.not_done` quelli non eseguiti.
- `meta.command_version` è la versione eseguita. Senza `--command-version`,
  `meta.defaults_applied` elenca `{"pointer": "", "value": 1, "reason":
  "current_command_version"}`: il puntatore è vuoto perché la versione è un'impostazione del
  contesto, non un membro dell'input. Quando `--command-version` indica una versione che il
  motore non esegue, anche il rifiuto ha il puntatore `""`, e `meta.command_version` è la
  versione corrente, di cui l'envelope ha la forma; quando l'input è anche un testo che il
  motore non può leggere come JSON, lo stesso rapporto elenca prima la versione, poi l'errore
  del testo. Un errore d'uso riporta la versione corrente
  del comando che indica, e 1 quando non indica un comando noto.
- `meta.input_hash` è lo SHA-256 del JSON canonico (RFC 8785) dell'input senza i suoi membri
  impostati a `null`, quindi un input con `"base": null` ha l'hash dell'input senza `base`. Per
  un testo che il motore rifiuta di leggere come JSON (non JSON, una chiave ripetuta in un
  oggetto, un numero oltre l'intervallo di un double), è lo SHA-256 dei byte ricevuti; due
  canali che inviano lo stesso testo con altri fine riga o con un byte order mark ottengono
  allora hash diversi. Quando nessun input è stato letto (uso errato, file illeggibile), è lo
  SHA-256 di zero byte.

## Errori

Ogni errore trovato è elencato in `error.errors`; il primo è ripetuto al livello superiore di
`error`. L'ordine è fisso:

1. Gli errori dell'input stesso (codice `command_input`) vengono per primi e fermano il comando:
   nessun documento viene controllato. Seguono il passaggio dello schema di input, poi le voci
   in ordine per le regole che lo schema non può esprimere.
2. Altrimenti, gli errori dei documenti, nell'ordine dei documenti; dentro un documento, per
   fase (prima tipo, versione e schema, poi identificativo); dentro una fase, nell'ordine del
   passaggio dello schema: i membri di un oggetto nell'ordine dei byte delle loro chiavi (`/aa`
   prima di `/zz`), gli elementi di un array per indice, i membri assenti dopo quelli presenti.

Il `pointer` di un errore dell'input punta nell'input canonico, per esempio
`/documents/0/format`. Il `pointer` di un errore di un documento punta in quel documento, per
esempio `/problem`; `document` dà l'indice, da 0, di quel documento in `documents`, e `source`
dice dove è memorizzato:

| Origine del documento | `source` |
| --- | --- |
| `{file}` | `{file, line}`: la riga data in `lines` per il puntatore dell'errore, o per il valore più vicino che lo contiene; solo `{file}` quando `lines` non dà nessuna delle due, mai una riga indovinata |
| `{issue}` | `{issue}` |
| `{milestone}` | `{milestone}` |
| assente | assente |

| Codice | Motivi | Codice di uscita | Significato |
| --- | --- | --- | --- |
| `command_input` | `syntax`, `duplicate`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `conflict` | 1 | L'input canonico è rifiutato: non è JSON, ripete una chiave, contiene un numero oltre un double, o viola il suo schema o una regola della sezione Input; oppure `--command-version` richiede una versione che il motore non esegue (`enum`, le versioni note in `expected`) |
| `artifact_schema` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires` | 1 | Un documento viola lo schema del suo tipo; anche un documento senza `kind`, o con un `kind` fuori dalla lista (`enum`, con i tipi in `expected`), uno `schema_version` assente, non intero (riportato con ogni altro errore rispetto alla versione corrente) od oltre più o meno (2^53 - 1) (`range`), e un file che non è stato possibile analizzare (`syntax`, al puntatore dell'intero documento, con la sua riga in `source`) |
| `schema_version_unknown` | `enum` | 1 | Lo `schema_version` non è una versione del tipo; `expected` elenca le versioni note |
| `identifier_format` | `syntax` | 1 | L'`id` non è nella forma canonica del suo tipo; `fix` dà l'identificativo corretto quando è possibile ricavarlo |
| `usage` | `syntax` | 2 | La riga di comando non è compresa; `command` è il comando indicato, vuoto quando nessuno |
| `input_unreadable` | `missing`, `state` | 3 | Il file di input, un file indicato o una cartella del progetto non esiste (`missing`) o non può essere letto (`state`); l'envelope non contiene percorsi |
| `internal_error` | `internal` | 5 | Il motore non ha potuto calcolare il risultato; l'input può essere valido |

Un membro sconosciuto riceve in `fix` il membro più vicino dello schema, per esempio `problem`
per `problema`.

## Codici di uscita

| Codice di uscita | Quando |
| --- | --- |
| 0 | L'insieme è accettato: l'envelope ha `data` |
| 1 | L'input o un documento è rifiutato: l'envelope ha `error` |
| 2 | La riga di comando non è compresa, per esempio `--input` senza valore, o un percorso fuori dal progetto: l'envelope ha `error` con il codice `usage` |
| 3 | Il file di input, un file indicato o una cartella del progetto non può essere letto (l'envelope ha `error` con il codice `input_unreadable`), o lo standard output non può essere scritto (nessun envelope può essere stampato) |
| 5 | Il motore ha fallito: l'envelope ha `error` con il motivo `internal` |

## Esempio

Un brief che scrive il suo problema sotto la chiave italiana `problema`, inviato direttamente:

```json
{"documents": [{"format": "document", "document": {
  "kind": "brief", "schema_version": 1, "id": "BRIEF-0003",
  "title": "Freelancers record their working time from the terminal",
  "status": "draft", "origin": "light_requirements", "owner": "@example-owner",
  "problema": "Freelancers lose billable hours because recording time breaks their flow.",
  "goals": ["Record a time entry with one command"],
  "users": ["Freelance developers"], "scope_in": ["Time entries"]}}]}
```

Il comando esce con il codice 1. Il report degli errori contiene due errori del documento di
indice 0: la chiave sconosciuta, con la sua correzione, poi quella mancante:

```json
{"code": "artifact_schema", "pointer": "/problema", "reason": "unknown_field", "fix": "problem", "document": 0}
{"code": "artifact_schema", "pointer": "/problem", "reason": "missing", "document": 0}
```
