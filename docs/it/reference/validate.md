---
source: reference/validate.md
source_hash: 0c66877c5750
---
# wayloop validate

`wayloop validate` controlla un insieme di artefatti (brief, verbali del comitato, task e gli
altri tipi del modello degli artefatti) e di input (pull request, commit, passaggi di mano)
rispetto ai loro schemi, in un solo passaggio, e riporta ogni errore con il puntatore del valore
errato e il luogo in cui è memorizzato. Non modifica nulla.

Oggi brief, verbali del comitato, issue di gate, milestone, epic, user story, specifiche vive,
modifiche a delta, task, record di passaggio di mano e gli input (pull request, commit e input
di passaggio di mano) sono validati rispetto ai loro schemi. Una specifica viva scritta in
Markdown viene prima normalizzata nel suo modello di documento (vedi Specifiche vive in Markdown
più sotto). Tra documenti, gli identificativi sono controllati per unicità e rispetto al
percorso del loro file, i collegamenti tra artefatti sono risolti (vedi Collegamenti più sotto),
le parti degli artefatti e le operazioni delle modifiche a delta sono controllate (vedi Parti
più sotto), e i titoli di epic e task sono controllati per la loro forma e la loro unicità; gli
altri controlli tra documenti (stati, nomi, aree) non sono eseguiti da questa versione e sono
elencati in `diagnostics.not_done`.

Questa pagina descrive la versione 1 del comando, compilata dai sorgenti. Il suo contratto è la
coppia di snapshot di schema `schemas/commands/validate/v1.json` (input) e
`schemas/commands/validate/v1.output.json` (output), generati dai tipi di `wayloop-schemas`
(ADR 0030); lo schema di ogni tipo di artefatto è `schemas/artifacts/<kind>/v1.json`, per
esempio `schemas/artifacts/task/v1.json`, e lo schema di ogni input è
`schemas/inputs/<input_kind>/v1.json`, per esempio `schemas/inputs/commit/v1.json`, tranne
l'input di passaggio di mano, il cui schema è `schemas/artifacts/handover/v1.input.json`.
Questa pagina li spiega.

## Uso

```console
$ wayloop validate [<path>...] [--command-version <n>]
$ wayloop validate --input <file> [--command-version <n>]
$ wayloop validate --input - [--command-version <n>]
```

Senza `--input`, il comando legge il progetto nella cartella corrente, che deve essere la
radice del progetto. Indica in `locations` le cartelle delle posizioni che ha trovato; eseguito
da una sottocartella, non ne trova nessuna e valida un insieme vuoto, con codice di uscita 0 e
l'avviso `artifact_location_missing`. Senza percorsi legge ogni artefatto
salvato: `plan/briefs/*.yaml`, `plan/council/*.yaml`, `plan/items/*.yaml` e
`specs/<NNNN>-<capability>/spec.md` (quindi non `specs/_templates/`). Una posizione raggiunta
attraverso un link simbolico, o che è un file e non una cartella, è trattata come assente, e
dentro una posizione i link, i file speciali e i nomi non UTF-8 sono saltati. Con dei percorsi legge solo quei file, come un unico
insieme: ogni percorso è relativo alla cartella corrente, non ha segmenti `..` né `\`, e indica
un file YAML (`.yaml`) o un file Markdown (`.md`), letto come specifica viva ovunque si trovi
(vedi Specifiche vive in Markdown); un percorso il cui file, risolti i link, è
fuori dal progetto o ha un'altra estensione è rifiutato come errore d'uso, e un percorso che non
è un file regolare (una cartella, una pipe, un dispositivo) non può essere letto (codice di
uscita 3). Un insieme di più di 10.000 file, trovati o indicati, è rifiutato come errore d'uso;
la ricerca smette di elencare al primo file oltre quel limite.
In entrambi i casi i file sono letti nell'ordine per byte dei loro percorsi, ognuno in una voce
di `documents` con il suo percorso come origine `file` e, per lo YAML, la riga di ogni valore
in `lines`; così ogni errore indica in `source` il file e la riga del valore errato.

Che i file siano trovati o indicati, il comando dà anche in `adrs` i numeri degli ADR del
progetto, letti dai nomi dei file direttamente in `docs/en/project/adr/`: un file
`<number>-<slug>.md` è l'ADR di quel numero, dove il numero è un numero di sequenza da `0001`
(quattro cifre, o cinque o più senza zero iniziale) e lo slug ha almeno un carattere; ogni
altro file, come `index.md`, non è un ADR ed è ignorato. Dentro la cartella i link, i file
speciali e i nomi non UTF-8 sono saltati, e una cartella senza alcun file di ADR dà
`adrs: []`, così ogni citazione è segnalata. Quando la cartella non esiste, è un file o è
raggiunta attraverso un link simbolico, `adrs` non è dato e il controllo `adr_citations` non è
eseguito. Una cartella con più di 10.000 file di ADR è rifiutata come errore d'uso.

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
| `markdown` | Ogni specifica viva inviata in Markdown viene normalizzata nel suo modello di documento, poi controllata come documento del tipo `specification` (vedi Specifiche vive in Markdown più sotto) |
| `document_kind` | Ogni documento ha un `kind` tra `brief`, `council_report`, `gate`, `milestone`, `epic`, `story`, `specification`, `change`, `task` e `handover`, oppure è un input con `input_kind` |
| `schema_version` | Lo `schema_version` di ogni documento è una versione che il suo tipo conosce; un documento di una versione precedente viene migrato alla versione corrente dopo aver superato lo schema della propria versione |
| `schema_<kind>`: `schema_brief`, `schema_change`, `schema_council_report`, `schema_epic`, `schema_gate`, `schema_handover`, `schema_milestone`, `schema_specification`, `schema_story`, `schema_task` | Ogni documento del tipo supera lo schema della sua versione (`schemas/artifacts/<kind>/v1.json`): membri obbligatori, membri sconosciuti, tipi, valori ammessi, lunghezze, formati, e i membri che uno stato, una verifica o un'operazione richiedono (vedi sotto) |
| `schema_<input_kind>`: `schema_commit`, `schema_pull_request` | Ogni input del tipo supera lo schema della sua versione (`schemas/inputs/<input_kind>/v1.json`): membri obbligatori, membri sconosciuti, tipi e lunghezze (vedi Input pull request e commit più sotto) |
| `schema_handover_input` | Ogni input di passaggio di mano supera lo schema di input della sua versione (`schemas/artifacts/handover/v1.input.json`), come fa un record di passaggio di mano (vedi sotto) |
| `identifier_format` | L'`id` è nella forma canonica del suo tipo, per esempio `BRIEF-0001` |
| `identifier_duplicate` | Due artefatti dell'insieme non portano mai lo stesso identificativo |
| `identifier_location` | Il nome del file di un brief, di un verbale del comitato o di un elemento del piano, e il numero della cartella di una specifica viva, indicano l'`id` dell'artefatto; altrimenti un avviso |
| `links` | Ogni collegamento di un artefatto porta a un artefatto dell'insieme, non in stato `superseded` dove conta, ogni requisito citato è in vigore, e un task o una user story appartengono alla milestone e all'epic dei loro genitori (vedi Collegamenti più sotto) |
| `parts` | Nessun identificativo di parte è dato due volte nella sua lista, un dissenso nomina una decisione del suo verbale, una raccomandazione o una risposta è una delle opzioni della sua domanda, un task non dipende da sé stesso, né da un task sostituito, né da un ciclo, una modifica a delta ha al più un'operazione per requisito e nessun membro che il suo `op` rifiuta, e né un task né un criterio di accettazione è verificato da `none` accanto a una tecnica (vedi Parti più sotto) |
| `adr_citations` | Ogni citazione `ADR <NNNN>` in `related` di una specifica viva indica un ADR di `adrs`; solo con `adrs` |
| `title_format` | Il titolo di ogni epic e di ogni task non ha un prefisso di tipo, né un tag tra parentesi quadre all'inizio o alla fine, né un punto finale (vedi Titoli di epic e task più sotto) |
| `title_duplicate` | Due titoli di epic e task, in qualunque stato tranne `superseded`, non sono mai uguali dopo la normalizzazione |
| `artifact_locations` | Nel progetto in cui i documenti sono stati scoperti esiste almeno una cartella di una posizione; solo con `locations`, dopo i controlli precedenti |

Ogni schema è un controllo, `schema_<kind>` o `schema_<input_kind>`, elencato in
`diagnostics.checks` quando è stato eseguito; ogni tipo di artefatto e ogni input ha il suo
schema in questa versione. Un documento di un tipo il cui schema il motore non ha caricato è un
errore del motore, l'errore `internal_error` che nomina il controllo che non ha potuto eseguire
(codice di uscita 5), mai un controllo non eseguito. Ogni altro controllo che il motore non ha
eseguito non è mai saltato in silenzio:

| Controllo | Non eseguito su |
| --- | --- |
| `statuses` | L'insieme: transizioni e stati iniziali rispetto a `base` |
| `naming` | L'insieme: nomi dei branch, intestazioni dei commit e collegamenti delle pull request |
| `areas` | L'insieme: l'`area` di ogni epic e di ogni task rispetto alle aree della policy del progetto |

Uno schema indica anche i membri che uno stato, una verifica o un'operazione richiedono; un
documento che ne è privo è rifiutato con il motivo `requires`, il membro mancante in `pointer` e
il membro della condizione in `pointers`:

| Tipo | Membro | Richiesto quando |
| --- | --- | --- |
| Ogni tipo | `superseded_by` | `status` è `superseded` |
| Issue di gate | `approved_by`, `issue` | `status` è `done` |
| Milestone | `tracker_number` | `status` è `implementing` o `done` |
| Epic, user story, task | `issue` | `status` è `implementing` o `done` |
| User story | `specification` | `status` è `approved`, `implementing` o `done` |
| Task | `rubric` | `verification` contiene `judge` |
| Specifica viva, ciascuno degli `acceptance_criteria` | `rubric` | la sua `verification` contiene `judge` |
| Modifica a delta, ciascuna delle `operations` | `requirement` | il suo `op` è `modified`, `removed` o `renamed` |
| Modifica a delta, ciascuna delle `operations` | `text` | il suo `op` è `added` o `modified` |
| Modifica a delta, ciascuna delle `operations` | `name` | il suo `op` è `renamed` |

Un task o una user story senza la sua epic, o un'epic, una user story o un task senza la sua
milestone, riceve un solo errore, `missing` in `/epic` o `/milestone`, e nessun errore di
collegamento. Lo schema controlla la forma di un collegamento (per esempio `EPIC-0001`), non
che il suo destinatario esista: questo è il controllo `links`. L'oggetto di una issue di gate
è un brief o un verbale del comitato, nella forma `BRIEF-<seq>` o `COUNCIL-<seq>`. `blocked`
non è lo stato di alcun artefatto: è uno stato del ciclo della issue.

Ogni tipo accetta solo i propri stati. Un verbale del comitato ha `draft`, `approved` e
`superseded`; una modifica a delta `draft`, `approved`, `implementing`, `done` e `superseded`;
una specifica viva ne ha sei, `draft`, `approved`, `surveyed`, `implementing`, `done` e
`superseded`, e ogni altro, per esempio `published`, è rifiutato con `enum` e i sei in
`expected`. La `capability` di una specifica viva è formata da parole minuscole di `a`-`z` e
`0`-`9` unite da un solo trattino, per esempio `time-entries`; un requisito è `active` o
`removed`, e assente significa `active`. Lo schema controlla la forma delle parti di un
artefatto (`D<k>`, `Q<k>`, `R<k>`, `AC<k>`) e che una domanda di un verbale del comitato abbia
almeno due opzioni, non che una parte sia unica, che un dissenso nomini una decisione del suo
verbale, che una raccomandazione o una risposta sia una delle opzioni o che una modifica a delta
abbia una sola operazione per requisito: questo è il controllo `parts` (vedi Parti più sotto).

Un passaggio di mano ha due forme di una stessa versione. L'input è ciò che un agente invia alla
fine di una sessione, con `input_kind: handover_input` e senza `kind`: `schema_version`,
`step`, `files_touched` (percorsi del progetto), `not_done`, `open_questions`, `rulings`
(ciascuna `{decision, reason, cost_if_wrong}`) e `next_step`, tutti obbligatori, le liste anche
`[]`, e `language`, facoltativo, come in ogni artefatto. Il record è ciò che il
motore scrive a partire dall'input: gli stessi membri, con `kind: handover`, `id`
(`HANDOVER-<issue>-<k>`), `issue`, `task` (facoltativo), `session`, `holder`, `executor`,
`branch`, `recorded_at` (una data e ora UTC con `Z`) e `tokens` (un intero da 1). Un membro del
motore in un input, per esempio `tokens`, è rifiutato con `unknown_field`. Ogni errore di un
input ha il codice `handover_input`; gli errori di un record hanno il codice `artifact_schema`.
`step` è un passo del ciclo della issue, scritto in `snake_case`, come lo stampa
[`wayloop steps`](index.md#comandi-di-una-build-dai-sorgenti). Un passaggio di mano non ha
titolo né stato; il riepilogo di un input in `data.documents` contiene il suo `input_kind` e,
quando è noto, la sua `origin`; nessun `kind`, `id` o `status`.

`artifact_locations` è elencato in `diagnostics.not_done` con il motivo `input_absent` quando
`locations` non è dato: i documenti sono stati indicati per nome o inviati direttamente.
`adr_citations` vi è elencato con lo stesso motivo quando `adrs` non è dato: l'input è stato
inviato direttamente senza di esso, o il progetto non ha la cartella `docs/en/project/adr/`.

### Collegamenti

Ogni collegamento è risolto per identificativo all'interno dell'insieme, mai per percorso: una
specifica viva spostata in un'altra cartella e rinominata viene comunque trovata. I collegamenti
di un artefatto sono letti solo quando non ha errori propri, del suo schema o della
normalizzazione del suo Markdown, così che un documento difettoso non causi un'ondata di errori
di collegamento; un tale artefatto è comunque trovato come destinazione, purché il suo `id` non
abbia dato errori. Un elemento che vive solo nell'issue tracker non è nell'insieme finché il
tracker non viene letto, quindi un collegamento verso di esso è mancante.

| Tipo | Membri che collegano |
| --- | --- |
| Verbale del comitato | `brief` |
| Issue di gate | `subject` |
| Milestone | `council_report` |
| Epic | `milestone` |
| User story | `epic`, `milestone`, `specification` |
| Specifica viva | `milestone`, `stories`; i `requirements` di ogni criterio di accettazione indicano requisiti della stessa specifica |
| Modifica a delta | `specification`, `tasks`; il `requirement` di un'operazione `modified`, `removed` o `renamed` indica un requisito di quella specifica |
| Task | `epic`, `milestone`, `story`, `specification` (ogni `spec` e i suoi `requirements`), `depends_on`, `expanded_from` |
| Record di passaggio di mano | `task` |
| Ogni tipo | `superseded_by` |

- Un collegamento a un identificativo che nessun artefatto dell'insieme porta è
  `link_missing`, motivo `missing`, al membro del collegamento.
- Un collegamento a un artefatto in stato `superseded` è `link_superseded`, motivo `state`. I
  suoi sostituti sono gli identificativi del suo `superseded_by`. Con esattamente uno,
  quell'identificativo è in `fix`, applicabile così com'è, per esempio `"SPEC-0001"`, ed
  `expected` elenca solo lui. Con più sostituti non sono dati né `fix` né `expected`: nessun
  errore copia una lista che cresce con l'input, quindi una voce di `related` indica dove si
  trova la lista, `{document, pointer: "/superseded_by"}` della destinazione con i membri della
  sua `source`, e il messaggio nomina solo la destinazione. Quando la destinazione è il documento
  stesso dell'errore, come in un riferimento a sé stesso nel suo `superseded_by`, è invece
  `pointers` a contenere `/superseded_by`, perché `related` nomina solo altri documenti. Un
  `fix` è un valore valido al suo puntatore, non la garanzia che l'insieme diventi valido una
  volta applicato.
- `superseded_by` indica artefatti non in stato `superseded`: un sostituto a sua volta
  sostituito, come in una catena o in un riferimento a sé stesso, è un collegamento interrotto
  del suo titolare. `expanded_from` (il task da cui un sottotask è stato espanso è sostituito da
  esso) e `depends_on` accettano una destinazione in stato `superseded`, perché registrano la
  storia del lavoro; una dipendenza da un task sostituito è una regola del controllo `parts`.
- I sostituti non comprendono mai la destinazione sostituita stessa, né l'artefatto che porta il
  collegamento, che non nomina mai sé stesso: `[titolare, X]` e `[destinazione, X]` danno il fix
  `X`; quando non ne resta nessuno, `expected`, `fix` e `related` sono assenti.
- Un requisito citato che la specifica non ha è `link_missing`; uno rimosso è
  `link_superseded`, motivo `state`, senza `expected` né `fix`, perché non ha sostituti. I
  requisiti sono controllati solo quando la specifica è nell'insieme e ha superato il suo
  schema.
- Un task o una user story la cui `milestone` è diversa da quella della sua epic, o un task la
  cui `epic` è diversa da quella della sua user story, è `hierarchy_mismatch`, motivo
  `conflict`, in `/milestone` o `/epic`, con il membro del genitore (`/epic` o `/story`) in
  `pointers` e il valore raggiunto attraverso il genitore in `expected`.
- Un artefatto in stato `done` o `superseded` non viene più modificato, e un record di passaggio
  di mano non viene mai modificato: ogni collegamento interrotto di un tale artefatto è l'avviso
  `link_broken_terminal`, con il motivo, `expected`, `fix` e `related` dell'errore che
  sostituisce, e la gerarchia non è controllata.
- Nessun altro errore di collegamento o di gerarchia porta `related`: il puntatore e il
  messaggio nominano la destinazione.
- Una citazione è la parola `ADR` seguita da un numero, per esempio `ADR 0014`; `ADR 14` cita
  lo stesso ADR. Un numero da solo che segue una citazione, nella stessa voce o nelle voci
  subito dopo, cita anch'esso un ADR: `ADR 0002, 0003` ne cita due, anche quando la riga di
  intestazione `Related` di una specifica in Markdown lo divide in due voci. Una parola che apre
  una parentesi chiude la sequenza: `ADR 0014 (2026)` cita solo 0014, mentre `(ADR 0008)` cita
  0008. Con `adrs`, una citazione il cui numero non è elencato è l'avviso
  `adr_citation_missing`, motivo `missing`, alla voce di `related` che la contiene.

Questa versione non controlla che una milestone colleghi un verbale del comitato approvato, né
che il task di un record di passaggio di mano porti la sua issue o che la issue sia un task del
piano.

Quando l'input stesso è rifiutato, o la riga di comando non è compresa, o l'input non può
essere letto, nessun controllo viene eseguito: `diagnostics.checks` e `diagnostics.not_done`
sono allora vuoti.

### Parti

Le parti di un artefatto sono lette dopo che ha superato il suo schema. Ogni regola qui sotto
rifiuta il valore al suo puntatore. Gli errori di questo controllo vengono dopo quelli dei
collegamenti e prima di quelli dei titoli, anche quelli con il codice `artifact_schema`. Solo una
dipendenza da un task sostituito con più sostituti nomina un altro documento in `related`, come
fa un collegamento.

| Regola | Codice | Motivo | Puntatore | Altri membri |
| --- | --- | --- | --- | --- |
| Un identificativo di parte dato due volte nella sua lista: `decisions` e `questions` di un verbale del comitato; `requirements`, `acceptance_criteria` e `open_questions` di una specifica viva; `acceptance_criteria` di una user story | `part_invalid` | `duplicate` | L'`id` di ogni parte dopo la prima che lo porta | `pointers`: l'`id` della prima, poi il proprio |
| Un dissenso che nomina una decisione che il verbale non ha | `part_invalid` | `missing` | `/dissent/<i>/decision` | `pointers`: `/decisions` |
| Una raccomandazione o una risposta che non è una delle opzioni della sua domanda | `part_invalid` | `enum` | `/questions/<i>/recommendation` o `/questions/<i>/answer` | `expected`: ogni opzione una volta, nel suo ordine; `pointers`: `/questions/<i>/options` |
| Un task che dipende da sé stesso | `part_invalid` | `conflict` | `/depends_on/<i>` | `pointers`: `/id` |
| Un task che dipende da un task sostituito | `part_invalid` | `conflict` | `/depends_on/<i>` | `expected`, `fix` o `related` come per un collegamento a un artefatto sostituito (vedi Collegamenti) |
| Un task che dipende da un task che riconduce a esso, direttamente o tramite altri task | `part_invalid` | `conflict` | `/depends_on/<i>` di ogni dipendenza del ciclo | nessuno; nessun `related` |
| Due operazioni di una modifica a delta sullo stesso requisito | `delta_conflict` | `conflict` | `/operations/<i>` di ogni operazione dopo la prima su quel requisito | `pointers`: la prima operazione, poi la propria |
| Un membro che l'`op` di un'operazione rifiuta: `requirement` con `added`, `text` con `removed` e `renamed` (una rinomina con un nuovo testo è una sola operazione `modified`), `name` con `removed` | `artifact_schema` | `unknown_field` | `/operations/<i>/<member>` | `pointers`: `/operations/<i>/op` |
| Un task o un criterio di accettazione di una specifica viva la cui `verification` contiene `none` accanto a una tecnica | `artifact_schema` | `enum` | `/verification/<i>` o `/acceptance_criteria/<i>/verification/<j>` di ogni `none` | `expected`: le tecniche; `pointers`: la prima tecnica della lista |

Lo schema non può indicare l'assenza di un membro sotto una condizione, quindi i membri che un
`op` rifiuta sono rifiutati da questo controllo. Un requisito rifiutato in un'operazione `added`
non indica alcun requisito, quindi non è in conflitto con nessun'altra operazione. L'ordine dei
valori della verifica e un valore ripetuto non hanno significato: `[none, none]` e `[tdd, tdd]`
sono accettati.

Le dipendenze di un task sono controllate solo quando il task non ha errori propri e non è in
stato `done` o `superseded`, perché un tale task non viene più modificato; le dipendenze di un
task in stato `done` o `superseded` non fanno neppure parte del grafo delle dipendenze. Ogni
task dell'insieme è una destinazione, purché il suo `id` non abbia dato errori. Ogni dipendenza
di un ciclo è rifiutata, non solo quella che lo ha chiuso, perché l'ordine dei documenti non
dice quale lo abbia fatto. Una dipendenza da un task che nessun artefatto dell'insieme porta è
soltanto un errore dei collegamenti.

Quando una modifica a delta viene unita, un requisito aggiunto riceve l'identificativo che
segue il numero più alto mai usato nella sua specifica viva, requisiti rimossi compresi: con da
`R1` a `R3` e `R4` rimosso, è `R5`. Il motore dà quell'identificativo con una funzione del suo
core; il merge stesso non è un comando di questa versione.

### Specifiche vive in Markdown

Una specifica viva è memorizzata come `specs/<NNNN>-<capability>/spec.md`, con la struttura del
modello `specs/_templates/spec.md`. Ogni file Markdown indicato sulla riga di comando è letto
come specifica viva, ovunque si trovi; fuori da una cartella `specs/<NNNN>-<capability>/` non
ha `capability`, che è quindi rifiutata con il motivo `missing`. Il motore legge il file riga
per riga, senza un parser Markdown, nel modello di documento del tipo `specification`, poi
applica lo schema; ogni errore riporta in `source` la riga da cui il valore è stato letto. La
forma non riporta `kind` e `schema_version`: sono scritti come `specification` e la versione
corrente, ed elencati in `meta.defaults_applied` con l'indice del documento in `document`, i
puntatori `/kind` e `/schema_version` e il motivo `implied_by_form`. Titoli di sezione, righe
di intestazione e parole di stato sono in inglese, qualunque sia la lingua del progetto; la
prosa è nella lingua del progetto.

| Markdown | Membro |
| --- | --- |
| `# SPEC-<seq>: <title>` | `id`, `title`; senza i due punti, tutto il testo è il titolo |
| Righe di intestazione, una tabella sotto il titolo | Una riga per ciascun membro qui sotto; una riga le cui due celle sono entrambe vuote o fatte solo di `-`, `:` e spazi, come l'intestazione e il separatore della tabella, è saltata |
| Cartella `specs/<NNNN>-<capability>/` | `capability`; assente quando il file è altrove o è inviato senza file |
| Riga di intestazione `Status` | `status`, in minuscolo: `Draft` dà `draft` |
| Riga di intestazione `Milestone` | `milestone`, la prima parola: `M0` di `M0 · Foundations` |
| Righe di intestazione `Owner`, `Commit scope`, `Language` | `owner`, `commit_scope`, `language`, come scritte |
| Riga di intestazione `Related` | `related`, divisa a ogni virgola e punto e virgola |
| Riga di intestazione `Superseded by` | `superseded_by`, divisa a ogni virgola |
| `## Problem` | `problem`, il testo come scritto |
| `## Scope`, paragrafi `**In scope:**` e `**Out of scope:**` | `scope_in`, `scope_out`: una voce per ogni elemento di elenco, oppure il paragrafo come una voce |
| `## User stories` | un elemento che è solo un identificativo `STORY` va in `stories`, ogni altro in `user_stories` |
| `## Requirements (EARS)`, elementi `- **R<k>**: <text>`, `- **R<k>** *<name>*: <text>`, `- **R<k>** (removed): <text>` | `requirements` |
| `## Acceptance criteria`, elementi `- **AC<k>** (R<i>, R<j>): <text> Verification: <words>. Rubric: <text>` | `acceptance_criteria`; `Rubric:` è facoltativo |
| `## Interfaces` | `interfaces`, una voce per elemento |
| `## Open questions`, elementi `- [ ] <text>` e `- [x] <text>`, che possono iniziare con `**Q<k>. <question>**` | `open_questions` |
| Ogni altra sezione `##` | `extra_sections`, `{heading, text}`, conservata come prosa e non validata |

Un elemento di elenco termina al successivo elemento, al successivo titolo di sezione o a una
riga vuota; le righe che lo seguono, e una riga rientrata dopo una riga vuota, lo continuano e
sono unite con uno spazio. Un paragrafo in una sezione letta come elenco è una voce, quindi un
requisito scritto senza il suo `**R<k>**` è rifiutato al suo `id`. Un blocco delimitato si apre
con tre o più backtick o tilde e si chiude solo con una sequenza dello stesso carattere almeno
altrettanto lunga, seguita solo da spazi; una riga al suo interno non è mai un titolo, un
titolo di sezione, una riga di intestazione, un elemento di elenco o un marcatore di Scope.

Una sezione il cui titolo non è nella tabella, compreso un titolo tradotto come `## Requisiti`,
è conservata in `extra_sections`; un membro obbligatorio la cui sezione manca è quindi
rifiutato con il motivo `missing`, per esempio in `/requirements`. Una sezione presente ma
vuota dà un valore vuoto, che lo schema rifiuta quando il membro ne richiede uno.

Le parole di verifica sono riconosciute senza distinguere maiuscole e minuscole nel testo dopo
`Verification:` fino al primo punto, nell'ordine in cui compaiono; le altre parole sono
ignorate, quindi `property test for production, TDD for validation` dà `[property, tdd]`:

| Parola | Valore |
| --- | --- |
| `TDD` | `tdd` |
| `BDD`, `BDD scenario` | `bdd` |
| `end-to-end` | `end_to_end` |
| `property`, `property test` | `property` |
| `contract`, `contract test` | `contract` |
| `benchmark` | `benchmark` |
| `judge` | `judge` |
| `none` | `none` |

Quattro difetti non hanno un membro nel modello di documento e sono rifiutati dalla
normalizzazione, con il codice `artifact_schema` e la loro riga: un testo di verifica senza
alcuna parola della tabella (`enum` in `/acceptance_criteria/<i>/verification`, le parole in
`expected`; lo schema poi non riporta altro su quel membro), una riga di intestazione che non è
nella tabella o ne ripete una (`unknown_field`, all'etichetta come scritta, per esempio
`/Stato`), una sezione del modello o un paragrafo di Scope dati due volte (`duplicate`, al
membro che riempiono, `/scope_in` per la sezione Scope e `/stories` per User stories,
`/scope_in` o `/scope_out` per un `**In scope:**` o un `**Out of scope:**` ripetuto, con la riga
del secondo titolo o marcatore; il loro contenuto non viene letto), e testo fuori dal titolo,
dalle righe di intestazione e dalle sezioni, o nella sezione Scope prima dei suoi paragrafi
(`syntax`, alla radice). Ogni altro valore è scritto come letto e controllato dallo schema al
suo membro: il testo di un criterio dopo le sue parole di verifica che non inizia con `Rubric:`
è aggiunto al suo `text`, e la radice è alla riga 1, come per un file YAML, quindi un membro
mancante e `capability` riportano la riga 1.

### Input pull request e commit

Un input non è un artefatto: non ha `kind`, identificativo né stato, e l'`input_kind` della sua
voce ne sceglie lo schema. Come un artefatto, porta lo `schema_version` del suo schema: è
controllato rispetto allo schema di quella versione e migrato alla versione corrente, e una
versione sconosciuta è rifiutata con `schema_version_unknown`. Un membro di un artefatto come
`kind` è rifiutato come `unknown_field`. Un documento privo sia di `kind` sia di `input_kind` è
rifiutato con il motivo `missing` in `/kind`; il nome di un input in `kind`, per esempio
`kind: commit`, è rifiutato con il motivo `enum`, perché `kind` nomina solo tipi di artefatto.
La riga di comando legge solo artefatti memorizzati; gli input si inviano con `--input`.

| Input | Membro | Tipo | Obbligatorio |
| --- | --- | --- | --- |
| `pull_request`, `commit` | `schema_version` | intero, la versione dello schema, oggi `1` | sì |
| `pull_request` | `title` | stringa di almeno un carattere, il titolo della pull request | sì |
| `pull_request` | `body` | stringa, dove è collegata la issue; può essere vuota | sì |
| `pull_request` | `branch` | nome di almeno un carattere, il branch di partenza come lo scrive git; non è prosa | sì |
| `pull_request` | `labels` | lista di nomi, ciascuno di almeno un carattere, come li scrive il tracker; non sono prosa; `[]` significa nessuna | sì |
| `commit` | `header` | stringa di almeno un carattere, la prima riga del messaggio | sì |
| `commit` | `body` | stringa di almeno un carattere, le righe dopo la riga vuota, trailer compresi; assente quando il messaggio ha una sola riga | no |

Un testo vuoto dove ne è richiesto uno è rifiutato con il motivo `length`. Lo schema controlla
solo la forma. La grammatica di un nome di branch e di un'intestazione di commit, e i
collegamenti alle issue di una pull request, sono il controllo `naming`, che questa versione
non esegue.

### Titoli di epic e task

Il titolo di un epic o di un task nomina il lavoro, senza il suo tipo e senza i marcatori di un
tracker. Il suo schema ne fissa la lunghezza, al massimo 120 caratteri; i controlli
`title_format` e `title_duplicate` leggono ogni epic e ogni task il cui `title` non ha dato
errori e rientra in quella lunghezza, qualunque siano i suoi altri errori.

| Regola | Esempio rifiutato | `fix` |
| --- | --- | --- |
| Nessun prefisso di tipo: un `<word>:`, `<word>(<anything>):`, `<word>!:` o `<word>(<anything>)!:` iniziale dove `<word>` è un tipo della policy, senza distinzione tra maiuscole e minuscole | `feat(core)!: Record time entries` | `Record time entries` |
| Nessun tag tra parentesi quadre `[...]` all'inizio o alla fine, chiuso dalla parentesi corrispondente, contando quelle annidate; un gruppo non bilanciato non è un tag | `[EPIC] Monthly export of time entries` | `Monthly export of time entries` |
| Nessun punto finale | `Monthly export of time entries.` | `Monthly export of time entries` |

Una parola che non è un tipo della policy non è un prefisso: `wayloop init creates a project
structure that validates` è accettato. Ogni regola violata è un errore `title_format`,
nell'ordine della tabella, e ogni `fix` è il titolo con tutte le regole applicate, quindi conta
anche una regola che emerge togliendone un'altra: `[EPIC] feat: Export.` dà tre errori il cui
`fix` è `Export`. Quando del titolo non resta nulla, l'errore non ha `fix`.

Due titoli sono uguali quando lo sono dopo la normalizzazione Unicode NFC, il case folding, di
nuovo la normalizzazione NFC, la rimozione degli spazi iniziali e finali e la riduzione di ogni
sequenza di spazi bianchi a un solo spazio: `Export of the café entries` e ` EXPORT of the  CAFÉ
entries` sono un solo titolo. I titoli di epic e task sono confrontati tra loro, in ogni stato
tranne `superseded`. Il case folding è quello completo di Unicode: la s tedesca `ß` equivale a
`ss`, e il sigma finale al sigma.

## Input

L'input canonico è un oggetto JSON. Un membro impostato a `null` non è dato, esattamente come
un membro assente. Lo schema di input è chiuso: un membro che non elenca è rifiutato.

| Membro | Tipo | Obbligatorio | Significato |
| --- | --- | --- | --- |
| `documents` | lista di oggetti | sì | Una voce per ogni artefatto o input memorizzato; `[]` significa nessuno |
| `base` | lista di `{id, status}` | no | Stati sul branch predefinito degli artefatti dell'insieme |
| `policy` | oggetto `{types, scopes, areas}`, ciascuno una lista di nomi della policy, identificativi di almeno un carattere | no | La parte della policy del progetto che il modello legge. Il controllo `title_format` ne legge i `types`; il controllo `areas` è elencato come non eseguito. Assente: vale la policy predefinita, la policy del repository di Wayloop, e `meta.defaults_applied` la elenca quando l'insieme contiene un epic o un task, i cui controlli la leggono |
| `adrs` | lista di interi da 1 | no | Numeri degli ADR del progetto, per il controllo `adr_citations`. Assente: le citazioni non sono controllate, e il controllo non è eseguito. La riga di comando lo compila dai nomi dei file di `docs/en/project/adr/` quando quella cartella esiste |
| `locations` | lista di `plan_briefs`, `plan_council`, `plan_items`, `specs`, per le cartelle `plan/briefs`, `plan/council`, `plan/items` e `specs` | no | Le cartelle delle posizioni che esistono nel progetto, quando i documenti vi sono stati scoperti; il loro ordine e una cartella ripetuta non hanno significato, non sono confrontate con le origini `file` dei documenti, e `[]` significa nessuna, e dà l'avviso `artifact_location_missing`. Assente: i documenti sono stati indicati per nome o inviati direttamente, e il controllo `artifact_locations` non è eseguito |
| `language` | tag BCP 47, per esempio `it` o `en-GB` | no | La lingua del progetto. Nessun controllo di questa versione la legge, quindi quando è assente nessun default viene applicato né riportato; un controllo che ne ha bisogno elenca il default in `meta.defaults_applied` |

Una voce di `documents`:

| Membro | Tipo | Obbligatorio | Significato |
| --- | --- | --- | --- |
| `format` | `document`, `markdown` o `unreadable` | sì | `document`: un valore JSON nella forma normalizzata del suo tipo; `markdown`: il testo di una specifica viva; `unreadable`: un file che non è stato possibile analizzare |
| `document` | oggetto | con `document` | Il documento |
| `lines` | oggetto | no; solo con `document` e un'origine `file` | La riga, da 1, di ogni valore del documento, indicizzata dal suo puntatore RFC 6901 nel documento; lo schema dichiara entrambe le regole: chiavi nel formato `json-pointer`, valori interi da 1. Una riga impostata a null non è data, come ogni membro (ADR 0009 C2); un validatore del semplice draft 2020-12 la rifiuta |
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
senza membri o con più di uno, una chiave di `lines` che non è un puntatore RFC 6901 (`syntax`,
al puntatore del membro, con un messaggio sul nome del membro) e una riga che non è un intero
da 1 (`type` o `range`).

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
  "current_command_version", "message": "No version of the command was requested: version 1
  runs."}`: il puntatore è vuoto perché la versione è un'impostazione del
  contesto, non un membro dell'input. Quando `--command-version` indica una versione che il
  motore non esegue, anche il rifiuto ha il puntatore `""`, e `meta.command_version` è la
  versione corrente, di cui l'envelope ha la forma; quando l'input è anche un testo che il
  motore non può leggere come JSON, lo stesso rapporto elenca prima la versione, poi l'errore
  del testo. Un errore d'uso riporta la versione corrente del comando che indica, e 1 quando
  non indica un comando noto.
- Quando l'input non ha `policy` e l'insieme contiene un epic o un task, la policy predefinita
  segue il valore predefinito della versione del comando: `{"pointer": "/policy", "value":
  {"types": [...], "scopes": [...], "areas": [...]}, "reason": "default_policy", "message":
  ...}`, senza `document`, perché è un valore predefinito dell'input stesso. I suoi tipi e
  scope sono quelli delle
  [convenzioni](https://salvo8519.github.io/wayloop/project/conventions/#commits) del
  repository di Wayloop, e le sue aree quelle delle sue etichette `area:`.
- I valori predefiniti dei documenti seguono quelli della versione del comando e della policy,
  nell'ordine dei documenti, poi nell'ordine dei byte dei loro puntatori. Ognuno porta
  `document`, l'indice del suo documento da 0, come un errore, e il suo `pointer` è relativo a
  quel documento: una specifica viva inviata in Markdown dà `{"pointer": "/kind", "value":
  "specification", "reason": "implied_by_form", "message": "The Markdown form of a living
  specification does not carry `kind`.", "document": <i>}` e lo stesso per `/schema_version` con
  la versione corrente. Il testo di `message` non fa parte del contratto.
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
   fase (prima tipo, versione e schema, poi identificativo, poi gli identificativi dell'insieme,
   poi i collegamenti, poi le parti, poi la forma del titolo e i titoli dell'insieme); dentro
   una fase, nell'ordine del passaggio dello schema: i membri di un oggetto nell'ordine dei byte
   delle loro chiavi (`/aa` prima di `/zz`), gli elementi di un array per indice, i membri
   assenti dopo quelli presenti, nell'ordine in cui lo schema li elenca come obbligatori. Un
   errore del codice `artifact_schema` trovato dal controllo `parts` viene nella fase delle
   parti. Gli errori del nome
   di un membro vengono prima degli errori del suo valore, allo stesso puntatore.

Ogni `pointer` del risultato è un puntatore RFC 6901; gli schemi dell'output e dell'envelope gli
danno il formato `json-pointer`, che un validatore JSON Schema controlla quando la verifica dei
formati è attiva (nel draft 2020-12 un formato è per default un'annotazione). Il `pointer` di
un errore dell'input punta nell'input canonico, per esempio `/documents/0/format`. Il
`pointer` di un errore di un documento punta in quel documento, per esempio `/problem`;
`document` dà l'indice, da 0, di quel documento in `documents`, e `source` dice dove è
memorizzato:

| Origine del documento | `source` |
| --- | --- |
| `{file}` | `{file, line}`: la riga data in `lines` per il puntatore dell'errore, o per il valore più vicino che lo contiene; solo `{file}` quando `lines` non dà nessuna delle due, mai una riga indovinata |
| `{issue}` | `{issue}` |
| `{milestone}` | `{milestone}` |
| assente | assente |

Un errore la cui condizione coinvolge altri documenti, come due artefatti con lo stesso
identificativo, ne nomina uno in `related`, così che nessun errore copi una lista che cresce con
l'input: `{document, pointer}`, con i membri della
`source` di quel documento (`file` e `line`, `issue` o `milestone`) nella voce stessa quando è
stato memorizzato. Non sono annidati in un membro `source`, così l'envelope resta entro quattro
livelli di oggetti (ADR 0009 C4). `related` è assente quando non c'è un altro documento, mai
`[]`.

| Codice | Motivi | Codice di uscita | Significato |
| --- | --- | --- | --- |
| `command_input` | `syntax`, `duplicate`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `conflict` | 1 | L'input canonico è rifiutato: non è JSON, ripete una chiave, contiene un numero oltre un double, o viola il suo schema o una regola della sezione Input; oppure `--command-version` richiede una versione che il motore non esegue (`enum`, le versioni note in `expected`) |
| `artifact_schema` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `duplicate` | 1 | Un documento viola lo schema del suo tipo o del suo input (una pull request, un commit); anche un documento senza `kind`, o con un `kind` fuori dalla lista (`enum`, con i tipi in `expected`), uno `schema_version` assente, non intero (riportato con ogni altro errore rispetto alla versione corrente) od oltre più o meno (2^53 - 1) (`range`), un file che non è stato possibile analizzare (`syntax`, al puntatore dell'intero documento, con la sua riga in `source`), i difetti di una specifica viva in Markdown, tra cui una sezione data due volte (`duplicate`; vedi Specifiche vive in Markdown), un membro che l'`op` di un'operazione rifiuta (`unknown_field`) e `none` accanto a una tecnica nella `verification` di un task o di un criterio di accettazione (`enum`; vedi Parti) |
| `handover_input` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires` | 1 | Un input di passaggio di mano viola il suo schema, per esempio impostando un membro che solo il motore imposta (`unknown_field`) |
| `schema_version_unknown` | `enum` | 1 | Lo `schema_version` non è una versione del tipo o dell'input; `expected` elenca le versioni note |
| `identifier_format` | `syntax` | 1 | L'`id` non è nella forma canonica del suo tipo; `fix` dà l'identificativo corretto quando è possibile ricavarlo |
| `identifier_duplicate` | `duplicate` | 1 | Due o più artefatti dell'insieme portano lo stesso identificativo: ciascuno è rifiutato in `/id`, e nomina un altro in `related`, il primo il secondo, ogni successivo il primo. È confrontato ogni documento di un `kind` noto il cui `id` non ha dato errori, qualunque siano i suoi altri errori, così i duplicati arrivano nello stesso passaggio |
| `link_missing` | `missing` | 1 | Un collegamento indica un identificativo che nessun artefatto dell'insieme porta, o un requisito che la sua specifica non ha (vedi Collegamenti) |
| `link_superseded` | `state` | 1 | Un collegamento indica un artefatto in stato `superseded`: con esattamente un sostituto, questo è in `expected` e `fix`; con più sostituti, `related` indica il `superseded_by` della destinazione; o un requisito rimosso, senza né l'uno né l'altro |
| `hierarchy_mismatch` | `conflict` | 1 | Un task o una user story indicano una milestone o un'epic diversa da quella raggiunta attraverso il loro genitore |
| `part_invalid` | `duplicate`, `missing`, `enum`, `conflict` | 1 | Un identificativo di parte dato due volte nella sua lista, un dissenso che non nomina una decisione del suo verbale, una raccomandazione o una risposta fuori dalle opzioni della sua domanda, o una dipendenza di un task da sé stesso, da un task sostituito o da un ciclo (vedi Parti) |
| `delta_conflict` | `conflict` | 1 | Due o più operazioni di una modifica a delta sullo stesso requisito: ognuna dopo la prima è rifiutata, con la prima in `pointers` |
| `title_format` | `syntax` | 1 | Il titolo di un epic o di un task ha un prefisso di tipo, un tag tra parentesi quadre all'inizio o alla fine, o un punto finale: un errore per regola in `/title`, ciascuno con `fix` il titolo con tutte le regole applicate, quando ne resta qualcosa |
| `title_duplicate` | `duplicate` | 1 | Due o più titoli di epic o task, in qualunque stato tranne `superseded`, sono uguali dopo la normalizzazione: ciascuno è rifiutato in `/title`, e nomina un altro in `related`, il primo il secondo, ogni successivo il primo |
| `usage` | `syntax` | 2 | La riga di comando non è compresa; `command` è il comando indicato, vuoto quando nessuno |
| `input_unreadable` | `missing`, `state` | 3 | Il file di input, un file indicato o una cartella del progetto non esiste (`missing`) o non può essere letto (`state`); l'envelope non contiene percorsi |
| `internal_error` | `internal` | 5 | Il motore non ha potuto calcolare il risultato; l'input può essere valido |

Gli avvisi sono elencati in `diagnostics.warnings`, nella forma di un errore; il risultato non
è rifiutato per essi e il codice di uscita non cambia. Arrivano in un ordine fisso: gli avvisi
`identifier_location` nell'ordine dei documenti, poi `link_broken_terminal`, poi
`adr_citation_missing`, ciascuno nell'ordine dei documenti, poi `artifact_location_missing`.

| Codice | Motivo | Puntatore | Significato |
| --- | --- | --- | --- |
| `identifier_location` | `conflict` | `/id` | L'artefatto è stato letto da `plan/briefs/<name>.yaml`, `plan/council/<name>.yaml`, `plan/items/<name>.yaml` o `specs/<number>-<capability>/spec.md` (`<number>` di cifre ASCII, in qualunque forma), e `<name>`, o `SPEC-<number>`, è diverso dal suo `id`; `fix` è l'`id`, che è l'identità. Un documento inviato direttamente, o letto da un altro percorso, non ne dà |
| `link_broken_terminal` | `missing`, `state` | Il membro del collegamento | Un collegamento di un artefatto in stato `done` o `superseded`, o di un record di passaggio di mano, è interrotto, come direbbero `link_missing` o `link_superseded` per un altro artefatto, con gli stessi `expected`, `fix` e `related` |
| `adr_citation_missing` | `missing` | `/related/<i>` | Una specifica viva cita un ADR il cui numero non è in `adrs` |
| `artifact_location_missing` | `missing` | `/locations` | Nel progetto in cui i documenti sono stati scoperti non esiste alcuna cartella di una posizione, per esempio perché il comando è stato eseguito in una sottocartella del progetto |

Un membro sconosciuto riceve in `fix` il membro più vicino, quando ce n'è uno vicino: per
esempio `problem` per `problema`.

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
