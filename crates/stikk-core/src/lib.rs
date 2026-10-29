//! The operation layer — one shared operation set both frontends drive (design `stikk-04` AR-03,
//! MOD-05).
//!
//! This crate owns no I/O and no widgets. It turns a user intent into a sequence of seam requests
//! plus state reads and view-model productions, applying capability gating and (in later increments)
//! the preview-first rule and confirmation tiers. Because both the TUI and the GUI drive *this* API
//! and neither defines operations of its own, an operation present in one frontend and not the other
//! is impossible (the mechanical guarantee behind TUI/GUI parity, FR-123).
//!
//! This foundation increment implements one operation — [`fn@orient`] — the read-only orientation a
//! session opens with. More operation families land against the same shape.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bundle;
pub mod change_token;
pub mod changes;
pub mod commit;
pub mod compare;
pub mod confirm;
pub mod files;
pub mod glossary;
pub mod history;
pub mod merge;
pub mod orient;
pub mod palette;
pub mod patch;
pub mod piece;
pub mod present;
pub mod queue;
pub mod refs;
pub mod refusal_history;
pub mod seal;
pub mod session;
pub mod switch;
pub mod trust;
pub mod verify;

pub use bundle::{
    ALREADY_HELD, ALREADY_RECEIVED, BUNDLE_EFFECTS_SHOWN, BUNDLE_FROM_MINOR, BUNDLE_LINEAGE_LIMIT,
    BundleOpened, BundleView, CLOSED_IS_NOT_EXPORTED, COMPARED_WITH_AFTER, COMPARED_WITH_BEFORE,
    DESTINATION_EXISTS, DIVERGED_CLEAN, DIVERGED_CONFLICT, DIVERGED_UNDETERMINED, EXPORT_OPERATION,
    ExportPreview, ExportTarget, IMPORT_OPERATION, INSIDE_THE_REPOSITORY, ImportPreview,
    ImportTarget, LINEAGE_TOO_LONG_TO_TELL, NEW_TO_THIS_REPOSITORY, RECEIVED_IS_NOT_EXPORTED,
    RECEIVED_IS_READ_ONLY, RECEIVED_PREFIX, SHARES_NOTHING, TAGS_ARE_NOT_EXPORTED,
    UNPUBLISHED_IS_NOT_EXPORTED, WOULD_BRING_FORWARD, WOULD_MOVE_THE_RECEIVED_BRANCH_BACK,
    bundle_unavailable, export_confirm_and_execute, export_preview, export_unavailable,
    import_confirm_and_execute, import_preview, open_bundle, sentence_for, suggested_destination,
    unshown_effects_line,
};
pub use change_token::{change_token, staleness_notice};
pub use changes::{
    Authoring, ChangeEntry, ChangeKind, ChangesRead, ChangesView, DeclarationResolution,
    DeclarationState, DeclaredRename, QueuedElsewhere, RENAME_CONTENT_NOTE, RENAMES_ALSO_COUNTED,
    RefHistory, RenameDeclaration, RenameHalf, UnpublishedQueue, changes_view,
    queued_elsewhere_clauses,
};
pub use commit::{
    COMMIT_OPERATION, CommitPreview, CommitPreviewOutcome, CommitToken, RefusedDeclaration,
    RefusedPath, commit_confirm_and_execute, commit_preview, would_refuse_next_steps,
};
pub use compare::{
    COMPARE_FROM_MINOR, CompareView, Compared, ComparedEntry, ComparedSide, HunkRow, compare,
    compared,
};
pub use confirm::{
    Aside, ConfirmationSummary, ConfirmedToken, Evidence, FilePlan, FrozenPatches, Intent,
    KeyClaim, Outcome, PreviewToken, SHORT_ID_CHARS, TargetId, branch_notice, capability_gate,
    confirm, execute, preview, signing_key_claim, unshown_declarations_line, unshown_patches_line,
    unshown_paths_line, unshown_plan_lines_line,
};
pub use files::{
    BlockCursorRow, COPY_CONSEQUENCE, DESTINATION_EXISTS as FILES_DESTINATION_EXISTS,
    F_SAVES_A_FILE, FILES_FROM_MINOR, INSIDE_THE_REPOSITORY as FILES_INSIDE_THE_REPOSITORY,
    LOSES_THE_EXECUTE_BIT, PROVISIONAL_REPLAY, SAVE_ALL_OPERATION, SAVE_FILE_OPERATION,
    SaveAllPreview, SaveAllTarget, SaveFilePreview, SaveFileTarget, block_cursor_file,
    block_cursor_len, block_cursor_row, block_short, files_unavailable, no_files_to_save,
    save_all_confirm_and_execute, save_all_preview, save_file_confirm_and_execute,
    save_file_preview, saved_all_lines, saved_file_line, suggested_all_destination,
    suggested_file_destination,
};
pub use glossary::{GlossaryEntry, TermMapping};
pub use history::{
    BlockDetailView, BlockPatchRow, BlockPatches, BlockState, HistoryView, RefList, block_detail,
    history_view, list_refs,
};
pub use merge::{
    CLOSED_IS_NOT_MERGED, INTO_THE_CURRENT_BRANCH, LINEAGE_TRUNCATED, MERGE_FROM_MINOR,
    MERGE_LINEAGE_LIMIT, MERGE_OPERATION, MERGED_ALREADY, MergeBaseline, MergePlanRead,
    MergePlanView, MergePreview, MergeTarget, NO_SHARED_BLOCK, PLAN_UNAVAILABLE,
    SWITCH_TO_SEE_THE_FILES, TAGS_ARE_NOT_MERGED, THE_BASELINE_MOVED, THE_SAME_BRANCH_TWICE,
    WORK_IS_QUEUED_FOR_INTO, merge_baseline, merge_confirm_and_execute, merge_plan_view,
    merge_preview, merge_unavailable, not_confluent,
};
pub use orient::{OrientationView, orient};
pub use palette::{Command, NO_FOCUSED_REF_REASON};
pub use patch::{
    AS_AT_BLOCK_END, CONTENT_DISPLAY_BYTES, CONTENT_FROM_MINOR, NO_PATH_REPORTED, OperationRow,
    OperationView, PatchDetailView, PatchMessageState, patch_detail, patch_detail_in_block,
};
pub use piece::{Piece, flatten};
pub use present::{
    NextStep, NextTarget, OperationContext, Presentation, RefusalCard, Target, present,
    stale_gloss, stale_headline,
};
pub use queue::{QueueView, QueuedPatchView, queue_view};
pub use refs::{
    CLOSE_BRANCH_OPERATION, CLOSING_THE_CURRENT_BRANCH, CREATE_BRANCH_OPERATION,
    CREATE_TAG_OPERATION, NewTag, REF_WRITES_FROM_MINOR, RefWritePreview,
    close_branch_confirm_and_execute, close_branch_preview, create_branch_confirm_and_execute,
    create_branch_preview, create_tag_confirm_and_execute, create_tag_preview,
    ref_writes_unavailable,
};
pub use refusal_history::{RefusalHistory, RefusalRecord};
pub use seal::{
    SEAL_CONSENT_COPY, SEAL_OPERATION, SealPreviewOutcome, seal_confirm_and_execute, seal_preview,
};
pub use session::{REMEMBERED_REF_IS_GONE, Restored, restore};
pub use switch::{
    ALREADY_THE_CURRENT_BRANCH, INCOMPLETE_PLAN, NO_FILE_PLAN, SWITCH_FROM_MINOR, SWITCH_OPERATION,
    SwitchPreview, WORK_IS_QUEUED_FOR_THIS_BRANCH, switch_confirm_and_execute, switch_preview,
    switch_unavailable,
};
pub use trust::{
    HOW_TO_ADOPT, NONE_ADOPTED, OWN_KEY_UNKNOWN, Sealer, SealerTrust, SealerTrustRead,
    TRUST_FROM_MINOR, TrustRow, TrustView, WHAT_TRUST_MEANS, WHERE_TO_GET_THE_KEY, sealers,
    trust_unavailable,
};
pub use verify::{
    Checked, ConditionRow, JSON_OMITS, StageRow, VERIFY_FROM_MINOR, VerifyView, checked, verify,
};
