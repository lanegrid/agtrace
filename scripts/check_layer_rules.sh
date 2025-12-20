#!/bin/bash
set -euo pipefail

# Layer Rules Checker for agtrace presentation layer architecture
#
# ============================================================================
# EXPECTED LAYER STRUCTURE
# ============================================================================
#
# This project follows a Layered Architecture with clear separation of concerns:
#
# Level 1: Crate-level Structure (Physical Boundaries)
# ────────────────────────────────────────────────────
#
#   ┌─────────────────────────────────────────────────────────┐
#   │ agtrace-cli (Top Level - Composition Root)              │
#   │  - Depends on: ALL crates                               │
#   │  - Role: User interface, command handling, orchestration│
#   └─────────────────────────────────────────────────────────┘
#                          ▼ depends on
#   ┌──────────────────────────────────────┐
#   │ agtrace-runtime (Service Level)      │
#   │  - Depends on: engine, index,        │
#   │                providers, types      │
#   │  - Role: Orchestration & workflows   │
#   └──────────────────────────────────────┘
#            ▼                 ▼              ▼
#   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
#   │ agtrace-     │  │ agtrace-     │  │ agtrace-     │
#   │ engine       │  │ index        │  │ providers    │
#   │              │  │              │  │              │
#   │ Core logic   │  │ DB adapter   │  │ File parsers │
#   │ (Pure)       │  │              │  │              │
#   └──────────────┘  └──────────────┘  └──────────────┘
#            ▼                 ▼              ▼
#                   ┌──────────────┐
#                   │ agtrace-types│
#                   │              │
#                   │ Shared kernel│
#                   │ Domain models│
#                   └──────────────┘
#
# Key Principle: Dependencies flow DOWNWARD only
#   - agtrace-engine is PURE: no dependencies on cli, index, or providers
#   - agtrace-types is SHARED: all crates depend on it
#
# Level 2: Module-level Structure within agtrace-cli (Logical Boundaries)
# ────────────────────────────────────────────────────────────────────────
#
#   agtrace-cli/
#   ├── handlers/          (Orchestrators)
#   │   └─[Uses all layers below]
#   │
#   ├── presentation/
#   │   ├── presenters/    (Domain → ViewModel converters)
#   │   │   ├─[Depends on] agtrace_engine, agtrace_types
#   │   │   ├─[Produces]   view_models
#   │   │   └─[Must NOT]   Have side effects (I/O, DB)
#   │   │
#   │   ├── view_models/   (Data Transfer Objects for display)
#   │   │   ├─[Contains]   ONLY primitives (String, Vec, bool, etc.)
#   │   │   └─[Must NOT]   Reference agtrace_engine or agtrace_types
#   │   │
#   │   ├── renderers/     (Output generators)
#   │   │   ├─[Consumes]   view_models ONLY
#   │   │   └─[Must NOT]   Know about domain types (AgentSession, etc.)
#   │   │
#   │   └── formatters/    (Pure utility functions)
#   │       ├─[Accepts]    Primitive types (String, &str, usize, etc.)
#   │       └─[Must NOT]   Depend on domain types or view_models
#   │
#   └── [Other modules]
#
# Data Flow (One Direction Only):
#   Domain Model (agtrace-engine)
#        ▼ [Presenter converts]
#   ViewModel (view_models/)
#        ▼ [Renderer displays]
#   Output (Terminal, TUI, JSON, etc.)
#
# ============================================================================
# ARCHITECTURAL INVARIANTS (Rules Enforced by This Script)
# ============================================================================
#
# Level 1: Crate-level Invariants (Physical Constraints)
#   ✓ Core Purity: agtrace-engine must not depend on agtrace-cli, agtrace-index, agtrace-providers
#   ✓ Type Sharing: All domain models should be defined in agtrace-types
#
# Level 2: Module-level Invariants within agtrace-cli (Logical Constraints)
#   ✓ Renderer Ignorance: renderers/ must not use agtrace_engine, agtrace_providers, agtrace_types
#   ✓ ViewModel Independence: view_models/ must not contain agtrace_engine types as fields
#   ✓ Presenter Direction: presenters/ should only convert Domain → ViewModel (no side effects)
#   ✓ Handler Mediation: handlers/ must pass ViewModels to Renderers, not raw domain types
#
# ============================================================================
# USAGE
# ============================================================================
#
# Run the checker:
#   ./scripts/check_layer_rules.sh
#
# Exit Codes:
#   0 - All rules satisfied
#   1 - Violations detected (see output for refactoring suggestions)
#
# Integration:
#   Add to CI/CD pipeline to prevent architectural decay:
#     - Pre-commit hook
#     - GitHub Actions workflow
#     - cargo make tasks
#
# ============================================================================

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

violation_count=0

echo "🔍 Checking presentation layer architecture rules..."
echo ""

# Helper function to check forbidden dependencies
check_forbidden_deps() {
    local layer="$1"
    local layer_path="$2"
    shift 2
    local forbidden_patterns=("$@")

    local files=$(find "$layer_path" -name "*.rs" 2>/dev/null || true)

    if [ -z "$files" ]; then
        return
    fi

    for file in $files; do
        for pattern in "${forbidden_patterns[@]}"; do
            local matches=$(grep -n "use.*$pattern" "$file" 2>/dev/null || true)
            if [ -n "$matches" ]; then
                echo -e "${RED}❌ VIOLATION in $layer${NC}"
                echo -e "   File: ${BLUE}$file${NC}"
                echo -e "   Issue: Forbidden dependency detected: ${YELLOW}$pattern${NC}"
                echo "$matches" | while IFS= read -r line; do
                    echo -e "   ${YELLOW}$line${NC}"
                done
                echo ""

                # Provide refactoring suggestion
                suggest_refactoring "$layer" "$pattern" "$file"
                echo ""
                ((violation_count++))
            fi
        done
    done
}

# Helper function to check forbidden re-exports
check_forbidden_reexports() {
    local layer="$1"
    local layer_path="$2"
    shift 2
    local forbidden_patterns=("$@")

    local files=$(find "$layer_path" -name "*.rs" 2>/dev/null || true)

    if [ -z "$files" ]; then
        return
    fi

    for file in $files; do
        for pattern in "${forbidden_patterns[@]}"; do
            local matches=$(grep -n "pub use.*$pattern" "$file" 2>/dev/null || true)
            if [ -n "$matches" ]; then
                echo -e "${RED}❌ VIOLATION in $layer${NC}"
                echo -e "   File: ${BLUE}$file${NC}"
                echo -e "   Issue: Forbidden re-export detected: ${YELLOW}$pattern${NC}"
                echo "$matches" | while IFS= read -r line; do
                    echo -e "   ${YELLOW}$line${NC}"
                done
                echo ""

                # Provide refactoring suggestion for re-exports
                suggest_reexport_refactoring "$layer" "$pattern" "$file"
                echo ""
                ((violation_count++))
            fi
        done
    done
}

# Suggest refactoring based on violation
suggest_refactoring() {
    local layer="$1"
    local forbidden="$2"
    local file="$3"

    echo -e "${BLUE}💡 Refactoring Suggestion:${NC}"

    case "$layer" in
        "view_models")
            if [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_types"* ]]; then
                echo "   ViewModel violates Level 2: ViewModel Independence"
                echo "   → ViewModels should only contain primitive types (String, Vec, bool, etc.)"
                echo "   → Move domain type references to presenters/"
                echo "   → Define new ViewModel struct with primitive fields"
                echo "   → Presenter will convert domain types to this ViewModel"
            elif [[ "$forbidden" == *"renderers"* ]]; then
                echo "   ViewModels should not know about rendering."
                echo "   → Remove renderer imports from ViewModels"
                echo "   → ViewModels are data contracts, not rendering logic"
            fi
            ;;
        "views")
            if [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_runtime"* ]] || [[ "$forbidden" == *"agtrace_index"* ]] || [[ "$forbidden" == *"agtrace_providers"* ]]; then
                echo "   Views should not have domain knowledge (similar to renderers)."
                echo "   → Views consume ViewModels only, not domain types"
                echo "   → Create a ViewModel in presentation/view_models/"
                echo "   → Create a Presenter to convert domain model to ViewModel"
                echo "   → Update View to accept only ViewModel types"
            elif [[ "$forbidden" == *"agtrace_types"* ]]; then
                echo "   Views should avoid complex domain types from agtrace_types."
                echo "   → Use ViewModels with primitive types instead"
            elif [[ "$forbidden" == *"presenters"* ]]; then
                echo "   Views should not call presenters directly."
                echo "   → Handler should call Presenter first, then pass ViewModel to View"
            fi
            ;;
        "renderers")
            if [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_providers"* ]]; then
                echo "   Renderer should not have domain knowledge (Level 2: Renderer Ignorance)."
                echo "   → Create a ViewModel in presentation/view_models/"
                echo "   → Create a Presenter to convert domain model to ViewModel"
                echo "   → Update Renderer to accept only ViewModel types"
            elif [[ "$forbidden" == *"agtrace_types"* ]]; then
                echo "   Renderer should avoid complex domain types from agtrace_types."
                echo "   → Use ViewModels with primitive types instead"
                echo "   → Simple enums (e.g., LogLevel) may be acceptable, but complex types should be avoided"
            elif [[ "$forbidden" == *"presenters"* ]]; then
                echo "   Renderer should not call presenters directly."
                echo "   → Handler should call Presenter first, then pass ViewModel to Renderer"
            fi
            ;;
        "presenters")
            if [[ "$forbidden" == *"renderers"* ]]; then
                echo "   Presenter should not know about rendering implementation."
                echo "   → Return ViewModels from Presenter"
                echo "   → Let Handler pass ViewModels to Renderer"
            fi
            ;;
        "formatters")
            if [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_types"* ]] || [[ "$forbidden" == *"agtrace_index"* ]] || [[ "$forbidden" == *"agtrace_runtime"* ]] || [[ "$forbidden" == *"agtrace_providers"* ]]; then
                echo "   Formatters should be pure utility functions (no domain knowledge)."
                echo "   → Current: Formatter knows about domain types"
                echo "   → Target: Formatter accepts only primitive types (String, &str, usize, etc.)"
                echo "   → Move domain-to-primitive conversion to presenters/"
                echo "   → Example: Instead of from_summaries(sessions: Vec<SessionSummary>)"
                echo "            Use: from_entries(entries: Vec<SessionEntry>) where SessionEntry is in formatters/"
                echo "            Presenter converts SessionSummary → SessionListEntryViewModel"
                echo "            Renderer converts SessionListEntryViewModel → SessionEntry (primitive struct)"
            elif [[ "$forbidden" == *"view_models"* ]]; then
                echo "   Formatters should not depend on ViewModels to avoid circular dependency."
                echo "   → Use primitive types or define shared types in formatters/"
                echo "   → ViewModels can use formatters, but not vice versa"
            fi
            ;;
    esac
}

# Suggest refactoring for re-export violations
suggest_reexport_refactoring() {
    local layer="$1"
    local forbidden="$2"
    local file="$3"

    echo -e "${BLUE}💡 Refactoring Suggestion:${NC}"
    echo "   Re-exporting types breaks layer boundaries."

    case "$layer" in
        "formatters")
            if [[ "$forbidden" == *"view_models"* ]]; then
                echo "   → ALLOWED ONLY for backward compatibility during migration"
                echo "   → Temporary re-exports should have a comment explaining why"
                echo "   → Plan to remove re-exports once callers are updated"
                echo "   → Example comment: // Re-export for backward compatibility"
            elif [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_types"* ]]; then
                echo "   → Remove re-export of domain types from formatters"
                echo "   → Formatters should only work with primitives"
                echo "   → Use views/ for complex formatting with ViewModels"
            fi
            ;;
        "view_models")
            echo "   → ViewModels should not re-export domain types"
            echo "   → Define primitive equivalents instead"
            echo "   → Presenters handle the conversion from domain to ViewModel"
            ;;
        "renderers")
            if [[ "$forbidden" == *"agtrace_engine"* ]] || [[ "$forbidden" == *"agtrace_types"* ]]; then
                echo "   → Renderers should not re-export domain types"
                echo "   → Update trait signatures to use ViewModels"
                echo "   → Remove re-exports once all callers updated"
            fi
            ;;
        *)
            echo "   → Remove the re-export and use the type directly where needed"
            echo "   → Re-exports can hide architectural violations"
            ;;
    esac
}

# Check view_models layer
echo "📦 Checking crates/agtrace-cli/src/presentation/view_models/..."
check_forbidden_deps \
    "view_models" \
    "crates/agtrace-cli/src/presentation/view_models" \
    "agtrace_engine::" \
    "agtrace_providers::" \
    "agtrace_index::" \
    "agtrace_types::" \
    "crate::handlers" \
    "crate::presentation::renderers"

# Check renderers layer
echo "🎨 Checking crates/agtrace-cli/src/presentation/renderers/..."
check_forbidden_deps \
    "renderers" \
    "crates/agtrace-cli/src/presentation/renderers" \
    "agtrace_engine::" \
    "agtrace_runtime::" \
    "agtrace_index::" \
    "agtrace_providers::" \
    "agtrace_types::" \
    "crate::presentation::presenters" \
    "crate::handlers"

# Check views layer (similar to renderers)
echo "👁️  Checking crates/agtrace-cli/src/presentation/views/..."
check_forbidden_deps \
    "views" \
    "crates/agtrace-cli/src/presentation/views" \
    "agtrace_engine::" \
    "agtrace_runtime::" \
    "agtrace_index::" \
    "agtrace_providers::" \
    "agtrace_types::" \
    "crate::presentation::presenters" \
    "crate::handlers"

# Check presenters layer
echo "🔄 Checking crates/agtrace-cli/src/presentation/presenters/..."
check_forbidden_deps \
    "presenters" \
    "crates/agtrace-cli/src/presentation/presenters" \
    "crate::presentation::renderers" \
    "crate::handlers"

# Check formatters layer
echo "✨ Checking crates/agtrace-cli/src/presentation/formatters/..."
check_forbidden_deps \
    "formatters" \
    "crates/agtrace-cli/src/presentation/formatters" \
    "agtrace_engine::" \
    "agtrace_runtime::" \
    "agtrace_index::" \
    "agtrace_providers::" \
    "agtrace_types::" \
    "crate::presentation::view_models"

# Check for forbidden re-exports
echo "🔁 Checking for forbidden re-exports..."

check_forbidden_reexports \
    "formatters" \
    "crates/agtrace-cli/src/presentation/formatters" \
    "agtrace_engine::" \
    "agtrace_types::"

check_forbidden_reexports \
    "view_models" \
    "crates/agtrace-cli/src/presentation/view_models" \
    "agtrace_engine::" \
    "agtrace_providers::" \
    "agtrace_types::"

check_forbidden_reexports \
    "renderers" \
    "crates/agtrace-cli/src/presentation/renderers" \
    "agtrace_engine::" \
    "agtrace_providers::" \
    "agtrace_types::"

# Check Renderer Traits for domain type contamination
echo "🎭 Checking Renderer Traits for domain type contamination..."
traits_file="crates/agtrace-cli/src/presentation/renderers/traits.rs"
if [ -f "$traits_file" ]; then
    # Check for forbidden use statements in traits.rs
    forbidden_imports=$(grep -n "^use agtrace_engine::\|^use agtrace_runtime::\|^use agtrace_index::\|^use agtrace_providers::" "$traits_file" 2>/dev/null || true)
    if [ -n "$forbidden_imports" ]; then
        echo -e "${RED}❌ VIOLATION: Renderer Traits import domain types${NC}"
        echo -e "   File: ${BLUE}$traits_file${NC}"
        echo "   Issue: Renderer traits must not import domain/runtime/DB types"
        echo ""
        echo "$forbidden_imports" | while IFS= read -r line; do
            echo -e "   ${YELLOW}$line${NC}"
        done
        echo ""
        echo -e "${BLUE}💡 Refactoring Suggestion:${NC}"
        echo "   Renderer Trait Invariant Violation Detected"
        echo "   → Remove imports of agtrace_engine, agtrace_runtime, agtrace_index, agtrace_providers"
        echo "   → Create corresponding ViewModels in presentation/view_models/"
        echo "   → Example violations and fixes:"
        echo "      ❌ fn render_session_list(&self, sessions: &[SessionSummary]) -> Result<()>"
        echo "      ✅ fn render_session_list(&self, sessions: &[SessionListEntryViewModel]) -> Result<()>"
        echo ""
        echo "      ❌ fn on_watch_reaction(&self, reaction: &Reaction) -> Result<()>"
        echo "      ✅ fn on_watch_reaction(&self, reaction: &ReactionViewModel) -> Result<()>"
        echo ""
        echo "      ❌ fn render_stream_update(&self, state: &SessionState, ...) -> Result<()>"
        echo "      ✅ fn render_stream_update(&self, state: &StreamStateViewModel, ...) -> Result<()>"
        echo ""
        ((violation_count++))
    fi

    # Check for Result<...> types as parameters (not return types) - indicates logic contamination
    # Look for lines with "result:" or similar parameter names with Result type
    result_params=$(grep -n "result:.*Result<\|Result<.*> *," "$traits_file" | grep -v "^[[:space:]]*/" | grep -v "^[[:space:]]*//" | grep -v ") -> Result<" || true)
    if [ -n "$result_params" ]; then
        echo -e "${RED}❌ VIOLATION: Renderer Traits contain Result<...> parameters${NC}"
        echo -e "   File: ${BLUE}$traits_file${NC}"
        echo "   Issue: Renderer traits must not accept Result<...> as parameters"
        echo "   Reason: This forces Renderer to perform logic (match/if on Ok/Err)"
        echo ""
        echo "$result_params" | while IFS= read -r line; do
            echo -e "   ${YELLOW}$line${NC}"
        done
        echo ""
        echo -e "${BLUE}💡 Refactoring Suggestion:${NC}"
        echo "   → Replace Result<T, E> parameters with ViewModels containing status fields"
        echo "   → Example:"
        echo "      ❌ fn render_doctor_check(&self, result: Result<&[EventViewModel], &Error>) -> Result<()>"
        echo "      ✅ fn render_doctor_check(&self, result: &DoctorCheckResultViewModel) -> Result<()>"
        echo ""
        echo "      where DoctorCheckResultViewModel contains:"
        echo "      pub struct DoctorCheckResultViewModel {"
        echo "          pub status: CheckStatus,  // enum { Success, Failure }"
        echo "          pub events: Vec<EventViewModel>,"
        echo "          pub error_message: Option<String>,"
        echo "      }"
        echo ""
        ((violation_count++))
    fi

    # Check trait method signatures for domain types (handles multi-line signatures)
    # Scan entire file for domain type usage in trait contexts
    # Use word boundaries to avoid matching ViewModels (e.g., ReactionViewModel)
    domain_type_usage=$(grep -n "SessionSummary\|SessionState\|&Reaction[^V]\|: Reaction[^V]\|&Reaction>\|: Reaction>" "$traits_file" | grep -v "^[[:space:]]*/" | grep -v "^[[:space:]]*//" || true)
    if [ -n "$domain_type_usage" ]; then
        echo -e "${RED}❌ VIOLATION: Renderer Trait methods use domain types${NC}"
        echo -e "   File: ${BLUE}$traits_file${NC}"
        echo "   Issue: Trait method signatures contain domain/runtime/DB types"
        echo ""
        echo "$domain_type_usage" | while IFS= read -r line; do
            echo -e "   ${YELLOW}$line${NC}"
        done
        echo ""
        echo -e "${BLUE}💡 Architectural Invariant:${NC}"
        echo "   Renderer Trait Invariants:"
        echo "   1. Parameter types must be crate::presentation::view_models::* or std primitives only"
        echo "   2. Must NOT accept types from agtrace_engine, agtrace_runtime, agtrace_index, agtrace_providers"
        echo "   3. Must NOT introduce control flow (Result, Option parameters for branching)"
        echo ""
        echo "   → Presenter converts Domain → ViewModel"
        echo "   → Renderer only knows ViewModels (keep it dumb)"
        echo ""
        ((violation_count++))
    fi
else
    echo -e "${YELLOW}⚠️  Renderer traits.rs not found (expected at $traits_file)${NC}"
    echo ""
fi

# Additional checks for common anti-patterns
echo "🔎 Checking for anti-patterns..."
echo ""

# Check if renderers have domain logic
renderer_files=$(find crates/agtrace-cli/src/presentation/renderers -name "*.rs" 2>/dev/null || true)
for file in $renderer_files; do
    # Check for direct AgentSession usage
    if grep -n "AgentSession" "$file" | grep -v "^[[:space:]]*/" | grep -v "^[[:space:]]*//" >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  WARNING in renderers${NC}"
        echo -e "   File: ${BLUE}$file${NC}"
        echo "   Issue: Direct usage of domain type 'AgentSession' detected"
        echo "   → This should be a ViewModel type instead"
        echo ""
        ((violation_count++))
    fi

    # Check for business logic in renderers (if-else on domain properties)
    if grep -n "\.is_error\|\.status\|\.error_kind" "$file" | grep "if\|match" >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  WARNING in renderers${NC}"
        echo -e "   File: ${BLUE}$file${NC}"
        echo "   Issue: Possible business logic in renderer (checking domain properties)"
        echo "   → Domain decisions should be made in Presenter"
        echo "   → ViewModel should contain pre-computed display properties (e.g., .style, .icon)"
        echo ""
    fi
done

# Check if handlers have direct rendering code
handler_files=$(find crates/agtrace-cli/src/handlers -name "*.rs" 2>/dev/null || true)
for file in $handler_files; do
    # Check for direct println! or crossterm usage
    if grep -n "println!\|print!\|crossterm::" "$file" >/dev/null 2>&1; then
        # Allow basic println for debugging, but warn about potential issues
        excessive_prints=$(grep -c "println!\|print!" "$file" || echo 0)
        if [ "$excessive_prints" -gt 3 ]; then
            echo -e "${YELLOW}⚠️  WARNING in handlers${NC}"
            echo -e "   File: ${BLUE}$file${NC}"
            echo "   Issue: Multiple print statements detected ($excessive_prints occurrences)"
            echo "   → Consider delegating to a Renderer"
            echo ""
        fi
    fi
done

# Check ViewModels for domain type fields (Level 2: ViewModel Independence)
echo "📋 Checking ViewModels for domain type fields..."
viewmodel_files=$(find crates/agtrace-cli/src/presentation/view_models -name "*.rs" 2>/dev/null || true)
for file in $viewmodel_files; do
    # Check for struct fields with domain types
    # Look for patterns like: pub field: AgentSession, field: Vec<AgentTurn>, etc.
    if grep -n ":\s*AgentSession\|:\s*AgentTurn\|:\s*AgentStep\|:\s*AgentEvent\|:\s*SessionDigest" "$file" | grep -v "^[[:space:]]*/" | grep -v "^[[:space:]]*//" >/dev/null 2>&1; then
        echo -e "${RED}❌ VIOLATION in view_models${NC}"
        echo -e "   File: ${BLUE}$file${NC}"
        echo "   Issue: ViewModel struct contains domain type fields"
        echo "   → ViewModels should only contain primitive types (String, Vec<String>, bool, etc.)"
        echo "   → Replace domain types with primitive equivalents"
        echo ""
        ((violation_count++))
    fi
done

# Check Presenters for side effects (Level 2: Presenter Direction)
echo "🔄 Checking Presenters for side effects..."
presenter_files=$(find crates/agtrace-cli/src/presentation/presenters -name "*.rs" 2>/dev/null || true)
for file in $presenter_files; do
    # Check for I/O operations that suggest side effects
    side_effects=$(grep -n "println!\|write!\|File::create\|File::open.*write\|\.execute\|\.insert\|\.update\|\.delete" "$file" 2>/dev/null | grep -v "^[[:space:]]*/" | grep -v "^[[:space:]]*//" || true)
    if [ -n "$side_effects" ]; then
        echo -e "${YELLOW}⚠️  WARNING in presenters${NC}"
        echo -e "   File: ${BLUE}$file${NC}"
        echo "   Issue: Presenter may have side effects (I/O, DB operations)"
        echo "   → Presenters should only perform pure transformations: Domain -> ViewModel"
        echo "   → Move side effects to handlers/"
        echo "$side_effects" | while IFS= read -r line; do
            echo -e "   ${YELLOW}$line${NC}"
        done
        echo ""
    fi
done

# Check for temporary backward compatibility re-exports (technical debt)
echo "🔧 Checking for temporary backward compatibility re-exports..."
temp_reexport_count=0
presentation_files=$(find crates/agtrace-cli/src/presentation -name "*.rs" 2>/dev/null || true)
for file in $presentation_files; do
    # Look for re-export comments mentioning backward compatibility
    if grep -qi "re-export.*backward compatibility\|backward compatibility.*re-export" "$file" 2>/dev/null; then
        # Count the re-export lines near these comments
        local_count=$(grep -A 5 -i "re-export.*backward compatibility\|backward compatibility.*re-export" "$file" 2>/dev/null | grep "pub use" | wc -l | tr -d ' ')
        if [ "$local_count" -gt 0 ]; then
            temp_reexport_count=$((temp_reexport_count + local_count))
            echo -e "${YELLOW}⚠️  TECHNICAL DEBT${NC}"
            echo -e "   File: ${BLUE}$file${NC}"
            echo "   Temporary re-exports for backward compatibility: $local_count"
            grep -n -A 3 -i "re-export.*backward compatibility\|backward compatibility.*re-export" "$file" 2>/dev/null | grep -E "^[0-9]+[-:].*pub use" | while IFS= read -r line; do
                echo -e "   ${YELLOW}$line${NC}"
            done
            echo ""
        fi
    fi
done

if [ $temp_reexport_count -gt 0 ]; then
    echo -e "${YELLOW}📊 Total temporary re-exports for backward compatibility: $temp_reexport_count${NC}"
    echo "   → These should be reduced over time"
    echo "   → Update callers to import directly from the correct layer"
    echo "   → Remove re-exports once migration is complete"
    echo ""
fi

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $violation_count -eq 0 ]; then
    echo -e "${GREEN}✅ All layer rules are satisfied!${NC}"
    echo "   Architecture is clean and maintainable."
else
    echo -e "${RED}❌ Found $violation_count violation(s)${NC}"
    echo "   Please review the suggestions above and refactor accordingly."
    exit 1
fi
