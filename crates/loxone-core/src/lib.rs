pub mod advanced_blocks_config;
pub mod block_schema;
pub mod discovery;
pub mod efm_config;
pub mod export;
pub mod factory;
pub mod light_config;
pub mod linter;
pub mod loxcc;
pub mod merge;
pub mod miniserver;
pub mod model;
pub mod object_config;
pub mod parser;
pub mod periphery_provisioning;
pub mod project_admin;
pub mod room_controller_config;
pub mod simulator;
pub mod state_alarm_config;
pub mod writer;
pub mod xref;

pub use advanced_blocks_config::*;
pub use block_schema::all_block_schemas;
pub use efm_config::{
    collect_efm_candidate_objects, resolve_efm_object_label, EfmConfig, EfmNode, EfmNodeType,
    EfmObjectCandidate, StatInterval,
};
pub use export::{
    build_project_bom, collect_terminal_schedule, export_component_bom_csv,
    export_terminal_schedule_csv, page_to_svg, project_documentation_html, technical_report_html,
    BomDeviceEntry, BomPartSummary, BomReport, TerminalScheduleEntry,
};
pub use light_config::{
    collect_operating_modes, decode_q_to_pct, encode_pct_to_q, AutomationAction, HclConfig,
    HclMode, LightAutomationRule, LightCircuit, LightCircuitType, LightControllerConfig,
    LightScene,
};
pub use linter::{
    export_project_analysis_csv, lint_project, AnalysisCategory, DiagnosticSeverity,
    ProjectDiagnostic,
};
pub use merge::{
    check_server_newer_version, compare_projects, extract_prog_archive_timestamp,
    merge_remote_project_server_priority, MergeSummary, ProjectDiffEntry, ProjectDiffReport,
    ServerDiffSummary, ServerVersionConflict,
};
pub use object_config::{
    build_object_config_edits, build_object_config_rows, format_room_type,
    resolve_function_block_object_type, resolve_periphery_object_type, supports_object_config,
    ObjectConfigRow, ObjectConfigTargetKind,
};
pub use parser::parse_raw_dom;
pub use periphery_provisioning::*;
pub use project_admin::{
    build_add_todo_edit, build_autopilot_rule_edits, build_convert_miniserver_model_edit,
    build_instantiate_macro_edits, build_remove_multiplier_unit_edit,
    build_remove_virtual_io_node_edit, build_resolve_todo_edit, build_upsert_multiplier_unit_edit,
    build_upsert_virtual_out_cmd_edit, build_upsert_virtual_out_edit, builtin_schematic_macros,
    collect_autopilot_rules, collect_multiplier_units, collect_project_todos,
    collect_virtual_out_endpoints, create_new_project, detect_miniserver_model,
    extract_macro_from_page, instantiate_multiplier_unit_project, is_multiplier_project,
    AutopilotRuleInfo, AutopilotRuleSpec, MiniserverModel, MultiplierUnitEntry,
    NewProjectRoomSpec, NewProjectSpec, ProjectTodoItem, SchematicMacroBlock,
    SchematicMacroTemplate, SchematicMacroWire, VirtualOutCommandEntry, VirtualOutEndpoint,
};
pub use room_controller_config::{
    collect_energy_source_candidates, collect_user_operating_modes_for_timer,
    supports_room_controller_config, EnergySourceCandidate, EnergySourceGroup,
    RoomControllerConfig, RoomScheduleConfig, RoomTempMode, TimerEntry, TimerModeRow,
};
pub use simulator::*;
pub use state_alarm_config::*;
pub use writer::{
    apply_edit, apply_edit_with_inverse, build_align_blocks_edits, build_page_auto_layout_edits,
    clone_blocks_for_paste, create_cross_page_link_edits, serialize_raw_node, serialize_raw_nodes,
    BlockAlignMode, ProjectEdit,
};
