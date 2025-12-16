---
title: "`refact/refact-agent/gui/src/features/Providers/ProviderForm/ProviderModelsList/hooks`"
created: 2025-12-17
tags: ["architecture", "gui", "providers", "providermodelslist", "hooks", "react-hooks", "refact-agent"]
---

### `refact/refact-agent/gui/src/features/Providers/ProviderForm/ProviderModelsList/hooks`

**Purpose**  
This directory contains custom React hooks that manage state and logic for the **Provider Models List** UI within the Refact Agent's web GUI. It handles dialog states, form interactions, and optimistic updates specifically for adding, editing, and deleting AI models associated with providers (e.g., configuring models like GPT-4 for OpenAI or Llama3 for Ollama). These hooks encapsulate complex UI behaviors like modal visibility, temporary model edits, and validation, keeping the presentational components in `./components` clean and reusable. This fits the Providers feature's role in abstracting Rust backend capabilities (`engine/src/caps/providers.rs`), enabling users to customize model routing for chat, completion, and agentic workflows via a declarative UI layer.

**Files**  
From project structure and sibling documentation:  
- **`index.ts`** - Barrel export for all hooks (e.g., `export { useModelDialogState } from './useModelDialogState';`), enabling clean imports in `ProviderModelsList.tsx`.  
- **`useModelDialogState.ts`** - Core hook managing add/edit/delete dialog lifecycle: tracks visibility, temporary model state (`tempModel: Partial<ProviderModel>`), editing mode (add vs. edit), and callbacks for save/cancel. Handles optimistic UI (local state before backend mutation) and error recovery.  

The directory follows the GUI's standard "hooks" convention: a single focused hook file + index export. Naming uses `use[Feature]State` pattern, emphasizing local dialog/form state over global Redux (which handles provider lists via `providersSlice`).

**Architecture**  
- **Hook Pattern & Single Responsibility**: `useModelDialogState` follows React's custom hook best practices—isolates mutable dialog logic (open/close, temp state, validation) from components. Returns an object like `{ isOpen, tempModel, openDialog(model?), closeDialog(), saveModel(), deleteModel() }` for ergonomic consumption.  
- **State Management**: Local `useState`/`useReducer` for transient UI state (dialogs, drafts); integrates with parent `useProviderForm.ts` for form submission → RTK Query mutations (`useUpdateProvider`). No direct Redux dispatch—lifts state via callbacks to respect unidirectional data flow.  
- **Data Flow**: Triggered by events from `./components` (e.g., `AddModelButton` clicks call `openDialog()`); effects sync with backend via `services/refact/providers.ts` → Rust `/v1/providers` endpoint. Error handling via try/catch + toast notifications (inferred from `ErrorState.tsx`).  
- **Design Patterns**:  
  - **Custom Hook Abstraction**: Encapsulates dialog boilerplate (reducer for state transitions: IDLE → EDITING → SAVING → SUCCESS/ERROR).  
  - **Optimistic Updates**: Local mutations before API calls, with rollback on failure.  
  - **Layered Architecture**: Hooks (logic) → components (UI) → parent list (`ProviderModelsList.tsx`) → form (`ProviderForm.tsx`). Builds on Redux Toolkit Query for caching/query invalidation.  
- Fits GUI's feature-slice organization (`features/Providers/ProviderForm/ProviderModelsList/`): hooks sit between presentational components and business logic, mirroring Rust's modular `caps/providers.rs` (provider → models → capabilities).  

**Key Symbols**  
- **Hooks**:  
  - `useModelDialogState(props: { models: ProviderModel[], onSave: (model: ProviderModel) => void, onDelete: (id: string) => void })` → `{ isOpen: boolean, mode: 'add' | 'edit' | 'delete', tempModel: Partial<ProviderModel>, openDialog(model?: ProviderModel), closeDialog(), confirmDelete(), saveChanges() }`.  
- **Internal State**: `dialogMode`, `tempModelId`, `isSubmitting` (loading states).  
- **Types**: Relies on `services/refact/types.ts` (`ProviderModel: { id: string, name: string, reasoningType?: string, capabilities: string[] }`); utilities like `./utils/extractHumanReadableReasoningType`.  
- **Dependencies**: `useCallback`, `useReducer` (state machine), RTK Query hooks from parent. No side effects beyond callbacks.  

**Integration**  
- **Used By**: `./components` (e.g., `AddModelButton`, `ModelCardPopup` consume dialog state); orchestrated in `ProviderModelsList.tsx` alongside `ModelCard.tsx`. Flows up to `ProviderForm.tsx` → `ProvidersView.tsx`.  
- **Uses**:  
  - Parent: `useProviderForm.ts` (form context), `useUpdateProvider.ts` (mutations).  
  - Sibling dirs: `./utils/*` (reasoning type formatting), `./components/*` (renders based on hook returns).  
  - Global: `useAppDispatch/useAppSelector` (via parents for `providersSlice`), `services/refact/providers.ts` (GraphQL/REST), icons from `features/Providers/icons/`.  
- **Relationships**:  
  - **Inbound**: Model lists from RTK Query (`useProvidersQuery`), synced with Rust `yaml_configs/default_providers/*.yaml` and `GlobalContext`.  
  - **Outbound**: Model mutations → backend capabilities update → runtime effects in `ChatForm/AgentCapabilities.tsx` (tool/model selection).  
  - **Cross-Feature**: Enables `Integrations/` (model configs power integration tools); consumed by `Chat/` for dynamic provider routing.  
- **Compares to Existing**: Unlike broader `useProviderForm.ts` (full provider CRUD), this is narrowly scoped to model dialogs—**builds upon provider list patterns from `ConfiguredProvidersView` by introducing inline editing modals**. Unlike flat lists (e.g., `IntegrationsList`), adds stateful dialogs for complex nested data (models-within-providers), supporting self-hosted extensibility (e.g., custom Ollama models). Introduces **hook-driven optimistic UI**, absent in simpler views like `ProviderPreview`. Extension point: Add new hooks for advanced features (e.g., `useModelValidation`).  

This hooks directory exemplifies the GUI's "logic extraction" principle, making model management declarative and testable while bridging UI to Rust's provider abstraction layer.

---
title: "`refact/refact-agent/gui/src/features/Providers/ProviderForm/ProviderModelsList/hooks`"
created: 2025-12-17
tags: ["architecture", "gui", "providers", "providermodelslist", "hooks", "react-hooks", "refact-agent"]
