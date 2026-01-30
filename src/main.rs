#![windows_subsystem = "windows"]

use fltk::{
    app, browser::HoldBrowser, button::Button, dialog,
    enums::{Color, FrameType, Shortcut, Key},
    prelude::*,
    window::Window,
    frame::Frame,
    group::{Pack, Group, Flex},
    menu::{SysMenuBar, MenuFlag},
};
use fltk_theme::{WidgetTheme, ThemeType, WidgetScheme, SchemeType};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};
use walkdir::WalkDir;
use rayon::prelude::*;
use serde::{Serialize, Deserialize};

mod repo;
use repo::Repository;

const CONFIG_FILE: &str = "configuration.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppConfig {
    repositories: Vec<PathBuf>,
    theme_idx: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            repositories: Vec::new(),
            theme_idx: 0, // Default to Greybird (idx 0 in our list)
        }
    }
}

// Config Helper
fn load_config() -> AppConfig {
    if let Ok(file) = std::fs::File::open(CONFIG_FILE) {
        // Try loading as AppConfig first
        if let Ok(cfg) = serde_json::from_reader(file) {
            return cfg;
        }
        // Fallback: Try loading strictly as Vec<PathBuf> for backward compatibility
        // Re-open file because reader consumed it? 
        if let Ok(file) = std::fs::File::open(CONFIG_FILE) {
               if let Ok(paths) = serde_json::from_reader::<_, Vec<PathBuf>>(file) {
                   return AppConfig { repositories: paths, theme_idx: 0 };
               }
        }
    }
    AppConfig::default()
}

fn save_config(repos: &[Repository], theme_idx: usize) {
    let paths: Vec<PathBuf> = repos.iter().map(|r| r.path.clone()).collect();
    let cfg = AppConfig {
        repositories: paths,
        theme_idx,
    };
    match std::fs::File::create(CONFIG_FILE) {
        Ok(file) => {
            if let Err(e) = serde_json::to_writer_pretty(file, &cfg) {
                eprintln!("Failed to write config: {}", e);
                dialog::alert(200, 200, &format!("Failed to write config: {}", e));
            }
        },
        Err(e) => {
            eprintln!("Failed to create config file: {}", e);
            dialog::alert(200, 200, &format!("Failed to create config file: {}", e));
        }
    }
}


#[derive(Clone)]
enum Message {
    ScanComplete(Vec<Repository>),
    PullAll,
    PullCurrent,
    UpdateLatest,
    Commit,
    OpenSwitchBranch,
    SwitchBranch(String),
    Refresh,
    RefreshAll,
    AddFolder,
    RemoveSelected,
    OpenPreferences,
    ChangeTheme(usize),
    SelectAll,
    SetStatus(PathBuf, String),
    SetGlobalStatus(String),
    RepoUpdated(Repository),
    Sort(usize), // Column Index
}

#[derive(Clone, Copy, PartialEq)]
enum SortOrder {
    Ascending,
    Descending,
    None, // Default (usually Path Ascending)
}

struct SortState {
    column: usize,
    order: SortOrder,
}
//...
const THEMES: &[(&str, ThemeType)] = &[
    ("Greybird", ThemeType::Greybird),
    ("Dark", ThemeType::Dark),
    ("HighContrast", ThemeType::HighContrast),
    ("Blue", ThemeType::Blue),
    ("Metro", ThemeType::Metro),
];

fn main() {
    let app = app::App::default();
    
    // Load config early
    let config = Arc::new(Mutex::new(load_config()));
    let initial_theme_idx = config.lock().unwrap().theme_idx;

    let widget_scheme = WidgetScheme::new(SchemeType::Fluent);
    widget_scheme.apply();

    // Apply saved theme
    if initial_theme_idx < THEMES.len() {
        let widget_theme = WidgetTheme::new(THEMES[initial_theme_idx].1); 
        widget_theme.apply();
    }

    let mut wind = Window::default()
        .with_size(1000, 750)
        .with_label("ManaHg");

    let (s, r) = app::channel::<Message>();

    // Menu Bar
    let mut menu = SysMenuBar::new(0, 0, 1000, 30, "");
    menu.add_emit(
        "&File/Add Repository...\t",
        Shortcut::Ctrl | '+',
        MenuFlag::Normal,
        s.clone(),
        Message::AddFolder,
    );
     menu.add_emit(
        "&File/Refresh Selection\t",
        Shortcut::None | Key::F5,
        MenuFlag::Normal,
        s.clone(),
        Message::Refresh,
    );
    menu.add_emit(
        "&File/Remove Selection\t",
        Shortcut::None | Key::Delete,
        MenuFlag::Normal,
        s.clone(),
        Message::RemoveSelected,
    );
    menu.add_emit(
        "&File/Preferences...\t",
        Shortcut::Ctrl | 'p',
        MenuFlag::Normal,
        s.clone(),
        Message::OpenPreferences,
    );
    menu.add(
        "&File/Quit\t",
        Shortcut::Ctrl | 'q',
        MenuFlag::Normal,
        |_| app::quit(),
    );
    
    // Actions menu
    menu.add_emit(
        "&Action/Pull All\t",
        Shortcut::None,
        MenuFlag::Normal,
        s.clone(),
        Message::PullAll,
    );
    menu.add_emit(
        "&Action/Pull Current\t",
        Shortcut::None,
        MenuFlag::Normal,
        s.clone(),
        Message::PullCurrent,
    );
    menu.add_emit(
        "&Action/Update To Last\t",
        Shortcut::None,
        MenuFlag::Normal,
        s.clone(),
        Message::UpdateLatest,
    );
     menu.add_emit(
        "&Action/Switch Branch...\t",
        Shortcut::None,
        MenuFlag::Normal,
        s.clone(),
        Message::OpenSwitchBranch,
    );
    menu.add_emit(
        "&Action/Commit...\t",
        Shortcut::None,
        MenuFlag::Normal,
        s.clone(),
        Message::Commit,
    );

    menu.add("&Edit/Copy", Shortcut::Ctrl | 'c', MenuFlag::Normal, |_| {});
    menu.add_emit(
        "&Selection/Select All",
        Shortcut::Ctrl | 'a',
        MenuFlag::Normal,
        s.clone(),
        Message::SelectAll,
    );
    menu.add("&View/Refresh", Shortcut::None | Key::F5, MenuFlag::Normal, |_| {});
    menu.add("&Help/About", Shortcut::None, MenuFlag::Normal, |_| {
        let mut help_win = Window::default().with_size(300, 180).with_label("About");
        help_win.set_border(true); // Ensure decorations
        let mut pack = Pack::new(10, 10, 280, 160, "");
        pack.set_spacing(10);
        let _frame = Frame::default().with_size(0, 80).with_label("ManaHg v0.1\nRust Implementation");
        
        let mut btn_close = Button::default().with_size(280, 30).with_label("Close");
        let mut win_c = help_win.clone();
        btn_close.set_callback(move |_| win_c.hide());
        
        pack.end();
        help_win.end();
        help_win.make_modal(true);
        help_win.show();
    });

    // Main Vertical Layout (Shifted down for menu)
    let mut flex = Flex::new(0, 30, 1000, 720, "").column();
    
    // Toolbar (Horizontal)
    let mut toolbar = Group::default().with_size(1000, 40);
    toolbar.set_frame(FrameType::FlatBox);
    let mut btn_refresh = Button::new(10, 5, 90, 30, "@refresh"); 
    btn_refresh.set_tooltip("Refresh selected repositories");

    // Actions
    let mut btn_pull = Button::new(590, 5, 90, 30, "Pull All Br.");
    btn_pull.set_tooltip("Pull all branches for selected repositories");
    let mut btn_pull_curr = Button::new(690, 5, 90, 30, "Pull Cur. Br.");
    btn_pull_curr.set_tooltip("Pull current branch for selected repositories");
    let mut btn_update = Button::new(790, 5, 80, 30, "Update");
    btn_update.set_tooltip("Update to latest commit for selected repositories");
    let mut btn_switch = Button::new(880, 5, 50, 30, "Switch");
    btn_switch.set_tooltip("Switch branch");
    let mut btn_commit = Button::new(940, 5, 50, 30, "Commit");
    btn_commit.set_tooltip("Commit pending changes");
    
    toolbar.end();
    flex.fixed(&toolbar, 40);

    // Header Row (Buttons)
    let header_group = Group::default().with_size(1000, 24);
    let col_widths = [450, 150, 80, 80, 100, 140]; // Total 1000
    let col_names = ["Path", "Branch", "Rev", "Mod", "Phase", "Status"];
    let mut x_off = 0;
    for (i, &w) in col_widths.iter().enumerate() {
        let mut btn = Button::new(x_off, 0, w, 24, col_names[i]);
        btn.set_frame(FrameType::ThinUpBox);
        btn.set_label_size(12);
        btn.emit(s.clone(), Message::Sort(i));
        x_off += w;
    }
    header_group.end();
    flex.fixed(&header_group, 24);

    // Repo List
    let mut browser = HoldBrowser::default(); 
    
    browser.set_column_char('\t');
    browser.set_column_widths(&col_widths); 

    browser.set_text_size(14);
    browser.set_type(fltk::browser::BrowserType::Multi); 
    // browser.add("Path\tBranch\tRev\tMod\tPhase\tStatus"); // Removed header line

    // Status Bar
    let mut status_bar = Frame::default().with_label("Ready");
    status_bar.set_frame(FrameType::FlatBox);
    status_bar.set_align(fltk::enums::Align::Left | fltk::enums::Align::Inside);
    status_bar.set_label_color(Color::Gray0);
    flex.fixed(&status_bar, 24);
    
    flex.end();
    
    // Resize handling
    wind.resizable(&flex);
    wind.show();

    // App State
    let repositories: Arc<Mutex<Vec<Repository>>> = Arc::new(Mutex::new(Vec::new()));
    let sort_state = Arc::new(Mutex::new(SortState { column: 0, order: SortOrder::None }));
    
    // Callbacks
    btn_refresh.emit(s.clone(), Message::Refresh);
    btn_pull.emit(s.clone(), Message::PullAll);
    btn_pull_curr.emit(s.clone(), Message::PullCurrent);
    btn_update.emit(s.clone(), Message::UpdateLatest);
    btn_switch.emit(s.clone(), Message::OpenSwitchBranch);
    btn_commit.emit(s.clone(), Message::Commit);

    let cloned_repos = config.lock().unwrap().repositories.clone();
    
    // Load saved repositories immediately (fast, no refresh)
    {
        let mut repos = repositories.lock().unwrap();
        for p in &cloned_repos {
            repos.push(Repository::new(p.clone()));
        }
    }
    update_browser(&mut browser, &repositories.lock().unwrap());
    
    if !cloned_repos.is_empty() {
        // Trigger background refresh
         let sender = s.clone();
         thread::spawn(move || {
             sender.send(Message::RefreshAll); 
         });
    }

    let mut current_theme_idx = initial_theme_idx;
    
    // Initial check: if args, scan them
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
         let mut dirs = Vec::new();
         for arg in args.iter().skip(1) {
             if !arg.starts_with('-') {
                 dirs.push(PathBuf::from(arg));
             }
         }
         if !dirs.is_empty() {
             let sender = s.clone();
             status_bar.set_label("Scanning...");
             thread::spawn(move || {
                scan_repositories(dirs, sender);
             });
         }
    }

    // Event Loop
    while app.wait() {
        if let Some(msg) = r.recv() {
            match msg {
                Message::AddFolder => {
                    let mut dialog = dialog::NativeFileChooser::new(dialog::NativeFileChooserType::BrowseDir);
                    dialog.show();
                    if !dialog.filename().as_os_str().is_empty() {
                         let path = dialog.filename();
                         let sender = s.clone();
                         status_bar.set_label(&format!("Scanning {}...", path.display()));
                         thread::spawn(move || {
                             scan_repositories(vec![path], sender);
                         });
                    }
                }
                Message::ScanComplete(new_repos) => {
                    let mut repos = repositories.lock().unwrap();
                    for nr in new_repos {
                        if !repos.iter().any(|r| r.path == nr.path) {
                            repos.push(nr);
                        }
                    }
                    repos.sort_by(|a, b| a.path.cmp(&b.path));
                    
                    save_config(&repos, current_theme_idx);

                    update_browser(&mut browser, &repos);
                    status_bar.set_label(&format!("Found {} repositories", repos.len()));
                }
                Message::RepoUpdated(updated_repo) => {
                    let mut repos = repositories.lock().unwrap();
                     if let Some(r) = repos.iter_mut().find(|r| r.path == updated_repo.path) {
                         // Preserve status if not set in updated_repo
                         let old_status = r.last_status.clone();
                         *r = updated_repo;
                         if r.last_status.is_empty() {
                             r.last_status = old_status;
                         }
                    }
                    update_browser(&mut browser, &repos);
                }
                Message::SetStatus(path, status_msg) => {
                    let mut repos = repositories.lock().unwrap();
                    if let Some(r) = repos.iter_mut().find(|r| r.path == path) {
                        r.last_status = status_msg;
                    }
                    update_browser(&mut browser, &repos);
                }
                Message::Sort(col) => {
                    let mut state = sort_state.lock().unwrap();
                    if state.column == col {
                        match state.order {
                             SortOrder::None => state.order = SortOrder::Ascending,
                             SortOrder::Ascending => state.order = SortOrder::Descending,
                             SortOrder::Descending => state.order = SortOrder::None,
                        }
                    } else {
                        state.column = col;
                        state.order = SortOrder::Ascending;
                    }

                    // Update header labels
                    let col_names = ["Path", "Branch", "Rev", "Mod", "Phase", "Status"];
                    for i in 0..col_names.len() {
                        if let Some(mut widget) = header_group.child(i as i32) {
                            let mut label = col_names[i].to_string();
                            if i == state.column {
                                match state.order {
                                    SortOrder::Ascending => label.push_str(" ▲"),
                                    SortOrder::Descending => label.push_str(" ▼"),
                                    SortOrder::None => {},
                                }
                            }
                            widget.set_label(&label);
                            widget.redraw();
                        }
                    }
                    
                    let mut repos = repositories.lock().unwrap();
                    sort_repos(&mut repos, &state);
                    update_browser(&mut browser, &repos);
                }
                Message::Refresh => {
                    let selected_repos = get_selected_repos(&browser, &repositories.lock().unwrap());
                    if selected_repos.is_empty() { 
                        status_bar.set_label("Select repositories to refresh.");
                        continue; 
                    }
                    status_bar.set_label("Refreshing selected...");
                    let sender = s.clone();
                    
                    {
                        let mut repos = repositories.lock().unwrap();
                        for r in repos.iter_mut() {
                            if selected_repos.iter().any(|sel| sel.path == r.path) {
                                r.last_status = "Refreshing...".to_string();
                            }
                        }
                        update_browser(&mut browser, &repos);
                    }

                    thread::spawn(move || {
                        selected_repos.par_iter().for_each(|r| {
                            let mut r = r.clone();
                            r.refresh();
                            r.last_status = "Ready".to_string(); 
                            sender.send(Message::RepoUpdated(r));
                        });

                        sender.send(Message::SetGlobalStatus("Ready".into()));
                    });
                }
                Message::RefreshAll => {
                     let repos_clone = repositories.lock().unwrap().clone();
                     if repos_clone.is_empty() { 
                         status_bar.set_label("No repositories to refresh");
                         continue; 
                     }
                     status_bar.set_label("Refreshing all...");
                     let sender = s.clone();
                     
                     {
                         let mut repos = repositories.lock().unwrap();
                         for r in repos.iter_mut() {
                             r.last_status = "Refreshing...".to_string();
                         }
                         update_browser(&mut browser, &repos);
                     }
 
                     thread::spawn(move || {
                         repos_clone.par_iter().for_each(|r| {
                             let mut r = r.clone();
                             r.refresh();
                             r.last_status = "Ready".to_string(); 
                             sender.send(Message::RepoUpdated(r));
                         });
 
                         sender.send(Message::SetGlobalStatus("Ready".into()));
                     });
                }
                Message::RemoveSelected => {
                    let selected = get_selected_repos(&browser, &repositories.lock().unwrap());
                    if selected.is_empty() { continue; }
                    
                    let mut repos = repositories.lock().unwrap();
                    let len_before = repos.len();
                    repos.retain(|r| !selected.iter().any(|sel| sel.path == r.path));
                    
                    if repos.len() != len_before {
                         save_config(&repos, current_theme_idx);
                         update_browser(&mut browser, &repos);
                    }
                }
                Message::OpenPreferences => {
                    let mut prefs_win = Window::default().with_size(300, 150).with_label("Preferences");
                    prefs_win.set_border(true);
                    let mut pack = Pack::new(10, 10, 280, 130, "");
                    pack.set_spacing(10);
                    
                    pack.add(&Frame::default().with_size(0, 30).with_label("Select a theme:"));
                    
                    let mut choice = fltk::menu::Choice::default().with_size(0, 30);
                    for (name, _) in THEMES {
                        choice.add_choice(name);
                    }
                    choice.set_value(current_theme_idx as i32);
                    
                    // Buttons in a Pack to ensure visibility
                    let mut btn_pack = Pack::new(0, 0, 280, 40, "");
                    btn_pack.set_type(fltk::group::PackType::Horizontal);
                    btn_pack.set_spacing(20);
                    
                    let mut btn_ok = Button::new(0, 0, 120, 30, "Apply");
                    let mut btn_close = Button::new(0, 0, 120, 30, "Close");
                    btn_pack.end();
                    
                    pack.end();
                    prefs_win.end();
                    prefs_win.make_modal(true);
                    prefs_win.show();

                    let sender = s.clone();
                    let choice_c = choice.clone();
                    btn_ok.set_callback(move |_| {
                        sender.send(Message::ChangeTheme(choice_c.value() as usize));
                    });
                    
                    let mut pw_c = prefs_win.clone();
                    btn_close.set_callback(move |_| pw_c.hide());
                }
                Message::ChangeTheme(idx) => {

                    if idx < THEMES.len() {
                        current_theme_idx = idx;
                        let widget_theme = WidgetTheme::new(THEMES[idx].1);
                        widget_theme.apply();
                        app::redraw();
                        
                        let repos = repositories.lock().unwrap();
                        save_config(&repos, current_theme_idx);
                    }
                }
                Message::SelectAll => {
                    // Start from 1 because line 0 is header? No, browser uses 1-based indexing for items.
                    // Wait, Browser::size() returns item count.
                    // Multi-select browser requires select(line) to be called for each line.
                    let count = browser.size();
                    if count > 0 {
                        for i in 1..=count {
                            browser.select(i);
                        }
                    }
                }
                Message::PullAll | Message::PullCurrent | Message::UpdateLatest => {
                    let sel = get_selected_repos(&browser, &repositories.lock().unwrap());
                    if sel.is_empty() {
                        status_bar.set_label("No repository selected");
                        continue;
                    }
                    
                    status_bar.set_label("Processing...");
                    let sender = s.clone();
                    let op = msg.clone();
                    
                    for repo in &sel {
                        // Create a unique task ID
                        let _task_id = repo.path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let op_name = match op {
                            Message::PullAll => "Pull All",
                            Message::PullCurrent => "Pull Current",
                            Message::UpdateLatest => "Update Latest",
                            _ => "Unknown",
                        };
                        sender.send(Message::SetStatus(repo.path.clone(), format!("{}...", op_name)));
                    }

                    thread::spawn(move || {
                        sel.par_iter().for_each(|repo| {
                            let _op_name = match op {
                                Message::PullAll => "Pull All",
                                Message::PullCurrent => "Pull Current",
                                Message::UpdateLatest => "Update Latest",
                                _ => "Unknown",
                            };
                            
                            let mut updated_repo = repo.clone(); // Clone to update state
                            let res = match op {
                                Message::PullAll => updated_repo.pull_all_branches(),
                                Message::PullCurrent => updated_repo.pull_current_branch(),
                                Message::UpdateLatest => updated_repo.update_to_latest(),
                                _ => Ok("".into()),
                            };
                            
                            // Refresh repo state after op (revision might change)
                            updated_repo.refresh();

                            match res {
                                Ok(_) => {
                                    updated_repo.last_status = "Success".to_string();
                                    sender.send(Message::RepoUpdated(updated_repo));
                                },
                                Err(e) => {
                                    updated_repo.last_status = format!("Error: {}", e);
                                    sender.send(Message::RepoUpdated(updated_repo));
                                }
                            }
                        });
                        sender.send(Message::SetGlobalStatus("Ready".into()));
                    });
                }
                Message::OpenSwitchBranch => {
                    let sel = get_selected_repos(&browser, &repositories.lock().unwrap());
                    if sel.is_empty() {
                         status_bar.set_label("Select repositories to switch branch");
                         continue;
                    }
                    
                    status_bar.set_label("Analyzing branches...");
                    
                    // Retrieve all branches with counts
                    use std::collections::HashMap;
                    
                    let mut branch_counts: HashMap<String, usize> = HashMap::new();
                    let total_sel = sel.len();
                    
                    for r in &sel {
                         if let Ok(branches) = r.get_all_branches() {
                             for b in branches {
                                 *branch_counts.entry(b).or_insert(0) += 1;
                             }
                         }
                    }
                    
                    let mut sorted_branches: Vec<(String, usize)> = branch_counts.into_iter().collect();
                    // Sort by count (descending) then name (ascending)
                    sorted_branches.sort_by(|a, b| {
                        b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
                    });

                    let branch_names: Vec<String> = sorted_branches.iter().map(|(n, _)| n.clone()).collect();
                    
                    // Show Dialog
                    let mut dialog = Window::default().with_size(300, 200).with_label("Switch Branch");
                    dialog.set_border(true);
                    let mut pack = Pack::new(10, 10, 280, 180, "");
                    pack.set_spacing(10);
                    
                    pack.add(&Frame::default().with_size(0, 20).with_label(&format!("Select branch (from {} repos):", total_sel)));
                    let mut choice = fltk::menu::Choice::default().with_size(0, 30);
                    for (name, count) in &sorted_branches {
                        // Escape slashes in branch name to avoid FLTK interpreting them as submenus
                        let safe_name = name.replace("/", "\\/");
                        choice.add_choice(&format!("{} ({})", safe_name, count));
                    }
                    if !sorted_branches.is_empty() {
                        choice.set_value(0);
                    }
                    
                    pack.add(&Frame::default().with_size(0, 20).with_label("Or type branch name:"));
                    let input = fltk::input::Input::default().with_size(0, 30);
                    
                    let btn_row = Flex::default().with_size(0, 30).row();
                    let mut btn_cancel = Button::default().with_label("Close");
                    let mut btn_ok = Button::default().with_label("Switch");
                    btn_row.end();
                    
                    pack.end();
                    dialog.end();
                    dialog.make_modal(true);
                    dialog.show();
                    
                    let s_clone = s.clone();
                    let mut d_clone = dialog.clone();
                    btn_cancel.set_callback(move |_| d_clone.hide());
                    
                    let mut d_clone2 = dialog.clone();
                    let names_clone = branch_names.clone();

                    btn_ok.set_callback(move |_| {
                        let idx = choice.value();
                        let target = if !input.value().is_empty() {
                            input.value()
                        } else if idx >= 0 && (idx as usize) < names_clone.len() {
                                names_clone[idx as usize].clone()
                        } else {
                            String::new()
                        };
                        
                        if !target.is_empty() {
                            s_clone.send(Message::SwitchBranch(target));
                            d_clone2.hide();
                        }
                    });
                }
                Message::SwitchBranch(target_branch) => {
                     let sel = get_selected_repos(&browser, &repositories.lock().unwrap());
                     if sel.is_empty() { continue; }
                     
                     status_bar.set_label(&format!("Switching to {}...", target_branch));
                     let sender = s.clone();
                     
                     for r in &sel {
                         sender.send(Message::SetStatus(r.path.clone(), "Switching...".to_string()));
                     }
                     
                     thread::spawn(move || {
                         sel.par_iter().for_each(|repo| {
                             let mut r = repo.clone();
                             let res = r.update_branch(&target_branch);
                             r.refresh();
                             match res {
                                 Ok(_) => {
                                     r.last_status = "Switched".to_string();
                                     sender.send(Message::RepoUpdated(r));
                                 }
                                 Err(e) => {
                                     r.last_status = format!("Error: {}", e);
                                     sender.send(Message::RepoUpdated(r));
                                 }
                             }
                         });
                         sender.send(Message::SetGlobalStatus("Ready".into()));
                     });
                }
                Message::Commit => {
                    let sel = get_selected_repos(&browser, &repositories.lock().unwrap());
                    if sel.len() != 1 {
                         dialog::alert(200, 200, "Please select exactly one repository for commit.");
                         continue;
                    }
                    let repo = sel[0].clone();
                    if let Some(msg_txt) = dialog::input(200, 200, "Commit message:", "") {
                        if !msg_txt.is_empty() {
                            let sender = s.clone();
                            sender.send(Message::SetStatus(repo.path.clone(), "Committing...".to_string()));
                            
                            thread::spawn(move || {
                                let mut updated_repo = repo.clone();
                                let res = updated_repo.commit(&msg_txt);
                                updated_repo.refresh();

                                match res {
                                    Ok(_) => {
                                         updated_repo.last_status = "Committed".to_string();
                                         sender.send(Message::RepoUpdated(updated_repo));
                                    },
                                    Err(e) => {
                                         updated_repo.last_status = format!("Error: {}", e);
                                         sender.send(Message::RepoUpdated(updated_repo));
                                    }
                                }
                                sender.send(Message::SetGlobalStatus("Ready".into()));
                            });
                        }
                    }
                }
                Message::SetGlobalStatus(msg) => {
                    status_bar.set_label(&msg);
                }
            }
        }
    }
}

fn scan_repositories(dirs: Vec<PathBuf>, sender: app::Sender<Message>) {
    sender.send(Message::SetGlobalStatus("Walking directories...".into()));
    let mut found_repos = Vec::new();
    
    // We can't par_iter WalkDir obviously, but we can notify progress.
    // Iteration is fast enough usually.
    for dir in dirs {
        sender.send(Message::SetGlobalStatus(format!("Walking {}...", dir.display())));
        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_dir() && entry.file_name() == ".hg" {
                if let Some(parent) = entry.path().parent() {
                    found_repos.push(parent.to_path_buf());
                }
            }
        }
    }

    sender.send(Message::SetGlobalStatus(format!("Analyzing {} repositories...", found_repos.len())));

    let valid_repos: Vec<Repository> = found_repos.par_iter().map(|p| {
        let mut r = Repository::new(p.clone());
        r.refresh();
        r
    }).collect();

    sender.send(Message::ScanComplete(valid_repos));
    sender.send(Message::SetGlobalStatus("Ready".into()));
}

fn update_browser(browser: &mut HoldBrowser, repos: &[Repository]) {
    browser.clear();
    
    for (_i, repo) in repos.iter().enumerate() {
        let path_str = repo.path.file_name().unwrap_or_default().to_string_lossy();
        let mod_str = if repo.modified { "Yes" } else { "No" };
        
        let status = &repo.last_status;
        
        let line = format!("{}\t{}\t{}\t{}\t{}\t{}", 
            path_str, 
            repo.current_branch, 
            repo.revision, 
            mod_str, 
            repo.commit_type,
            status
        );
        browser.add(&line);
    }
}

fn sort_repos(repos: &mut Vec<Repository>, state: &SortState) {
    if state.order == SortOrder::None {
        // Default sort by path
        repos.sort_by(|a, b| a.path.cmp(&b.path));
        return;
    }

    repos.sort_by(|a, b| {
        let order = match state.column {
            0 => a.path.cmp(&b.path),
            1 => a.current_branch.cmp(&b.current_branch), // Branch
            2 => a.revision.cmp(&b.revision), // Rev
            3 => a.modified.cmp(&b.modified), // Mod
            4 => a.commit_type.cmp(&b.commit_type), // Phase
            5 => a.last_status.cmp(&b.last_status), // Status
            _ => std::cmp::Ordering::Equal,
        };
        
        if state.order == SortOrder::Descending {
            order.reverse()
        } else {
            order
        }
    });
}

fn get_selected_repos(browser: &HoldBrowser, repos: &[Repository]) -> Vec<Repository> {
    let mut selected = Vec::new();
    let lines = browser.selected_items();
    for idx in lines {
        if idx > 0 { // 1-based index but no header anymore so item 1 is index 0
            let repo_idx = (idx - 1) as usize; 
            if repo_idx < repos.len() {
                selected.push(repos[repo_idx].clone());
            }
        }
    }
    selected
}

