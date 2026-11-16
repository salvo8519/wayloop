---
source: concepts/ledger.md
source_hash: 66699d1042c9
---
# Ledger

Il ledger è un registro di eventi in sola aggiunta, scritto solo dal motore e mai da un
modello. Lo stato del loop è una funzione del ledger: rileggendo gli eventi dall'inizio si
ottiene sempre lo stesso stato.

| Elemento | Formato | Regola |
| --- | --- | --- |
| Evento | JSON Lines, un evento per riga | Campi fissi: sequenza, istante UTC, attore, issue, passo, esito, token, hash dell'input, hash dell'evento precedente |
| Stato corrente | YAML | Una proiezione ricostruita dal ledger, mai modificata a mano |
| Riassunto di sessione | JSON validato da uno schema | Fatto, decisioni, domande aperte, prossimi passi, file toccati, costo; pubblicato sull'issue come commento di passaggio |
| Istantanea | JSON | Lo stato completo a un punto del ledger, per non rigiocare mesi di eventi |

## Perché è deterministico

- **I modelli non scrivono il ledger.** Le loro uscite entrano solo dopo la validazione dello
  schema; un'uscita non valida diventa un evento di errore, mai uno stato corrotto.
- **Catena di hash.** Ogni evento contiene l'hash del precedente: una modifica o un buco nel
  registro si vedono subito.
- **Il replay è testato.** Un test della CI rigioca il ledger e confronta lo stato con la
  proiezione YAML.

## Cicli di vita lunghi

Un progetto può durare mesi e centinaia di sessioni. Tre meccanismi tengono limitata la
memoria.

1. **Arrotolamento gerarchico.** I riassunti di sessione si fondono nel riassunto dell'issue
   alla chiusura; i riassunti delle issue nel riassunto della milestone. Ogni livello ha una
   dimensione massima in token.
2. **Istantanee periodiche.** Dopo un'istantanea, gli eventi precedenti si possono archiviare.
3. **Schemi versionati.** Ogni evento e ogni riassunto dichiarano la versione dello schema; il
   motore migra le versioni vecchie.

Un agente che arriva dopo settimane legge il riassunto della milestone, quello dell'issue e
l'ultimo commento di passaggio: un contesto piccolo e di dimensione nota, qualunque sia la
storia del progetto.

## Trovare le cose

I file sono la fonte; da essi si costruisce un indice SQLite con ricerca full-text, che si può
sempre ricostruire. Gli agenti lo interrogano tramite la CLI e il server MCP, mai
direttamente. Vedi
[ADR 0007](https://salvo8519.github.io/wayloop/project/adr/0007-files-as-source-sqlite-as-index/).

## Due memorie, dette chiaramente

Il ledger locale è il registro operativo di una macchina e si perde con essa. La memoria
durevole del progetto è l'insieme dei riassunti pubblicati sulle issue.
