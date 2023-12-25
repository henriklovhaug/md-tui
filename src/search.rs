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
