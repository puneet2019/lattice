// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::Emitter;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

fn main() {
    // Check for --mcp-stdio flag before starting Tauri.
    // In MCP stdio mode, we skip the GUI entirely and run a headless
    // JSON-RPC server over stdin/stdout for Claude Desktop/Code.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-stdio") {
        run_mcp_stdio();
        return;
    }
    if args.iter().any(|a| a == "--mcp-http") {
        let port = parse_port_arg(&args);
        run_mcp_http(port);
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            // Build the macOS menu bar.
            let file_new = MenuItemBuilder::with_id("file_new", "New")
                .accelerator("CmdOrCtrl+N")
                .build(app)?;
            let file_open = MenuItemBuilder::with_id("file_open", "Open...")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;
            let file_save = MenuItemBuilder::with_id("file_save", "Save")
                .accelerator("CmdOrCtrl+S")
                .build(app)?;
            let file_save_as = MenuItemBuilder::with_id("file_save_as", "Save As...")
                .accelerator("CmdOrCtrl+Shift+S")
                .build(app)?;
            let file_quit = MenuItemBuilder::with_id("file_quit", "Quit Lattice")
                .accelerator("CmdOrCtrl+Q")
                .build(app)?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&file_new)
                .item(&file_open)
                .separator()
                .item(&file_save)
                .item(&file_save_as)
                .separator()
                .item(&file_quit)
                .build()?;

            let edit_undo = MenuItemBuilder::with_id("edit_undo", "Undo")
                .accelerator("CmdOrCtrl+Z")
                .build(app)?;
            let edit_redo = MenuItemBuilder::with_id("edit_redo", "Redo")
                .accelerator("CmdOrCtrl+Shift+Z")
                .build(app)?;
            let edit_cut = MenuItemBuilder::with_id("edit_cut", "Cut")
                .accelerator("CmdOrCtrl+X")
                .build(app)?;
            let edit_copy = MenuItemBuilder::with_id("edit_copy", "Copy")
                .accelerator("CmdOrCtrl+C")
                .build(app)?;
            let edit_paste = MenuItemBuilder::with_id("edit_paste", "Paste")
                .accelerator("CmdOrCtrl+V")
                .build(app)?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&edit_undo)
                .item(&edit_redo)
                .separator()
                .item(&edit_cut)
                .item(&edit_copy)
                .item(&edit_paste)
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&file_menu)
                .item(&edit_menu)
                .build()?;

            app.set_menu(menu)?;

            // Handle menu events.
            app.on_menu_event(move |app_handle, event| {
                match event.id().as_ref() {
                    "file_quit" => {
                        app_handle.exit(0);
                    }
                    "file_new" | "file_open" | "file_save" | "file_save_as" | "edit_undo"
                    | "edit_redo" | "edit_cut" | "edit_copy" | "edit_paste" => {
                        // Emit the menu event to the frontend so it can handle it.
                        let _ = app_handle.emit("menu-event", event.id().as_ref());
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Cell commands
            commands::cell::get_cell,
            commands::cell::set_cell,
            commands::cell::get_range,
            // Sheet commands
            commands::sheet::list_sheets,
            commands::sheet::add_sheet,
            commands::sheet::rename_sheet,
            commands::sheet::delete_sheet,
            commands::sheet::set_active_sheet,
            // File commands
            commands::file::open_file,
            commands::file::save_file,
            commands::file::new_workbook,
            // Edit commands
            commands::edit::undo,
            commands::edit::redo,
            // Format commands
            commands::format::format_cells,
            // Data commands
            commands::data::find_in_sheet,
            commands::data::duplicate_sheet,
            commands::data::insert_rows,
            commands::data::delete_rows,
            commands::data::insert_cols,
            commands::data::delete_cols,
            // Chart commands
            commands::chart::create_chart,
            commands::chart::render_chart_svg,
            commands::chart::list_charts,
            commands::chart::delete_chart,
            // Autosave commands
            commands::autosave::get_autosave_config,
            commands::autosave::set_autosave_config,
            commands::autosave::trigger_autosave,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lattice");
}

/// Run the MCP server in headless stdio mode.
///
/// This bypasses Tauri entirely and runs a simple stdin/stdout loop
/// for use with Claude Desktop, Claude Code, or other MCP clients.
fn run_mcp_stdio() {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let mut server = lattice_mcp::McpServer::new_default();
        if let Err(e) = server.run_stdio().await {
            eprintln!("lattice: MCP server error: {}", e);
            std::process::exit(1);
        }
    });
}

/// Run the MCP server as a Streamable HTTP service.
///
/// This bypasses Tauri entirely and runs an HTTP server on the given port
/// for use with networked MCP clients.
fn run_mcp_http(port: u16) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let server = lattice_mcp::McpServer::new_default();
        if let Err(e) = server.run_http(port).await {
            eprintln!("lattice: MCP HTTP server error: {}", e);
            std::process::exit(1);
        }
    });
}

/// Parse `--port <N>` from CLI args, defaulting to 3141.
fn parse_port_arg(args: &[String]) -> u16 {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--port" {
            if let Some(port_str) = args.get(i + 1) {
                if let Ok(port) = port_str.parse::<u16>() {
                    return port;
                }
            }
        }
    }
    3141
}
