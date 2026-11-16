<!-- i18n source: README.md source_hash: 73c89cf60880 -->
# Wayloop

**Spec-driven agentic loop.**

Dall'idea al codice unito, dentro le tue issue, con le persone ai gate.

[English](README.md) | Italiano · [Documentazione](https://salvo8519.github.io/wayloop/it/) · [Roadmap](https://salvo8519.github.io/wayloop/project/roadmap/)

> Stato: pre-alpha, fase di progettazione. Non c'è ancora nulla da installare. L'architettura
> è descritta nella documentazione e la prima milestone viene pianificata in pubblico.

## Perché Wayloop

Gli agenti di programmazione scrivono codice in fretta. I progetti però falliscono ancora nel
lavoro intorno al codice: intenti mai scritti, piani che si allontanano da ciò che viene
davvero costruito, sessioni che il prossimo agente non riesce a riprendere, costi che nessuno
misura.

Due famiglie di strumenti affrontano ciascuna metà del problema.

- **Gli strumenti spec-driven** fissano l'intento prima del codice, ma si fermano al file del
  piano.
- **I loop agentici** tengono un agente al lavoro finché l'obiettivo è raggiunto, ma non
  conoscono il piano del progetto su cui lavorano.

Wayloop unisce le due cose alla scala di un intero progetto: un motore deterministico guida il
loop, il piano vive nel tuo issue tracker e le persone decidono ai gate.

## Come funziona il loop

1. **Scomposizione.** Un'idea, o un codice già esistente, diventa milestone, epic, user story,
   specifiche, task e issue. Gli agenti scrivono ogni livello; il motore lo valida prima che
   parta il livello successivo.
2. **Esecuzione.** Ogni issue segue lo stesso ciclo: presa in carico, rianalisi,
   controdeduzione critica, checklist del task, sviluppo, verifica deterministica, review
   proporzionata al rischio, pull request.
3. **Riconciliazione.** Ogni evento (una pull request unita, un'issue chiusa, una nuova
   richiesta, lavoro scoperto durante lo sviluppo) produce una proposta di modifica della
   roadmap. Niente cambia il piano in silenzio.
4. **Ripianificazione.** Approvata la modifica, il motore ricalcola dipendenze, prossimi task
   e avanzamento di ogni milestone, e aggiorna il tracker.

## Cosa lo rende diverso

| Principio | In pratica |
| --- | --- |
| Il motore decide, gli agenti scrivono | Una macchina a stati scritta in Rust sceglie ogni passo e lo verifica con controlli deterministici. Un modello non dichiara mai finito il proprio lavoro. |
| Il tracker è la base dati | Piano, avanzamento, decisioni e domande vivono nelle issue di GitHub. Le specifiche vivono nel repository. In locale restano solo i checkpoint delle sessioni. Togli Wayloop e il progetto resta leggibile. |
| Le persone ai gate | Stati di approvazione che il loop non può superare da solo: il verbale del comitato, la roadmap, una specifica approvata, un merge. Gli agenti fanno domande sull'issue, in un formato fisso. |
| Un ledger deterministico | Eventi in sola aggiunta, concatenati da hash. Lo stato è una funzione del ledger. I riassunti di sessione sono JSON validato, non prosa libera. |
| I token sono un budget | Ogni fase ne ha uno, e il ledger misura il costo di ogni issue e di ogni fase. |
| Progetti nuovi ed esistenti | Per un codice esistente, una fase di rilievo mappa il codice in modo deterministico e scrive una documentazione che cita file e riga per ogni affermazione. Senza embedding. |

## Architettura prevista

- **Motore e CLI** in Rust: un solo motore, un solo binario, JSON in ingresso e in uscita.
- **SDK Python** (PyO3) e **SDK TypeScript** (napi-rs, disponibile tramite `npx`).
- **Server MCP**, perché qualunque agente compatibile possa usare il motore.
- **Plugin e skill** per Claude Code e Codex, nel formato aperto Agent Skills.
- **Integrazione con GitHub**: workflow riusabili per la validazione e l'esecuzione remota
  delle issue.

Gli adattatori traducono; solo il motore decide. Le ragioni di ogni scelta sono registrate
nelle [decisioni di architettura](https://salvo8519.github.io/wayloop/project/adr/).

## Roadmap

| Milestone | Contenuto |
| --- | --- |
| M0 Fondamenta | Repository, sito della documentazione, schemi degli artefatti, `init` |
| M1 Esecuzione | Macchina a stati, ledger, ciclo delle issue, sprint a corsie, passaggio di mano tra sessioni |
| M2 Specifiche vive | Rilievo dei progetti esistenti, specifiche e modifiche a delta, controlli di coerenza |
| M3 A monte | Brief, comitato di esperti, roadmap dal verbale, user story |
| M4 Distribuzione | SDK Python e TypeScript, server MCP, plugin, rilascio 0.1 |

Il piano di dettaglio vive nelle issue del repository e nella
[pagina della roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Documentazione

La documentazione è pubblicata in inglese e in italiano e si ricostruisce automaticamente a
ogni modifica su `main`: [English](https://salvo8519.github.io/wayloop/) ·
[Italiano](https://salvo8519.github.io/wayloop/it/). Gli agenti possono leggere
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt).

## Contribuire

Wayloop è costruito con il processo che descrive. Prima di aprire una pull request leggi
[CONTRIBUTING.md](CONTRIBUTING.md) e [AGENTS.md](AGENTS.md).

## Licenza

[Apache License 2.0](LICENSE).
