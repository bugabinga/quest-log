use crate::database::Database;
use crate::models::{CreateQuestRequest, CreateRewardRequest, Quest, UpdateSettingsRequest};
use chrono::Datelike;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, List, ListItem, Paragraph, Tabs},
};
use std::io;

#[derive(Clone)]
pub struct QuestWithCompletion {
    pub quest: Quest,
    pub completed: bool,
}

pub struct AppState {
    pub db: Database,
    pub quests: Vec<Vec<QuestWithCompletion>>,
    pub rewards: Vec<RewardWithClaimed>,
    pub selected_tab: Tab,
    pub selected_day: usize,
    pub selected_quest: usize,
    pub selected_reward: usize,
    pub weekly_goal: i32,
    pub message: Option<String>,
    pub current_view: View,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_exp: i32,
    pub input_day: i32,
    pub input_goal: String,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Quests,
    Rewards,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum View {
    List,
    Create,
    Delete,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InputMode {
    None,
    Title,
    Exp,
    Day,
    ConfirmDelete,
    Goal,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RewardWithClaimed {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub required_exp: i32,
    pub is_active: bool,
}

#[allow(dead_code)]
impl AppState {
    pub async fn new() -> io::Result<Self> {
        let db = Database::new().await.map_err(io::Error::other)?;
        let today = chrono::Utc::now()
            .date_naive()
            .weekday()
            .num_days_from_sunday() as usize;

        let mut quests: Vec<Vec<QuestWithCompletion>> = Vec::new();

        for dow in 0..7 {
            let day_quests = db.get_quests_for_day(dow).await.map_err(io::Error::other)?;
            let mut quests_with_completion = Vec::new();

            for quest in day_quests {
                let completed = db
                    .is_quest_completed_today(quest.id, chrono::Utc::now().date_naive())
                    .await
                    .unwrap_or(false);
                quests_with_completion.push(QuestWithCompletion { quest, completed });
            }
            quests.push(quests_with_completion);
        }

        let rewards_data = db.get_available_rewards().await.map_err(io::Error::other)?;
        let rewards: Vec<RewardWithClaimed> = rewards_data
            .into_iter()
            .map(|r| RewardWithClaimed {
                id: r.id,
                title: r.title,
                description: r.description,
                required_exp: r.required_exp,
                is_active: r.is_active,
            })
            .collect();

        let settings = db.get_settings().await.map_err(io::Error::other)?;

        Ok(Self {
            db,
            quests,
            rewards,
            selected_tab: Tab::Quests,
            selected_day: today,
            selected_quest: 0,
            selected_reward: 0,
            weekly_goal: settings.weekly_exp_goal,
            message: None,
            current_view: View::List,
            input_mode: InputMode::None,
            input_buffer: String::new(),
            input_exp: 10,
            input_day: today as i32,
            input_goal: settings.weekly_exp_goal.to_string(),
        })
    }

    fn get_weekly_exp(&self) -> (i32, i32) {
        let mut completed = 0i32;
        let mut total = 0i32;

        for day_quests in &self.quests {
            for q in day_quests {
                total += q.quest.exp_value;
                if q.completed {
                    completed += q.quest.exp_value;
                }
            }
        }

        (completed, total)
    }

    /// Get total quest count across all days
    pub fn total_quest_count(&self) -> usize {
        self.quests.iter().map(|day| day.len()).sum()
    }

    /// Get quest count for a specific day
    pub fn quest_count_for_day(&self, day: usize) -> usize {
        self.quests.get(day).map(|q| q.len()).unwrap_or(0)
    }

    /// Check if current view is List
    pub fn is_view_list(&self) -> bool {
        self.current_view == View::List
    }

    /// Check if current view is Create
    pub fn is_view_create(&self) -> bool {
        self.current_view == View::Create
    }

    /// Check if current view is Help
    pub fn is_view_help(&self) -> bool {
        self.current_view == View::Help
    }

    /// Get current tab
    pub fn current_tab(&self) -> Tab {
        self.selected_tab
    }

    /// Get current day
    pub fn current_day(&self) -> usize {
        self.selected_day
    }

    /// Get current quest index
    pub fn current_quest_index(&self) -> usize {
        self.selected_quest
    }

    /// Get reward count
    pub fn reward_count(&self) -> usize {
        self.rewards.len()
    }

    async fn refresh(&mut self) -> io::Result<()> {
        for dow in 0..7 {
            let day_quests = self
                .db
                .get_quests_for_day(dow as i32)
                .await
                .map_err(io::Error::other)?;
            self.quests[dow].clear();

            for quest in day_quests {
                let completed = self
                    .db
                    .is_quest_completed_today(quest.id, chrono::Utc::now().date_naive())
                    .await
                    .unwrap_or(false);
                self.quests[dow].push(QuestWithCompletion { quest, completed });
            }
        }

        let rewards_data = self
            .db
            .get_available_rewards()
            .await
            .map_err(io::Error::other)?;
        self.rewards = rewards_data
            .into_iter()
            .map(|r| RewardWithClaimed {
                id: r.id,
                title: r.title,
                description: r.description,
                required_exp: r.required_exp,
                is_active: r.is_active,
            })
            .collect();

        let settings = self.db.get_settings().await.map_err(io::Error::other)?;
        self.weekly_goal = settings.weekly_exp_goal;

        Ok(())
    }

    async fn create_quest(&mut self) -> io::Result<()> {
        if self.input_buffer.trim().is_empty() {
            self.message = Some("❌ Title cannot be empty".to_string());
            return Ok(());
        }

        let req = CreateQuestRequest {
            title: self.input_buffer.clone(),
            description: None,
            exp_value: Some(self.input_exp),
            day_of_week: self.input_day,
        };

        self.db.create_quest(req).await.map_err(io::Error::other)?;

        self.message = Some("✅ Quest created!".to_string());
        self.input_buffer.clear();
        self.input_exp = 10;

        self.refresh().await?;
        self.current_view = View::List;
        self.input_mode = InputMode::None;

        Ok(())
    }

    async fn create_reward(&mut self) -> io::Result<()> {
        if self.input_buffer.trim().is_empty() {
            self.message = Some("❌ Title cannot be empty".to_string());
            return Ok(());
        }

        let req = CreateRewardRequest {
            title: self.input_buffer.clone(),
            description: None,
            required_exp: self.input_exp,
        };

        self.db.create_reward(req).await.map_err(io::Error::other)?;

        self.message = Some("✅ Reward created!".to_string());
        self.input_buffer.clear();
        self.input_exp = 50;

        self.refresh().await?;
        self.current_view = View::List;
        self.input_mode = InputMode::None;

        Ok(())
    }

    async fn delete_quest(&mut self) -> io::Result<()> {
        let day_len = self.quests[self.selected_day].len();
        if day_len > 0 && self.selected_quest < day_len {
            let quest = &self.quests[self.selected_day][self.selected_quest];
            self.db
                .delete_quest(quest.quest.id)
                .await
                .map_err(io::Error::other)?;

            self.message = Some("🗑️ Quest deleted".to_string());
            self.selected_quest = self.selected_quest.saturating_sub(1);

            self.refresh().await?;
        }

        self.current_view = View::List;
        self.input_mode = InputMode::None;

        Ok(())
    }

    async fn toggle_quest(&mut self) -> io::Result<()> {
        let day_len = self.quests[self.selected_day].len();
        if day_len > 0 && self.selected_quest < day_len {
            let quest = &self.quests[self.selected_day][self.selected_quest];
            let today = chrono::Utc::now().date_naive();

            self.db
                .toggle_quest_completion(quest.quest.id, today)
                .await
                .map_err(io::Error::other)?;

            self.refresh().await?;
            self.message = Some("🎯 Toggled quest".to_string());
        }

        Ok(())
    }

    async fn save_settings(&mut self) -> io::Result<()> {
        let goal: i32 = self.input_goal.parse().unwrap_or(self.weekly_goal);

        let req = UpdateSettingsRequest {
            weekly_exp_goal: goal,
        };
        self.db
            .update_settings(req)
            .await
            .map_err(io::Error::other)?;

        self.weekly_goal = goal;
        self.message = Some("✅ Settings saved!".to_string());
        self.current_view = View::List;
        self.input_mode = InputMode::None;

        Ok(())
    }
}

pub async fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new().await?;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
) -> io::Result<()> {
    let tab_names = ["📋 Quests", "🎁 Rewards", "⚙️ Settings"];

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            let title = Paragraph::new("✨ QUEST LOG")
                .style(Style::default().fg(Color::Yellow).bold())
                .block(Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title_style(Style::default().fg(Color::Yellow)));
            f.render_widget(title, chunks[0]);

            let tabs = Tabs::new(tab_names.iter().cloned().map(Line::from).collect::<Vec<_>>())
                .block(Block::bordered().title("Tabs"))
                .select(app.selected_tab as usize)
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow).bold());
            f.render_widget(tabs, chunks[1]);

            let main_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(Rect::new(chunks[1].x + 1, chunks[1].y + 1, chunks[1].width - 2, chunks[1].height - 2));

            match app.selected_tab {
                Tab::Quests => render_quests_tab(f, app, main_area[0]),
                Tab::Rewards => render_rewards_tab(f, app, main_area[0]),
                Tab::Settings => render_settings_tab(f, app, main_area[0]),
            }

            let footer_text = match (app.current_view, app.selected_tab) {
                (View::Help, _) => "Esc Back",
                (View::Create, Tab::Quests) => "Type title | Tab exp/day | Enter create | Esc cancel",
                (View::Create, Tab::Rewards) => "Type title | Tab exp | Enter create | Esc cancel",
                (View::Create, Tab::Settings) => "Type goal | Enter save | Esc cancel",
                (View::Delete, _) => "Enter/y confirm | n/Esc cancel",
                (View::List, Tab::Quests) => "←→ days | ↑↓ quests | Enter toggle | n new | d delete | t rewards | s settings | h help | q quit",
                (View::List, Tab::Rewards) => "↑↓ rewards | n new | d delete | t quests | s settings | h help | q quit",
                (View::List, Tab::Settings) => "Type goal | Enter save | t quests | r rewards | h help | q quit",
            };

            let footer = Paragraph::new(Line::from(footer_text).centered())
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::bordered().border_type(BorderType::Plain));
            f.render_widget(footer, chunks[2]);

            if let Some(ref msg) = app.message {
                let popup = Paragraph::new(msg.clone())
                    .style(Style::default().fg(Color::White).bg(Color::DarkGray))
                    .block(Block::bordered().border_type(BorderType::Rounded));
                let area = Rect::new(
                    chunks[1].x + 5,
                    chunks[1].y + 2,
                    chunks[1].width - 10,
                    3,
                );
                f.render_widget(popup, area);
            }

            if app.current_view == View::Create {
                render_create_popup(f, app, chunks[1]);
            }

            if app.current_view == View::Delete {
                render_delete_popup(f, app, chunks[1]);
            }

            if app.current_view == View::Help {
                render_help_popup(f, app, chunks[1]);
            }
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.current_view {
                View::Help => {
                    if key.code == KeyCode::Esc {
                        app.current_view = View::List;
                    }
                }
                View::Create => handle_create_input(app, key).await?,
                View::Delete => handle_delete_input(app, key).await?,
                View::List => handle_list_input(app, key).await?,
            }
        }

        app.message = None;
    }
}

fn render_quests_tab(f: &mut Frame, app: &mut AppState, area: Rect) {
    let day_names = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let day_colors = [
        Color::Red,
        Color::Blue,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::LightRed,
    ];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Min(0)])
        .split(area);

    let mut tab_titles = Vec::new();
    for (i, name) in day_names.iter().enumerate() {
        let is_today = i == app.selected_day;
        let style = if is_today {
            Style::default().fg(day_colors[i]).bold()
        } else {
            Style::default().fg(Color::White)
        };
        tab_titles.push(Line::from(*name).style(style));
    }

    let tabs = Tabs::new(tab_titles)
        .select(app.selected_day)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).bold());
    f.render_widget(tabs, chunks[0]);

    let day_quests = &app.quests[app.selected_day];
    let mut list_items = Vec::new();

    for (i, q) in day_quests.iter().enumerate() {
        let is_selected = i == app.selected_quest;
        let checkbox = if q.completed { "☑" } else { "☐" };
        let status = if q.completed { "✅" } else { "  " };
        let exp_str = format!("{} EXP", q.quest.exp_value);

        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::LightBlue)
        } else if q.completed {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let item = ListItem::new(format!(
            "{} {} [{}] {}",
            status, checkbox, exp_str, q.quest.title
        ))
        .style(style);
        list_items.push(item);
    }

    let list = List::new(list_items).block(
        Block::bordered()
            .title(format!("📋 {} Quests", day_names[app.selected_day]))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, chunks[1]);

    let (completed_exp, total_exp) = app.get_weekly_exp();
    let progress = if total_exp > 0 {
        (completed_exp as f64 / total_exp as f64 * 100.0) as u16
    } else {
        0
    };

    let progress_bar = format!(
        "📊 Weekly: {}/{} EXP [{:>3}%] {}{}",
        completed_exp,
        total_exp,
        progress,
        "█".repeat(progress as usize / 5),
        "░".repeat(20 - (progress as usize / 5))
    );

    let stats = Paragraph::new(Line::from(progress_bar).centered())
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::bordered()
                .title("📈 Progress")
                .border_type(BorderType::Rounded),
        );
    f.render_widget(
        stats,
        Rect::new(chunks[1].x, area.y + area.height - 4, chunks[1].width, 3),
    );
}

fn render_rewards_tab(f: &mut Frame, app: &mut AppState, area: Rect) {
    let mut list_items = Vec::new();

    for (i, r) in app.rewards.iter().enumerate() {
        let is_selected = i == app.selected_reward;
        let exp_str = format!("{} EXP", r.required_exp);

        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::LightBlue)
        } else {
            Style::default().fg(Color::White)
        };

        let item = ListItem::new(format!("[{}] {} ({})", exp_str, r.title, r.title)).style(style);
        list_items.push(item);
    }

    if list_items.is_empty() {
        list_items.push(
            ListItem::new("No rewards yet - create one!")
                .style(Style::default().fg(Color::DarkGray)),
        );
    }

    let list = List::new(list_items).block(
        Block::bordered()
            .title("🎁 Available Rewards")
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, area);
}

fn render_settings_tab(f: &mut Frame, app: &mut AppState, area: Rect) {
    let (completed_exp, total_exp) = app.get_weekly_exp();

    let content = vec![
        Line::from("⚙️ Settings").style(Style::default().fg(Color::Yellow).bold()),
        Line::from(""),
        Line::from(format!("Weekly EXP Goal: {}", app.weekly_goal)),
        Line::from(format!(
            "Current Week: {} / {} EXP",
            completed_exp, total_exp
        )),
        Line::from(""),
        Line::from("Type new goal and press Enter to save")
            .style(Style::default().fg(Color::DarkGray)),
    ];

    let block = Paragraph::new(content).block(
        Block::bordered()
            .title("⚙️ Settings")
            .border_type(BorderType::Rounded),
    );
    f.render_widget(block, area);
}

fn render_create_popup(f: &mut Frame, app: &mut AppState, area: Rect) {
    let popup_area = Rect::new(area.x + 5, area.y + 3, area.width - 10, 8);

    let (title, _input_title, _input_exp, _input_day) = match app.selected_tab {
        Tab::Quests => ("✨ New Quest", "Title: ", "EXP: ", "Day: "),
        Tab::Rewards => ("🎁 New Reward", "Title: ", "EXP required: ", ""),
        Tab::Settings => return,
    };

    let _input_value = match app.input_mode {
        InputMode::Title => &app.input_buffer,
        InputMode::Exp => &app.input_exp.to_string(),
        InputMode::Day => &app.input_day.to_string(),
        _ => "",
    };

    let marker = if app.input_mode == InputMode::Title {
        "▶"
    } else {
        " "
    };

    let content = if app.selected_tab == Tab::Quests {
        format!(
            "{}{}\n{}{}\n{}{} (0=Sun, 6=Sat)",
            marker,
            app.input_buffer,
            if app.input_mode == InputMode::Exp {
                "▶"
            } else {
                " "
            },
            app.input_exp,
            if app.input_mode == InputMode::Day {
                "▶"
            } else {
                " "
            },
            app.input_day
        )
    } else {
        format!(
            "{}{}\n{}{}",
            marker,
            app.input_buffer,
            if app.input_mode == InputMode::Exp {
                "▶"
            } else {
                " "
            },
            app.input_exp
        )
    };

    let block = Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let text = Paragraph::new(content).style(Style::default().fg(Color::White));

    f.render_widget(block, popup_area);
    f.render_widget(
        text,
        Rect::new(
            popup_area.x + 1,
            popup_area.y + 1,
            popup_area.width - 2,
            popup_area.height - 2,
        ),
    );
}

fn render_delete_popup(f: &mut Frame, app: &mut AppState, area: Rect) {
    let popup_area = Rect::new(area.x + 5, area.y + 3, area.width - 10, 5);

    let name = match app.selected_tab {
        Tab::Quests => {
            let day_len = app.quests[app.selected_day].len();
            if day_len > 0 && app.selected_quest < day_len {
                app.quests[app.selected_day][app.selected_quest]
                    .quest
                    .title
                    .clone()
            } else {
                "quest".to_string()
            }
        }
        Tab::Rewards => {
            if app.selected_reward < app.rewards.len() {
                app.rewards[app.selected_reward].title.clone()
            } else {
                "reward".to_string()
            }
        }
        Tab::Settings => "settings".to_string(),
    };

    let block = Block::bordered()
        .title("🗑️ Confirm Delete")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    let text = Paragraph::new(format!(
        "Delete '{}'?\nPress Enter or y to confirm, n or Esc to cancel",
        name
    ))
    .style(Style::default().fg(Color::White));

    f.render_widget(block, popup_area);
    f.render_widget(
        text,
        Rect::new(
            popup_area.x + 1,
            popup_area.y + 1,
            popup_area.width - 2,
            popup_area.height - 2,
        ),
    );
}

fn render_help_popup(f: &mut Frame, _app: &mut AppState, area: Rect) {
    let popup_area = Rect::new(area.x + 3, area.y + 2, area.width - 6, 14);

    let content = "\
↑↓         Navigate items
←→         Navigate days (Quests tab)
Enter      Toggle quest / Save settings
n          Create new item
d          Delete selected item
t          Switch to Quests tab
r          Switch to Rewards tab
s          Switch to Settings tab
h          Toggle this help
q          Quit
Esc        Cancel / Close";

    let block = Block::bordered()
        .title("❓ Help")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let text = Paragraph::new(content).style(Style::default().fg(Color::White));

    f.render_widget(block, popup_area);
    f.render_widget(
        text,
        Rect::new(
            popup_area.x + 1,
            popup_area.y + 1,
            popup_area.width - 2,
            popup_area.height - 2,
        ),
    );
}

async fn handle_list_input(app: &mut AppState, key: crossterm::event::KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') => {
            std::process::exit(0);
        }
        KeyCode::Char('h') => {
            app.current_view = View::Help;
        }
        KeyCode::Char('t') => {
            app.selected_tab = Tab::Quests;
        }
        KeyCode::Char('r') => {
            app.selected_tab = Tab::Rewards;
        }
        KeyCode::Char('s') => {
            app.selected_tab = Tab::Settings;
        }
        KeyCode::Char('n') => {
            app.current_view = View::Create;
            app.input_mode = InputMode::Title;
            app.input_buffer.clear();
            app.input_exp = match app.selected_tab {
                Tab::Quests => 10,
                Tab::Rewards => 50,
                Tab::Settings => app.weekly_goal,
            };
            app.input_day = app.selected_day as i32;
        }
        KeyCode::Char('d') => {
            let has_items = match app.selected_tab {
                Tab::Quests => !app.quests[app.selected_day].is_empty(),
                Tab::Rewards => !app.rewards.is_empty(),
                Tab::Settings => false,
            };
            if has_items {
                app.current_view = View::Delete;
                app.input_mode = InputMode::ConfirmDelete;
            }
        }
        KeyCode::Tab => {
            app.selected_tab = match app.selected_tab {
                Tab::Quests => Tab::Rewards,
                Tab::Rewards => Tab::Settings,
                Tab::Settings => Tab::Quests,
            };
        }
        KeyCode::Left => {
            if app.selected_tab == Tab::Quests {
                if app.selected_day > 0 {
                    app.selected_day -= 1;
                } else {
                    app.selected_day = 6;
                }
                app.selected_quest = 0;
            }
        }
        KeyCode::Right => {
            if app.selected_tab == Tab::Quests {
                if app.selected_day < 6 {
                    app.selected_day += 1;
                } else {
                    app.selected_day = 0;
                }
                app.selected_quest = 0;
            }
        }
        KeyCode::Up => match app.selected_tab {
            Tab::Quests => {
                if app.selected_quest > 0 {
                    app.selected_quest -= 1;
                }
            }
            Tab::Rewards => {
                if app.selected_reward > 0 {
                    app.selected_reward -= 1;
                }
            }
            Tab::Settings => {}
        },
        KeyCode::Down => match app.selected_tab {
            Tab::Quests => {
                let day_len = app.quests[app.selected_day].len();
                if day_len > 0 && app.selected_quest + 1 < day_len {
                    app.selected_quest += 1;
                }
            }
            Tab::Rewards => {
                if app.selected_reward + 1 < app.rewards.len() {
                    app.selected_reward += 1;
                }
            }
            Tab::Settings => {}
        },
        KeyCode::Enter => match app.selected_tab {
            Tab::Quests => {
                app.toggle_quest().await?;
            }
            Tab::Rewards => {}
            Tab::Settings => {
                app.save_settings().await?;
            }
        },
        _ => {}
    }
    Ok(())
}

async fn handle_create_input(
    app: &mut AppState,
    key: crossterm::event::KeyEvent,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.current_view = View::List;
            app.input_mode = InputMode::None;
            app.input_buffer.clear();
        }
        KeyCode::Tab => {
            app.input_mode = match app.selected_tab {
                Tab::Quests => match app.input_mode {
                    InputMode::Title => InputMode::Exp,
                    InputMode::Exp => InputMode::Day,
                    InputMode::Day => InputMode::Title,
                    _ => InputMode::Title,
                },
                Tab::Rewards => match app.input_mode {
                    InputMode::Title => InputMode::Exp,
                    InputMode::Exp => InputMode::Title,
                    _ => InputMode::Title,
                },
                Tab::Settings => InputMode::Goal,
            };
        }
        KeyCode::Char(c) => match app.input_mode {
            InputMode::Title => {
                app.input_buffer.push(c);
            }
            InputMode::Exp => {
                if c.is_numeric() {
                    app.input_exp = app.input_exp * 10 + c.to_digit(10).unwrap() as i32;
                }
            }
            InputMode::Day => {
                if c.is_numeric() {
                    let d = c.to_digit(10).unwrap() as i32;
                    if d <= 6 {
                        app.input_day = d;
                    }
                }
            }
            InputMode::Goal => {
                if c.is_numeric() {
                    app.input_goal.push(c);
                }
            }
            _ => {}
        },
        KeyCode::Backspace => match app.input_mode {
            InputMode::Title => {
                app.input_buffer.pop();
            }
            InputMode::Exp => {
                app.input_exp /= 10;
            }
            InputMode::Goal => {
                app.input_goal.pop();
            }
            _ => {}
        },
        KeyCode::Enter => match app.selected_tab {
            Tab::Quests => app.create_quest().await?,
            Tab::Rewards => app.create_reward().await?,
            Tab::Settings => app.save_settings().await?,
        },
        _ => {}
    }
    Ok(())
}

async fn handle_delete_input(
    app: &mut AppState,
    key: crossterm::event::KeyEvent,
) -> io::Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => {
            app.current_view = View::List;
            app.input_mode = InputMode::None;
        }
        KeyCode::Enter | KeyCode::Char('y') => match app.selected_tab {
            Tab::Quests => app.delete_quest().await?,
            Tab::Rewards => {
                if app.selected_reward < app.rewards.len() {
                    app.message = Some("⚠️ Reward deletion not implemented".to_string());
                }
                app.current_view = View::List;
                app.input_mode = InputMode::None;
            }
            Tab::Settings => {}
        },
        _ => {}
    }
    Ok(())
}
