---
source: concepts/brownfield-survey.md
source_hash: 8108cc4cc905
---
# Rilievo dei progetti esistenti

Su un codice esistente il loop parte con un passo in più, il rilievo. Il motore mappa il codice
in modo deterministico; gli agenti scrivono una documentazione sostenuta da prove; il risultato
diventa il contesto indicizzato di tutti i passi successivi.

!!! note "Stato"
    Questa pagina descrive il progetto del rilievo. Oggi nessuna sua parte è costruita. Ciò
    che viene dopo è nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

| Passo | Chi lo fa | Risultato |
| --- | --- | --- |
| Scansione | Motore | Albero dei moduli, linguaggi, punti di ingresso, dipendenze, conteggio dei token per file |
| Mappa | Motore | Simboli estratti con tree-sitter, grafo di import e chiamate, classifica dei file centrali, una mappa entro un budget di token |
| Documentazione con prove | Agenti, sopra la mappa | Architettura, convenzioni, glossario di dominio, flussi principali; ogni affermazione cita file e riga, e ciò che non si può dimostrare va in una sezione di elementi non verificati |
| Specifiche rilevate | Agenti | Specifiche vive ricostruite dal comportamento attuale, in stato `Surveyed` finché il maintainer non le approva |
| Lacune | Agenti | Un documento delle sole lacune (test mancanti, parti non documentate, rischi), senza ridocumentare ciò che esiste già |
| Indice | Motore | Un indice lessicale e strutturale, aggiornato in modo incrementale, interrogato tramite il motore |

## Prove, non plausibilità

Le risposte distinguono osservazioni, limiti e cose sconosciute, e "non trovato" è una
risposta valida. È questo che impedisce a un agente di inventare un'architettura.

## Niente embedding

L'indice è lessicale (BM25) e strutturale (simboli tree-sitter): veloce, deterministico e
ripetibile. Gli embedding aggiungerebbero un modello, un archivio da mantenere e risultati non
ripetibili, per aiutare solo con le domande in linguaggio naturale. Chi vuole la ricerca
semantica usa uno strumento esterno accanto a Wayloop. Vedi
[ADR 0004](https://salvo8519.github.io/wayloop/project/adr/0004-no-embeddings/).

## Risparmiare token

- Una mappa entro un budget invece dei file interi.
- Letture filtrate delle specifiche: solo i requisiti che contano per il passo.
- Uscite JSON compatte da ogni comando.
- Per l'output della shell, l'integrazione con un compressore esistente invece di riscriverne
  uno.
