use std::{collections::HashMap, time::Instant};

#[derive(Default)]
pub struct UseCaseRunner {
    pub data_collectors: DataCollections,
    pub use_cases: Vec<Box<dyn UseCase>>,
}

impl UseCaseRunner {
    pub fn new() -> Self {
        // Make something to add datatypes
        let data_type1 = DataType {
            collection: vec![],
            last_updated: None,
            name: "ms_graph_users".into(),
        };

        let mut data_collectors = DataCollections::new();

        data_collectors.insert(data_type1);

        Self {
            data_collectors,
            use_cases: vec![Box::new(UseCaseCis01::new())],
        }
    }

    // fn step(&mut self) {
    //     self.data_collectors
    //         .collections
    //         .iter_mut()
    //         .for_each(|(_name, data)| data.update());

    //     for use_case in self.use_cases.iter_mut() {
    //         let _result = use_case.run(&self.data_collectors);
    //     }
    // }
}

#[derive(Default)]
pub struct DataCollections {
    pub collections: HashMap<String, DataType>,
}

impl DataCollections {
    fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    fn insert(&mut self, data_type: DataType) {
        let _ = self.collections
            .insert(data_type.name.to_string(), data_type);
    }
}

struct UseCaseCis01 {
    data: UseCaseData,
}

pub struct UseCaseData {
    requires_data: Vec<String>,
    name: String,
    last_run_instant: Option<Instant>,
    // Possible Vec?
    last_result: Option<UseCaseResult>,
}

impl UseCaseData {}

impl UseCase for UseCaseCis01 {
    fn new() -> Self {
        Self {
            data: UseCaseData {
                requires_data: vec!["ms_graph_users".into()],
                name: "CIS 01".into(),
                last_run_instant: None,
                last_result: None,
            },
        }
    }

    fn use_case_logic(&self, data_types: &DataCollections) -> UseCaseResult {
        let ms_graph_data = data_types.collections.get("ms_graph_users").expect("Data should exist");

        let result = ms_graph_data
            .collection
            .iter()
            .any(|event| event.contains("foo"));
        UseCaseResult { status: result }
    }

    fn use_case_data(&self) -> &UseCaseData {
        &self.data
    }

    fn use_case_data_mut(&mut self) -> &mut UseCaseData {
        &mut self.data
    }
}

pub trait UseCase {
    fn use_case_data(&self) -> &UseCaseData;
    fn use_case_data_mut(&mut self) -> &mut UseCaseData;

    fn name(&self) -> &str {
        &self.use_case_data().name
    }

    fn new() -> Self
    where
        Self: Sized;

    fn data_requirements(&self) -> &[String] {
        self.use_case_data().requires_data.as_slice()
    }

    fn run(&mut self, data_types: &DataCollections) {
        if !self.is_data_newer_than_most_recent_run(data_types) && self.result().is_some() {
            return;
        }
        let result = self.use_case_logic(data_types);
        self.set_result(result);
        self.set_last_run_instant(Instant::now());
    }

    fn result(&self) -> Option<&UseCaseResult> {
        self.use_case_data().last_result.as_ref()
    }

    fn set_result(&mut self, result: UseCaseResult) {
        self.use_case_data_mut().last_result = Some(result);
    }

    fn use_case_logic(&self, data_types: &DataCollections) -> UseCaseResult;
    fn last_run_instant(&self) -> Option<&Instant> {
        self.use_case_data().last_run_instant.as_ref()
    }

    fn set_last_run_instant(&mut self, instant: Instant) {
        self.use_case_data_mut().last_run_instant = Some(instant);
    }

    fn is_data_newer_than_most_recent_run(&self, data_types: &DataCollections) -> bool {
        let recent_run = if let Some(recent_run) = self.last_run_instant() {
            recent_run
        } else {
            return false
        };
        self
            .data_requirements()
            .iter()
            .map(|data_name| data_types.collections.get(data_name))
            .all(|data_type| {
                let data_type = if let Some(data_type) = data_type {
                    data_type
                } else {
                    return false
                };

                let last_updated = if let Some(last_updated) = data_type.last_updated {
                    last_updated
                } else {
                    return false
                };

                last_updated > *recent_run 
            })
    }
}

#[derive(Debug, Clone)]
pub struct UseCaseResult {
    //non_compliant_records: Vec<impl Serialize>,
    status: bool,
}

impl UseCaseResult {
    pub fn status(&self) -> bool {
        self.status
    }
}

// struct DataTypes {
//     data_types: Vec<DataType>,
// }

// impl DataTypes {
//     // fn get_index(&self, index: DataTypeIndex) -> &DataType {

//     // }

//     fn get_by_name(&self, name: &str) -> Option<&DataType> {
//         self.data_types
//             .iter()
//             .find(|data_type| data_type.name == name)
//     }
// }

pub struct DataType {
    pub last_updated: Option<Instant>,
    pub name: String,
    pub collection: Vec<String>,
}

pub trait DataTypeTrait {
    fn update(&mut self);
}

impl DataTypeTrait for DataType {
    fn update(&mut self) {
        self.collection.push("foo".into());
        self.last_updated = Some(Instant::now());
    }
}
