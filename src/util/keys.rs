use std::sync::LazyLock;

use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Search,
    SelectLink,
    SelectLinkAlt,
    SelectDetails,
    SearchNext,
    SearchPrevious,
    Edit,
    Hover,
    Enter,
    Escape,
    ToTop,
    ToBottom,
    Help,
    Back,
    ToFileTree,
    Sort,
    ToggleCaretMode,
    CaretLineStart,
    CaretLineEnd,
    BookmarkSetPending,
    BookmarkJumpPending,
    EnterCommentMode,
    StartCommentSelect,
    Outline,
    None,
}

#[derive(Debug)]
pub struct KeyConfig {
    pub up: char,
    pub down: char,
    pub page_up: char,
    pub page_down: char,
    pub half_page_up: char,
    pub half_page_down: char,
    pub search: char,
    pub search_next: char,
    pub search_previous: char,
    pub select_link: char,
    pub select_link_alt: char,
    pub select_details: char,
    pub edit: char,
    pub hover: char,
    pub top: char,
    pub bottom: char,
    pub back: char,
    pub file_tree: char,
    pub sort: char,
    pub toggle_caret: char,
    pub bookmark_set: char,
    pub bookmark_jump: char,
    pub outline: char,
    pub comment: char,
    pub comment_select: char,
    pub caret_line_start: char,
    pub caret_line_end: char,
    pub help: char,
    pub quit: char,
}

/// Renders a configured key for display in user-facing surfaces (help table,
/// hints). Whitespace and other invisible chars get a readable label so they
/// don't disappear in the buffer; everything else passes through verbatim.
#[must_use]
pub fn display_key(c: char) -> String {
    match c {
        ' ' => "<Space>".to_string(),
        '\t' => "<Tab>".to_string(),
        other => other.to_string(),
    }
}

#[must_use]
pub fn key_to_action(key: KeyCode) -> Action {
    match key {
        KeyCode::Char(c) => char_to_action(c),
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Right => Action::PageDown,
        KeyCode::Left => Action::PageUp,
        KeyCode::Enter => Action::Enter,
        KeyCode::Esc => Action::Escape,
        _ => Action::None,
    }
}

fn char_to_action(c: char) -> Action {
    // '/' is always a Search alias regardless of KEY_CONFIG.search.
    if c == '/' {
        return Action::Search;
    }

    // Order matters where two actions share a default binding:
    //   Outline before Sort (both default to 'o'). In the file tree
    //   `Sort | Outline` both sort, so the shared 'o' default sorts regardless
    //   of which wins; `sort` only matters when bound to a distinct key.
    //   `Action::Sort` is file-tree-only and is intentionally a no-op in the
    //   document view (there is nothing to sort there).
    // Adding an entry here is enough to wire a new key.
    let table: &[(char, Action)] = &[
        // navigation
        (KEY_CONFIG.up, Action::Up),
        (KEY_CONFIG.down, Action::Down),
        (KEY_CONFIG.page_up, Action::PageUp),
        (KEY_CONFIG.page_down, Action::PageDown),
        (KEY_CONFIG.half_page_up, Action::HalfPageUp),
        (KEY_CONFIG.half_page_down, Action::HalfPageDown),
        (KEY_CONFIG.top, Action::ToTop),
        (KEY_CONFIG.bottom, Action::ToBottom),
        (KEY_CONFIG.caret_line_start, Action::CaretLineStart),
        (KEY_CONFIG.caret_line_end, Action::CaretLineEnd),
        // search
        (KEY_CONFIG.search, Action::Search),
        (KEY_CONFIG.search_next, Action::SearchNext),
        (KEY_CONFIG.search_previous, Action::SearchPrevious),
        // selection
        (KEY_CONFIG.select_link, Action::SelectLink),
        (KEY_CONFIG.select_link_alt, Action::SelectLinkAlt),
        (KEY_CONFIG.select_details, Action::SelectDetails),
        (KEY_CONFIG.hover, Action::Hover),
        // editing / commenting
        (KEY_CONFIG.edit, Action::Edit),
        (KEY_CONFIG.comment, Action::EnterCommentMode),
        (KEY_CONFIG.comment_select, Action::StartCommentSelect),
        // mode
        (KEY_CONFIG.back, Action::Back),
        (KEY_CONFIG.file_tree, Action::ToFileTree),
        (KEY_CONFIG.outline, Action::Outline),
        (KEY_CONFIG.sort, Action::Sort),
        (KEY_CONFIG.toggle_caret, Action::ToggleCaretMode),
        // bookmarks
        (KEY_CONFIG.bookmark_set, Action::BookmarkSetPending),
        (KEY_CONFIG.bookmark_jump, Action::BookmarkJumpPending),
        // system
        (KEY_CONFIG.help, Action::Help),
    ];

    table
        .iter()
        .find(|(key, _)| *key == c)
        .map_or(Action::None, |(_, action)| *action)
}

pub static KEY_CONFIG: LazyLock<KeyConfig> = LazyLock::new(|| {
    let settings = super::load_user_config();

    KeyConfig {
        up: settings.get::<char>("up").unwrap_or('k'),
        down: settings.get::<char>("down").unwrap_or('j'),
        page_up: settings.get::<char>("page_up").unwrap_or('u'),
        page_down: settings.get::<char>("page_down").unwrap_or('d'),
        half_page_up: settings.get::<char>("half_page_up").unwrap_or('h'),
        half_page_down: settings.get::<char>("half_page_down").unwrap_or('l'),
        search: settings.get::<char>("search").unwrap_or('f'),
        select_link: settings.get::<char>("select_link").unwrap_or('s'),
        select_link_alt: settings.get::<char>("select_link_alt").unwrap_or('S'),
        select_details: settings.get::<char>("select_details").unwrap_or('D'),
        search_next: settings.get::<char>("search_next").unwrap_or('n'),
        search_previous: settings.get::<char>("search_previous").unwrap_or('N'),
        edit: settings.get::<char>("edit").unwrap_or('e'),
        hover: settings.get::<char>("hover").unwrap_or('K'),
        top: settings.get::<char>("top").unwrap_or('g'),
        bottom: settings.get::<char>("bottom").unwrap_or('G'),
        back: settings.get::<char>("back").unwrap_or('b'),
        file_tree: settings.get::<char>("file_tree").unwrap_or('t'),
        sort: settings.get::<char>("sort").unwrap_or('o'),
        toggle_caret: settings.get::<char>("toggle_caret").unwrap_or('v'),
        bookmark_set: settings.get::<char>("bookmark_set").unwrap_or('m'),
        bookmark_jump: settings.get::<char>("bookmark_jump").unwrap_or('\''),
        outline: settings.get::<char>("outline").unwrap_or('o'),
        comment: settings.get::<char>("comment").unwrap_or('c'),
        comment_select: settings.get::<char>("comment_select").unwrap_or(' '),
        caret_line_start: settings.get::<char>("caret_line_start").unwrap_or('0'),
        caret_line_end: settings.get::<char>("caret_line_end").unwrap_or('$'),
        help: settings.get::<char>("help").unwrap_or('?'),
        quit: settings.get::<char>("quit").unwrap_or('q'),
    }
});
