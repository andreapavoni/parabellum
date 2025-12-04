## Enhancing Building Details in `building.html`

### The Change
Modified `parabellum_web/templates/village/building.html` to display building descriptions. This change affects both already constructed buildings and the list of available buildings in an empty slot.

### The Reasoning
The previous `building.html` template provided minimal information about buildings. By adding the description from `en.yml`, users now have immediate context and details about each building's function and purpose directly within the building view. This improves the clarity and user-friendliness of the interface.

### The Tech Debt
The display of `slot.building.value` for existing buildings remains generic. The user's request indicated that this "value" should be presented in a way specific to each building type. Further development is needed to implement conditional rendering or specialized components to interpret and display this value appropriately for different buildings (e.g., production rates for resource buildings, capacity for warehouses, training speed for barracks).

## Refactoring to Building-Specific Partials

### The Change
Refactored `parabellum_web/templates/village/building.html` to use a partial-based system for displaying building details. A new directory `parabellum_web/templates/village/buildings` was created, and a `_resource_field.html` partial was added for resource-producing buildings. The main template now uses a `match` statement to include the appropriate partial. The dynamic i18n key issue was resolved by using static keys within `match` statements for both the partials and the "available buildings" list.

### The Reasoning
The previous implementation had two critical flaws:
1.  **Unsupported Dynamic i18n Keys:** The use of `t!(&format!(...))` is not supported by the `askama` template engine, as translations must be resolved at compile time. This would have caused the build to fail.
2.  **Lack of Scalability:** A single template for all buildings made it difficult to show specialized information for different building types.

The new partial-based architecture solves both problems. It ensures that all i18n keys are static strings, and it provides a clean, scalable structure for adding building-specific views. This makes the template system more robust and easier to extend.

### The Tech Debt
The foundation for specialized views is now in place, but the work is not complete. The immediate tech debt is to create specific partials for all other building types (e.g., Barracks, Rally Point, etc.), which currently fall back to a generic placeholder display. Additionally, the `_resource_field.html` partial can be further enhanced to show more detailed production information, such as bonuses from other buildings like the Sawmill.

## Barracks Training & Academy Research Fix

### The Change
- Marked `AcademyResearch` as `#[serde(transparent)]` and gave it helper methods so existing JSON rows deserialize correctly while exposing a cleaner API (`get`/`set`).
- Added `Village::available_units_for_training`, a Barracks-specific partial, and a new `/army/train` handler so the Barracks page can show infantry research status, training times, and CSRF-protected training forms.
- Introduced `unit_display_name` and `building_description` helpers plus `view_helpers::building_description` wiring so translations stay centralized, and routed `/army/train` in `http.rs`.
- Added the missing building description plumbing to the available-buildings list and switched to helper-driven text instead of fragile template matches.

### The Reasoning
We hit a runtime error because the new `AcademyResearch` struct serialized as an object while the database still held the legacy array payloads. The transparent serde wrapper restores backward compatibility without giving up the richer API. With that fixed we could finish the Barracks UI work: the page now surfaces how training time scales with level and exposes only the infantry units that were actually researched, mirroring Travian’s behavior. Moving translation lookups into Rust helpers avoids repetitive (and error-prone) string matching inside Askama templates.

### The Tech Debt
- The same helper infrastructure should backfill other building-specific partials (e.g., Stable, Workshop) so they can display bespoke stats and training forms.
- `/army/train` presently trusts the posted `unit_idx`/`building_name`; we should add server-side assertions that the slot really holds a Barracks and that the unit belongs to the player’s tribe.

## Training Queue UI + Stable/Workshop Support

### The Change
- Added a new `GetVillageTrainingQueue` query/handler plus repository support so we can list pending `TrainUnits` jobs per village (`parabellum_app/cqrs/queries.rs`, `parabellum_app/queries_handlers/get_village_training_queue.rs`, `parabellum_db/repository/job_repository.rs`).
- Surface that queue in the web layer through `training_queue_or_empty`, `training_queue_to_views`, and new `UnitTrainingQueueItemView` structs, then render it with countdown timers on the Barracks/Stable/Workshop partials (`parabellum_web/handlers/building_handler.rs`, `parabellum_web/view_helpers.rs`, `parabellum_web/templates/village/buildings/_*.html`).
- Generalized the training option builder so Stable (cavalry) and Workshop (siege) now expose research-aware training forms, leveraging the existing `/army/train` command.
- Localized the new UI text and documented the session in the dev log.

### The Reasoning
Players need real feedback about troops currently in production. By plumbing the training jobs through the query layer we can show a live countdown and remaining quantity per building, matching the request. Extending the same machinery to Stable/Workshop keeps the UI consistent: once a unit is researched it automatically appears in the relevant building form, and the page shows how building level impacts training speed for all three military buildings.

### The Tech Debt
- Training jobs should be sequential. Each job, regardless of the quantity of units to train, should be executed 1 at a time. This means that if there's a job with 10 units, when player adds a new job with 5 more units, that job should start after the first one is completed at all. This, however, needs to be checked better, because the actual behaviour in [@train_units.rs](file:///Users/andrea/Code/Apps/parabellum/parabellum_app/command_handlers/train_units.rs) is to schedule 1 job for the first unit, then in [@train_units.rs](file:///Users/andrea/Code/Apps/parabellum/parabellum_app/job_handlers/train_units.rs) to schedule another one for the next while decreasing the quantity of units to train. This aspect will do wrong calculations to determine when the job to train N troops will finish, and schedule the next. Maybe we should track the total completion time and keep it in mind when scheduling new training jobs for the same slot.
- Training jobs list is always ordered the same way
- Troops in the village should be visibile in [@resources.html](file:///Users/andrea/Code/Apps/parabellum/parabellum_web/templates/village/resources.html)
- `/army/train` should validate that the slot actually contains the building type indicated, even though upstream logic should prevent tampering.

## Training Queue Scheduling & Village Troops

### The Change
- Updated `TrainUnitsCommandHandler` to inspect the existing training backlog per slot (using `list_village_training_queue`) so new jobs are scheduled after all queued units, and added a regression test to cover the ordering logic.
- Surfaces the home garrison on `resources.html`, reusing `unit_display_name` to render the player’s real troop counts instead of the static “no units” placeholder.
- Hardened `/army/train` by checking that the posted slot holds the requested building, and added a dedicated i18n error string.

### The Reasoning
Players were seeing new training jobs finish too early because the scheduler only accounted for the first unit of each queued order. Summing the remaining duration per slot guarantees true sequential execution and predictable countdowns. While touching that flow we also addressed the longstanding “troops not visible on the resource page” feedback and closed the validation loophole in the training form.

### The Tech Debt
- Reinforcements are still hidden on the resource screen; we only list the village’s own garrison for now.
- Great Barracks/Stable/Workshop still bypass the specialized partials—extend the new helper once those buildings go live.

## Build Options & Queue Guardrails

### The Change
- Added `candidate_buildings_for_slot` so empty-slot UIs can list every possible structure while `available_buildings_for_slot` keeps enforcing the stricter “can build right now” rules (`parabellum_game/models/village.rs`).
- Reworked the building page to show those candidates with requirement callouts, queue-aware filtering, and disabled CTAs until both resources and prerequisites are met; the UI now hides unique/conflicting buildings that are already under construction elsewhere (`parabellum_web/handlers/building_handler.rs`, `parabellum_web/templates/village/building.html`, locales/exports).
- Hardened `AddBuildingCommandHandler` so backend jobs refuse duplicates or conflicting constructions if a matching job already sits in the queue, plus new regression tests for the scenarios (`parabellum_app/command_handlers/add_building.rs`).

### The Reasoning
Players couldn’t discover buildings like the Smithy, Residence, or Palace because we filtered them out whenever prerequisites weren’t met, and unique buildings still appeared on other slots even while a job was already queued. By separating “candidate” vs. “buildable” logic we can now show the full catalog with a clear explanation of what’s missing, while queue-aware filtering prevents Palace/Residence and other mutually-exclusive structures from resurfacing elsewhere. The server-side guardrail mirrors the UI so tampering with the form or racing requests can’t sneak a second Palace into the pipeline.

### The Tech Debt
- Requirement callouts only cover level prerequisites; future passes could surface capital-only constraints, tribe locks, or conflicts directly in the UI.
- Great Barracks/Stable/Workshop still bypass the bespoke partials, so their cards continue to show the generic “awaiting implementation” panel until those views are implemented.

## Multi-Build Unlocks & Split Lists

### The Change
- Fixed `Village::validate_building_construction` so buildings that allow multiples (Warehouse/Granary) only require that at least one existing copy has reached max level, instead of incorrectly demanding the *new* building be at level 20; added regression tests to cover both failure and success paths once the first structure is maxed (`parabellum_game/models/village.rs`).
- Introduced `building_options_for_slot` to return two groups—ready-to-build and requirements-missing—and updated the building page to render separate sections with requirement callouts; the UI now shows multi-build candidates as soon as they’re unlocked and clearly lists remaining locked options (`parabellum_web/handlers/building_handler.rs`, `parabellum_web/templates/village/building.html`, locales/exports).
- Added handler-level tests to ensure multi-build options appear once unlocked and that we actually populate both lists (`parabellum_web/handlers/building_handler.rs`).

### The Reasoning
Players who had a level 20 Warehouse or Granary still couldn’t add a second copy because our duplicate guard mistakenly looked at the prospective building’s level (always 1) instead of the existing ones. That bug also meant the building picker never listed additional storage even after maxing the first structure. By checking the village’s current buildings we now mirror Travian’s rule set. Splitting the picker into “ready” vs. “locked” groups keeps the catalog scannable and makes it obvious which requirements are still missing while ensuring newly unlocked multi-build options appear in the correct column.

### The Tech Debt
- Capital/tribe constraints are still implicit in the locked list; surfacing those rules inline would further reduce confusion.

## Great Military Partials

### The Change
- Updated the military building partials to use dynamic descriptions and wired `building.html` to reuse them for Great Barracks/Stable/Workshop, so those variants now render the same rich stats, queues, and training forms (`parabellum_web/templates/village/buildings/_*.html`, `parabellum_web/templates/village/building.html`).
- Extended the training-option helper to accept both regular and Great building names, ensuring Great variants expose the correct infantry/cavalry/siege options, plus a regression test to guard the scenario (`parabellum_web/handlers/building_handler.rs`).

### The Reasoning
Great military buildings share the same UI expectations as their standard counterparts, but we only matched on the base names, so those slots fell back to the generic placeholder and hid the training workflows. By widening the match and the helper inputs we can display the specialty layouts regardless of whether the player builds the normal or Great version.

### The Tech Debt
- We still don’t explain the Great-building 3× resource cost multiplier anywhere in the UI; adding a succinct blurb near the training form would make the difference clearer.
- Capital-only requirements remain implicit; the locked list still doesn’t explicitly mention when a Great structure can’t be built outside the capital.

## Academy Research & UI

### The Change
- Added `academy_options_for_village` plus supporting view structs so the Academy page can group infantry unlocks into ready/locked/researched buckets and expose i18n requirement callouts (`parabellum_web/handlers/building_handler.rs`, `parabellum_web/templates/village.rs`).
- Introduced a dedicated `_academy.html` partial with research cards, CSRF-protected forms, and resource-aware buttons, then routed `/academy/research` through the new handler that posts `ResearchAcademy` commands (`parabellum_web/templates/village/building.html`, `parabellum_web/templates/village/buildings/_academy.html`, `parabellum_web/handlers/academy_handler.rs`, `parabellum_web/http.rs`).
- Localized the new strings, re-exported the template data, and documented the feature here.

### The Reasoning
Players couldn’t research fresh troop types even after investing in the Academy because there was no UI surface or backend endpoint to start those jobs. Mirroring the building picker pattern keeps the UX consistent: unlocked units render actionable forms, locked ones list missing prerequisites, and completed research moves into a compact badge list. The handler validates that the slot really hosts an Academy and funnels everything through the existing `ResearchAcademy` pipeline so resource deductions, queueing, and job execution stay authoritative server-side.

### The Tech Debt
- We still don’t show in-progress Academy jobs or countdown timers; once the queue plumbing exposes those durations we should mirror the training queue UI here.

## Academy Research Queue

### The Change
- Added `GetVillageAcademyQueue` plus repository support so we can fetch `ResearchAcademy` jobs per village, and exposed it through the web helpers for graceful error handling (`parabellum_app/cqrs/queries.rs`, `parabellum_app/queries_handlers/get_village_academy_queue.rs`, `parabellum_app/repository/job_repository.rs`, `parabellum_db/repository/job_repository.rs`, `parabellum_web/handlers/helpers.rs`).
- Introduced `AcademyResearchQueueItemView` together with `academy_queue_to_views`, threaded it through `BuildingTemplate`, and rendered a countdown-enabled queue section in the Academy partial with new localization strings (`parabellum_web/templates/village.rs`, `parabellum_web/view_helpers.rs`, `parabellum_web/templates/village/buildings/_academy.html`, locales).
- Added a regression test to ensure queue items correctly flag processing jobs and documented the work here.

### The Reasoning
Academy research jobs previously disappeared into the void once scheduled, so players couldn’t tell which unit was unlocking or how long it would take. By exposing the queue and mirroring the countdown UI we already use for troop training, the Academy panel now provides immediate feedback: in-progress units show a timer, and pending jobs remain visible so players understand why the CTA might be disabled.

### The Tech Debt
- We still don’t block new research submissions when the queue isn’t empty; once design clarifies whether serial execution should lock the button, we can disable the form when work is pending.

## Smithy Upgrades & Queue

### The Change
- Added `GetVillageSmithyQueue`, repository support, and helper plumbing so the web layer can list `ResearchSmithy` jobs alongside building/training/academy queues (`parabellum_app/cqrs/queries.rs`, `parabellum_app/queries_handlers/get_village_smithy_queue.rs`, `parabellum_app/repository/job_repository.rs`, `parabellum_db/repository/job_repository.rs`, `parabellum_web/handlers/helpers.rs`).
- Introduced smithy-specific view models (`SmithyUpgradeOption`, `SmithyQueueItemView`) plus `smithy_queue_to_views`/`smithy_options_for_village`, then wired `BuildingTemplate` and the new `_smithy.html` partial to show researched units, upgrade availability capped by the building level, queued progress, and a countdown-driven queue list with localized copy (`parabellum_web/templates/village.rs`, `parabellum_web/templates/village/buildings/_smithy.html`, `parabellum_web/view_helpers.rs`, locales).
- Exposed a CSRF-protected `/smithy/research` endpoint that validates the slot before posting `ResearchSmithy`, added regression tests covering smithy queue math and option gating, and documented everything here (`parabellum_web/handlers/smithy_handler.rs`, `parabellum_web/handlers/building_handler.rs`, `parabellum_web/http.rs`).

### The Reasoning
Players could pay for smithy upgrades but never saw which units were eligible, how far they’d been boosted, or what was currently queued. By mirroring the Academy workflow we now surface the smithy level cap, per-unit progress, and sequential queue so it’s obvious why a button is disabled (unresearched, maxed, or already enqueued). The dedicated handler keeps validation server-side while reusing the existing job pipeline.

### The Tech Debt
- Posting smithy upgrades still doesn’t consider other pending upgrades when validating on the command handler; mirroring the building queue guardrails there would prevent ordering more levels than the current smithy can support.
