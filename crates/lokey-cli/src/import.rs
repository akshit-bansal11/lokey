//! Parses pasted pairs for `lokey import`. Two shapes are accepted, in any mix:
//!
//! ```text
//! DATABASE_URL=postgres://...        .env style, one pair per line
//! export API_KEY="sk-123"            shell style
//! key1,value1; key2,value2;          the paste format Vercel and similar accept
//! ```
//!
//! A line whose first delimiter is `=` is one pair and its value may contain
//! `;` and `,`. A line whose first delimiter is `,` may hold several pairs
//! separated by `;`, so those values cannot contain `;`.

#[derive(Debug, PartialEq, Eq)]
pub struct Parsed {
    pub pairs: Vec<(String, String)>,
    /// 1-based line numbers that are not a recognisable pair.
    pub bad_lines: Vec<usize>,
}

pub fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed {
        pairs: Vec::new(),
        bad_lines: Vec::new(),
    };
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if line.is_empty() || line.starts_with('#') || line == "\"" {
            continue;
        }
        let chunks: Vec<&str> = match line.find(['=', ',']) {
            Some(at) if line[at..].starts_with('=') => vec![line.strip_suffix(';').unwrap_or(line)],
            Some(_) => line.split(';').collect(),
            None => vec![line],
        };
        for chunk in chunks {
            let chunk = unquote(chunk.trim());
            if chunk.is_empty() {
                continue;
            }
            match pair(chunk) {
                Some(found) => parsed.pairs.push(found),
                None => {
                    parsed.bad_lines.push(index + 1);
                    break;
                }
            }
        }
    }
    parsed
}

fn pair(chunk: &str) -> Option<(String, String)> {
    let at = chunk.find(['=', ','])?;
    let name = unquote(chunk[..at].trim());
    let value = unquote(chunk[at + 1..].trim());
    (!name.is_empty()).then(|| (name.to_string(), value.to_string()))
}

/// Strips one layer of matching single or double quotes.
fn unquote(text: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|q| text.strip_prefix(*q)?.strip_suffix(*q))
        .unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn parse_reads_dotenv_lines() {
        let parsed = parse("A=1\nexport B=\"two words\"\n# comment\n\nC='x;y,z'\n");

        assert_eq!(
            parsed.pairs,
            pairs(&[("A", "1"), ("B", "two words"), ("C", "x;y,z")])
        );
    }

    #[test]
    fn parse_reads_comma_semicolon_pairs_on_one_line() {
        let parsed = parse("\"\nkey1,value1; key2,value2;\n\"");

        assert_eq!(
            parsed.pairs,
            pairs(&[("key1", "value1"), ("key2", "value2")])
        );
    }

    #[test]
    fn parse_splits_on_the_first_delimiter_only() {
        let parsed = parse("URL,https://h/?a=1,2\nX=a=b");

        assert_eq!(
            parsed.pairs,
            pairs(&[("URL", "https://h/?a=1,2"), ("X", "a=b")])
        );
    }

    #[test]
    fn parse_reports_line_numbers_of_bad_lines() {
        let parsed = parse("A=1\njust some words\n=novalue");

        assert_eq!(parsed.bad_lines, vec![2, 3]);
    }
}
