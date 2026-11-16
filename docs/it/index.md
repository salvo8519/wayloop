---
source: index.md
source_hash: 1fb3e0da0124
---
# Wayloop

**Spec-driven agentic loop.** Dall'idea al codice unito, dentro le tue issue, con le persone ai
gate.

Wayloop è un motore open source che porta avanti un progetto software con gli agenti di
programmazione, dalla prima idea fino alla pull request unita. Le specifiche fissano l'intento,
un loop deterministico fa avanzare il lavoro, il piano vive nel tuo issue tracker e le persone
decidono ai gate.

!!! note "Stato"
    Pre-alpha, fase di progettazione. Non c'è ancora nulla da installare. Questo sito descrive
    l'architettura e segue il progetto mentre viene costruito.

## Il loop in una pagina

1. **Scomposizione**: un'idea o un codice esistente diventa milestone, epic, user story,
   specifiche, task e issue.
2. **Esecuzione**: ogni issue segue lo stesso ciclo, dalla presa in carico alla pull request.
3. **Riconciliazione**: ogni evento produce una proposta di modifica della roadmap.
4. **Ripianificazione**: dopo l'approvazione si ricalcolano dipendenze e prossimi task e si
   aggiorna il tracker.

Leggi [Il loop](concepts/the-loop.md) per il quadro completo.

## Principi

| Principio | Dove approfondire |
| --- | --- |
| Il motore decide, gli agenti scrivono | [Architettura](concepts/architecture.md) |
| Il tracker è la base dati | [Lo stato nel tracker](concepts/state-in-the-tracker.md) |
| Le persone ai gate | [Gate](concepts/gates.md) |
| Un ledger deterministico | [Ledger](concepts/ledger.md) |
| Progetti nuovi ed esistenti | [Rilievo dei progetti esistenti](concepts/brownfield-survey.md) |

## Per gli agenti

Questa documentazione è pubblicata anche in testo semplice per gli agenti, in inglese:
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt) e
[`llms-full.txt`](https://salvo8519.github.io/wayloop/llms-full.txt).
