use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use md_tui::nodes::root::{Component, ComponentRoot};
use md_tui::parser;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;
use ratatui::{Frame, Terminal};

const CONTENT: &str = r#"
# Mihi contigit dextra

## Copia praeda Autolyci parcite

Lorem markdownum genus, modo veniebat at viribus latus. Auxiliare fit inquit,
tenetur maciem manuque nexilibusque lucus, qui. Iuli tellus vertitur, *et*
vacavit nympha pallada.

- Terga volucresque fatale quae aut videnda rudis
- Deus multas prohibes ignis sequentis Latonae marm

```rust
// This is the main function.
fn main() {
    // Statements here are executed when the compiled binary is called.

    // Print text to the console.
    println!("Hello World!");
}

```
"#;

#[must_use]
struct App {
    markdown: Option<ComponentRoot>,
    area: Rect,
    scroll: u16,
}

impl App {
    fn new() -> Self {
        Self {
            markdown: None,
            area: Rect::default(),
            scroll: 0,
        }
    }

    fn scroll_down(&mut self) -> bool {
        if let Some(markdown) = &self.markdown {
            let len = markdown.height();
            if self.area.height > len {
                self.scroll = 0;
            } else {
                self.scroll = std::cmp::min(
                    self.scroll.saturating_add(1),
                    len.saturating_sub(self.area.height),
                )
            }
        }
        true
    }

    fn scroll_up(&mut self) -> bool {
        self.scroll = self.scroll.saturating_sub(1);
        true
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.area = frame.area();

        self.markdown = Some(parser::parse_markdown(None, CONTENT, self.area.width));

        if let Some(markdown) = &mut self.markdown {
            markdown.set_scroll(self.scroll);

            let area = Rect {
                width: self.area.width - 1,
                height: self.area.height - 1,
                x: 1,
                ..self.area
            };

            for child in markdown.children() {
                if let Component::TextComponent(comp) = child {
                    if comp.y_offset().saturating_sub(comp.scroll_offset()) >= area.height
                        || (comp.y_offset() + comp.height()).saturating_sub(comp.scroll_offset())
                            == 0
                    {
                        continue;
                    }

                    frame.render_widget(comp.clone(), area);
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    // Terminal initialization
    let mut stdout = std::io::stdout();

    terminal::enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // App
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> std::io::Result<()> {
    const DEBOUNCE: Duration = Duration::from_millis(20); // 50 FPS

    terminal.draw(|frame| app.draw(frame))?;

    let mut debounce: Option<Instant> = None;

    loop {
        let timeout = debounce.map_or(DEBOUNCE, |start| DEBOUNCE.saturating_sub(start.elapsed()));
        if crossterm::event::poll(timeout)? {
            let update = match crossterm::event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => app.scroll_up(),
                    KeyCode::Down => app.scroll_down(),
                    _ => false,
                },
                Event::Resize(_, _) => true,
                _ => false,
            };
            if update {
                debounce.get_or_insert_with(Instant::now);
            }
        }
        if debounce.is_some_and(|debounce| debounce.elapsed() > DEBOUNCE) {
            terminal.draw(|frame| {
                app.draw(frame);
            })?;

            debounce = None;
        }
    }
}
