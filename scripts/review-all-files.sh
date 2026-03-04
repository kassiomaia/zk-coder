#!/bin/bash

# Script to run code-reviewer on all codebase files using Claude Code with haiku model
# Usage: ./scripts/review-all-files.sh [options]
#
# Options:
#   --ext comma,separated,extensions   File extensions to review (default: ts,tsx,js,jsx,svelte,rs)
#   --exclude pattern                   Exclude files matching pattern (can be used multiple times)
#   --dry-run                           Show files that would be reviewed without running
#   --output-dir dir                    Directory to save review outputs (default: .claude/reviews)
#   --continue                          Continue from previous run (skips already reviewed files)
#   --max-retries N                     Maximum retry attempts for failed reviews (default: 3)
#   --retry-delay N                     Seconds to wait between retries (default: 5)
#   --failed-only                       Only retry files that failed in the previous run
#   -h, --help                          Show this help message

set -e

# Default configuration
DEFAULT_EXTENSIONS="ts,tsx,js,jsx,svelte,rs"
OUTPUT_DIR=".claude/reviews"
DRY_RUN=false
CONTINUE=false
FAILED_ONLY=false
MAX_RETRIES=3
RETRY_DELAY=5
EXTENSIONS="${DEFAULT_EXTENSIONS}"
EXCLUDE_PATTERNS=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --ext)
            EXTENSIONS="$2"
            shift 2
            ;;
        --exclude)
            EXCLUDE_PATTERNS+=("$2")
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --continue)
            CONTINUE=true
            shift
            ;;
        --max-retries)
            MAX_RETRIES="$2"
            shift 2
            ;;
        --retry-delay)
            RETRY_DELAY="$2"
            shift 2
            ;;
        --failed-only)
            FAILED_ONLY=true
            shift
            ;;
        -h|--help)
            sed 's/^# //' "$0" | sed -n '/^Usage:/,/^$/p'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Convert extensions to find arguments
IFS=',' read -ra EXT_ARRAY <<< "$EXTENSIONS"
FIND_ARGS=()
for ext in "${EXT_ARRAY[@]}"; do
    FIND_ARGS+=(-name "*.${ext}" -o)
done
# Remove last -o
unset 'FIND_ARGS[-1]'

# Default exclude patterns
DEFAULT_EXCLUDES=(
    "node_modules"
    "target"
    "dist"
    "build"
    ".git"
    ".claude"
    "coverage"
    ".next"
    ".svelte-kit"
)

# Build exclude arguments for find
EXCLUDE_FIND_ARGS=()
for pattern in "${DEFAULT_EXCLUDES[@]}"; do
    EXCLUDE_FIND_ARGS+=(-type d -name "$pattern" -prune -o)
done
for pattern in "${EXCLUDE_PATTERNS[@]}"; do
    EXCLUDE_FIND_ARGS+=(-type d -name "$pattern" -prune -o)
done

# Get project root (assuming script is in scripts/ directory)
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Track statistics
TOTAL_FILES=0
REVIEWED_FILES=0
SKIPPED_FILES=0
FAILED_FILES=0
declare -a FAILED_FILE_LIST=()

# Log file
LOG_FILE="$OUTPUT_DIR/review-log-$(date +%Y%m%d-%H%M%S).txt"
touch "$LOG_FILE"

# Failed files tracking
FAILED_FILES_LIST="$OUTPUT_DIR/failed-files.txt"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Find all matching files
log "Finding files with extensions: ${EXTENSIONS}"
log "Exclude patterns: ${EXCLUDE_PATTERNS[*]:-${DEFAULT_EXCLUDES[*]}}"

mapfile -t ALL_FILES < <(find . "${EXCLUDE_FIND_ARGS[@]}" -type f \( "${FIND_ARGS[@]}" \) -print | sort)

# Filter for failed files only if --failed-only is specified
if $FAILED_ONLY; then
    if [[ -f "$FAILED_FILES_LIST" ]]; then
        log "Loading failed files from: $FAILED_FILES_LIST"
        mapfile -t FAILED_PREVIOUS < <(cat "$FAILED_FILES_LIST")
        # Intersect ALL_FILES with FAILED_PREVIOUS
        FILES=()
        for file in "${ALL_FILES[@]}"; do
            file="${file#./}"
            for failed in "${FAILED_PREVIOUS[@]}"; do
                if [[ "$file" == "$failed" ]]; then
                    FILES+=("./$file")
                    break
                fi
            done
        done
        log "Found ${#FILES[@]} failed files to retry"
    else
        log "No failed files list found. Running full review."
        FILES=("${ALL_FILES[@]}")
    fi
else
    FILES=("${ALL_FILES[@]}")
fi

TOTAL_FILES=${#FILES[@]}
log "Found ${TOTAL_FILES} files to review"

if $DRY_RUN; then
    echo ""
    echo "Files that would be reviewed:"
    for file in "${FILES[@]}"; do
        echo "  - ${file#./}"
    done
    echo ""
    echo "Total: ${TOTAL_FILES} files"
    exit 0
fi

log "Starting code review..."
log "Max retries: ${MAX_RETRIES}, Retry delay: ${RETRY_DELAY}s"

# Clear failed files list at start
> "$FAILED_FILES_LIST"

# Function to run review with retries
run_review() {
    local file="$1"
    local attempt=1

    while [[ $attempt -le $MAX_RETRIES ]]; do
        log "Reviewing (attempt $attempt/$MAX_RETRIES): $file"

        # Run Claude Code with haiku model and code-reviewer skill
        # Use PIPESTATUS to capture the actual exit code (not tee's)
        set +e
        claude --model haiku "/code-reviewer $file" 2>&1 | tee -a "$LOG_FILE"
        local exit_code=${PIPESTATUS[0]}
        set -e

        if [[ $exit_code -eq 0 ]]; then
            return 0
        fi

        if [[ $attempt -lt $MAX_RETRIES ]]; then
            log "✗ Attempt $attempt failed for: $file (exit code: $exit_code)"
            log "Retrying in ${RETRY_DELAY} seconds..."
            sleep "$RETRY_DELAY"
        fi

        ((attempt++))
    done

    return 1
}

# Process each file
for file in "${FILES[@]}"; do
    # Remove leading ./
    file="${file#./}"

    # Check if already reviewed (when using --continue)
    REVIEW_FILE="$OUTPUT_DIR/${file//\//__}.review.md"
    if $CONTINUE && [[ -f "$REVIEW_FILE" ]]; then
        log "Skipping (already reviewed): $file"
        SKIPPED_FILES=$((SKIPPED_FILES + 1))
        continue
    fi

    # Run review with retries
    if run_review "$file"; then
        REVIEWED_FILES=$((REVIEWED_FILES + 1))
        log "✓ Completed: $file"
    else
        FAILED_FILES=$((FAILED_FILES + 1))
        FAILED_FILE_LIST+=("$file")
        echo "$file" >> "$FAILED_FILES_LIST"
        log "✗ All retries exhausted for: $file"
    fi

    # Small delay between reviews to avoid rate limiting
    sleep 1
done

# Summary
log ""
log "================================"
log "Code Review Complete"
log "================================"
log "Total files found:     ${TOTAL_FILES}"
log "Successfully reviewed: ${REVIEWED_FILES}"
log "Skipped (cached):      ${SKIPPED_FILES}"
log "Failed:                ${FAILED_FILES}"
log "Log file:              ${LOG_FILE}"

if [[ ${#FAILED_FILE_LIST[@]} -gt 0 ]]; then
    log ""
    log "Failed files saved to: $FAILED_FILES_LIST"
    log ""
    log "To retry failed files, run:"
    log "  ./scripts/review-all-files.sh --failed-only"
    log ""
    log "Failed files:"
    for failed_file in "${FAILED_FILE_LIST[@]}"; do
        log "  - $failed_file"
    done
fi

# Exit with error code if any files failed
if [[ ${#FAILED_FILE_LIST[@]} -gt 0 ]]; then
    exit 1
fi
