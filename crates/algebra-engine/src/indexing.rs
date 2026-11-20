//! Indexing and shape queries on matrix and vector literals (after matrix variables are expanded):
//! `A[1,2]`, `A[1][2]`, `A[2]` (a row), `A[1,:]`, `A[:,2]`, `v[3]`, `v[-1]`, `row(A, 1)`, `col(A, 2)`, `size(A)`, `rows(A)`,
//! `cols(A)`, `diag(A)`, and element assignment `A[1,2] = 7`, `v[2] = 99` on stored variables.
//!
//! Indices start at 1; a negative index counts from the end (`-1` is the last).

use crate::session::UraeSession;

/// A rectangular grid of cell texts. A flat list `[a, b, c]` is a vector.
struct Grid {
    rows: Vec<Vec<String>>,
    vector: bool,
}

fn split_top(text: &str) -> Vec<String> {
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// The balanced bracket group at the start of `text`: `(group, rest)`.
fn leading_group(text: &str) -> Option<(&str, &str)> {
    let t = text.trim_start();
    if !t.starts_with('[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, c) in t.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&t[..=i], &t[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_grid(text: &str) -> Option<Grid> {
    let (group, rest) = leading_group(text)?;
    if !rest.trim().is_empty() {
        return None;
    }
    let inner = &group[1..group.len() - 1];
    let items = split_top(inner);
    if items.iter().all(|i| i.starts_with('[') && i.ends_with(']')) && !items.is_empty() {
        let rows: Vec<Vec<String>> = items
            .iter()
            .map(|row| split_top(&row[1..row.len() - 1]))
            .collect();
        let width = rows[0].len();
        if rows.iter().any(|r| r.len() != width || width == 0) {
            return None;
        }
        return Some(Grid {
            rows,
            vector: false,
        });
    }
    if items.iter().any(|i| i.starts_with('[')) || items.iter().any(String::is_empty) {
        return None;
    }
    Some(Grid {
        rows: vec![items],
        vector: true,
    })
}

fn render(grid: &Grid) -> String {
    if grid.vector {
        format!("[{}]", grid.rows[0].join(","))
    } else {
        let rows: Vec<String> = grid
            .rows
            .iter()
            .map(|r| format!("[{}]", r.join(",")))
            .collect();
        format!("[{}]", rows.join(","))
    }
}

/// One index: a 1-based position (negative from the end), or `:` for everything.
enum Index {
    At(usize),
    All,
}

fn parse_index(session: &mut UraeSession, text: &str, len: usize) -> Result<Index, String> {
    let t = text.trim();
    if t == ":" {
        return Ok(Index::All);
    }
    let value = session.eval(t)?;
    let n: i64 = value
        .trim()
        .parse()
        .map_err(|_| format!("the index {t} is not an integer"))?;
    let position = if n > 0 {
        n as usize
    } else if n < 0 {
        let back = (-n) as usize;
        if back > len {
            return Err(format!("the index {n} is out of range (length {len})"));
        }
        len - back + 1
    } else {
        return Err("indices start at 1 (index 0 does not exist)".into());
    };
    if position > len {
        return Err(format!("the index {n} is out of range (length {len})"));
    }
    Ok(Index::At(position))
}

/// Apply one index group `[i]`, `[i, j]`, `[i, :]`, `[:, j]` to a grid.
fn apply(session: &mut UraeSession, grid: &Grid, indices: &[String]) -> Result<Grid, String> {
    if grid.vector {
        let [one] = indices else {
            return Err("a list takes one index".into());
        };
        return match parse_index(session, one, grid.rows[0].len())? {
            Index::At(p) => Ok(Grid {
                rows: vec![vec![grid.rows[0][p - 1].clone()]],
                vector: true,
            }),
            Index::All => Ok(Grid {
                rows: grid.rows.clone(),
                vector: true,
            }),
        };
    }
    let (rows, cols) = (grid.rows.len(), grid.rows[0].len());
    match indices {
        [one] => match parse_index(session, one, rows)? {
            Index::At(r) => Ok(Grid {
                rows: vec![grid.rows[r - 1].clone()],
                vector: true,
            }),
            Index::All => Ok(Grid {
                rows: grid.rows.clone(),
                vector: false,
            }),
        },
        [i, j] => {
            let (i, j) = (
                parse_index(session, i, rows)?,
                parse_index(session, j, cols)?,
            );
            match (i, j) {
                (Index::At(r), Index::At(c)) => Ok(Grid {
                    rows: vec![vec![grid.rows[r - 1][c - 1].clone()]],
                    vector: true,
                }),
                (Index::At(r), Index::All) => Ok(Grid {
                    rows: vec![grid.rows[r - 1].clone()],
                    vector: true,
                }),
                (Index::All, Index::At(c)) => Ok(Grid {
                    rows: grid
                        .rows
                        .iter()
                        .map(|row| vec![row[c - 1].clone()])
                        .collect(),
                    vector: false,
                }),
                (Index::All, Index::All) => Ok(Grid {
                    rows: grid.rows.clone(),
                    vector: false,
                }),
            }
        }
        _ => Err("a matrix takes one or two indices".into()),
    }
}

/// The text of a result grid: a single cell is its value, anything else a matrix or list.
fn show(session: &mut UraeSession, grid: &Grid) -> Result<String, String> {
    if grid.rows.len() == 1 && grid.rows[0].len() == 1 {
        return session.eval(&grid.rows[0][0]);
    }
    session.eval(&render(grid))
}

/// `Some(result)` when `input` is an indexing expression or a shape query on a literal.
pub fn run(session: &mut UraeSession, input: &str) -> Option<Result<String, String>> {
    let t = input.trim();
    // `[[1,2],[3,4]][1,2]`, `[10,20,30][2]`, `[[1,2],[3,4]][1][2]`.
    if let Some((group, mut rest)) = leading_group(t)
        && rest.trim_start().starts_with('[')
    {
        let mut grid = parse_grid(group)?;
        while let Some((index, tail)) = leading_group(rest) {
            let indices = split_top(&index[1..index.len() - 1]);
            match apply(session, &grid, &indices) {
                Ok(g) => grid = g,
                Err(e) => return Some(Err(e)),
            }
            rest = tail;
            if rest.trim().is_empty() {
                return Some(show(session, &grid));
            }
            if !rest.trim_start().starts_with('[') {
                return None;
            }
        }
        return None;
    }
    // Calls on a literal: row, col, size, shape, rows, cols, ncols, nrows, diag, diagonal.
    let open = t.find('(')?;
    let name = t[..open].trim().to_ascii_lowercase();
    if !t.ends_with(')') {
        return None;
    }
    let args = split_top(&t[open + 1..t.len() - 1]);
    let grid = parse_grid(args.first()?)?;
    let (rows, cols) = (grid.rows.len(), grid.rows[0].len());
    match name.as_str() {
        "size" | "shape" | "dimensions" | "dim" if args.len() == 1 => Some(Ok(if grid.vector {
            format!("{cols}")
        } else {
            format!("{rows} x {cols}")
        })),
        "rows" | "nrows" | "num_rows" if args.len() == 1 && !grid.vector => {
            Some(Ok(rows.to_string()))
        }
        "cols" | "ncols" | "num_cols" | "columns" if args.len() == 1 && !grid.vector => {
            Some(Ok(cols.to_string()))
        }
        "row" | "col" | "column" if args.len() == 2 && !grid.vector => {
            let wanted = if name == "row" {
                vec![args[1].clone(), ":".into()]
            } else {
                vec![":".into(), args[1].clone()]
            };
            Some(apply(session, &grid, &wanted).and_then(|g| show(session, &g)))
        }
        "diag" | "diagonal" if args.len() == 1 && !grid.vector => {
            let diagonal: Vec<String> = (0..rows.min(cols))
                .map(|k| grid.rows[k][k].clone())
                .collect();
            Some(show(
                session,
                &Grid {
                    rows: vec![diagonal],
                    vector: true,
                },
            ))
        }
        _ => None,
    }
}

/// `A[1,2] = 7`, `v[2] = 99`: the new text of the stored variable, or `None` when this is not an element assignment.
pub fn assign(
    session: &mut UraeSession,
    input: &str,
) -> Option<Result<(String, String, String), String>> {
    let (left, value) = input.split_once('=')?;
    if value.starts_with('=') || left.ends_with(['<', '>', '!']) {
        return None;
    }
    let left = left.trim();
    let open = left.find('[')?;
    let name = left[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let stored = session.matrix_vars.get(name)?.clone();
    let (index, tail) = leading_group(&left[open..])?;
    if !tail.trim().is_empty() {
        return None;
    }
    let indices = split_top(&index[1..index.len() - 1]);
    let mut grid = parse_grid(&stored)?;
    let new_value = match session.eval(value.trim()) {
        Ok(v) => v.replace(' ', ""),
        Err(e) => return Some(Err(e)),
    };
    let (r, c) = if grid.vector {
        let [one] = indices.as_slice() else {
            return Some(Err("a list takes one index".into()));
        };
        match parse_index(session, one, grid.rows[0].len()) {
            Ok(Index::At(p)) => (1, p),
            Ok(Index::All) => return Some(Err("assign to a single element".into())),
            Err(e) => return Some(Err(e)),
        }
    } else {
        let [i, j] = indices.as_slice() else {
            return Some(Err(
                "a matrix element needs two indices, A[i, j] = value".into()
            ));
        };
        let (rows, cols) = (grid.rows.len(), grid.rows[0].len());
        match (parse_index(session, i, rows), parse_index(session, j, cols)) {
            (Ok(Index::At(r)), Ok(Index::At(c))) => (r, c),
            (Err(e), _) | (_, Err(e)) => return Some(Err(e)),
            _ => return Some(Err("assign to a single element".into())),
        }
    };
    grid.rows[r - 1][c - 1] = new_value;
    let text = render(&grid);
    let shown = match session.eval(&text) {
        Ok(s) => s,
        Err(e) => return Some(Err(e)),
    };
    Some(Ok((name.to_string(), text, shown)))
}
