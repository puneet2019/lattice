// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::{Emitter, Manager};
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
            let file_export_csv = MenuItemBuilder::with_id("file_export_csv", "Download as CSV")
                .build(app)?;
            let file_export_tsv = MenuItemBuilder::with_id("file_export_tsv", "Download as TSV")
                .build(app)?;
            let file_export_pdf = MenuItemBuilder::with_id("file_export_pdf", "Download as PDF")
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
                .item(&file_export_csv)
                .item(&file_export_tsv)
                .item(&file_export_pdf)
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

            // -- View menu --------------------------------------------------
            let view_freeze_1row =
                MenuItemBuilder::with_id("view_freeze_1row", "1 Row").build(app)?;
            let view_freeze_2rows =
                MenuItemBuilder::with_id("view_freeze_2rows", "2 Rows").build(app)?;
            let view_freeze_1col =
                MenuItemBuilder::with_id("view_freeze_1col", "1 Column").build(app)?;
            let view_freeze_2cols =
                MenuItemBuilder::with_id("view_freeze_2cols", "2 Columns").build(app)?;
            let view_freeze_none =
                MenuItemBuilder::with_id("view_freeze_none", "No Freeze").build(app)?;

            let freeze_submenu = SubmenuBuilder::new(app, "Freeze")
                .item(&view_freeze_1row)
                .item(&view_freeze_2rows)
                .item(&view_freeze_1col)
                .item(&view_freeze_2cols)
                .separator()
                .item(&view_freeze_none)
                .build()?;

            let view_show_formulas =
                MenuItemBuilder::with_id("view_show_formulas", "Show Formulas")
                    .accelerator("Ctrl+`")
                    .build(app)?;
            let view_toggle_gridlines =
                MenuItemBuilder::with_id("view_toggle_gridlines", "Show Gridlines").build(app)?;
            let view_zoom_in = MenuItemBuilder::with_id("view_zoom_in", "Zoom In")
                .accelerator("CmdOrCtrl+=")
                .build(app)?;
            let view_zoom_out = MenuItemBuilder::with_id("view_zoom_out", "Zoom Out")
                .accelerator("CmdOrCtrl+-")
                .build(app)?;
            let view_zoom_reset = MenuItemBuilder::with_id("view_zoom_reset", "Zoom Reset")
                .accelerator("CmdOrCtrl+0")
                .build(app)?;
            let view_fullscreen =
                MenuItemBuilder::with_id("view_fullscreen", "Full Screen").build(app)?;

            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&freeze_submenu)
                .separator()
                .item(&view_show_formulas)
                .item(&view_toggle_gridlines)
                .separator()
                .item(&view_zoom_in)
                .item(&view_zoom_out)
                .item(&view_zoom_reset)
                .separator()
                .item(&view_fullscreen)
                .build()?;

            // -- Insert menu ------------------------------------------------
            let insert_row_above =
                MenuItemBuilder::with_id("insert_row_above", "Row Above").build(app)?;
            let insert_row_below =
                MenuItemBuilder::with_id("insert_row_below", "Row Below").build(app)?;
            let insert_col_left =
                MenuItemBuilder::with_id("insert_col_left", "Column Left").build(app)?;
            let insert_col_right =
                MenuItemBuilder::with_id("insert_col_right", "Column Right").build(app)?;
            let insert_chart =
                MenuItemBuilder::with_id("insert_chart", "Chart").build(app)?;
            let insert_note =
                MenuItemBuilder::with_id("insert_note", "Comment/Note").build(app)?;
            let insert_checkbox =
                MenuItemBuilder::with_id("insert_checkbox", "Checkbox").build(app)?;
            let insert_named_range =
                MenuItemBuilder::with_id("insert_named_range", "Named Range").build(app)?;

            let insert_menu = SubmenuBuilder::new(app, "Insert")
                .item(&insert_row_above)
                .item(&insert_row_below)
                .separator()
                .item(&insert_col_left)
                .item(&insert_col_right)
                .separator()
                .item(&insert_chart)
                .item(&insert_note)
                .item(&insert_checkbox)
                .separator()
                .item(&insert_named_range)
                .build()?;

            // -- Format menu ------------------------------------------------
            let fmt_general =
                MenuItemBuilder::with_id("format_num_general", "General").build(app)?;
            let fmt_number =
                MenuItemBuilder::with_id("format_num_number", "Number").build(app)?;
            let fmt_currency =
                MenuItemBuilder::with_id("format_num_currency", "Currency").build(app)?;
            let fmt_percentage =
                MenuItemBuilder::with_id("format_num_percentage", "Percentage").build(app)?;
            let fmt_date =
                MenuItemBuilder::with_id("format_num_date", "Date").build(app)?;
            let fmt_time =
                MenuItemBuilder::with_id("format_num_time", "Time").build(app)?;
            let fmt_scientific =
                MenuItemBuilder::with_id("format_num_scientific", "Scientific").build(app)?;

            let number_submenu = SubmenuBuilder::new(app, "Number")
                .item(&fmt_general)
                .item(&fmt_number)
                .item(&fmt_currency)
                .item(&fmt_percentage)
                .item(&fmt_date)
                .item(&fmt_time)
                .item(&fmt_scientific)
                .build()?;

            let format_bold = MenuItemBuilder::with_id("format_bold", "Bold")
                .accelerator("CmdOrCtrl+B")
                .build(app)?;
            let format_italic = MenuItemBuilder::with_id("format_italic", "Italic")
                .accelerator("CmdOrCtrl+I")
                .build(app)?;
            let format_underline = MenuItemBuilder::with_id("format_underline", "Underline")
                .accelerator("CmdOrCtrl+U")
                .build(app)?;
            let format_strikethrough =
                MenuItemBuilder::with_id("format_strikethrough", "Strikethrough").build(app)?;

            let fmt_size_increase =
                MenuItemBuilder::with_id("format_size_increase", "Increase Font Size").build(app)?;
            let fmt_size_decrease =
                MenuItemBuilder::with_id("format_size_decrease", "Decrease Font Size").build(app)?;

            let font_size_submenu = SubmenuBuilder::new(app, "Font Size")
                .item(&fmt_size_increase)
                .item(&fmt_size_decrease)
                .build()?;

            let format_text_color =
                MenuItemBuilder::with_id("format_text_color", "Text Color...").build(app)?;
            let format_fill_color =
                MenuItemBuilder::with_id("format_fill_color", "Fill Color...").build(app)?;

            let fmt_align_left =
                MenuItemBuilder::with_id("format_align_left", "Left").build(app)?;
            let fmt_align_center =
                MenuItemBuilder::with_id("format_align_center", "Center").build(app)?;
            let fmt_align_right =
                MenuItemBuilder::with_id("format_align_right", "Right").build(app)?;

            let alignment_submenu = SubmenuBuilder::new(app, "Alignment")
                .item(&fmt_align_left)
                .item(&fmt_align_center)
                .item(&fmt_align_right)
                .build()?;

            let format_merge =
                MenuItemBuilder::with_id("format_merge", "Merge Cells").build(app)?;
            let format_conditional =
                MenuItemBuilder::with_id("format_conditional", "Conditional Formatting...")
                    .build(app)?;
            let format_alternating =
                MenuItemBuilder::with_id("format_alternating", "Alternating Colors...")
                    .build(app)?;
            let format_clear = MenuItemBuilder::with_id("format_clear", "Clear Formatting")
                .accelerator("CmdOrCtrl+\\")
                .build(app)?;

            let format_menu = SubmenuBuilder::new(app, "Format")
                .item(&number_submenu)
                .separator()
                .item(&format_bold)
                .item(&format_italic)
                .item(&format_underline)
                .item(&format_strikethrough)
                .separator()
                .item(&font_size_submenu)
                .item(&format_text_color)
                .item(&format_fill_color)
                .separator()
                .item(&alignment_submenu)
                .item(&format_merge)
                .separator()
                .item(&format_conditional)
                .item(&format_alternating)
                .separator()
                .item(&format_clear)
                .build()?;

            // -- Data menu --------------------------------------------------
            let data_sort_az =
                MenuItemBuilder::with_id("data_sort_az", "Sort A \u{2192} Z").build(app)?;
            let data_sort_za =
                MenuItemBuilder::with_id("data_sort_za", "Sort Z \u{2192} A").build(app)?;
            let data_sort_custom =
                MenuItemBuilder::with_id("data_sort_custom", "Custom Sort...").build(app)?;

            let sort_submenu = SubmenuBuilder::new(app, "Sort Range")
                .item(&data_sort_az)
                .item(&data_sort_za)
                .separator()
                .item(&data_sort_custom)
                .build()?;

            let data_create_filter =
                MenuItemBuilder::with_id("data_create_filter", "Create Filter").build(app)?;
            let data_named_ranges =
                MenuItemBuilder::with_id("data_named_ranges", "Named Ranges...").build(app)?;
            let data_validation =
                MenuItemBuilder::with_id("data_validation", "Data Validation...").build(app)?;
            let data_remove_duplicates =
                MenuItemBuilder::with_id("data_remove_duplicates", "Remove Duplicates...")
                    .build(app)?;
            let data_text_to_columns =
                MenuItemBuilder::with_id("data_text_to_columns", "Text to Columns...")
                    .build(app)?;
            let data_pivot_table =
                MenuItemBuilder::with_id("data_pivot_table", "Pivot Table...").build(app)?;

            let data_menu = SubmenuBuilder::new(app, "Data")
                .item(&sort_submenu)
                .separator()
                .item(&data_create_filter)
                .item(&data_named_ranges)
                .item(&data_validation)
                .separator()
                .item(&data_remove_duplicates)
                .item(&data_text_to_columns)
                .separator()
                .item(&data_pivot_table)
                .build()?;

            // -- Assemble the full menu bar ---------------------------------
            let menu = MenuBuilder::new(app)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .item(&insert_menu)
                .item(&format_menu)
                .item(&data_menu)
                .build()?;

            app.set_menu(menu)?;

            // Handle menu events.
            app.on_menu_event(move |app_handle, event| {
                match event.id().as_ref() {
                    "file_quit" => {
                        app_handle.exit(0);
                    }
                    // Fullscreen is handled entirely on the backend side.
                    "view_fullscreen" => {
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let is_full = win.is_fullscreen().unwrap_or(false);
                            let _ = win.set_fullscreen(!is_full);
                        }
                    }
                    // Everything else is forwarded to the frontend.
                    id => {
                        let _ = app_handle.emit("menu-event", id);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Cell commands
            commands::cell::get_cell,
            commands::cell::set_cell,
            commands::cell::get_range,
            // Comment commands
            commands::cell::set_comment,
            commands::cell::get_comment,
            commands::cell::remove_comment,
            // Protection commands
            commands::cell::is_cell_protected,
            commands::cell::get_sheet_protection,
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
            commands::format::merge_cells,
            commands::format::unmerge_cells,
            commands::format::get_merged_regions,
            commands::format::set_banded_rows,
            commands::format::get_banded_rows,
            // Conditional format commands
            commands::conditional_format::add_conditional_format,
            commands::conditional_format::list_conditional_formats,
            commands::conditional_format::remove_conditional_format,
            // Data commands
            commands::data::find_in_sheet,
            commands::data::duplicate_sheet,
            commands::data::insert_rows,
            commands::data::delete_rows,
            commands::data::insert_cols,
            commands::data::delete_cols,
            commands::data::hide_rows,
            commands::data::unhide_rows,
            commands::data::hide_cols,
            commands::data::unhide_cols,
            commands::data::get_hidden_cols,
            commands::data::sort_range,
            commands::data::add_named_range,
            commands::data::list_named_ranges,
            commands::data::remove_named_range,
            commands::data::resolve_named_range,
            commands::data::add_row_group,
            commands::data::remove_row_group,
            commands::data::toggle_row_group,
            commands::data::get_row_groups,
            // Chart commands
            commands::chart::create_chart,
            commands::chart::render_chart_svg,
            commands::chart::list_charts,
            commands::chart::delete_chart,
            // Autosave commands
            commands::autosave::get_autosave_config,
            commands::autosave::set_autosave_config,
            commands::autosave::trigger_autosave,
            // Cloud commands
            commands::cloud::list_cloud_providers,
            commands::cloud::list_cloud_files,
            commands::cloud::open_cloud_file,
            commands::cloud::save_to_cloud,
            // Validation commands
            commands::validation::set_validation,
            commands::validation::get_validation,
            commands::validation::remove_validation,
            commands::validation::list_validations,
            // Filter commands
            commands::filter::set_auto_filter,
            commands::filter::get_column_values,
            commands::filter::apply_column_filter,
            commands::filter::clear_filter,
            commands::filter::get_filter_info,
            commands::filter::get_hidden_rows,
            // Export / import commands
            commands::export::export_csv,
            commands::export::export_tsv,
            commands::export::export_html,
            commands::export::open_csv,
            commands::export::open_tsv,
            commands::export::get_recent_files,
            commands::export::add_recent_file,
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
