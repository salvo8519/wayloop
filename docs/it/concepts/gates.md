---
source: concepts/gates.md
source_hash: de1463cf7cfd
---
# Gate

Un gate è uno stato che il loop non può superare da solo. Tra un gate e l'altro il loop gira
senza sorveglianza; a un gate si ferma, chiede e aspetta una persona.

!!! note "Stato"
    Questa pagina descrive il progetto dei gate. Nel core, `merge` è il passo di approvazione
    del ciclo di un'issue. In questo repository i gate si applicano a mano: gli agenti fanno
    domande con l'etichetta `needs-decision`, e il maintainer approva la roadmap, le specifiche
    e i merge delle modifiche al motore. Ciò che viene dopo è nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Stati di approvazione

| Gate | Decide |
| --- | --- |
| Verbale del comitato | Maintainer |
| Roadmap e ogni proposta di modifica | Maintainer |
| Specifica `Approved` | Maintainer o responsabile dell'area |
| Merge | Maintainer, oppure in automatico per il livello di rischio più basso se il maintainer lo consente |

## Domande

Quando un agente ha bisogno di una decisione, l'issue riceve l'etichetta `needs-decision` e una
domanda in formato fisso:

1. **Contesto**, in tre righe al massimo.
2. **Opzioni**, ciascuna con il suo costo.
3. **Raccomandazione** dell'agente, con il motivo.

La risposta è un commento o la scelta di un'opzione. Il loop riparte da lì. Le domande non si
perdono in una sessione di chat chiusa: restano sull'issue, accanto al lavoro che riguardano.

## Quando un agente non si ferma

Tra un gate e l'altro un agente decide e registra una decisione nel ledger: cosa ha deciso,
perché, e quanto costa se la decisione è sbagliata. Si ferma solo per un'operazione
irreversibile o distruttiva, un'azione delicata per la sicurezza, un effetto fuori dal proprio
worktree, un piano così rotto che ogni strada è un'ipotesi, o una scelta di metodo che spetta
al maintainer.
