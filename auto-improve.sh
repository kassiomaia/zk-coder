#!/bin/bash

# ==============================================================================
# Continuous Codebase Improvement Automation Script (auto-improve.sh)
# ==============================================================================
# This script runs in an infinite loop, executing a predefined sequence of 
# Claude Code tasks to continuously improve the codebase.
#
# Tasks:
# 1. Review codebase
# 2. Find and fix bugs
# 3. Improve Rust code (types, traits, functions, modules, size)
# 4. Restructure architecture
# 5. Review documentation
# 6. Review tests
# 7. Implement more tests
# 8. Improve coverage
# ==============================================================================

set -e

# Configuration
OUTPUT_DIR=".claude/auto-improve"
CYCLE_DELAY=60 # Seconds between cycles
MAX_RETRIES=3
RETRY_DELAY=10

# Task definitions (Label|Prompt)
TASKS=(
  "task-01-review-codebase|Review the entire codebase and provide a comprehensive analysis of current state, potential issues, and improvement opportunities."
  "task-02-fix-bugs|Find and fix all bugs in the codebase. Analyze the codebase for logical errors, edge cases, and potential runtime issues, then implement fixes."
  "task-03-improve-rust-code|Analyze the Rust codebase in src-tauri/ and improve it by: (1) identifying and improving type definitions, (2) creating and enhancing traits for better abstraction, (3) extracting functions to improve modularity and reduce duplication, (4) organizing modules for better structure, (5) reducing overall code size through refactoring. Verify changes with 'cargo clippy' and 'cargo test' in src-tauri/."
  "task-04-restructure-architecture|Review and restructure the architecture for better maintainability. Focus on separation of concerns, reduction of coupling, and improved scalability."
  "task-05-review-docs|Review and improve all documentation, including READMEs, inline comments, and architectural docs. Ensure they are accurate, up-to-date, and helpful."
  "task-06-review-tests|Review existing tests and coverage. Identify critical areas that lack testing and analyze the quality of existing tests."
  "task-07-implement-tests|Implement additional tests for uncovered areas. Focus on edge cases, complex logic, and critical user paths."
  "task-08-improve-coverage|Improve test coverage across the codebase by adding missing tests and refining existing ones to cover more scenarios."
)

# Initialize output directory
mkdir -p "$OUTPUT_DIR"
LOG_FILE="$OUTPUT_DIR/auto-improve-$(date +%Y%m%d-%H%M%S).log"
CYCLE_FILE="$OUTPUT_DIR/cycle-count.txt"

# Initialize cycle count if not exists
if [[ ! -f "$CYCLE_FILE" ]]; then
  echo "0" > "$CYCLE_FILE"
fi

# Logging function
log() {
  local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
  echo "[$timestamp] $1" | tee -a "$LOG_FILE"
}

# Cleanup function for graceful shutdown
cleanup() {
  log "Received termination signal. Cleaning up..."
  exit 0
}

trap cleanup SIGINT SIGTERM

# Function to run a single task
run_task() {
  local task_def="$1"
  local label="${task_def%%|*}"
  local prompt="${task_def#*|}"
  local cycle_dir="$2"
  local task_log="$cycle_dir/$label.log"
  local attempt=1

  log "Starting task: $label"
  
  while [[ $attempt -le $MAX_RETRIES ]]; do
    log "Executing (attempt $attempt/$MAX_RETRIES): $label"
    
    # Run Claude Code in non-interactive mode
    # We use -p for non-interactive output
    set +e
    claude "$prompt" --dangerously-skip-permissions
    local exit_code=${PIPESTATUS[0]}
    set -e

    if [[ $exit_code -eq 0 ]]; then
      log "✓ Task completed successfully: $label"
      return 0
    fi

    log "✗ Task failed: $label (exit code: $exit_code)"
    if [[ $attempt -lt $MAX_RETRIES ]]; then
      log "Retrying in ${RETRY_DELAY}s..."
      sleep "$RETRY_DELAY"
    fi
    ((attempt++))
  done

  log "!! All retries exhausted for task: $label"
  return 1
}

# Main loop
log "================================================================"
log "Starting Continuous Codebase Improvement Loop"
log "Output directory: $OUTPUT_DIR"
log "Main log file: $LOG_FILE"
log "================================================================"

while true; do
  # Increment cycle count
  CURRENT_CYCLE=$(cat "$CYCLE_FILE")
  CURRENT_CYCLE=$((CURRENT_CYCLE + 1))
  echo "$CURRENT_CYCLE" > "$CYCLE_FILE"
  
  CYCLE_DIR="$OUTPUT_DIR/cycle-$(printf "%04d" "$CURRENT_CYCLE")"
  mkdir -p "$CYCLE_DIR"
  
  log ">>> STARTING CYCLE #$CURRENT_CYCLE <<<"
  
  for task_def in "${TASKS[@]}"; do
    run_task "$task_def" "$CYCLE_DIR"
    # Optional small delay between tasks to avoid overwhelming the system
    sleep 2
  done
  
  log ">>> COMPLETED CYCLE #$CURRENT_CYCLE <<<"
  log "Waiting ${CYCLE_DELAY}s before next cycle..."
  sleep "$CYCLE_DELAY"
done
