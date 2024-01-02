use strsim::damerau_levenshtein;

pub fn find_with_backoff(query: &str, text: &str) -> Vec<usize> {
    let precision = 0;
    let mut result = find(query, text, precision);
    if result.is_empty() {
        let precision = 1;
        result = find(query, text, precision);
    }
    result
}

pub fn find(query: &str, text: &str, precision: usize) -> Vec<usize> {
    let mut result = Vec::new();

    char_windows(text, query.len())
        .enumerate()
        .for_each(|(i, window)| {
            let score = damerau_levenshtein(query, window);
            if score <= precision {
                result.push(i);
            }
        });

    result
}

/// Returns line numbers that match the query with the given precision.
pub fn line_match(query: &str, text: Vec<&str>, precision: usize) -> Vec<usize> {
    text.iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if find(query, line, precision).is_empty() {
                None
            } else {
                Some(i)
            }
        })
        .collect()
}

pub fn find_line_match_and_index(
    query: &str,
    line: Vec<&str>,
    precision: usize,
) -> Vec<(usize, usize)> {
    line.iter()
        .enumerate()
        .flat_map(|(i, line)| {
            find(query, line, precision)
                .into_iter()
                .map(move |j| (i, j))
        })
        .collect()
}

fn char_windows(src: &str, win_size: usize) -> impl Iterator<Item = &'_ str> {
    src.char_indices().flat_map(move |(from, _)| {
        src[from..]
            .char_indices()
            .nth(win_size - 1)
            .map(|(to, c)| &src[from..from + to + c.len_utf8()])
    })
}

#[cfg(test)]
#[test]
fn test_find() {
    let text = "Hello, world!";
    let query = "world";
    let precision = 0;
    let result = find(query, text, precision);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_find_with_backoff() {
    let text = "Hello, world!";
    let query = "world";
    let result = find_with_backoff(query, text);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_find_with_backoff_with_typo() {
    let text = "Hello, world!";
    let query = "wrold";
    let result = find_with_backoff(query, text);
    assert_eq!(result, vec![7]);
}

#[test]
fn test_vec_find() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "world";
    let precision = 0;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2]);
}

#[test]
fn test_vec_find_less_precision() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "world";
    let precision = 1;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_vec_find_with_typo() {
    let text = vec!["Hello", "hello", "world", "World"];
    let query = "wrold";
    let precision = 2;
    let result = line_match(query, text, precision);
    assert_eq!(result, vec![2, 3]);
}

#[test]
fn test_find_line_match_and_index() {
    let text = vec!["Hello", "hello", "world", "hello world"];
    let query = "world";
    let precision = 0;
    let result = find_line_match_and_index(query, text, precision);
    assert_eq!(result, vec![(2, 0), (3, 6)]);
}
