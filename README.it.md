<!-- i18n source: README.md source_hash: e5b5be23add7 -->
<div align="center">

<a href="https://salvo8519.github.io/wayloop/it/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/_shared/assets/brand/banner-dark.svg">
    <img alt="Wayloop. Spec-driven agentic loop." src="docs/_shared/assets/brand/banner-light.svg" width="100%">
  </picture>
</a>

[English](README.md) · **Italiano**

<h3>Lo spec-driven agentic loop per interi progetti software.</h3>

I tuoi agenti scrivono il codice. Wayloop guida il progetto intorno —<br/>
dall'idea al codice unito, dentro le tue issue, con le persone ai gate.

[![CI](https://github.com/salvo8519/wayloop/actions/workflows/ci.yml/badge.svg)](https://github.com/salvo8519/wayloop/actions/workflows/ci.yml)
[![Docs](https://github.com/salvo8519/wayloop/actions/workflows/docs.yml/badge.svg)](https://salvo8519.github.io/wayloop/it/)
[![Status](https://img.shields.io/badge/status-pre--alpha-0B1F3A)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![License](https://img.shields.io/badge/license-Apache--2.0-F59E0B)](LICENSE)

[![Rust core](https://img.shields.io/badge/core-Rust-12325C?logo=rust)](crates/wayloop-core)
[![Python SDK](https://img.shields.io/badge/SDK-Python%20%C2%B7%20planned-12325C?logo=python&logoColor=white)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![TypeScript SDK](https://img.shields.io/badge/SDK-TypeScript%20%C2%B7%20planned-12325C?logo=typescript&logoColor=white)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![MCP](https://img.shields.io/badge/MCP-planned-12325C)](https://salvo8519.github.io/wayloop/concepts/architecture/)
[![uv](https://img.shields.io/badge/built%20with-uv-12325C)](https://docs.astral.sh/uv/)

[![Spec-driven](https://img.shields.io/badge/development-spec--driven-0F766E)](https://salvo8519.github.io/wayloop/concepts/the-loop/)
[![Agentic loop](https://img.shields.io/badge/agentic%20loop-human--gated-0F766E)](https://salvo8519.github.io/wayloop/concepts/gates/)
[![Docs EN IT](https://img.shields.io/badge/docs-English%20%C2%B7%20Italiano-0F766E)](https://salvo8519.github.io/wayloop/it/)

**[Documentazione](https://salvo8519.github.io/wayloop/it/)** ·
**[Come funziona il loop](https://salvo8519.github.io/wayloop/it/concepts/the-loop/)** ·
**[Architettura](https://salvo8519.github.io/wayloop/it/concepts/architecture/)** ·
**[Roadmap](https://salvo8519.github.io/wayloop/project/roadmap/)** ·
**[Contribuire](CONTRIBUTING.md)**

</div>

---

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
