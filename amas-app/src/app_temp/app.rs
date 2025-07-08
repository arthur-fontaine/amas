#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    io::{BufReader, IsTerminal, Write},
    ops::Range,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::AtomicU64,
        mpsc::{SyncSender, sync_channel},
    },
};

use anyhow::{Result, anyhow};
use floem::{
    IntoView, View,
    event::{Event, EventListener, EventPropagation},
    ext_event::{create_ext_action, create_signal_from_channel},
    peniko::kurbo::{Point, Rect, Size},
    reactive::{
        ReadSignal, RwSignal, Scope, SignalGet, SignalUpdate, SignalWith,
        create_effect, create_rw_signal, provide_context,
    },
    style::{
        AlignItems, CursorStyle, Display, FlexDirection, JustifyContent, Position,
        Style,
    },
    text::Weight,
    unit::PxPctAuto,
    views::{
        Decorators, VirtualVector, container, dyn_stack, empty, rich_text,
        scroll::{PropagatePointerWheel, scroll},
        stack, tab, text, virtual_stack,
    },
    window::{WindowConfig, WindowId},
};
use lapce_core::{
    directory::Directory,
    syntax::{Syntax, highlight::reset_highlight_configs},
};
use lapce_rpc::{
    RpcMessage,
    core::{CoreMessage, CoreNotification},
    file::{LineCol, PathObject},
};
use lsp_types::CompletionItemKind;
use serde::{Deserialize, Serialize};

use lapce_app::db::LapceDb;
use lapce_app::{
    about, alert,
    app::AppCommand,
    code_action::CodeActionStatus,
    command::InternalCommand,
    config::{LapceConfig, color::LapceColor},
    editor::{
        diff::diff_show_more_section_view,
        location::{EditorLocation, EditorPosition},
        view::editor_container_view,
    },
    editor_tab::{EditorTabChild, EditorTabData},
    focus_text::focus_text,
    id::{EditorTabId, SplitId},
    keymap::keymap_view,
    listener::Listener,
    main_split::{SplitContent, SplitData, SplitDirection, SplitMoveDirection},
    markdown::MarkdownContent,
    panel::position::PanelContainerPosition,
    plugin::{PluginData, plugin_info_view},
    settings::{settings_view, theme_color_settings_view},
    text_input::TextInputBuilder,
    tracing::*,
    update::ReleaseInfo,
    window::{TabsInfo, WindowData, WindowInfo},
    window_tab::{Focus, WindowTabData},
    workspace::LapceWorkspace,
};

mod grammars;
mod logging;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub windows: Vec<WindowInfo>,
}

#[derive(Clone)]
pub struct AppData {
    pub windows: RwSignal<im::HashMap<WindowId, WindowData>>,
    pub active_window: RwSignal<WindowId>,
    pub window_scale: RwSignal<f64>,
    pub app_command: Listener<AppCommand>,
    pub app_terminated: RwSignal<bool>,
    /// The latest release information
    pub latest_release: RwSignal<Arc<Option<ReleaseInfo>>>,
    pub config: RwSignal<Arc<LapceConfig>>,
    /// Paths to extra plugins to load
    pub plugin_paths: Arc<Vec<PathBuf>>,
}

impl AppData {
    pub fn reload_config(&self) {
        let config =
            LapceConfig::load(&LapceWorkspace::default(), &[], &self.plugin_paths);
        self.config.set(Arc::new(config));
        let windows = self.windows.get_untracked();
        for (_, window) in windows {
            window.reload_config();
        }
    }

    pub fn active_window_tab(&self) -> Option<Rc<WindowTabData>> {
        if let Some(window) = self.active_window() {
            return window.active_window_tab();
        }
        None
    }

    fn active_window(&self) -> Option<WindowData> {
        let windows = self.windows.get_untracked();
        let active_window = self.active_window.get_untracked();
        windows
            .get(&active_window)
            .cloned()
            .or_else(|| windows.iter().next().map(|(_, window)| window.clone()))
    }

    fn default_window_config(&self) -> WindowConfig {
        WindowConfig::default()
            .apply_default_theme(false)
            .title("Amas")
    }

    fn create_windows(
        &self,
        db: Arc<LapceDb>,
        paths: Vec<PathObject>,
    ) -> floem::Application {
        let mut app = floem::Application::new();

        let mut info = db.get_window().unwrap_or_else(|_| WindowInfo {
            size: Size::new(800.0, 600.0),
            pos: Point::ZERO,
            maximised: false,
            tabs: TabsInfo {
                active_tab: 0,
                workspaces: vec![LapceWorkspace::default()],
            },
        });
        info.tabs = TabsInfo {
            active_tab: 0,
            workspaces: vec![LapceWorkspace::default()],
        };
        let config = self
            .default_window_config()
            .size(info.size)
            .position(info.pos);
        let app_data = self.clone();
        app = app.window(
            {
                let app_data = app_data.clone();
                move |window_id| {
                    app_data
                        .clone()
                        .into_view(window_id, db.clone(), paths.clone())
                }
            },
            Some(config),
        );

        app
    }

    pub fn into_view(
        &self,
        window_id: WindowId,
        db: Arc<LapceDb>,
        paths: Vec<PathObject>,
    ) -> impl floem::IntoView + use<> {
        // Split user input into known existing directors and
        // file paths that exist or not
        let (_, files): (Vec<&PathObject>, Vec<&PathObject>) =
            paths.iter().partition(|p| p.is_dir);

        let files: Vec<PathObject> = files.into_iter().cloned().collect();
        let mut files = if files.is_empty() { None } else { Some(files) };

        let mut info = db.get_window().unwrap_or_else(|_| WindowInfo {
            size: Size::new(800.0, 600.0),
            pos: Point::ZERO,
            maximised: false,
            tabs: TabsInfo {
                active_tab: 0,
                workspaces: vec![LapceWorkspace::default()],
            },
        });
        info.tabs = TabsInfo {
            active_tab: 0,
            workspaces: vec![LapceWorkspace::default()],
        };

        let app_data = self.clone();
        app_data.app_view(window_id, info, files.take().unwrap_or_default())
    }

    fn app_view(
        &self,
        window_id: WindowId,
        info: WindowInfo,
        files: Vec<PathObject>,
    ) -> impl View + use<> {
        let app_view_id = create_rw_signal(floem::ViewId::new());
        let window_data = WindowData::new(
            window_id,
            app_view_id,
            info,
            self.window_scale,
            self.latest_release.read_only(),
            self.plugin_paths.clone(),
            self.app_command,
        );

        {
            let cur_window_tab = window_data.active.get_untracked();
            let (_, window_tab) =
                &window_data.window_tabs.get_untracked()[cur_window_tab];
            for file in files {
                let position = file.linecol.map(|pos| {
                    EditorPosition::Position(lsp_types::Position {
                        line: pos.line.saturating_sub(1) as u32,
                        character: pos.column.saturating_sub(1) as u32,
                    })
                });

                window_tab.run_internal_command(InternalCommand::GoToLocation {
                    location: EditorLocation {
                        path: file.path.clone(),
                        position,
                        scroll_offset: None,
                        // Create a new editor for the file, so we don't change any current unconfirmed
                        // editor
                        ignore_unconfirmed: true,
                        same_editor_tab: false,
                    },
                });
            }
        }

        self.windows.update(|windows| {
            windows.insert(window_id, window_data.clone());
        });
        let window_size = window_data.common.size;
        let position = window_data.position;
        let window_scale = window_data.window_scale;
        let app_command = window_data.app_command;
        // The KeyDown and PointerDown event handlers both need ownership of a WindowData object.
        let key_down_window_data = window_data.clone();
        let view = window(window_data.clone()).style(|s| s.flex_col().size_full());
        let view_id = view.id();
        app_view_id.set(view_id);

        view_id.request_focus();

        view.window_scale(move || window_scale.get())
            .keyboard_navigable()
            .on_event(EventListener::KeyDown, move |event| {
                if let Event::KeyDown(key_event) = event {
                    if key_down_window_data.key_down(key_event) {
                        view_id.request_focus();
                    }
                    EventPropagation::Stop
                } else {
                    EventPropagation::Continue
                }
            })
            .on_event(EventListener::PointerDown, {
                let window_data = window_data.clone();
                move |event| {
                    if let Event::PointerDown(pointer_event) = event {
                        window_data.key_down(pointer_event);
                        EventPropagation::Stop
                    } else {
                        EventPropagation::Continue
                    }
                }
            })
            .on_event_stop(EventListener::WindowResized, move |event| {
                if let Event::WindowResized(size) = event {
                    window_size.set(*size);
                }
            })
            .on_event_stop(EventListener::WindowMoved, move |event| {
                if let Event::WindowMoved(point) = event {
                    position.set(*point);
                }
            })
            .on_event_stop(EventListener::WindowGotFocus, move |_| {
                app_command.send(AppCommand::WindowGotFocus(window_id));
            })
            .on_event_stop(EventListener::WindowClosed, move |_| {
                app_command.send(AppCommand::WindowClosed(window_id));
            })
            .on_event_stop(EventListener::DroppedFile, move |event: &Event| {
                if let Event::DroppedFile(file) = event {
                    if file.path.is_dir() {
                        app_command.send(AppCommand::NewWindow {
                            folder: Some(file.path.clone()),
                        });
                    } else if let Some(win_tab_data) =
                        window_data.active_window_tab()
                    {
                        win_tab_data.common.internal_command.send(
                            InternalCommand::GoToLocation {
                                location: EditorLocation {
                                    path: file.path.clone(),
                                    position: None,
                                    scroll_offset: None,
                                    ignore_unconfirmed: false,
                                    same_editor_tab: false,
                                },
                            },
                        )
                    }
                }
            })
            .debug_name("App View")
    }
}

fn editor_tab_content(
    window_tab_data: Rc<WindowTabData>,
    plugin: PluginData,
    active_editor_tab: ReadSignal<Option<EditorTabId>>,
    editor_tab: RwSignal<EditorTabData>,
) -> impl View {
    let main_split = window_tab_data.main_split.clone();
    let common = main_split.common.clone();
    let workspace = common.workspace.clone();
    let editors = main_split.editors;
    let diff_editors = main_split.diff_editors;
    let config = common.config;
    let focus = common.focus;
    let items = move || {
        editor_tab
            .get()
            .children
            .into_iter()
            .map(|(_, _, child)| child)
    };
    let key = |child: &EditorTabChild| child.id();
    let view_fn = move |child| {
        let common = common.clone();
        let child = match child {
            EditorTabChild::Editor(editor_id) => {
                if let Some(editor_data) = editors.editor_untracked(editor_id) {
                    let editor_scope = editor_data.scope;
                    let editor_tab_id = editor_data.editor_tab_id;
                    let is_active = move |tracked: bool| {
                        editor_scope.track();
                        let focus = if tracked {
                            focus.get()
                        } else {
                            focus.get_untracked()
                        };
                        if let Focus::Workbench = focus {
                            let active_editor_tab = if tracked {
                                active_editor_tab.get()
                            } else {
                                active_editor_tab.get_untracked()
                            };
                            let editor_tab = if tracked {
                                editor_tab_id.get()
                            } else {
                                editor_tab_id.get_untracked()
                            };
                            editor_tab.is_some() && editor_tab == active_editor_tab
                        } else {
                            false
                        }
                    };
                    let editor_data = create_rw_signal(editor_data);
                    editor_container_view(
                        window_tab_data.clone(),
                        workspace.clone(),
                        is_active,
                        editor_data,
                    )
                    .into_any()
                } else {
                    text("empty editor").into_any()
                }
            }
            EditorTabChild::DiffEditor(diff_editor_id) => {
                let diff_editor_data = diff_editors.with_untracked(|diff_editors| {
                    diff_editors.get(&diff_editor_id).cloned()
                });
                if let Some(diff_editor_data) = diff_editor_data {
                    let focus_right = diff_editor_data.focus_right;
                    let diff_editor_tab_id = diff_editor_data.editor_tab_id;
                    let diff_editor_scope = diff_editor_data.scope;
                    let is_active = move |tracked: bool| {
                        let focus = if tracked {
                            focus.get()
                        } else {
                            focus.get_untracked()
                        };
                        if let Focus::Workbench = focus {
                            let active_editor_tab = if tracked {
                                active_editor_tab.get()
                            } else {
                                active_editor_tab.get_untracked()
                            };
                            let diff_editor_tab_id = if tracked {
                                diff_editor_tab_id.get()
                            } else {
                                diff_editor_tab_id.get_untracked()
                            };
                            Some(diff_editor_tab_id) == active_editor_tab
                        } else {
                            false
                        }
                    };
                    let left_viewport = diff_editor_data.left.viewport();
                    let left_scroll_to = diff_editor_data.left.scroll_to();
                    let right_viewport = diff_editor_data.right.viewport();
                    let right_scroll_to = diff_editor_data.right.scroll_to();
                    create_effect(move |_| {
                        let left_viewport = left_viewport.get();
                        if right_viewport.get_untracked() != left_viewport {
                            right_scroll_to
                                .set(Some(left_viewport.origin().to_vec2()));
                        }
                    });
                    create_effect(move |_| {
                        let right_viewport = right_viewport.get();
                        if left_viewport.get_untracked() != right_viewport {
                            left_scroll_to
                                .set(Some(right_viewport.origin().to_vec2()));
                        }
                    });
                    let left_editor =
                        create_rw_signal(diff_editor_data.left.clone());
                    let right_editor =
                        create_rw_signal(diff_editor_data.right.clone());
                    stack((
                        container(
                            editor_container_view(
                                window_tab_data.clone(),
                                workspace.clone(),
                                move |track| {
                                    is_active(track)
                                        && if track {
                                            !focus_right.get()
                                        } else {
                                            !focus_right.get_untracked()
                                        }
                                },
                                left_editor,
                            )
                            .debug_name("Left Editor"),
                        )
                        .on_event_cont(EventListener::PointerDown, move |_| {
                            focus_right.set(false);
                        })
                        .style(move |s| {
                            s.height_full()
                                .flex_grow(1.0)
                                .flex_basis(0.0)
                                .border_right(1.0)
                                .border_color(
                                    config.get().color(LapceColor::LAPCE_BORDER),
                                )
                        }),
                        container(
                            editor_container_view(
                                window_tab_data.clone(),
                                workspace.clone(),
                                move |track| {
                                    is_active(track)
                                        && if track {
                                            focus_right.get()
                                        } else {
                                            focus_right.get_untracked()
                                        }
                                },
                                right_editor,
                            )
                            .debug_name("Right Editor"),
                        )
                        .on_event_cont(EventListener::PointerDown, move |_| {
                            focus_right.set(true);
                        })
                        .style(|s| s.height_full().flex_grow(1.0).flex_basis(0.0)),
                        diff_show_more_section_view(
                            &diff_editor_data.left,
                            &diff_editor_data.right,
                        ),
                    ))
                    .style(|s: Style| s.size_full())
                    .on_cleanup(move || {
                        diff_editor_scope.dispose();
                    })
                    .into_any()
                } else {
                    text("empty diff editor").into_any()
                }
            }
            EditorTabChild::Settings(_) => {
                settings_view(plugin.installed, editors, common).into_any()
            }
            EditorTabChild::ThemeColorSettings(_) => {
                theme_color_settings_view(editors, common).into_any()
            }
            EditorTabChild::Keymap(_) => keymap_view(editors, common).into_any(),
            EditorTabChild::Volt(_, id) => {
                plugin_info_view(plugin.clone(), id).into_any()
            }
        };
        child.style(|s| s.size_full())
    };
    let active = move || editor_tab.with(|t| t.active);

    tab(active, items, key, view_fn)
        .style(|s| s.size_full())
        .debug_name("Editor Tab Content")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragOverPosition {
    Top,
    Bottom,
    Left,
    Right,
    Middle,
}

fn editor_tab(
    window_tab_data: Rc<WindowTabData>,
    plugin: PluginData,
    active_editor_tab: ReadSignal<Option<EditorTabId>>,
    editor_tab: RwSignal<EditorTabData>,
    dragging: RwSignal<Option<(RwSignal<usize>, EditorTabId)>>,
) -> impl View {
    let main_split = window_tab_data.main_split.clone();
    let common = main_split.common.clone();
    let editor_tabs = main_split.editor_tabs;
    let editor_tab_id =
        editor_tab.with_untracked(|editor_tab| editor_tab.editor_tab_id);
    let config = common.config;
    let focus = common.focus;
    let internal_command = main_split.common.internal_command;
    let tab_size = create_rw_signal(Size::ZERO);
    let drag_over: RwSignal<Option<DragOverPosition>> = create_rw_signal(None);
    stack((stack((
        editor_tab_content(
            window_tab_data.clone(),
            plugin.clone(),
            active_editor_tab,
            editor_tab,
        ),
        empty()
            .style(move |s| {
                let pos = drag_over.get();
                let width = match pos {
                    Some(pos) => match pos {
                        DragOverPosition::Top => 100.0,
                        DragOverPosition::Bottom => 100.0,
                        DragOverPosition::Left => 50.0,
                        DragOverPosition::Right => 50.0,
                        DragOverPosition::Middle => 100.0,
                    },
                    None => 100.0,
                };
                let height = match pos {
                    Some(pos) => match pos {
                        DragOverPosition::Top => 50.0,
                        DragOverPosition::Bottom => 50.0,
                        DragOverPosition::Left => 100.0,
                        DragOverPosition::Right => 100.0,
                        DragOverPosition::Middle => 100.0,
                    },
                    None => 100.0,
                };
                let size = tab_size.get_untracked();
                let margin_left = match pos {
                    Some(pos) => match pos {
                        DragOverPosition::Top => 0.0,
                        DragOverPosition::Bottom => 0.0,
                        DragOverPosition::Left => 0.0,
                        DragOverPosition::Right => size.width / 2.0,
                        DragOverPosition::Middle => 0.0,
                    },
                    None => 0.0,
                };
                let margin_top = match pos {
                    Some(pos) => match pos {
                        DragOverPosition::Top => 0.0,
                        DragOverPosition::Bottom => size.height / 2.0,
                        DragOverPosition::Left => 0.0,
                        DragOverPosition::Right => 0.0,
                        DragOverPosition::Middle => 0.0,
                    },
                    None => 0.0,
                };
                s.absolute()
                    .size_pct(width, height)
                    .margin_top(margin_top as f32)
                    .margin_left(margin_left as f32)
                    .apply_if(pos.is_none(), |s| s.hide())
                    .background(
                        config.get().color(LapceColor::EDITOR_DRAG_DROP_BACKGROUND),
                    )
            })
            .debug_name("Drag Over Handle"),
        empty()
            .on_event_stop(EventListener::DragOver, move |event| {
                if dragging.with_untracked(|dragging| dragging.is_some()) {
                    if let Event::PointerMove(pointer_event) = event {
                        let size = tab_size.get_untracked();
                        let pos = pointer_event.pos;
                        let new_drag_over = if pos.x < size.width / 4.0 {
                            DragOverPosition::Left
                        } else if pos.x > size.width * 3.0 / 4.0 {
                            DragOverPosition::Right
                        } else if pos.y < size.height / 4.0 {
                            DragOverPosition::Top
                        } else if pos.y > size.height * 3.0 / 4.0 {
                            DragOverPosition::Bottom
                        } else {
                            DragOverPosition::Middle
                        };
                        if drag_over.get_untracked() != Some(new_drag_over) {
                            drag_over.set(Some(new_drag_over));
                        }
                    }
                }
            })
            .on_event_stop(EventListener::DragLeave, move |_| {
                drag_over.set(None);
            })
            .on_event(EventListener::Drop, move |_| {
                if let Some((from_index, from_editor_tab_id)) =
                    dragging.get_untracked()
                {
                    if let Some(pos) = drag_over.get_untracked() {
                        match pos {
                            DragOverPosition::Top => {
                                main_split.move_editor_tab_child_to_new_split(
                                    from_editor_tab_id,
                                    from_index.get_untracked(),
                                    editor_tab_id,
                                    SplitMoveDirection::Up,
                                );
                            }
                            DragOverPosition::Bottom => {
                                main_split.move_editor_tab_child_to_new_split(
                                    from_editor_tab_id,
                                    from_index.get_untracked(),
                                    editor_tab_id,
                                    SplitMoveDirection::Down,
                                );
                            }
                            DragOverPosition::Left => {
                                main_split.move_editor_tab_child_to_new_split(
                                    from_editor_tab_id,
                                    from_index.get_untracked(),
                                    editor_tab_id,
                                    SplitMoveDirection::Left,
                                );
                            }
                            DragOverPosition::Right => {
                                main_split.move_editor_tab_child_to_new_split(
                                    from_editor_tab_id,
                                    from_index.get_untracked(),
                                    editor_tab_id,
                                    SplitMoveDirection::Right,
                                );
                            }
                            DragOverPosition::Middle => {
                                main_split.move_editor_tab_child(
                                    from_editor_tab_id,
                                    editor_tab_id,
                                    from_index.get_untracked(),
                                    editor_tab.with_untracked(|editor_tab| {
                                        editor_tab.active + 1
                                    }),
                                );
                            }
                        }
                    }
                    drag_over.set(None);
                    EventPropagation::Stop
                } else {
                    EventPropagation::Continue
                }
            })
            .on_resize(move |rect| {
                tab_size.set(rect.size());
            })
            .style(move |s| {
                s.absolute()
                    .size_full()
                    .apply_if(dragging.get().is_none(), |s| s.pointer_events_none())
            }),
    ))
    .debug_name("Editor Content and Drag Over")
    .style(|s| s.size_full()),))
    .on_event_cont(EventListener::PointerDown, move |_| {
        if focus.get_untracked() != Focus::Workbench {
            focus.set(Focus::Workbench);
        }
        let editor_tab_id = editor_tab.with_untracked(|t| t.editor_tab_id);
        internal_command.send(InternalCommand::FocusEditorTab { editor_tab_id });
    })
    .on_cleanup(move || {
        if editor_tabs
            .with_untracked(|editor_tabs| editor_tabs.contains_key(&editor_tab_id))
        {
            return;
        }
        editor_tab
            .with_untracked(|editor_tab| editor_tab.scope)
            .dispose();
    })
    .style(|s| s.flex_col().size_full())
    .debug_name("Editor Tab (Content + Header)")
}

fn split_resize_border(
    splits: ReadSignal<im::HashMap<SplitId, RwSignal<SplitData>>>,
    editor_tabs: ReadSignal<im::HashMap<EditorTabId, RwSignal<EditorTabData>>>,
    split: ReadSignal<SplitData>,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    let content_rect = move |content: &SplitContent, tracked: bool| {
        if tracked {
            match content {
                SplitContent::EditorTab(editor_tab_id) => {
                    let editor_tab_data =
                        editor_tabs.with(|tabs| tabs.get(editor_tab_id).cloned());
                    if let Some(editor_tab_data) = editor_tab_data {
                        editor_tab_data.with(|editor_tab| editor_tab.layout_rect)
                    } else {
                        Rect::ZERO
                    }
                }
                SplitContent::Split(split_id) => {
                    if let Some(split) =
                        splits.with(|splits| splits.get(split_id).cloned())
                    {
                        split.with(|split| split.layout_rect)
                    } else {
                        Rect::ZERO
                    }
                }
            }
        } else {
            match content {
                SplitContent::EditorTab(editor_tab_id) => {
                    let editor_tab_data = editor_tabs
                        .with_untracked(|tabs| tabs.get(editor_tab_id).cloned());
                    if let Some(editor_tab_data) = editor_tab_data {
                        editor_tab_data
                            .with_untracked(|editor_tab| editor_tab.layout_rect)
                    } else {
                        Rect::ZERO
                    }
                }
                SplitContent::Split(split_id) => {
                    if let Some(split) =
                        splits.with_untracked(|splits| splits.get(split_id).cloned())
                    {
                        split.with_untracked(|split| split.layout_rect)
                    } else {
                        Rect::ZERO
                    }
                }
            }
        }
    };
    let direction = move |tracked: bool| {
        if tracked {
            split.with(|split| split.direction)
        } else {
            split.with_untracked(|split| split.direction)
        }
    };
    dyn_stack(
        move || {
            let data = split.get();
            data.children.into_iter().enumerate().skip(1)
        },
        |(index, (_, content))| (*index, content.id()),
        move |(index, (_, content))| {
            let drag_start: RwSignal<Option<Point>> = create_rw_signal(None);
            let view = empty();
            let view_id = view.id();
            view.on_event_stop(EventListener::PointerDown, move |event| {
                view_id.request_active();
                if let Event::PointerDown(pointer_event) = event {
                    drag_start.set(Some(pointer_event.pos));
                }
            })
            .on_event_stop(EventListener::PointerUp, move |_| {
                drag_start.set(None);
            })
            .on_event_stop(EventListener::PointerMove, move |event| {
                if let Event::PointerMove(pointer_event) = event {
                    if let Some(drag_start_point) = drag_start.get_untracked() {
                        let rects = split.with_untracked(|split| {
                            split
                                .children
                                .iter()
                                .map(|(_, c)| content_rect(c, false))
                                .collect::<Vec<Rect>>()
                        });
                        let direction = direction(false);
                        match direction {
                            SplitDirection::Vertical => {
                                let left = rects[index - 1].width();
                                let right = rects[index].width();
                                let shift = pointer_event.pos.x - drag_start_point.x;
                                let left = left + shift;
                                let right = right - shift;
                                let total_width =
                                    rects.iter().map(|r| r.width()).sum::<f64>();
                                split.with_untracked(|split| {
                                    for (i, (size, _)) in
                                        split.children.iter().enumerate()
                                    {
                                        if i == index - 1 {
                                            size.set(left / total_width);
                                        } else if i == index {
                                            size.set(right / total_width);
                                        } else {
                                            size.set(rects[i].width() / total_width);
                                        }
                                    }
                                })
                            }
                            SplitDirection::Horizontal => {
                                let up = rects[index - 1].height();
                                let down = rects[index].height();
                                let shift = pointer_event.pos.y - drag_start_point.y;
                                let up = up + shift;
                                let down = down - shift;
                                let total_height =
                                    rects.iter().map(|r| r.height()).sum::<f64>();
                                split.with_untracked(|split| {
                                    for (i, (size, _)) in
                                        split.children.iter().enumerate()
                                    {
                                        if i == index - 1 {
                                            size.set(up / total_height);
                                        } else if i == index {
                                            size.set(down / total_height);
                                        } else {
                                            size.set(
                                                rects[i].height() / total_height,
                                            );
                                        }
                                    }
                                })
                            }
                        }
                    }
                }
            })
            .style(move |s| {
                let rect = content_rect(&content, true);
                let is_dragging = drag_start.get().is_some();
                let direction = direction(true);
                s.position(Position::Absolute)
                    .apply_if(direction == SplitDirection::Vertical, |style| {
                        style.margin_left(rect.x0 as f32 - 0.0)
                    })
                    .apply_if(direction == SplitDirection::Horizontal, |style| {
                        style.margin_top(rect.y0 as f32 - 0.0)
                    })
                    .width(match direction {
                        SplitDirection::Vertical => PxPctAuto::Px(4.0),
                        SplitDirection::Horizontal => PxPctAuto::Pct(100.0),
                    })
                    .height(match direction {
                        SplitDirection::Vertical => PxPctAuto::Pct(100.0),
                        SplitDirection::Horizontal => PxPctAuto::Px(4.0),
                    })
                    .flex_direction(match direction {
                        SplitDirection::Vertical => FlexDirection::Row,
                        SplitDirection::Horizontal => FlexDirection::Column,
                    })
                    .apply_if(is_dragging, |s| {
                        s.cursor(match direction {
                            SplitDirection::Vertical => CursorStyle::ColResize,
                            SplitDirection::Horizontal => CursorStyle::RowResize,
                        })
                        .background(config.get().color(LapceColor::EDITOR_CARET))
                    })
                    .hover(|s| {
                        s.cursor(match direction {
                            SplitDirection::Vertical => CursorStyle::ColResize,
                            SplitDirection::Horizontal => CursorStyle::RowResize,
                        })
                        .background(config.get().color(LapceColor::EDITOR_CARET))
                    })
                    .pointer_events_auto()
            })
        },
    )
    .style(|s| {
        s.position(Position::Absolute)
            .size_full()
            .pointer_events_none()
    })
    .debug_name("Split Resize Border")
}

fn split_border(
    splits: ReadSignal<im::HashMap<SplitId, RwSignal<SplitData>>>,
    editor_tabs: ReadSignal<im::HashMap<EditorTabId, RwSignal<EditorTabData>>>,
    split: ReadSignal<SplitData>,
    config: ReadSignal<Arc<LapceConfig>>,
) -> impl View {
    let direction = move || split.with(|split| split.direction);
    dyn_stack(
        move || split.get().children.into_iter().skip(1),
        |(_, content)| content.id(),
        move |(_, content)| {
            container(empty().style(move |s| {
                let direction = direction();
                s.width(match direction {
                    SplitDirection::Vertical => PxPctAuto::Px(1.0),
                    SplitDirection::Horizontal => PxPctAuto::Pct(100.0),
                })
                .height(match direction {
                    SplitDirection::Vertical => PxPctAuto::Pct(100.0),
                    SplitDirection::Horizontal => PxPctAuto::Px(1.0),
                })
                .background(config.get().color(LapceColor::LAPCE_BORDER))
            }))
            .style(move |s| {
                let rect = match &content {
                    SplitContent::EditorTab(editor_tab_id) => {
                        let editor_tab_data = editor_tabs
                            .with(|tabs| tabs.get(editor_tab_id).cloned());
                        if let Some(editor_tab_data) = editor_tab_data {
                            editor_tab_data.with(|editor_tab| editor_tab.layout_rect)
                        } else {
                            Rect::ZERO
                        }
                    }
                    SplitContent::Split(split_id) => {
                        if let Some(split) =
                            splits.with(|splits| splits.get(split_id).cloned())
                        {
                            split.with(|split| split.layout_rect)
                        } else {
                            Rect::ZERO
                        }
                    }
                };
                let direction = direction();
                s.position(Position::Absolute)
                    .apply_if(direction == SplitDirection::Vertical, |style| {
                        style.margin_left(rect.x0 as f32 - 2.0)
                    })
                    .apply_if(direction == SplitDirection::Horizontal, |style| {
                        style.margin_top(rect.y0 as f32 - 2.0)
                    })
                    .width(match direction {
                        SplitDirection::Vertical => PxPctAuto::Px(4.0),
                        SplitDirection::Horizontal => PxPctAuto::Pct(100.0),
                    })
                    .height(match direction {
                        SplitDirection::Vertical => PxPctAuto::Pct(100.0),
                        SplitDirection::Horizontal => PxPctAuto::Px(4.0),
                    })
                    .flex_direction(match direction {
                        SplitDirection::Vertical => FlexDirection::Row,
                        SplitDirection::Horizontal => FlexDirection::Column,
                    })
                    .justify_content(Some(JustifyContent::Center))
            })
        },
    )
    .style(|s| {
        s.position(Position::Absolute)
            .size_full()
            .pointer_events_none()
    })
    .debug_name("Split Border")
}

fn split_list(
    split: ReadSignal<SplitData>,
    window_tab_data: Rc<WindowTabData>,
    plugin: PluginData,
    dragging: RwSignal<Option<(RwSignal<usize>, EditorTabId)>>,
) -> impl View {
    let main_split = window_tab_data.main_split.clone();
    let editor_tabs = main_split.editor_tabs.read_only();
    let active_editor_tab = main_split.active_editor_tab.read_only();
    let splits = main_split.splits.read_only();
    let config = main_split.common.config;
    let split_id = split.with_untracked(|split| split.split_id);

    let direction = move || split.with(|split| split.direction);
    let items = move || split.get().children.into_iter().enumerate();
    let key = |(_index, (_, content)): &(usize, (RwSignal<f64>, SplitContent))| {
        content.id()
    };
    let view_fn = {
        let main_split = main_split.clone();
        let window_tab_data = window_tab_data.clone();
        move |(_index, (split_size, content)): (
            usize,
            (RwSignal<f64>, SplitContent),
        )| {
            let plugin = plugin.clone();
            let child = match &content {
                SplitContent::EditorTab(editor_tab_id) => {
                    let editor_tab_data = editor_tabs
                        .with_untracked(|tabs| tabs.get(editor_tab_id).cloned());
                    if let Some(editor_tab_data) = editor_tab_data {
                        editor_tab(
                            window_tab_data.clone(),
                            plugin.clone(),
                            active_editor_tab,
                            editor_tab_data,
                            dragging,
                        )
                        .into_any()
                    } else {
                        text("empty editor tab").into_any()
                    }
                }
                SplitContent::Split(split_id) => {
                    if let Some(split) =
                        splits.with(|splits| splits.get(split_id).cloned())
                    {
                        split_list(
                            split.read_only(),
                            window_tab_data.clone(),
                            plugin.clone(),
                            dragging,
                        )
                        .into_any()
                    } else {
                        text("empty split").into_any()
                    }
                }
            };
            let local_main_split = main_split.clone();
            let local_local_main_split = main_split.clone();
            child
                .on_resize(move |rect| match &content {
                    SplitContent::EditorTab(editor_tab_id) => {
                        local_main_split.editor_tab_update_layout(
                            editor_tab_id,
                            None,
                            Some(rect),
                        );
                    }
                    SplitContent::Split(split_id) => {
                        let split_data =
                            splits.with(|splits| splits.get(split_id).cloned());
                        if let Some(split_data) = split_data {
                            split_data.update(|split| {
                                split.layout_rect = rect;
                            });
                        }
                    }
                })
                .on_move(move |point| match &content {
                    SplitContent::EditorTab(editor_tab_id) => {
                        local_local_main_split.editor_tab_update_layout(
                            editor_tab_id,
                            Some(point),
                            None,
                        );
                    }
                    SplitContent::Split(split_id) => {
                        let split_data =
                            splits.with(|splits| splits.get(split_id).cloned());
                        if let Some(split_data) = split_data {
                            split_data.update(|split| {
                                split.window_origin = point;
                            });
                        }
                    }
                })
                .style(move |s| s.flex_grow(split_size.get() as f32).flex_basis(0.0))
        }
    };
    container(
        stack((
            dyn_stack(items, key, view_fn).style(move |s| {
                s.flex_direction(match direction() {
                    SplitDirection::Vertical => FlexDirection::Row,
                    SplitDirection::Horizontal => FlexDirection::Column,
                })
                .size_full()
            }),
            split_border(splits, editor_tabs, split, config),
            split_resize_border(splits, editor_tabs, split, config),
        ))
        .style(|s| s.size_full()),
    )
    .on_cleanup(move || {
        if splits.with_untracked(|splits| splits.contains_key(&split_id)) {
            return;
        }
        split
            .with_untracked(|split_data| split_data.scope)
            .dispose();
    })
    .debug_name("Split List")
}

fn main_split(window_tab_data: Rc<WindowTabData>) -> impl View {
    let root_split = window_tab_data.main_split.root_split;
    let root_split = window_tab_data
        .main_split
        .splits
        .get_untracked()
        .get(&root_split)
        .unwrap()
        .read_only();
    let config = window_tab_data.main_split.common.config;
    let panel = window_tab_data.panel.clone();
    let plugin = window_tab_data.plugin.clone();
    let dragging: RwSignal<Option<(RwSignal<usize>, EditorTabId)>> =
        create_rw_signal(None);
    split_list(
        root_split,
        window_tab_data.clone(),
        plugin.clone(),
        dragging,
    )
    .style(move |s| {
        let config = config.get();
        let is_hidden = panel.panel_bottom_maximized(true)
            && panel.is_container_shown(&PanelContainerPosition::Bottom, true);
        s.border_color(config.color(LapceColor::LAPCE_BORDER))
            .background(config.color(LapceColor::EDITOR_BACKGROUND))
            .apply_if(is_hidden, |s| s.display(Display::None))
            .width_full()
            .flex_grow(1.0)
            .flex_basis(0.0)
    })
    .debug_name("Main Split")
}

fn workbench(window_tab_data: Rc<WindowTabData>) -> impl View {
    let workbench_size = window_tab_data.common.workbench_size;
    let main_split_width = window_tab_data.main_split.width;
    {
        let window_tab_data = window_tab_data.clone();
        main_split(window_tab_data.clone())
            .on_resize(move |rect| {
                let width = rect.size().width;
                if main_split_width.get_untracked() != width {
                    main_split_width.set(width);
                }
            })
            .style(|s| s.flex_col().flex_grow(1.0))
    }
    .on_resize(move |rect| {
        let size = rect.size();
        if size != workbench_size.get_untracked() {
            workbench_size.set(size);
        }
    })
    .style(move |s| s.size_full())
    .debug_name("Workbench")
}

struct VectorItems<V>(im::Vector<V>);

impl<V: Clone + 'static> VirtualVector<(usize, V)> for VectorItems<V> {
    fn total_len(&self) -> usize {
        self.0.len()
    }

    fn slice(&mut self, range: Range<usize>) -> impl Iterator<Item = (usize, V)> {
        let start = range.start;
        self.0
            .slice(range)
            .into_iter()
            .enumerate()
            .map(move |(i, item)| (i + start, item))
    }
}

fn completion_kind_to_str(kind: CompletionItemKind) -> &'static str {
    match kind {
        CompletionItemKind::METHOD => "f",
        CompletionItemKind::FUNCTION => "f",
        CompletionItemKind::CLASS => "c",
        CompletionItemKind::STRUCT => "s",
        CompletionItemKind::VARIABLE => "v",
        CompletionItemKind::INTERFACE => "i",
        CompletionItemKind::ENUM => "e",
        CompletionItemKind::ENUM_MEMBER => "e",
        CompletionItemKind::FIELD => "v",
        CompletionItemKind::PROPERTY => "p",
        CompletionItemKind::CONSTANT => "d",
        CompletionItemKind::MODULE => "m",
        CompletionItemKind::KEYWORD => "k",
        CompletionItemKind::SNIPPET => "n",
        _ => "t",
    }
}

fn hover(window_tab_data: Rc<WindowTabData>) -> impl View {
    let hover_data = window_tab_data.common.hover.clone();
    let config = window_tab_data.common.config;
    let id = AtomicU64::new(0);
    let layout_rect = window_tab_data.common.hover.layout_rect;

    scroll(
        dyn_stack(
            move || hover_data.content.get(),
            move |_| id.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            move |content| match content {
                MarkdownContent::Text(text_layout) => container(
                    rich_text(move || text_layout.clone())
                        .style(|s| s.max_width(600.0)),
                )
                .style(|s| s.max_width_full()),
                MarkdownContent::Image { .. } => container(empty()),
                MarkdownContent::Separator => container(empty().style(move |s| {
                    s.width_full()
                        .margin_vert(5.0)
                        .height(1.0)
                        .background(config.get().color(LapceColor::LAPCE_BORDER))
                })),
            },
        )
        .style(|s| s.flex_col().padding_horiz(10.0).padding_vert(5.0)),
    )
    .on_resize(move |rect| {
        layout_rect.set(rect);
    })
    .on_event_stop(EventListener::PointerMove, |_| {})
    .on_event_stop(EventListener::PointerDown, |_| {})
    .style(move |s| {
        let active = window_tab_data.common.hover.active.get();
        if !active {
            s.hide()
        } else {
            let config = config.get();
            if let Some(origin) = window_tab_data.hover_origin() {
                s.absolute()
                    .margin_left(origin.x as f32)
                    .margin_top(origin.y as f32)
                    .max_height(300.0)
                    .border(1.0)
                    .border_radius(6.0)
                    .border_color(config.color(LapceColor::LAPCE_BORDER))
                    .background(config.color(LapceColor::PANEL_BACKGROUND))
                    .set(PropagatePointerWheel, false)
            } else {
                s.hide()
            }
        }
    })
    .debug_name("Hover Layer")
}

fn completion(window_tab_data: Rc<WindowTabData>) -> impl View {
    let completion_data = window_tab_data.common.completion;
    let active_editor = window_tab_data.main_split.active_editor;
    let config = window_tab_data.common.config;
    let active = completion_data.with_untracked(|c| c.active);
    let request_id =
        move || completion_data.with_untracked(|c| (c.request_id, c.input_id));
    scroll(
        virtual_stack(
            move || completion_data.with(|c| VectorItems(c.filtered_items.clone())),
            move |(i, _item)| (request_id(), *i),
            move |(i, item)| {
                stack((
                    container(
                        text(
                            item.item.kind.map(completion_kind_to_str).unwrap_or(""),
                        )
                        .style(move |s| {
                            s.width_full()
                                .justify_content(Some(JustifyContent::Center))
                        }),
                    )
                    .style(move |s| {
                        let config = config.get();
                        let width = config.editor.line_height() as f32;
                        s.width(width)
                            .min_width(width)
                            .height_full()
                            .align_items(Some(AlignItems::Center))
                            .font_weight(Weight::BOLD)
                            .apply_opt(
                                config.completion_color(item.item.kind),
                                |s, c| s.color(c).background(c.multiply_alpha(0.3)),
                            )
                    }),
                    focus_text(
                        move || {
                            if config.get().editor.completion_item_show_detail {
                                item.item
                                    .detail
                                    .clone()
                                    .unwrap_or(item.item.label.clone())
                            } else {
                                item.item.label.clone()
                            }
                        },
                        move || item.indices.clone(),
                        move || config.get().color(LapceColor::EDITOR_FOCUS),
                    )
                    .on_click_stop(move |_| {
                        active.set(i);
                        if let Some(editor) = active_editor.get_untracked() {
                            editor.select_completion();
                        }
                    })
                    .on_event_stop(EventListener::PointerDown, |_| {})
                    .style(move |s| {
                        let config = config.get();
                        s.padding_horiz(5.0)
                            .min_width(0.0)
                            .align_items(Some(AlignItems::Center))
                            .size_full()
                            .cursor(CursorStyle::Pointer)
                            .apply_if(active.get() == i, |s| {
                                s.background(
                                    config.color(LapceColor::COMPLETION_CURRENT),
                                )
                            })
                            .hover(move |s| {
                                s.background(
                                    config
                                        .color(LapceColor::PANEL_HOVERED_BACKGROUND),
                                )
                            })
                    }),
                ))
                .style(move |s| {
                    s.align_items(Some(AlignItems::Center))
                        .width_full()
                        .height(config.get().editor.line_height() as f32)
                })
            },
        )
        .item_size_fixed(move || config.get().editor.line_height() as f64)
        .style(|s| {
            s.align_items(Some(AlignItems::Center))
                .width_full()
                .flex_col()
        }),
    )
    .ensure_visible(move || {
        let config = config.get();
        let active = active.get();
        Size::new(1.0, config.editor.line_height() as f64)
            .to_rect()
            .with_origin(Point::new(
                0.0,
                active as f64 * config.editor.line_height() as f64,
            ))
    })
    .on_resize(move |rect| {
        completion_data.update(|c| {
            c.layout_rect = rect;
        });
    })
    .on_event_stop(EventListener::PointerMove, |_| {})
    .style(move |s| {
        let config = config.get();
        let origin = window_tab_data.completion_origin();
        s.position(Position::Absolute)
            .width(config.editor.completion_width as i32)
            .max_height(400.0)
            .margin_left(origin.x as f32)
            .margin_top(origin.y as f32)
            .background(config.color(LapceColor::COMPLETION_BACKGROUND))
            .font_family(config.editor.font_family.clone())
            .font_size(config.editor.font_size() as f32)
            .border_radius(6.0)
    })
    .debug_name("Completion Layer")
}

fn code_action(window_tab_data: Rc<WindowTabData>) -> impl View {
    let config = window_tab_data.common.config;
    let code_action = window_tab_data.code_action;
    let (status, active) = code_action
        .with_untracked(|code_action| (code_action.status, code_action.active));
    let request_id =
        move || code_action.with_untracked(|code_action| code_action.request_id);
    scroll(
        container(
            dyn_stack(
                move || {
                    code_action.with(|code_action| {
                        code_action.filtered_items.clone().into_iter().enumerate()
                    })
                },
                move |(i, _item)| (request_id(), *i),
                move |(i, item)| {
                    container(
                        text(item.title().replace('\n', " "))
                            .style(|s| s.text_ellipsis().min_width(0.0)),
                    )
                    .on_click_stop(move |_| {
                        let code_action = code_action.get_untracked();
                        code_action.active.set(i);
                        code_action.select();
                    })
                    .on_event_stop(EventListener::PointerDown, |_| {})
                    .style(move |s| {
                        let config = config.get();
                        s.padding_horiz(10.0)
                            .align_items(Some(AlignItems::Center))
                            .min_width(0.0)
                            .width_full()
                            .line_height(1.8)
                            .border_radius(6.0)
                            .cursor(CursorStyle::Pointer)
                            .apply_if(active.get() == i, |s| {
                                s.background(
                                    config.color(LapceColor::COMPLETION_CURRENT),
                                )
                            })
                            .hover(move |s| {
                                s.background(
                                    config
                                        .color(LapceColor::PANEL_HOVERED_BACKGROUND),
                                )
                            })
                    })
                },
            )
            .style(|s| s.width_full().flex_col()),
        )
        .style(|s| s.width_full().padding_vert(4.0)),
    )
    .ensure_visible(move || {
        let config = config.get();
        let active = active.get();
        Size::new(1.0, config.editor.line_height() as f64)
            .to_rect()
            .with_origin(Point::new(
                0.0,
                active as f64 * config.editor.line_height() as f64,
            ))
    })
    .on_resize(move |rect| {
        code_action.update(|c| {
            c.layout_rect = rect;
        });
    })
    .on_event_stop(EventListener::PointerMove, |_| {})
    .style(move |s| {
        let origin = window_tab_data.code_action_origin();
        s.display(match status.get() {
            CodeActionStatus::Inactive => Display::None,
            CodeActionStatus::Active => Display::Flex,
        })
        .position(Position::Absolute)
        .width(400.0)
        .max_height(400.0)
        .margin_left(origin.x as f32)
        .margin_top(origin.y as f32)
        .background(config.get().color(LapceColor::COMPLETION_BACKGROUND))
        .border_radius(6.0)
    })
    .debug_name("Code Action Layer")
}

fn rename(window_tab_data: Rc<WindowTabData>) -> impl View {
    let editor = window_tab_data.rename.editor.clone();
    let active = window_tab_data.rename.active;
    let layout_rect = window_tab_data.rename.layout_rect;
    let config = window_tab_data.common.config;

    container(
        container(
            TextInputBuilder::new()
                .is_focused(move || active.get())
                .build_editor(editor)
                .style(|s| s.width(150.0)),
        )
        .style(move |s| {
            let config = config.get();
            s.font_family(config.editor.font_family.clone())
                .font_size(config.editor.font_size() as f32)
                .border(1.0)
                .border_radius(6.0)
                .border_color(config.color(LapceColor::LAPCE_BORDER))
                .background(config.color(LapceColor::EDITOR_BACKGROUND))
        }),
    )
    .on_resize(move |rect| {
        layout_rect.set(rect);
    })
    .on_event_stop(EventListener::PointerMove, |_| {})
    .on_event_stop(EventListener::PointerDown, |_| {})
    .style(move |s| {
        let origin = window_tab_data.rename_origin();
        s.position(Position::Absolute)
            .apply_if(!active.get(), |s| s.hide())
            .margin_left(origin.x as f32)
            .margin_top(origin.y as f32)
            .background(config.get().color(LapceColor::PANEL_BACKGROUND))
            .border_radius(6.0)
            .padding(6.0)
    })
    .debug_name("Rename Layer")
}

fn window_tab(window_tab_data: Rc<WindowTabData>) -> impl View {
    let window_origin = window_tab_data.common.window_origin;
    let layout_rect = window_tab_data.layout_rect;
    let config = window_tab_data.common.config;
    let window_tab_scope = window_tab_data.scope;
    let hover_active = window_tab_data.common.hover.active;

    let view = stack((
        workbench(window_tab_data.clone())
            .on_resize(move |rect| {
                layout_rect.set(rect);
            })
            .on_move(move |point| {
                window_origin.set(point);
            })
            .style(|s| s.size_full().flex_col())
            .debug_name("Base Layer"),
        completion(window_tab_data.clone()),
        hover(window_tab_data.clone()),
        code_action(window_tab_data.clone()), // TODO: what is this?
        rename(window_tab_data.clone()),
        about::about_popup(window_tab_data.clone()),
        alert::alert_box(window_tab_data.alert_data.clone()),
    ))
    .on_cleanup(move || {
        window_tab_scope.dispose();
    })
    .on_event_cont(EventListener::PointerMove, move |_| {
        if hover_active.get_untracked() {
            hover_active.set(false);
        }
    })
    .style(move |s| {
        let config = config.get();
        s.size_full()
            .color(config.color(LapceColor::EDITOR_FOREGROUND))
            .background(config.color(LapceColor::EDITOR_BACKGROUND))
            .font_size(config.ui.font_size() as f32)
            .apply_if(!config.ui.font_family.is_empty(), |s| {
                s.font_family(config.ui.font_family.clone())
            })
            .class(floem::views::scroll::Handle, |s| {
                s.background(config.color(LapceColor::LAPCE_SCROLL_BAR))
            })
    })
    .debug_name("Window Tab");

    let view_id = view.id();
    window_tab_data.common.view_id.set(view_id);
    view
}

fn window(window_data: WindowData) -> impl View {
    let window_tabs = window_data.window_tabs.read_only();
    let active = window_data.active.read_only();
    let items = move || window_tabs.get();
    let key = |(_, window_tab): &(RwSignal<usize>, Rc<WindowTabData>)| {
        window_tab.window_tab_id
    };
    let active = move || active.get();
    let window_focus = create_rw_signal(false);
    let ime_enabled = window_data.ime_enabled;
    let window_maximized = window_data.common.window_maximized;

    tab(active, items, key, |(_, window_tab_data)| {
        window_tab(window_tab_data)
    })
    .window_title(move || {
        let active = active();
        let window_tabs = window_tabs.get();
        let workspace = window_tabs
            .get(active)
            .or_else(|| window_tabs.last())
            .and_then(|(_, window_tab)| window_tab.workspace.display());
        match workspace {
            Some(workspace) => format!("{workspace} - Amas"),
            None => "Amas".to_string(),
        }
    })
    .on_event_stop(EventListener::ImeEnabled, move |_| {
        ime_enabled.set(true);
    })
    .on_event_stop(EventListener::ImeDisabled, move |_| {
        ime_enabled.set(false);
    })
    .on_event_cont(EventListener::WindowGotFocus, move |_| {
        window_focus.set(true);
    })
    .on_event_cont(EventListener::WindowMaximizeChanged, move |event| {
        if let Event::WindowMaximizeChanged(maximized) = event {
            window_maximized.set(*maximized);
        }
    })
    .style(|s| s.size_full())
    .debug_name("Window")
}

pub fn into_view(
    window_id: WindowId,
    path: &str,
    linecol: Option<LineCol>,
) -> impl IntoView {
    trace!(TraceLevel::INFO, "Starting up Amas..");

    #[cfg(feature = "vendored-fonts")]
    {
        use floem::text::{FONT_SYSTEM, fontdb::Source};

        const FONT_DEJAVU_SANS_REGULAR: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../extra/fonts/DejaVu/DejaVuSans.ttf"
        ));
        const FONT_DEJAVU_SANS_MONO_REGULAR: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../extra/fonts/DejaVu/DejaVuSansMono.ttf"
        ));

        FONT_SYSTEM
            .lock()
            .db_mut()
            .load_font_source(Source::Binary(Arc::new(FONT_DEJAVU_SANS_REGULAR)));
        FONT_SYSTEM
            .lock()
            .db_mut()
            .load_font_source(Source::Binary(Arc::new(
                FONT_DEJAVU_SANS_MONO_REGULAR,
            )));
    }

    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        trace!(TraceLevel::INFO, "Loading custom environment from shell");
        load_shell_env();
    }

    #[cfg(feature = "updater")]
    crate::update::cleanup();

    if let Err(err) = lapce_proxy::register_lapce_path() {
        tracing::error!("{:?}", err);
    }
    let db = match LapceDb::new() {
        Ok(db) => Arc::new(db),
        Err(e) => {
            #[cfg(windows)]
            logging::error_modal("Error", &format!("Failed to create LapceDb: {e}"));

            trace!(TraceLevel::ERROR, "Failed to create LapceDb: {e}");
            std::process::exit(1);
        }
    };
    let scope = Scope::new();
    provide_context(db.clone());

    let window_scale = scope.create_rw_signal(1.0);
    let latest_release = scope.create_rw_signal(Arc::new(None));
    let app_command = Listener::new_empty(scope);

    let plugin_paths = Arc::new(vec![]);

    let windows = scope.create_rw_signal(im::HashMap::new());
    let config = LapceConfig::load(&LapceWorkspace::default(), &[], &plugin_paths);

    // Restore scale from config
    window_scale.set(config.ui.scale());

    let config = scope.create_rw_signal(Arc::new(config));
    let app_data = AppData {
        windows,
        active_window: scope.create_rw_signal(WindowId::from_raw(0)),
        window_scale,
        app_terminated: scope.create_rw_signal(false),
        latest_release,
        app_command,
        config,
        plugin_paths,
    };

    let app_view = app_data.into_view(
        window_id,
        db.clone(),
        vec![PathObject {
            path: std::path::PathBuf::from(path),
            is_dir: false,
            linecol,
        }],
    );

    // Updates grammars and refreshes syntax highlighting if needed
    {
        let cx = Scope::new();
        let app_data = app_data.clone();
        let send = create_ext_action(cx, move |updated| {
            if updated {
                trace!(
                    TraceLevel::INFO,
                    "grammar or query got updated, reset highlight configs"
                );
                reset_highlight_configs();
                for (_, window) in app_data.windows.get_untracked() {
                    for (_, tab) in window.window_tabs.get_untracked() {
                        for (_, doc) in tab.main_split.docs.get_untracked() {
                            doc.syntax.update(|syntaxt| {
                                *syntaxt = Syntax::from_language(syntaxt.language);
                            });
                            doc.trigger_syntax_change(None);
                        }
                    }
                }
            }
        });
        std::thread::Builder::new()
            .name("FindGrammar".to_owned())
            .spawn(move || {
                use self::grammars::*;
                let updated = match find_grammar_release() {
                    Ok(release) => {
                        let mut updated = false;
                        match fetch_grammars(&release) {
                            Err(e) => {
                                trace!(
                                    TraceLevel::ERROR,
                                    "failed to fetch grammars: {e}"
                                );
                            }
                            Ok(u) => updated |= u,
                        }
                        match fetch_queries(&release) {
                            Err(e) => {
                                trace!(
                                    TraceLevel::ERROR,
                                    "failed to fetch grammars: {e}"
                                );
                            }
                            Ok(u) => updated |= u,
                        }
                        updated
                    }
                    Err(e) => {
                        trace!(
                            TraceLevel::ERROR,
                            "failed to obtain release info: {e}"
                        );
                        false
                    }
                };
                send(updated);
            })
            .unwrap();
    }

    {
        let (tx, rx) = sync_channel(1);
        let notification = create_signal_from_channel(rx);
        let app_data = app_data.clone();
        create_effect(move |_| {
            if let Some(CoreNotification::OpenPaths { paths }) = notification.get() {
                if let Some(window_tab) = app_data.active_window_tab() {
                    window_tab.open_paths(&paths);
                    // focus window after open doc
                    floem::action::focus_window();
                }
            }
        });
        std::thread::Builder::new()
            .name("ListenLocalSocket".to_owned())
            .spawn(move || {
                if let Err(err) = listen_local_socket(tx) {
                    tracing::error!("{:?}", err);
                }
            })
            .unwrap();
    }

    app_view
}

/// Uses a login shell to load the correct shell environment for the current user.
pub fn load_shell_env() {
    use std::process::Command;

    use tracing::warn;

    #[cfg(not(windows))]
    let shell = match std::env::var("SHELL") {
        Ok(s) => s,
        Err(error) => {
            // Shell variable is not set, so we can't determine the correct shell executable.
            trace!(
                TraceLevel::ERROR,
                "Failed to obtain shell environment: {error}"
            );
            return;
        }
    };

    #[cfg(windows)]
    let shell = "powershell";

    let mut command = Command::new(shell);

    #[cfg(not(windows))]
    command.args(["--login", "-c", "printenv"]);

    #[cfg(windows)]
    command.args([
        "-Command",
        "Get-ChildItem env: | ForEach-Object { \"{0}={1}\" -f $_.Name, $_.Value }",
    ]);

    #[cfg(windows)]
    command.creation_flags(windows::Win32::System::Threading::CREATE_NO_WINDOW);

    let env = match command.output() {
        Ok(output) => String::from_utf8(output.stdout).unwrap_or_default(),

        Err(error) => {
            trace!(
                TraceLevel::ERROR,
                "Failed to obtain shell environment: {error}"
            );
            return;
        }
    };

    env.split('\n')
        .filter_map(|line| line.split_once('='))
        .for_each(|(key, value)| unsafe {
            let value = value.trim_matches('\r');
            if let Ok(v) = std::env::var(key) {
                if v != value {
                    warn!("Overwriting '{key}', previous value: '{v}', new value '{value}'");
                }
            };
            std::env::set_var(key, value);
        })
}

fn listen_local_socket(tx: SyncSender<CoreNotification>) -> Result<()> {
    let local_socket = Directory::local_socket()
        .ok_or_else(|| anyhow!("can't get local socket folder"))?;
    if local_socket.exists() {
        if let Err(err) = std::fs::remove_file(&local_socket) {
            tracing::error!("{:?}", err);
        }
    }
    let socket =
        interprocess::local_socket::LocalSocketListener::bind(local_socket)?;

    for stream in socket.incoming().flatten() {
        let tx = tx.clone();
        std::thread::spawn(move || -> Result<()> {
            let mut reader = BufReader::new(stream);
            loop {
                let msg: Option<CoreMessage> =
                    lapce_rpc::stdio::read_msg(&mut reader)?;

                if let Some(RpcMessage::Notification(msg)) = msg {
                    tx.send(msg)?;
                } else {
                    trace!(TraceLevel::ERROR, "Unhandled message: {msg:?}");
                }

                let stream_ref = reader.get_mut();
                if let Err(err) = stream_ref.write_all(b"received") {
                    tracing::error!("{:?}", err);
                }
                if let Err(err) = stream_ref.flush() {
                    tracing::error!("{:?}", err);
                }
            }
        });
    }
    Ok(())
}
