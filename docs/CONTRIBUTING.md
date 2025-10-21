# Testing Guide

This document describes how to run tests for the Zenoh Recorder project.

## Quick Start

### Unit Tests Only (No Docker Required)
```bash
cargo test
```

### All Tests with Docker Infrastructure
```bash
./scripts/run_tests_with_docker.sh
```

### With Coverage Report
```bash
./scripts/run_tests_with_docker.sh --coverage
```

## Test Environment

### Docker Infrastructure

The project uses Docker Compose to provide test infrastructure:

- **Zenoh Router**: Port 27447 (avoids conflict with default 7447)
- **ReductStore**: Port 28383 (avoids conflict with default 8383)

### Manual Docker Management

Start test infrastructure:
```bash
docker-compose -f docker-compose.test.yml up -d
```

Check service health:
```bash
docker-compose -f docker-compose.test.yml ps
```

View logs:
```bash
docker-compose -f docker-compose.test.yml logs -f
```

Stop and cleanup:
```bash
docker-compose -f docker-compose.test.yml down -v
```

## Test Categories

### 1. Unit Tests
- **Location**: `tests/*_tests.rs`, `src/*/tests`
- **Requirements**: None (no external dependencies)
- **Run**: `cargo test --lib --tests`

### 2. Integration Tests with ReductStore
- **Location**: `tests/storage_integration_tests.rs`
- **Requirements**: ReductStore running on port 28383
- **Run**: `cargo test --test storage_integration_tests`

### 3. E2E Tests with Zenoh + ReductStore
- **Location**: `tests/e2e_docker_tests.rs`
- **Requirements**: Both Zenoh and ReductStore running
- **Run**: `cargo test --test e2e_docker_tests`

## Environment Variables

Test configuration can be customized via `test.env`:

```bash
# Zenoh configuration
ZENOH_TEST_PORT=27447
ZENOH_TEST_ENDPOINT=tcp/127.0.0.1:27447

# ReductStore configuration
REDUCTSTORE_TEST_PORT=28383
REDUCTSTORE_TEST_URL=http://127.0.0.1:28383
REDUCTSTORE_TEST_BUCKET=zenoh-recorder-test

# Test configuration
TEST_DEVICE_ID=test-device-001
TEST_TIMEOUT_SECONDS=30
```

## Coverage Analysis

### Generate Coverage Report
```bash
cargo llvm-cov --all-features --workspace --html
```

### View Coverage Report
```bash
open target/llvm-cov/html/index.html
```

### Coverage with Docker
```bash
./scripts/run_tests_with_docker.sh --coverage
```

## Troubleshooting

### Port Conflicts
If ports 27447 or 28383 are already in use:
1. Edit `docker-compose.test.yml` to use different ports
2. Update `test.env` to match
3. Restart Docker services

### Docker Not Running
```bash
# macOS
open -a Docker

# Check status
docker info
```

### Test Failures
1. Check Docker logs: `docker-compose -f docker-compose.test.yml logs`
2. Verify services are healthy: `docker-compose -f docker-compose.test.yml ps`
3. Check connectivity: `curl http://127.0.0.1:28383/api/v1/info`

### Clean Start
```bash
docker-compose -f docker-compose.test.yml down -v
docker system prune -f
./scripts/run_tests_with_docker.sh
```

## CI/CD Integration

### GitHub Actions
```yaml
- name: Run tests with Docker
  run: ./scripts/run_tests_with_docker.sh --ci --coverage
```

### GitLab CI
```yaml
test:
  script:
    - ./scripts/run_tests_with_docker.sh --ci --coverage
```

## Test Structure

```
tests/
├── control_unit_tests.rs           # Control module unit tests (no Docker)
├── storage_integration_tests.rs    # Storage with ReductStore (Docker)
├── e2e_docker_tests.rs             # Full E2E tests (Docker)
├── buffer_tests.rs                 # Buffer unit tests
├── mcap_serialization_tests.rs     # MCAP unit tests
├── protocol_tests.rs               # Protocol unit tests
└── ... (other test files)
```

## Performance

- Unit tests: < 5 seconds
- Integration tests: < 30 seconds
- E2E tests: < 60 seconds
- Total (with Docker): < 2 minutes

## Requirements

- Rust 1.75+
- Docker 20.10+
- Docker Compose 2.0+
- cargo-llvm-cov (for coverage)

Install cargo-llvm-cov:
```bash
cargo install cargo-llvm-cov
```

