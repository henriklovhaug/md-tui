pub mod config;
pub mod highlight;
pub mod nodes;
pub mod parser;

fn windows_mut_for_each<T>(v: &mut [T], n: usize, f: impl Fn(&mut [T])) {
    let mut start = 0;
    let mut end = n;
    while end <= v.len() {
        f(&mut v[start..end]);
        start += 1;
        end += 1;
    }
}
