---
source: concepts/state-in-the-tracker.md
source_hash: d8b1079f1330
---
# Lo stato nel tracker

Wayloop non ha uno stato proprio. Il piano, l'avanzamento, le decisioni e le domande vivono
nell'issue tracker; le specifiche vivono nel repository; in locale restano solo i checkpoint
delle sessioni.

| Cosa | Dove vive | Perché |
| --- | --- | --- |
| Roadmap, epic, story, task | Issue, sotto-issue, dipendenze, tipi di issue, milestone | Visibile al team e a qualunque strumento; sopravvive alla rimozione di Wayloop |
| Avanzamento, domande, decisioni, esiti dei gate | Campi e commenti delle issue | I gate diventano asincroni: si può decidere anche dal telefono |
| Specifiche vive e modifiche a delta | Il repository, collegato dall'issue | Cambiano insieme al codice e si revisionano nelle pull request |
| Checkpoint di sessione | La macchina locale, mai committati | Servono solo a riprendere un lavoro interrotto |
| Cache del tracker | La macchina locale, ricostruibile | Velocità e determinismo; mai una fonte di verità |

## Dove il progetto è sotto sforzo, e come regge

- **Velocità.** Le API remote sono lente e hanno limiti di frequenza. Il motore lavora su una
  cache locale aggiornata in modo incrementale, che si può cancellare e ricostruire in
  qualunque momento.
- **Testo libero.** Anche le persone modificano il corpo delle issue. Il motore usa i campi
  nativi (sotto-issue, dipendenze "blocked by", tipi di issue) e un blocco strutturato validato
  da uno schema; ciò che non valida diventa una domanda, mai un errore silenzioso.
- **Storia del piano.** Ogni proposta di modifica è un commento con la differenza e
  un'etichetta di gate; l'approvazione resta nella cronologia dell'issue.
- **Concorrenza.** Un'issue viene presa in carico con un assegnatario e un'etichetta, e
  scritta solo se non è cambiata dall'ultima lettura.
- **GitLab.** Sul piano gratuito le relazioni "blocks" e le epic non sono disponibili.
  L'adattatore GitLab ripiega sul blocco strutturato. GitHub è il primo backend.
- **Eventi a catena.** Un merge riavvia la riconciliazione tramite un workflow. Gli eventi
  prodotti dal bot creano solo proposte, sono idempotenti e vengono raggruppati, così il loop
  non si alimenta da solo.

## La prova della rimozione

L'impostazione è giusta solo se un progetto resta leggibile dopo la rimozione di Wayloop: issue,
specifiche e commenti di passaggio devono raccontare tutta la storia da soli.
