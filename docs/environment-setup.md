# Environment Setup

## Prerequisites

- Python 3.11+
- `pip`

## Setup

```bash
cd <repository-root>
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Run unit tests

```bash
cd <repository-root>
PYTHONPATH=src python -m unittest discover -s tests/unit -v
```

## Run API locally (future)

```bash
cd <repository-root>
python -c "from omnira.api import create_app; app = create_app(); print(app.title)"
```
