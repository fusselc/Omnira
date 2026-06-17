# Environment Setup

## Prerequisites

- Python 3.11+
- `pip`

## Setup

```bash
cd /home/runner/work/Omnira/Omnira
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Run unit tests

```bash
cd /home/runner/work/Omnira/Omnira
PYTHONPATH=src python -m unittest discover -s tests/unit -v
```

## Run API locally (future)

```bash
cd /home/runner/work/Omnira/Omnira
python -c "from omnira.api import create_app; app = create_app(); print(app.title)"
```
