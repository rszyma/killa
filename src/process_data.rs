use bottom::data_collection::memory::MemHarvest;

#[derive(Clone, Default)]
pub struct KillaData {
    // todo: optimization by keeping a field of ordered ones?
    rows: Vec<crate::Row>,
    // todo: should we keep the original data? I mean this will hold original data, no?
    pub memory: MemHarvest,
}

impl From<Box<bottom::data_collection::Data>> for KillaData {
    fn from(data: Box<bottom::data_collection::Data>) -> Self {
        let rows = data
            .list_of_processes
            .unwrap_or_default()
            .iter()
            .map(|ps| crate::Row {
                program_name: ps.name.clone(),
                program_name_lowercase: ps.name.to_lowercase(),
                mem: ps.mem_usage_bytes / 1_000_000,
                cpu_perc: (((ps.cpu_usage_percent) / (num_cpus::get() as f32) * 10.0) as i32)
                    as f32
                    / 10.0,
                pid: ps.pid,
                command: ps.command.clone(),
                command_lowercase: ps.command.to_lowercase(),
                cpu_time: ps.time,
            })
            .collect();
        Self {
            rows,
            memory: data.memory.unwrap_or_default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum SearchFilterColumn {
    Any,
    Name,
    Pid,
    Command,
}

impl TryFrom<&str> for SearchFilterColumn {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "any" | "*" => Ok(SearchFilterColumn::Any),
            "name" => Ok(SearchFilterColumn::Name),
            "pid" | "id" => Ok(SearchFilterColumn::Pid),
            "cmd" | "command" => Ok(SearchFilterColumn::Command),
            other => Err(anyhow::format_err!("unknown column filter: '{other}'")),
        }
    }
}

// FUTUREWORK: Add SearchFilterType, adding/forcing specific search algos like:
// Regex, Exact, Plain, Auto [current] (Exact for PID, otherwise Plain).
#[derive(Debug)]
struct SearchFilter {
    is_negative: bool,
    column: anyhow::Result<SearchFilterColumn>,
    // NOTE: empty phrases are allowed, but will whole filter will be skipped in that case.
    phrase: String,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            is_negative: false,
            column: Ok(SearchFilterColumn::Any),
            phrase: String::new(),
        }
    }
}

#[derive(Debug)]
struct SearchFilters(Vec<SearchFilter>);

impl SearchFilters {
    fn best_effort_parse_from_string(search_phrase: &str) -> Self {
        let xs: Vec<_> = search_phrase
            .split_ascii_whitespace()
            .map(|mut search_word| {
                let mut sf = SearchFilter::default();

                if let Some(x) = search_word.strip_prefix("-") {
                    search_word = x;
                    sf.is_negative = true;
                } else {
                    sf.is_negative = false;
                }

                if let Some((typ_str, x2)) = search_word.split_once(":") {
                    // TODO: If this fails, maybe don't just return error and instead take a step
                    // back and treat the whole thing like the ":" was part of search phrase?
                    sf.column = SearchFilterColumn::try_from(typ_str);
                    search_word = x2;
                }

                sf.phrase = search_word.to_string();

                sf
            })
            .collect();
        SearchFilters(xs)
    }
}

impl KillaData {
    pub fn search(mut self, search_phrase: &str) -> Self {
        let filters = SearchFilters::best_effort_parse_from_string(&search_phrase.to_lowercase());

        self.rows.retain(|row| {
            filters.0.iter().all(|filter| {
                let s = &filter.phrase;

                if s.is_empty() {
                    return true; // The whole filter doesn't make sense if the phrase is empty.
                }

                let is_match = match &filter.column {
                    Ok(SearchFilterColumn::Any) => {
                        row.program_name_lowercase.contains(s)
                            || row.command_lowercase.contains(s)
                            || format!("{}", row.pid) == *s
                    }
                    Ok(SearchFilterColumn::Command) => row.command_lowercase.contains(s),
                    Ok(SearchFilterColumn::Name) => row.program_name_lowercase.contains(s),
                    Ok(SearchFilterColumn::Pid) => format!("{}", row.pid) == *s,
                    Err(_) => {
                        // TODO: inform user that their input is has wrong syntax.
                        // At least a red border around search box would be nice (but it might be
                        // confused with plain "no results", so we also want to display "Syntax error").
                        return false;
                    }
                };

                // XOR inverts the result if filter.is_negative is true.
                is_match ^ filter.is_negative
            })
        });

        self
    }

    pub fn sort_by_column(&mut self, col: crate::ColumnKind, order: SortOrder) -> &mut Self {
        match col {
            crate::ColumnKind::Name => {}
            crate::ColumnKind::Memory => match order {
                SortOrder::Ascending => self.rows.sort_by_key(|row| row.mem),
                SortOrder::Descending => self.rows.sort_by_key(|row| u64::MAX - row.mem),
            },
            crate::ColumnKind::CPU => match order {
                SortOrder::Ascending => self
                    .rows
                    .sort_by_key(|row| (row.cpu_perc * 10000.0f32) as u32 - (100 * 10000)),
                SortOrder::Descending => self
                    .rows
                    .sort_by_key(|row| (100 * 10000) - (row.cpu_perc * 10000.0f32) as u32),
            },
            crate::ColumnKind::PID => self.rows.sort_by_key(|row| i32::MAX - row.pid),
            crate::ColumnKind::Command => {}
            crate::ColumnKind::CpuTime => {}
            crate::ColumnKind::Started => {}
            crate::ColumnKind::Index => {}
            crate::ColumnKind::Delete => {}
        };
        self
    }
}

impl From<KillaData> for Vec<crate::Row> {
    fn from(val: KillaData) -> Self {
        val.rows
    }
}

#[derive(Default, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    #[default]
    Descending,
}

#[derive(Clone, Copy)]
pub struct ProcessListSort {
    pub(crate) column: crate::ColumnKind,
    pub(crate) order: SortOrder,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_searchfilters_parse_pid() {
        let case = "pid:123";
        let sf = SearchFilters::best_effort_parse_from_string(case).0;
        assert_eq!(sf.len(), 1);
        assert!(!sf[0].is_negative);
        assert!(matches!(sf[0].column, Ok(SearchFilterColumn::Pid)));
        assert_eq!(sf[0].phrase, "123");
    }

    #[test]
    fn test_searchfilters_parse_negative_name() {
        let case = "-name:killa";
        let sf = SearchFilters::best_effort_parse_from_string(case).0;
        assert_eq!(sf.len(), 1);
        assert!(sf[0].is_negative);
        assert!(matches!(sf[0].column, Ok(SearchFilterColumn::Name)));
        assert_eq!(sf[0].phrase, "killa");
    }

    #[test]
    fn test_searchfilters_parse_many() {
        let case = "cmd:cargo name:rust";
        let sf = SearchFilters::best_effort_parse_from_string(case).0;

        assert_eq!(sf.len(), 2);

        assert!(!sf[0].is_negative);
        assert!(matches!(sf[0].column, Ok(SearchFilterColumn::Command)));
        assert_eq!(sf[0].phrase, "cargo");

        assert!(!sf[1].is_negative);
        assert!(matches!(sf[1].column, Ok(SearchFilterColumn::Name)));
        assert_eq!(sf[1].phrase, "rust");
    }

    #[test]
    fn test_searchfilters_parse_unknown_column() {
        let case = "-unknown:killa";
        let sf = SearchFilters::best_effort_parse_from_string(case).0;
        assert_eq!(sf.len(), 1);
        assert!(sf[0].is_negative);
        assert!(sf[0].column.is_err());
        assert_eq!(sf[0].phrase, "killa");
    }

    #[test]
    fn test_search() {
        let data = KillaData {
            rows: vec![
                crate::Row {
                    program_name: "init".to_string(),
                    program_name_lowercase: "init".to_string(),
                    mem: 100_000,
                    cpu_perc: 0.01,
                    pid: 1,
                    command: "init".to_string(),
                    command_lowercase: "init".to_string(),
                    cpu_time: Duration::from_secs(20),
                },
                crate::Row {
                    program_name: "killa".to_string(),
                    program_name_lowercase: "killa".to_string(),
                    mem: 300_000,
                    cpu_perc: 2.22,
                    pid: 2,
                    command: "/nix/store/xxxxxxxxxxxx-killa".to_string(),
                    command_lowercase: "/nix/store/xxxxxxxxxxxx-killa".to_string(),
                    cpu_time: Duration::from_secs(10),
                },
                crate::Row {
                    program_name: "firefox".to_string(),
                    program_name_lowercase: "firefox".to_string(),
                    mem: 100_000_000,
                    cpu_perc: 10.00,
                    pid: 3,
                    command: "firefox --flag1".to_string(),
                    command_lowercase: "firefox --flag1".to_string(),
                    cpu_time: Duration::from_secs(100),
                },
            ],
            memory: MemHarvest {
                used_bytes: 50,
                total_bytes: 100,
            },
        };
        let s = "-pid:1 name:killa";
        let result = KillaData::search(data.clone(), s);

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].program_name, "killa");
    }
}
