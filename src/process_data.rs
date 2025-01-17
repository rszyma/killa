#[derive(Clone, Default)]
pub struct KillaData {
    // todo: optimization by keeping a field of ordered ones?
    rows: Vec<crate::Row>,
    // todo: should we keep the original data? I mean this will hold original data, no?
}

impl From<Box<bottom::data_collection::Data>> for KillaData {
    fn from(data: Box<bottom::data_collection::Data>) -> Self {
        let rows = data
            .list_of_processes
            .unwrap_or_default()
            .iter()
            .map(|ps| crate::Row {
                program_name: ps.name.clone(),
                mem: ps.mem_usage_bytes / 1_000_000,
                cpu_perc: (((ps.cpu_usage_percent) / (num_cpus::get() as f32) * 10.0) as i32)
                    as f32
                    / 10.0,
                pid: ps.pid,
                command: ps.command.clone(),
            })
            .collect();
        Self { rows }
    }
}

impl KillaData {
    pub fn search(self, search_phrase: &str) -> Self {
        Self {
            rows: self
                .rows
                .into_iter()
                .filter(|x| {
                    // NOTE: for now this is basic filter, without results ranking.
                    // We might want to implement something better in the future.
                    x.program_name.contains(search_phrase)
                        || x.command.contains(search_phrase)
                        || format!("{}", x.pid).contains(search_phrase)
                })
                .collect(),
        }
    }

    pub fn sort_by_column(&mut self, col: crate::ColumnKind, order: SortOrder) -> &mut Self {
        match col {
            crate::ColumnKind::Name => {}
            crate::ColumnKind::Memory => {}
            crate::ColumnKind::Cpu => match order {
                SortOrder::Ascending => self
                    .rows
                    .sort_by_key(|row| (row.cpu_perc * 10000.0f32) as u32 - (100 * 10000)),
                SortOrder::Descending => self
                    .rows
                    .sort_by_key(|row| (100 * 10000) - (row.cpu_perc * 10000.0f32) as u32),
            },
            crate::ColumnKind::Pid => {}
            crate::ColumnKind::Command => {}
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
