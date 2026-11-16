---
source: concepts/the-loop.md
source_hash: a51cde3b1553
---
# Il loop

Il loop tiene allineati il piano e il lavoro. Scompone il progetto, esegue il lavoro e, a ogni
evento, propone come dovrebbe cambiare il piano. Non cambia mai il piano da solo.

## Quattro movimenti

1. **Scomposizione.** Da un'idea, o dal rilievo di un codice esistente, il motore guida la
   discesa attraverso i livelli del progetto: milestone, epic, user story, specifiche, task e
   issue. Gli agenti scrivono ogni livello; il motore lo valida prima che parta il successivo.
2. **Esecuzione.** Le issue attraversano il ciclo descritto sotto. Ogni passo è registrato nel
   [ledger](ledger.md).
3. **Riconciliazione.** Ogni evento produce una proposta di modifica della roadmap: un'issue
   chiusa, una pull request unita, una nuova richiesta del maintainer, lavoro scoperto durante
   lo sviluppo, una specifica cambiata, una lacuna trovata dal rilievo. La proposta è una
   differenza leggibile che una persona approva o corregge.
4. **Ripianificazione.** Dopo l'approvazione il motore ricalcola dipendenze, prossimi task,
   corsie dello sprint e avanzamento di ogni milestone, e aggiorna il tracker.

## Il ciclo di un'issue

| Passo | Cosa succede | Chi |
| --- | --- | --- |
| Presa in carico | L'issue viene assegnata a una sessione, etichettata come in corso, con un branch e un worktree | Motore |
| Rianalisi | Si rileggono l'issue, la sua story, la sua specifica e il codice coinvolto; la specifica viene confrontata con il codice attuale | Agente |
| Controdeduzione critica | Un secondo agente cerca lacune, casi limite e contraddizioni; ogni obiezione si chiude con una decisione registrata o con una domanda | Agente |
| Checklist | La checklist del task viene generata dalla specifica e pubblicata sull'issue | Agente, validato dal motore |
| Sviluppo | Test dove conviene, poi il codice | Agente |
| Verifica | Controlli deterministici decidono se il lavoro è finito | Motore |
| Review | Proporzionata al rischio: nessuna, un passaggio o due | Agente |
| Pull request | Collegata all'issue e ai requisiti che implementa | Motore |
| Merge | Stato di approvazione: decide una persona | Maintainer |

## Un giro del loop

Il motore possiede la macchina a stati; gli agenti sono esecutori chiamati per un passo alla
volta.

1. Legge lo stato: tracker, repository e ledger.
2. Sceglie la prossima transizione ammessa dallo stato, con le dipendenze soddisfatte.
3. Prepara un brief minimo: solo i file e gli estratti che servono al passo, entro il budget di
   token della fase.
4. Chiama un agente: Claude Code, Codex o un altro esecutore configurato.
5. Verifica con controlli deterministici e registra esito, costo e decisioni.

Un modello non dichiara mai finito il proprio lavoro: lo fa un controllo.

## Dove il loop si ferma

- **Stati di approvazione**: il verbale del comitato, la roadmap, una specifica approvata, un
  merge.
- **Limiti**: un numero massimo di giri per issue e un budget di token per fase e per sprint.
- **Fallimento ripetuto**: se lo stesso controllo fallisce un numero configurato di volte, il
  loop apre una domanda invece di riprovare.

Vedi [Gate](gates.md).
