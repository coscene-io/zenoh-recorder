#!/bin/bash
# Helper script to run tests with Docker infrastructure
# Usage: ./scripts/run_tests_with_docker.sh [--ci] [--coverage]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="$PROJECT_DIR/docker-compose.test.yml"

# Parse arguments
CI_MODE=false
COVERAGE=false
CLEANUP=true

for arg in "$@"; do
  case $arg in
    --ci)
      CI_MODE=true
      ;;
    --coverage)
      COVERAGE=true
      ;;
    --no-cleanup)
      CLEANUP=false
      ;;
    --help)
      echo "Usage: $0 [--ci] [--coverage] [--no-cleanup]"
      echo "  --ci          Run in CI mode (non-interactive)"
      echo "  --coverage    Generate coverage report"
      echo "  --no-cleanup  Don't stop Docker containers after tests"
      exit 0
      ;;
  esac
done

# Function to print colored output
print_info() {
  echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
  echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if Docker is running
check_docker() {
  if ! docker info > /dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker and try again."
    exit 1
  fi
  print_info "Docker is running"
}

# Function to start test infrastructure
start_infrastructure() {
  print_info "Starting test infrastructure..."
  cd "$PROJECT_DIR"
  
  docker-compose -f "$COMPOSE_FILE" down -v > /dev/null 2>&1 || true
  docker-compose -f "$COMPOSE_FILE" up -d
  
  print_info "Waiting for services to be healthy..."
  
  # Wait for ReductStore
  for i in {1..30}; do
    if curl -sf http://127.0.0.1:28383/api/v1/info > /dev/null 2>&1; then
      print_info "ReductStore is ready"
      break
    fi
    if [ $i -eq 30 ]; then
      print_error "ReductStore failed to start"
      docker-compose -f "$COMPOSE_FILE" logs reductstore-test
      exit 1
    fi
    sleep 1
  done
  
  # Wait for Zenoh (simple port check)
  for i in {1..30}; do
    if nc -z 127.0.0.1 27447 > /dev/null 2>&1; then
      print_info "Zenoh router is ready"
      break
    fi
    if [ $i -eq 30 ]; then
      print_error "Zenoh router failed to start"
      docker-compose -f "$COMPOSE_FILE" logs zenoh-test
      exit 1
    fi
    sleep 1
  done
  
  print_info "All services are ready!"
}

# Function to stop test infrastructure
stop_infrastructure() {
  print_info "Stopping test infrastructure..."
  cd "$PROJECT_DIR"
  docker-compose -f "$COMPOSE_FILE" down -v
  print_info "Infrastructure stopped and cleaned up"
}

# Function to run tests
run_tests() {
  print_info "Running tests..."
  cd "$PROJECT_DIR"
  
  # Load test environment variables
  if [ -f test.env ]; then
    export $(cat test.env | grep -v '^#' | xargs)
  fi
  
  if [ "$COVERAGE" = true ]; then
    print_info "Running tests with coverage..."
    cargo llvm-cov --all-features --workspace --html
    print_info "Coverage report generated at target/llvm-cov/html/index.html"
  else
    print_info "Running all tests..."
    cargo test --all-features --workspace -- --test-threads=1
  fi
}

# Main execution
main() {
  print_info "=== Zenoh Recorder Test Runner ==="
  print_info "CI Mode: $CI_MODE"
  print_info "Coverage: $COVERAGE"
  
  # Check Docker
  check_docker
  
  # Trap to ensure cleanup on exit
  if [ "$CLEANUP" = true ]; then
    trap stop_infrastructure EXIT
  fi
  
  # Start infrastructure
  start_infrastructure
  
  # Run tests
  run_tests
  
  print_info "=== Tests completed successfully! ==="
}

# Run main
main

