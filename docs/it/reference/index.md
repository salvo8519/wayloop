---
source: reference/index.md
source_hash: 8c0a87afc689
---
# Riferimento

Questa sezione raccoglie il riferimento dei comandi, dei contratti JSON e dei codici di errore
di Wayloop. Non c'è ancora una release; ciò che viene dopo è nella [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Comandi di una build dai sorgenti

Il binario `wayloop` costruito da `crates/wayloop-cli` risponde a due comandi. Entrambi
stampano una riga di JSON sullo standard output ed escono con codice 0. La versione è quella
del manifest del workspace.

| Comando | Uscita |
| --- | --- |
| `wayloop version` (anche `wayloop --version`) | Il nome e la versione del motore |
| `wayloop steps` | Gli identificativi dei passi del ciclo di un'issue, in ordine |

```console
$ cargo run -q -p wayloop-cli -- version
{"name":"wayloop","version":"0.0.0"}
$ cargo run -q -p wayloop-cli -- steps
{"issue_steps":["claim","reanalysis","counter-review","checklist","build","verify","review","pull-request","merge"]}
```

Qualunque altro comando, o nessun comando, stampa una riga d'uso sullo standard error ed esce
con codice 2:

```console
$ cargo run -q -p wayloop-cli
wayloop 0.0.0: pre-alpha. Commands: version, steps.
```

Gli identificativi dei passi sono stabili: un passo non viene mai rinominato. I passi sono
descritti in [Il loop](../concepts/the-loop.md#il-ciclo-di-unissue).
