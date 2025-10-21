# CI/CD Documentation

This document describes the Continuous Integration and Continuous Deployment setup for the Zenoh Recorder project.

## Overview

The project uses GitHub Actions for automated testing, building, and releasing. The CI/CD pipeline consists of multiple workflows optimized for different purposes.

---

## Workflows

### 1. Main CI Pipeline (`.github/workflows/ci.yml`)

**Trigger**: Push to `main`/`develop` branches, Pull Requests

**Jobs**:

#### Unit Tests
- **Runtime**: ~2-3 minutes
- **Purpose**: Fast feedback on core logic
- **What it runs**:
  - All unit tests (no Docker required)
  - Documentation tests
  - Control module tests
  - MCAP serialization tests
  - Protocol tests

#### Lint and Format
- **Runtime**: ~1 minute
- **Purpose**: Code quality enforcement
- **Checks**:
  - `cargo fmt` - Rust formatting
  - `cargo clippy` - Linting with warnings as errors

#### Build
- **Runtime**: ~3-5 minutes
- **Purpose**: Verify release build succeeds
- **Artifacts**: Compiled `zenoh-recorder` binary

#### Integration Tests
- **Runtime**: ~5-8 minutes
- **Purpose**: Test with real infrastructure
- **Infrastructure**:
  - Docker Compose (Zenoh + ReductStore)
  - Ports: 27447 (Zenoh), 28383 (ReductStore)
- **What it runs**:
  - Storage integration tests
  - E2E tests with full stack
- **Environment Variables**:
  ```yaml
  REDUCTSTORE_TEST_URL: http://127.0.0.1:28383
  ZENOH_TEST_ENDPOINT: tcp/127.0.0.1:27447
  REDUCTSTORE_TEST_BUCKET: zenoh-recorder-test
  ```

#### Coverage
- **Runtime**: ~6-10 minutes
- **Purpose**: Generate and upload code coverage reports
- **Tools**: `cargo-llvm-cov`
- **Outputs**:
  - LCOV report (uploaded to Codecov)
  - HTML report (downloadable artifact)

#### Security Audit
- **Runtime**: ~1 minute
- **Purpose**: Check for known vulnerabilities
- **Tool**: `cargo-audit`

---

### 2. Quick Check (`.github/workflows/quick-check.yml`)

**Trigger**: Pull Requests only

**Purpose**: Provide fast feedback (~2 minutes) for PRs

**What it runs**:
- Format check
- Clippy
- Build check
- Fast unit tests only (no Docker)

**Use case**: Developers get immediate feedback without waiting for full CI

---

### 3. Nightly Tests (`.github/workflows/nightly.yml`)

**Trigger**: 
- Scheduled: 2 AM UTC daily
- Manual: `workflow_dispatch`

**Purpose**: Comprehensive testing with extended duration

**Jobs**:

#### Comprehensive Tests
- Full test suite with coverage
- All tests (unit + integration + E2E)
- Coverage report generation
- Benchmarks (if available)

#### Long Running Tests
- Stress testing (runs tests 5x to catch race conditions)
- Extended duration validation
- Memory leak detection with Valgrind

**Timeout**: 2 hours

---

### 4. Release (`.github/workflows/release.yml`)

**Trigger**: Git tags matching `v*.*.*` (e.g., `v0.1.0`)

**Jobs**:

#### Create Release
- Creates GitHub release from tag

#### Build Binaries
- **Platforms**:
  - Linux AMD64
  - Linux ARM64
  - macOS AMD64  
  - macOS ARM64 (Apple Silicon)
- **Outputs**: Stripped, optimized binaries in `.tar.gz`

#### Publish Docker
- Builds multi-platform Docker image
- Tags: `latest` and version-specific
- Platforms: `linux/amd64`, `linux/arm64`
- Registry: Docker Hub (requires secrets)

---

## Required Secrets

Configure these in GitHub repository settings:

| Secret | Purpose | Example |
|--------|---------|---------|
| `DOCKER_USERNAME` | Docker Hub username | `mycompany` |
| `DOCKER_PASSWORD` | Docker Hub token/password | `dckr_pat_xxx` |
| `CODECOV_TOKEN` | Codecov upload token (optional) | Auto-detected for public repos |

---

## Local Testing

### Test What CI Tests

```bash
# Unit tests (like CI quick check)
cd zenoh-recorder-example
cargo fmt --check
cargo clippy --all-features -- -D warnings
cargo test --lib

# Integration tests (like CI integration job)
docker-compose -f docker-compose.test.yml up -d
cargo test --test storage_integration_tests -- --test-threads=1
cargo test --test e2e_docker_tests -- --test-threads=1
docker-compose -f docker-compose.test.yml down -v

# Full suite (like CI main)
./scripts/run_tests_with_docker.sh

# With coverage (like CI coverage job)
./scripts/run_tests_with_docker.sh --coverage
```

---

## CI Performance

### Expected Runtimes

| Workflow | Trigger | Duration | Cost* |
|----------|---------|----------|-------|
| Quick Check | PR | 2-3 min | Low |
| Main CI | Push/PR | 15-20 min | Medium |
| Nightly | Schedule | 60-90 min | High |
| Release | Tag | 30-45 min | Medium |

*GitHub Actions provides 2,000 free minutes/month for public repos

---

## Optimization Strategies

### Caching

The workflows use multiple caching strategies:

1. **Cargo Registry Cache**
   ```yaml
   path: ~/.cargo/registry
   key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
   ```

2. **Cargo Index Cache**
   ```yaml
   path: ~/.cargo/git
   key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
   ```

3. **Build Cache**
   ```yaml
   path: target
   key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
   ```

### Parallelization

- Unit tests and integration tests run in parallel jobs
- Different test suites run concurrently
- Docker layer caching for faster builds

---

## Monitoring CI Health

### Success Metrics

Track these in GitHub Actions dashboard:

- **Test Pass Rate**: Should be 100%
- **Average Duration**: Should stay under 20 minutes for main CI
- **Coverage Trend**: Monitor in Codecov dashboard

### Alerts

The nightly workflow includes failure notifications:
```yaml
- name: Notify on failure
  if: failure()
  run: echo "::warning::Nightly tests failed"
```

---

## Troubleshooting

### Common Issues

#### 1. Docker Service Timeout

**Symptom**: Integration tests fail waiting for services

**Solution**:
```yaml
# Increase wait time in workflow
for i in {1..60}; do  # Was 30
  # ... wait logic
done
```

#### 2. Flaky E2E Tests

**Symptom**: E2E tests fail intermittently

**Solutions**:
- Run tests sequentially: `--test-threads=1`
- Increase sleep delays between operations
- Add retry logic to network operations

#### 3. Coverage Upload Fails

**Symptom**: Codecov upload step fails

**Solution**:
- Check `CODECOV_TOKEN` secret is set
- Use `fail_ci_if_error: false` for non-blocking

#### 4. Out of Disk Space

**Symptom**: Build fails with disk space error

**Solution**:
```yaml
- name: Free disk space
  run: |
    sudo rm -rf /usr/share/dotnet
    sudo rm -rf /opt/ghc
    sudo rm -rf "/usr/local/share/boost"
```

---

## Best Practices

### 1. Keep Workflows Fast

- Run quick checks first (format, clippy)
- Run unit tests before integration tests
- Cache aggressively
- Run expensive tests (nightly) on schedule only

### 2. Use Job Dependencies

```yaml
jobs:
  unit-tests:
    # ...
  
  integration-tests:
    needs: unit-tests  # Only run if unit tests pass
    # ...
```

### 3. Fail Fast

```yaml
strategy:
  fail-fast: true  # Stop all jobs if one fails
```

### 4. Artifact Management

- Keep artifacts for 30 days (default)
- Only upload essential artifacts
- Compress before uploading

---

## Adding New Tests to CI

### Unit Tests

Add to existing unit test job - they run automatically:

```yaml
- name: Run unit tests
  run: cargo test --lib --tests
```

### Integration Tests

If test requires Docker:

```yaml
- name: Run new integration test
  run: cargo test --test my_new_integration_test -- --test-threads=1
  env:
    REDUCTSTORE_TEST_URL: http://127.0.0.1:28383
```

### Benchmarks

Add to nightly workflow:

```yaml
- name: Run benchmarks
  run: cargo bench --bench my_benchmark
```

---

## Release Process

### Creating a Release

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit changes
4. Create and push tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
5. GitHub Actions automatically:
   - Builds binaries for all platforms
   - Creates GitHub release
   - Publishes Docker image

### Manual Release (if needed)

```bash
cd zenoh-recorder-example
cargo build --release
strip target/release/zenoh-recorder
tar czf zenoh-recorder-v0.2.0.tar.gz -C target/release zenoh-recorder
```

---

## Docker Image Usage

After release, the image is available:

```bash
docker pull yourusername/zenoh-recorder:latest
docker pull yourusername/zenoh-recorder:0.2.0

# Run
docker run -e DEVICE_ID=robot-01 \
  -e REDUCTSTORE_URL=http://reductstore:8383 \
  -e BUCKET_NAME=sensor_data \
  yourusername/zenoh-recorder:latest
```

---

## Future Improvements

### Potential Additions

1. **Performance Regression Testing**
   - Run benchmarks on every PR
   - Compare against baseline
   - Alert on >10% regression

2. **Cross-compilation**
   - Add Windows targets
   - Add embedded targets (ARM Cortex-M)

3. **E2E Environment**
   - Deploy to test cluster
   - Run real-world scenarios
   - Automated acceptance tests

4. **Documentation Generation**
   - Auto-generate API docs
   - Publish to GitHub Pages
   - Update on every release

5. **Dependency Updates**
   - Dependabot integration
   - Automated update PRs
   - Security patch automation

---

## Maintenance

### Regular Tasks

- **Weekly**: Review failed nightly builds
- **Monthly**: Update dependencies
- **Quarterly**: Review and optimize CI performance
- **Yearly**: Audit security configurations

### Monitoring

Check GitHub Actions dashboard for:
- Build success rate over time
- Average build duration trends
- Resource usage patterns
- Flaky test identification

---

## Contact

For CI/CD issues or questions:
- Open an issue with `ci` label
- Check existing CI documentation
- Review GitHub Actions logs

---

**Last Updated**: October 18, 2025  
**CI Version**: 1.0  
**Status**: ✅ **PRODUCTION-READY**

