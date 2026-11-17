---
hide:
- navigation
- toc
source: index.md
source_hash: af94d270a185
---
<div class="wl-hero" markdown>

![Wayloop](assets/brand/mark.svg){ width="96" }

# Wayloop

<p class="wl-tagline">Spec-driven agentic loop.</p>

<p class="wl-promise">Dall'idea al codice unito, dentro le tue issue, con le persone ai gate.</p>

Wayloop porta avanti un progetto software con gli agenti di programmazione, dalla prima idea
fino alla pull request unita. Le specifiche fissano l'intento, un loop deterministico fa
avanzare il lavoro, il piano vive nel tuo issue tracker e le persone decidono ai gate.

[Come funziona il loop](concepts/the-loop.md){ .md-button .md-button--primary }
[Architettura](concepts/architecture.md){ .md-button }

</div>

!!! note "Stato"
    Pre-alpha. Non c'è ancora una release. Le pagine dei concetti descrivono il progetto e
    dicono, ciascuna, che cosa ne esiste oggi. Ciò che viene dopo è nella
    [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

<div class="grid cards wl-features" markdown>

-   __Il motore decide__

    ---

    Una macchina a stati scritta in Rust sceglie ogni passo e lo verifica con controlli
    deterministici. Un modello non dichiara mai finito il proprio lavoro.

    [Architettura](concepts/architecture.md)

-   __Il tracker è la base dati__

    ---

    Piano, avanzamento, decisioni e domande vivono nelle tue issue; le specifiche vivono nel
    repository. Togli Wayloop e il progetto resta leggibile.

    [Lo stato nel tracker](concepts/state-in-the-tracker.md)

-   __Le persone ai gate__

    ---

    Il loop gira senza sorveglianza tra uno stato di approvazione e l'altro e si ferma a
    ciascuno, con una domanda in formato fisso sull'issue.

    [Gate](concepts/gates.md)

-   __Un ledger deterministico__

    ---

    Eventi in sola aggiunta, concatenati da hash. Lo stato è una funzione del ledger, e ogni
    sessione passa la mano con un riassunto validato.

    [Ledger](concepts/ledger.md)

-   __Progetti nuovi ed esistenti__

    ---

    Su un codice esistente, un rilievo mappa il codice e scrive una documentazione che cita
    file e riga per ogni affermazione. Senza embedding.

    [Rilievo dei progetti esistenti](concepts/brownfield-survey.md)

-   __I token sono un budget__

    ---

    Ogni fase ha un budget e il ledger misura il costo di ogni issue, così il costo del loop è
    noto, non stimato.

    [Il loop](concepts/the-loop.md)

</div>

## Per gli agenti

Questa documentazione è pubblicata anche in testo semplice per gli agenti, in inglese:
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt) e
[`llms-full.txt`](https://salvo8519.github.io/wayloop/llms-full.txt).
