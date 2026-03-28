.PHONY: setup dev test lint format check clean

setup:
	pip install -e ".[dev]"

dev:
	python -m agentos.main

test:
	pytest tests/ -v --tb=short

test-cov:
	pytest tests/ -v --tb=short --cov=agentos --cov-report=term-missing

lint:
	ruff check agentos/ tests/

format:
	ruff format agentos/ tests/
	ruff check --fix agentos/ tests/

check: lint
	mypy agentos/

clean:
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name .pytest_cache -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	rm -rf .mypy_cache .ruff_cache htmlcov .coverage
