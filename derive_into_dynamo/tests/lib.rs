#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use derive_into_dynamo::IntoDynamoItem;

    type FakeUsize = usize;

    #[derive(IntoDynamoItem, Debug)]
    pub struct SubStruct {
        test: String,
    }

    #[derive(IntoDynamoItem, Debug, Default)]
    pub enum TestEnum {
        Test1,
        #[default]
        Test2,
        Test3,
        TestStruct {
            test: String,
        },
    }

    #[derive(IntoDynamoItem, Debug)]
    pub struct TestStruct {
        #[dynamo(default)]
        string_name: String,
        usize_name: FakeUsize,
        isize_name: isize,
        bool_name: bool,
        vec_string_name: Vec<String>,
        option_name_some: Option<String>,
        option_name_none: Option<Vec<String>>,
        substruct_name: SubStruct,
        string_set_name: HashSet<String>,
        #[dynamo(default)]
        enum_name: TestEnum,
        hash_map_name: HashMap<String, String>,
    }

    #[test]
    fn it_works() {
        let test = TestStruct {
            string_name: "test_value".to_string(),
            usize_name: 25,
            isize_name: -5000,
            bool_name: true,
            vec_string_name: vec!["test_value".to_string(), "test_value2".to_string()],
            option_name_some: Some("x".to_string()),
            option_name_none: None,
            substruct_name: SubStruct {
                test: "substruct_string".to_string(),
            },
            string_set_name: HashSet::from_iter(["test_value".to_string()]),
            enum_name: TestEnum::TestStruct {
                test: "abc".to_string(),
            },
            hash_map_name: HashMap::from_iter([("test_key".to_string(), "test_value".to_string())]),
        };

        let mut item = test.into_item();
        //item.remove("enum_name");

        let test = TestStruct::from_item(item).unwrap();

        panic!("{test:?}");
    }
}
