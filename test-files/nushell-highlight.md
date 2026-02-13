# Nushell Syntax Highlighting

Test file for Nushell tree-sitter highlighting.

## Variables and strings

```nu
let name = "world"
mut counter = 0
const MAX = 100

print $"Hello, ($name)!"
print $'literal: ($counter)'
```

## Control flow

```nu
if $counter < $MAX {
    $counter += 1
} else {
    print "done"
}

for item in [1 2 3] {
    print $item
}

loop {
    if $counter >= 10 { break }
    $counter += 1
}

match $name {
    "world" => { print "hi" }
    _ => { print "who?" }
}
```

## Functions and types

```nu
def greet [name: string, --loud(-l)] -> string {
    if $loud {
        $name | str upcase
    } else {
        $name
    }
}

def sum-all [...nums: int] -> int {
    $nums | math sum
}
```

## Pipelines and builtins

```nu
ls | where size > 1kb | sort-by modified | first 5

open data.csv | select name age | where age > 30

ps | where cpu > 10 | each { |row| $row.name }

seq 1 100 | par-each { |n| $n * $n } | math sum

http get https://example.com/api | from json | get data
```

## Records and tables

```nu
let person = { name: "Alice", age: 30 }
let scores = [[name, score]; ["Alice", 95], ["Bob", 87]]

$person | get name
$scores | sort-by score | reverse
```

## Error handling

```nu
try {
    open missing.txt
} catch {
    print "file not found"
}
```

## Operators and ranges

```nu
let x = 2 + 3 * 4
let flag = not false
let items = 1..10
let half = 0..<5
let slice = 0..=3
```

## Redirects and externals

```nu
^cargo build o+e>| save build.log
ls | save files.txt
```

## Modules

```nu
module utils {
    export def double [n: int] -> int {
        $n * 2
    }
}

use utils double
```
