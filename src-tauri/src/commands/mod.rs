pub mod agent;
pub mod build;
pub mod models;
pub mod performance;
pub mod profiles;
pub mod progress;
pub mod server;
pub mod settings;
pub mod sources;
pub mod system;
pub mod updater;

/// Every command exposed to the frontend.
///
/// The frontend has no other way to reach the operating system: the shell plugin is not
/// installed, so this list is the complete privileged surface.
#[macro_export]
macro_rules! generated_command_handler {
    () => {
        tauri::generate_handler![
            $crate::commands::updater::get_app_update_status,
            $crate::commands::updater::check_app_update,
            $crate::commands::updater::download_app_update,
            $crate::commands::updater::install_app_update,
            $crate::commands::updater::defer_app_update,
            $crate::commands::agent::test_agent_connection,
            $crate::commands::settings::get_settings,
            $crate::commands::settings::update_settings,
            $crate::commands::settings::reset_settings,
            $crate::commands::system::get_app_info,
            $crate::commands::system::get_hardware_snapshot,
            $crate::commands::system::get_git_version,
            $crate::commands::system::reveal_path,
            $crate::commands::sources::list_sources,
            $crate::commands::sources::add_existing_source,
            $crate::commands::sources::clone_source,
            $crate::commands::sources::remove_source,
            $crate::commands::sources::get_source_status,
            $crate::commands::sources::fetch_source,
            $crate::commands::sources::update_source,
            $crate::commands::sources::list_source_refs,
            $crate::commands::sources::switch_source_ref,
            $crate::commands::sources::list_source_remotes,
            $crate::commands::sources::add_source_remote,
            $crate::commands::sources::remove_source_remote,
            $crate::commands::sources::discover_default_branch,
            $crate::commands::build::detect_toolchain,
            $crate::commands::build::build_runtime,
            $crate::commands::build::cancel_build,
            $crate::commands::build::is_build_running,
            $crate::commands::build::list_runtimes,
            $crate::commands::build::get_runtime_capabilities,
            $crate::commands::build::inspect_runtime_capabilities,
            $crate::commands::build::delete_runtime,
            $crate::commands::models::scan_models,
            $crate::commands::models::set_model_projector,
            $crate::commands::models::inspect_hugging_face_repository,
            $crate::commands::models::download_hugging_face_model,
            $crate::commands::models::cancel_model_download,
            $crate::commands::models::is_model_download_running,
            $crate::commands::performance::get_performance_plan,
            $crate::commands::performance::list_performance_benchmarks,
            $crate::commands::performance::run_performance_benchmark,
            $crate::commands::performance::run_performance_sweep,
            $crate::commands::performance::cancel_performance_sweep,
            $crate::commands::performance::is_performance_sweep_running,
            $crate::commands::profiles::list_profiles,
            $crate::commands::profiles::create_profile,
            $crate::commands::profiles::update_profile,
            $crate::commands::profiles::delete_profile,
            $crate::commands::profiles::preview_profile_command,
            $crate::commands::server::get_server_status,
            $crate::commands::server::get_server_logs,
            $crate::commands::server::subscribe_server_events,
            $crate::commands::server::unsubscribe_server_events,
            $crate::commands::server::clear_server_logs,
            $crate::commands::server::start_server,
            $crate::commands::server::stop_server,
            $crate::commands::server::restart_server,
        ]
    };
}
